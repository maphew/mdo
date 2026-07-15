//! Integration tests for the `mdo-open` desktop wrapper.
//!
//! `mdo-open` exists to avoid a console flash when Explorer (Windows) or a
//! file manager (Linux) double-click-launches Markdown files; see its module
//! docs in `src/bin/mdo-open.rs`. It now waits on the `mdo`/`mdo --open`
//! child and mirrors its exit status, so a render failure or a failed
//! browser launch should surface as a non-zero exit from `mdo-open` too
//! instead of always reporting success.
//!
//! SAFETY: none of these tests may open a real browser window on the
//! machine running them (developer box or CI runner). Every test here
//! either points at a nonexistent input (so `mdo` fails during render,
//! before it ever attempts to launch a browser) or, on Unix, points `PATH`
//! at an empty directory so every opener `launch_browser` tries fails to
//! spawn (see `empty_path_dir` below and the equivalent trick in
//! `tests/cli.rs`). Do not add a Windows success-path test here: Windows
//! launches via `ShellExecuteW` (through the `opener` crate), which is not
//! `PATH`-based, so there is no safe, reliable way to force it to fail
//! without risking a real launch on a misconfigured runner. Windows
//! coverage of the browser-launch-failure *contract* rides on the Linux
//! empty-PATH test below (`mdo-open` forwards to the same `mdo::launch_browser`
//! code path on every platform); Windows-specific coverage here is limited
//! to render-failure propagation, which is safe to exercise on every OS.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "mdo-wrapper-test-{name}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("failed to create temp fixture dir");
    dir
}

/// Forwarding + error propagation via a missing input file: `mdo-open`
/// forwards its args (plus `--open`) to the sibling `mdo`/`mdo.exe` binary,
/// which fails during render (before ever reaching `launch_browser`) because
/// the file does not exist. This is safe on every platform since no browser
/// launch is ever attempted.
#[test]
fn mdo_open_propagates_render_failure_for_missing_file() {
    let dir = fixture_dir("missing-file");
    let missing_input = dir.join("does-not-exist.md");

    let output = Command::new(env!("CARGO_BIN_EXE_mdo-open"))
        .arg(&missing_input)
        .output()
        .expect("failed to run mdo-open");

    assert!(
        !output.status.success(),
        "mdo-open should exit non-zero when the wrapped render fails: {output:?}"
    );

    fs::remove_dir_all(dir).expect("failed to clean up temp fixture dir");
}

/// An empty directory suitable for use as `PATH` when a test needs to make
/// sure no desktop opener (xdg-open, gio, ...) can be found on it. Mirrors
/// `empty_path_dir` in `tests/cli.rs`; duplicated here because integration
/// test binaries do not share a module tree.
#[cfg(target_os = "linux")]
fn empty_path_dir(dir: &std::path::Path) -> PathBuf {
    let empty = dir.join("empty-path");
    fs::create_dir_all(&empty).expect("failed to create empty PATH dir");
    empty
}

/// Forwarding + error propagation through a successful render: with `PATH`
/// pointed at an empty directory, `launch_browser` cannot find any opener,
/// so the wrapped `mdo --open` renders successfully but fails to launch a
/// browser and exits non-zero. `mdo-open` must forward the file argument
/// (proven by the render actually happening) and propagate that non-zero
/// exit rather than reporting success once the child is spawned.
///
/// Linux/Unix-only: `launch_browser` there shells out to openers looked up
/// on `PATH` (xdg-open, gio, ...), so the empty-PATH trick reliably forces a
/// failure without spawning anything. Windows launches via `ShellExecuteW`,
/// which does not consult `PATH`, so this trick does not apply there — see
/// the module-level doc comment for how Windows coverage is scoped instead.
#[cfg(target_os = "linux")]
#[test]
fn mdo_open_forwards_file_arg_and_propagates_browser_launch_failure() {
    let dir = fixture_dir("open-launch-failure");
    let input = dir.join("sample.md");
    fs::write(&input, "# Sample\n\nSome text.\n").expect("failed to write markdown fixture");

    let empty_path = empty_path_dir(&dir);

    let output = Command::new(env!("CARGO_BIN_EXE_mdo-open"))
        .arg(&input)
        .env("PATH", &empty_path)
        .output()
        .expect("failed to run mdo-open");

    assert!(
        !output.status.success(),
        "mdo-open should exit non-zero when the wrapped browser launch fails: {output:?}"
    );

    // The render itself must have happened (proving the file arg reached
    // the wrapped `mdo --open <file>` invocation) even though the overall
    // exit is a failure.
    let rendered_path =
        mdo_cli::temp_output_for(&input).expect("failed to compute expected temp output path");
    assert!(
        rendered_path.exists(),
        "expected mdo-open to have forwarded the file arg and rendered {rendered_path:?}"
    );

    fs::remove_dir_all(dir).expect("failed to clean up temp fixture dir");
}
