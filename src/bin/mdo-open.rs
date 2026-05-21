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
//! Instead, `mdo-open.exe` is a tiny windows-subsystem wrapper that:
//!   1. Locates `mdo[.exe]` next to itself.
//!   2. Spawns it with `--open` and the rest of the args, using
//!      `CREATE_NO_WINDOW` so the child never gets a console allocated.
//!   3. Exits immediately. The child runs detached and pops the browser.
//!
//! Net result: registering `mdo-open.exe "%1"` in the Explorer file
//! association gives a flash-free double-click experience without changing
//! how the regular CLI behaves in a terminal.
//!
//! On non-Windows targets this is just a passthrough that calls the sibling
//! `mdo` binary; nothing about the subsystem flag applies there.

// Mark this binary as windows-subsystem on Windows so Windows itself does not
// allocate a console for it. The attribute is a no-op (and the cfg_attr makes
// it absent) on other platforms.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::env;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut exe_path = match env::current_exe() {
        Ok(p) => p,
        Err(_) => return ExitCode::from(1),
    };
    exe_path.pop(); // strip the file name, keep the directory

    let target_name = if cfg!(target_os = "windows") {
        "mdo.exe"
    } else {
        "mdo"
    };
    let target = exe_path.join(target_name);

    let mut cmd = Command::new(&target);
    cmd.arg("--open");
    cmd.args(env::args_os().skip(1));

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
