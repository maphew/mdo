//! Launch: browser openers. Non-blocking, fire-and-forget platform
//! openers plus the xdg-open/gio fallback chain.

use std::io;
use std::path::Path;
use std::time::Duration;
/// Launch the platform's default handler for `path` (typically a web browser
/// for `.html`). Non-blocking — the spawned process runs independently.
pub fn launch_browser(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        // SECURITY: open via the Win32 ShellExecuteW API (through the `opener`
        // crate) rather than `cmd /C start`. `start` runs the path through
        // cmd.exe's command-line parser, so cmd metacharacters (`&`, `^`, …) —
        // which are legal in Windows filenames and reach us via the input
        // file's stem in temp_output_for — would be interpreted as commands
        // (e.g. opening `a&calc&.md` would launch calc.exe). ShellExecuteW
        // takes the path as a single typed argument, so those characters stay
        // inert. It is non-blocking, preserving the fire-and-forget behavior.
        opener::open(path).map_err(io::Error::other)?;
    }
    #[cfg(target_os = "macos")]
    {
        // `open` returns promptly once Launch Services accepts the request,
        // so waiting for it doesn't block on the browser itself — and a
        // nonzero exit (no handler for the file) must surface as a launch
        // failure rather than being discarded with the child handle.
        let status = std::process::Command::new("open").arg(path).status()?;
        if !status.success() {
            return Err(io::Error::other(format!("open exited with {status}")));
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        launch_via_xdg_open(path)?;
    }
    Ok(())
}

/// Open `path` with the first available freedesktop opener. Falls back through
/// common launchers when `xdg-open` (xdg-utils) is not installed, and reaps the
/// launcher process in a detached thread so it does not linger as a zombie
/// during long `--watch` sessions.
///
/// Spawning an opener is not the same as launching a browser: `xdg-open` can
/// exist, start, and then exit nonzero because no browser handler is
/// configured. We therefore listen for each opener's exit for a short window
/// — a quick nonzero exit is an honest failure (and we fall through to the
/// next opener), while an opener still running after the window has almost
/// certainly handed off (or, in xdg-open's no-desktop fallback, IS the
/// browser), so we detach and call it success. We never block on the browser
/// itself beyond that window.
#[cfg(all(unix, not(target_os = "macos")))]
fn launch_via_xdg_open(path: &Path) -> std::io::Result<()> {
    const OPENERS: &[(&str, &[&str])] = &[
        ("xdg-open", &[]),
        ("gio", &["open"]),
        ("gnome-open", &[]),
        ("kde-open5", &[]),
        ("kde-open", &[]),
        ("wslview", &[]),
    ];
    const LAUNCH_FAILURE_WINDOW: Duration = Duration::from_secs(2);

    let mut last_failure: Option<String> = None;
    for (program, leading) in OPENERS {
        let mut command = std::process::Command::new(program);
        command.args(*leading).arg(path);
        let Ok(mut child) = command.spawn() else {
            continue; // not installed; try the next opener
        };

        // The waiting thread doubles as the reaper for the detached-success
        // case: it always waits the child to completion, we just stop
        // listening for the result after the failure window.
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(child.wait());
        });

        match rx.recv_timeout(LAUNCH_FAILURE_WINDOW) {
            Ok(Ok(status)) if status.success() => return Ok(()),
            Ok(Ok(status)) => {
                last_failure = Some(format!("{program} exited with {status}"));
            }
            Ok(Err(e)) => {
                last_failure = Some(format!("failed waiting on {program}: {e}"));
            }
            // Still running after the window: it handed off to (or is) the
            // browser. The waiting thread reaps it eventually.
            Err(_) => return Ok(()),
        }
    }

    match last_failure {
        Some(failure) => Err(io::Error::other(failure)),
        None => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no desktop opener found (install xdg-utils)",
        )),
    }
}
