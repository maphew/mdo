//! # mdo-open
//!
//! GUI-subsystem launcher for [`mdo`] intended for use as the Windows
//! Explorer "Open with" / right-click handler for `.md` files.
//!
//! `mdo.exe` is a normal *console* subsystem binary. When Explorer
//! launches a console binary, Windows allocates a console window for it
//! before the process even starts — the user sees a black flash. Marking
//! `mdo.exe` itself as a windows-subsystem app would suppress the flash
//! but break terminal usage (no stdout to the parent shell, output races the
//! returning prompt under cmd/PowerShell).
//!
//! Instead, `mdo-open[.exe]` is a tiny desktop wrapper that consumes the
//! shared handler invocation contract (`mdo_cli::handler`):
//!   1. Locates `mdo[.exe]` next to itself (the one sibling-binary
//!      discovery function, so a missing sibling gets one clear message).
//!   2. Spawns it with the contract argv (`--open` plus the file args),
//!      using `CREATE_NO_WINDOW` so the child never gets a console
//!      allocated.
//!   3. If launched directly with no file args, opens onboarding: Windows
//!      starts the terminal setup in Windows Terminal when available; Linux
//!      opens `mdo-setup` for native first-run onboarding. This step is
//!      fire-and-forget (there is nothing meaningful to wait on: the
//!      interactive setup runs in a separately-launched window).
//!   4. For the `--open` case, waits for the `mdo`/`mdo-open` child to exit
//!      and mirrors its exit code. `CREATE_NO_WINDOW` (Windows) means the
//!      child never gets a console regardless of whether we wait on it, so
//!      waiting does not reintroduce the flash; it just lets a real render
//!      or browser-launch failure surface as a non-zero exit from
//!      `mdo-open` instead of always reporting success. The wait is brief —
//!      the child only has to render and hand off to the browser launcher.
//!
//! Net result: registering `mdo-open.exe "%1"` in the Explorer file
//! association gives a flash-free double-click experience without changing
//! how the regular CLI behaves in a terminal.
//!
//! On Windows, no-file launches open the terminal setup in a fresh `wt` window
//! with the One Half Light color scheme and then centers that window on the
//! active display. If `wt` cannot be started, mdo falls back to a plain new
//! console. On Linux, no-file launches open the sibling `mdo-setup` desktop
//! onboarding flow. Other non-Windows targets remain a passthrough to the
//! sibling `mdo` binary; nothing about the subsystem flag applies there.

// Mark this binary as windows-subsystem on Windows so Windows itself does not
// allocate a console for it. The attribute is a no-op (and the cfg_attr makes
// it absent) on other platforms.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::env;
use std::process::{Command, ExitCode};

#[cfg(target_os = "windows")]
use std::io;

use mdo_cli::handler;

#[cfg(target_os = "windows")]
const MDO_BIN: &str = "mdo.exe";

#[cfg(not(target_os = "windows"))]
const MDO_BIN: &str = "mdo";
#[cfg(target_os = "linux")]
const SETUP_BIN: &str = "mdo-setup";

fn main() -> ExitCode {
    // A GUI-launched binary often has no console attached, so this message
    // may still go nowhere on Windows — but silent failure is worse
    // everywhere, and on Linux it reaches the file manager's stderr log.
    let exe_path = match env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("mdo-open: cannot locate its own executable: {e}");
            return ExitCode::from(1);
        }
    };
    let args = env::args_os().skip(1).collect::<Vec<_>>();

    #[cfg(target_os = "windows")]
    {
        // Onboarding has no exit status worth propagating: it fires off a
        // detached `wt` (or a new console as a fallback) and returns as soon
        // as that spawn succeeds, well before the interactive setup inside
        // it runs. There is nothing to wait on there.
        if args.is_empty() {
            return match spawn_windows_onboarding(&exe_path) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("mdo-open: {e}");
                    ExitCode::from(1)
                }
            };
        }

        // The `mdo --open` child is CREATE_NO_WINDOW either way, so waiting
        // on it does not reintroduce the console flash this binary exists to
        // avoid — no console is ever allocated for it, waited-on or not. It
        // also exits quickly (render + browser launch), so this keeps
        // `mdo-open.exe` responsive while letting a real failure (bad
        // Markdown path, no browser handler, ...) surface as a non-zero exit
        // instead of always reporting success.
        match spawn_windows_open(&exe_path, args) {
            Ok(status) => exit_code_from_status(status),
            Err(e) => {
                eprintln!("mdo-open: {e}");
                ExitCode::from(1)
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        #[cfg(target_os = "linux")]
        if args.is_empty() {
            // No-file launch is onboarding, not open: hand off to the
            // sibling `mdo-setup`.
            let setup = match handler::sibling_binary(&exe_path, SETUP_BIN) {
                Ok(path) => path,
                Err(e) => {
                    eprintln!("mdo-open: {e}");
                    return ExitCode::from(1);
                }
            };
            let mut cmd = Command::new(setup);
            match cmd.status() {
                Ok(status) => return exit_code_from_status(status),
                Err(e) => {
                    eprintln!("mdo-open: {e}");
                    return ExitCode::from(1);
                }
            }
        }

        // Everything else is the forward: sibling mdo with the contract
        // argv (`--open` plus the files as received).
        let mdo_bin = match handler::sibling_binary(&exe_path, MDO_BIN) {
            Ok(path) => path,
            Err(e) => {
                eprintln!("mdo-open: {e}");
                return ExitCode::from(1);
            }
        };
        let mut cmd = Command::new(mdo_bin);
        cmd.args(handler::open_argv(&args));

        // Non-Windows binaries never had a console-flash problem to begin
        // with (that's Windows Explorer + console-subsystem-exe behavior),
        // so there's no reason to stay fire-and-forget here: wait for the
        // child (either `mdo --open`, which exits quickly, or `mdo-setup`,
        // which itself only blocks when it decides to run onboarding inline
        // in an already-attached terminal) and propagate its exit status.
        match cmd.status() {
            Ok(status) => exit_code_from_status(status),
            Err(e) => {
                eprintln!("mdo-open: {e}");
                ExitCode::from(1)
            }
        }
    }
}

/// Map a child's exit status onto an `ExitCode`. Signals/unknown termination
/// (no exit code available) fall back to a generic failure code rather than
/// panicking or silently reporting success. Windows exit codes are 32-bit, so
/// a failure whose low byte is zero (e.g. 0x100) must not truncate to
/// "success"; any failing status maps to a non-zero code.
fn exit_code_from_status(status: std::process::ExitStatus) -> ExitCode {
    exit_code_from_parts(status.success(), status.code())
}

/// The truncation rule of [`exit_code_from_status`], split from
/// `ExitStatus` so it is unit-testable portably (Unix cannot even produce a
/// failed status with a zero low byte; Windows can).
fn exit_code_from_parts(success: bool, code: Option<i32>) -> ExitCode {
    if success {
        return ExitCode::SUCCESS;
    }
    match code {
        Some(code) if code as u8 != 0 => ExitCode::from(code as u8),
        _ => ExitCode::FAILURE,
    }
}

#[cfg(target_os = "windows")]
fn spawn_windows_open(
    anchor_exe: &std::path::Path,
    args: Vec<std::ffi::OsString>,
) -> io::Result<std::process::ExitStatus> {
    let mdo_bin = handler::sibling_binary(anchor_exe, MDO_BIN)?;
    let mut cmd = Command::new(mdo_bin);
    cmd.args(handler::open_argv(&args));
    spawn_without_console(cmd)
}

#[cfg(target_os = "windows")]
fn spawn_windows_onboarding(anchor_exe: &std::path::Path) -> io::Result<()> {
    let mdo = handler::sibling_binary(anchor_exe, MDO_BIN)?;
    mdo_cli::windows_setup::spawn_terminal_setup(&mdo)
}

#[cfg(target_os = "windows")]
fn spawn_without_console(mut cmd: Command) -> io::Result<std::process::ExitStatus> {
    use std::os::windows::process::CommandExt;

    // CREATE_NO_WINDOW (0x0800_0000) tells Windows not to give the child a
    // console of its own. Without it, even though *we* have no console, the
    // child (a console-subsystem exe) would briefly get one allocated —
    // exactly the flash we are trying to avoid. This holds whether we wait
    // on the child afterwards (as we now do, to propagate its exit status)
    // or not: the flag controls console allocation at spawn time, not
    // whether the parent waits.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
    let mut child = cmd.spawn()?;
    child.wait()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_success_is_success() {
        assert_eq!(exit_code_from_parts(true, Some(0)), ExitCode::SUCCESS);
        assert_eq!(exit_code_from_parts(true, None), ExitCode::SUCCESS);
    }

    #[test]
    fn small_exit_codes_pass_through() {
        assert_eq!(exit_code_from_parts(false, Some(1)), ExitCode::from(1));
        assert_eq!(exit_code_from_parts(false, Some(7)), ExitCode::from(7));
        assert_eq!(exit_code_from_parts(false, Some(255)), ExitCode::from(255));
    }

    #[test]
    fn failure_with_zero_low_byte_must_not_read_as_success() {
        // The truncation trap: 0x100 has a zero low byte, but the child
        // FAILED. Truncating to `ExitCode::from(0)` would report success.
        assert_eq!(exit_code_from_parts(false, Some(0x100)), ExitCode::FAILURE);
        assert_eq!(exit_code_from_parts(false, Some(0x200)), ExitCode::FAILURE);
        assert_eq!(exit_code_from_parts(false, Some(0xFF00)), ExitCode::FAILURE);
    }

    #[test]
    fn unknown_termination_falls_back_to_failure() {
        // Signal deaths (and nested nested process terminations) offer no
        // exit code at all: a generic failure keeps mdo-open honest.
        assert_eq!(exit_code_from_parts(false, None), ExitCode::FAILURE);
    }

    // Windows-only end-to-end of the truncation rule: only Windows can
    // actually produce a failed child whose exit code has a zero low byte.
    #[cfg(target_os = "windows")]
    #[test]
    fn real_windows_exit_code_256_means_failure() {
        let status = Command::new("cmd.exe")
            .args(["/C", "exit", "256"])
            .status()
            .expect("cmd.exe should be spawnable on Windows");
        assert!(!status.success());
        assert_eq!(status.code(), Some(256));
        assert_eq!(exit_code_from_status(status), ExitCode::FAILURE);
    }
}
