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
//! Instead, `mdo-open[.exe]` is a tiny desktop wrapper that:
//!   1. Locates `mdo[.exe]` next to itself.
//!   2. Spawns it with `--open` and the rest of the args, using
//!      `CREATE_NO_WINDOW` so the child never gets a console allocated.
//!   3. If launched directly on Windows or Linux with no file args, opens
//!      `mdo-setup[.exe]` for native first-run onboarding.
//!   4. Exits immediately. The child runs detached and pops the browser.
//!
//! Net result: registering `mdo-open.exe "%1"` in the Explorer file
//! association gives a flash-free double-click experience without changing
//! how the regular CLI behaves in a terminal.
//!
//! On Linux, no-file launches open the sibling `mdo-setup` desktop onboarding
//! flow. Other non-Windows targets remain a passthrough to the sibling `mdo`
//! binary; nothing about the subsystem flag applies there.

// Mark this binary as windows-subsystem on Windows so Windows itself does not
// allocate a console for it. The attribute is a no-op (and the cfg_attr makes
// it absent) on other platforms.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::env;
use std::process::{Command, ExitCode};

#[cfg(target_os = "windows")]
const MDO_BIN: &str = "mdo.exe";
#[cfg(target_os = "windows")]
const SETUP_BIN: &str = "mdo-setup.exe";

#[cfg(not(target_os = "windows"))]
const MDO_BIN: &str = "mdo";
#[cfg(target_os = "linux")]
const SETUP_BIN: &str = "mdo-setup";

fn main() -> ExitCode {
    let mut exe_path = match env::current_exe() {
        Ok(p) => p,
        Err(_) => return ExitCode::from(1),
    };
    exe_path.pop(); // strip the file name, keep the directory

    let args = env::args_os().skip(1).collect::<Vec<_>>();

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    let mut cmd = if args.is_empty() {
        let setup = exe_path.join(SETUP_BIN);
        if !setup.exists() {
            return ExitCode::from(1);
        }
        Command::new(setup)
    } else {
        let mut cmd = Command::new(exe_path.join(MDO_BIN));
        cmd.arg("--open");
        cmd.args(args);
        cmd
    };

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    let mut cmd = {
        let mut cmd = Command::new(exe_path.join(MDO_BIN));
        cmd.arg("--open");
        cmd.args(args);
        cmd
    };

    // CREATE_NO_WINDOW (0x0800_0000) tells Windows not to give the child a
    // console of its own. Without it, even though *we* have no console, the
    // child (a console-subsystem exe) would briefly get one allocated —
    // exactly the flash we are trying to avoid.
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    match cmd.spawn() {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::from(1),
    }
}
