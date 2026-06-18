//! Native first-run setup launcher for mdo.
//!
//! `mdo` stays the normal CLI. This companion binary gives desktop users a
//! no-terminal entry point into onboarding: it opens the interactive
//! `mdo --tour` (a single screen with one Y/N prompt) in a terminal window so
//! double-clicking from a file manager behaves like running the tour from a
//! shell. It deliberately does not reimplement onboarding as a chain of GUI
//! dialogs.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::process::ExitCode;

#[cfg(target_os = "windows")]
fn main() -> ExitCode {
    match windows_setup::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            windows_setup::error_dialog(
                "mdo setup failed",
                "mdo setup could not start the tour",
                &e.to_string(),
            );
            ExitCode::from(1)
        }
    }
}

#[cfg(target_os = "linux")]
fn main() -> ExitCode {
    match linux_setup::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            linux_setup::error_dialog(
                "mdo setup failed",
                "mdo setup could not start the tour",
                &e.to_string(),
            );
            ExitCode::from(1)
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn main() -> ExitCode {
    eprintln!("mdo-setup is only needed for native Windows and Linux onboarding.");
    eprintln!("Run `mdo --tour` for the command-line tour on this platform.");
    ExitCode::from(2)
}

#[cfg(target_os = "linux")]
mod linux_setup {
    use std::ffi::OsString;
    use std::io::{self, IsTerminal};
    use std::path::{Path, PathBuf};
    use std::process::Command;

    /// Terminal emulators we know how to launch a command in, in preference
    /// order, each paired with the option(s) that introduce the command and
    /// its arguments as a real argv (never a shell string, so paths with
    /// spaces are safe). `tilix` and friends that take `-e "cmd string"` are
    /// intentionally omitted to avoid quoting ambiguity.
    ///
    /// The named terminals come first because their argument semantics are
    /// known and stable. `x-terminal-emulator` is the Debian "alternatives"
    /// indirection: its `-e` is delegated to whatever it points at (often
    /// gnome-terminal, whose legacy `-e` shell-splits and would orphan
    /// `--tour`), so it is a best-effort last resort, tried only after the
    /// terminals we control.
    const TERMINALS: &[(&str, &[&str])] = &[
        ("gnome-terminal", &["--"]),
        ("konsole", &["-e"]),
        ("xfce4-terminal", &["-x"]),
        ("mate-terminal", &["--"]),
        ("kitty", &[]),
        ("alacritty", &["-e"]),
        ("wezterm", &["start", "--"]),
        ("foot", &[]),
        ("xterm", &["-e"]),
        ("x-terminal-emulator", &["-e"]),
    ];

    pub fn run() -> io::Result<()> {
        let mdo = sibling_binary("mdo")?;
        if !mdo.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("expected mdo next to mdo-setup at {}", mdo.display()),
            ));
        }

        // Already attached to a terminal (e.g. run from a shell): show the tour
        // right here instead of spawning a second window. The `?` still
        // propagates a genuine spawn failure (e.g. mdo missing), but a nonzero
        // tour exit is the tour's own business — it surfaces its own errors, so
        // we do not re-report it as "could not start the tour".
        if io::stdin().is_terminal() && io::stdout().is_terminal() {
            Command::new(&mdo).arg("--tour").status()?;
            return Ok(());
        }

        // Launched from a file manager with no terminal: open one and run the
        // tour inside it.
        if spawn_tour_in_terminal(&mdo) {
            return Ok(());
        }

        // No terminal emulator on PATH: point the user at the CLI tour rather
        // than failing invisibly. We report here and return Ok so main() does
        // not pop a second, redundant error dialog.
        error_dialog(
            "mdo setup needs a terminal",
            "Could not find a terminal program to open",
            "Install a terminal emulator (for example gnome-terminal, konsole, or xterm), then run `mdo --tour` to finish onboarding.",
        );
        Ok(())
    }

    fn spawn_tour_in_terminal(mdo: &Path) -> bool {
        for (program, lead) in preferred_terminals() {
            if !command_on_path(program) {
                continue;
            }
            let args = terminal_tour_args(lead, mdo);
            if Command::new(program).args(&args).spawn().is_ok() {
                return true;
            }
        }
        false
    }

    /// `$TERMINAL` wins when it names a terminal we understand; otherwise use
    /// the built-in preference order.
    fn preferred_terminals() -> Vec<(&'static str, &'static [&'static str])> {
        let mut ordered = TERMINALS.to_vec();
        if let Some(value) = std::env::var_os("TERMINAL") {
            if let Some(base) = Path::new(&value).file_name().and_then(|n| n.to_str()) {
                if let Some(pos) = ordered.iter().position(|(program, _)| *program == base) {
                    let chosen = ordered.remove(pos);
                    ordered.insert(0, chosen);
                }
            }
        }
        ordered
    }

    fn terminal_tour_args(lead: &[&str], mdo: &Path) -> Vec<OsString> {
        let mut args: Vec<OsString> = lead.iter().map(OsString::from).collect();
        args.push(mdo.as_os_str().to_os_string());
        args.push(OsString::from("--tour"));
        args
    }

    pub fn error_dialog(title: &str, main_instruction: &str, content: &str) {
        if let Some(dialog) = DialogTool::detect() {
            let _ = dialog.error(title, main_instruction, content);
        } else {
            eprintln!("{title}: {main_instruction}: {content}");
        }
    }

    /// Minimal GUI error reporting for the rare no-terminal case. Onboarding
    /// itself is the terminal tour, not a dialog chain.
    #[derive(Clone, Copy)]
    enum DialogTool {
        Zenity,
        KDialog,
        Yad,
    }

    impl DialogTool {
        fn detect() -> Option<Self> {
            for (program, dialog) in [
                ("zenity", DialogTool::Zenity),
                ("kdialog", DialogTool::KDialog),
                ("yad", DialogTool::Yad),
            ] {
                if command_on_path(program) {
                    return Some(dialog);
                }
            }
            None
        }

        fn error(self, title: &str, main_instruction: &str, content: &str) -> io::Result<()> {
            let body = format!("{main_instruction}\n\n{content}");
            let status = match self {
                DialogTool::Zenity => Command::new("zenity")
                    .arg("--error")
                    .arg("--title")
                    .arg(title)
                    .arg("--text")
                    .arg(&body)
                    .arg("--no-wrap")
                    .status()?,
                DialogTool::KDialog => Command::new("kdialog")
                    .arg("--title")
                    .arg(title)
                    .arg("--error")
                    .arg(&body)
                    .status()?,
                DialogTool::Yad => Command::new("yad")
                    .arg("--error")
                    .arg("--title")
                    .arg(title)
                    .arg("--text")
                    .arg(&body)
                    .arg("--button=OK:0")
                    .status()?,
            };

            if status.success() {
                Ok(())
            } else {
                Err(io::Error::other(format!(
                    "error dialog exited with status {status}"
                )))
            }
        }
    }

    fn command_on_path(program: &str) -> bool {
        std::env::var_os("PATH")
            .map(|path| std::env::split_paths(&path).any(|dir| dir.join(program).is_file()))
            .unwrap_or(false)
    }

    fn sibling_binary(name: &str) -> io::Result<PathBuf> {
        let mut path = std::env::current_exe()?;
        path.pop();
        Ok(path.join(name))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn rendered(lead: &[&str]) -> Vec<String> {
            terminal_tour_args(lead, Path::new("/opt/my tools/mdo"))
                .into_iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect()
        }

        #[test]
        fn dash_e_terminals_pass_command_as_argv() {
            assert_eq!(rendered(&["-e"]), ["-e", "/opt/my tools/mdo", "--tour"]);
        }

        #[test]
        fn double_dash_terminals_pass_command_as_argv() {
            assert_eq!(rendered(&["--"]), ["--", "/opt/my tools/mdo", "--tour"]);
        }

        #[test]
        fn positional_terminals_pass_command_as_argv() {
            assert_eq!(rendered(&[]), ["/opt/my tools/mdo", "--tour"]);
        }

        #[test]
        fn multi_arg_lead_is_preserved_in_order() {
            assert_eq!(
                rendered(&["start", "--"]),
                ["start", "--", "/opt/my tools/mdo", "--tour"]
            );
        }

        #[test]
        fn terminal_table_only_lists_argv_safe_invocations() {
            // Every entry must launch `mdo --tour` as a real argv. Bare `-e`
            // that takes a single shell string (e.g. tilix) must not creep in.
            for (program, lead) in TERMINALS {
                let args = rendered(lead);
                assert!(
                    args.ends_with(&["/opt/my tools/mdo".to_string(), "--tour".to_string()]),
                    "{program} must forward the mdo argv unchanged"
                );
            }
        }
    }
}

#[cfg(target_os = "windows")]
mod windows_setup {
    use std::io;
    use std::iter;
    use std::path::PathBuf;
    use std::ptr;

    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_ICONERROR, MB_OK, MESSAGEBOX_STYLE,
    };

    pub fn run() -> io::Result<()> {
        let mdo = sibling_binary("mdo.exe")?;
        if !mdo.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "expected mdo.exe next to mdo-setup.exe at {}",
                    mdo.display()
                ),
            ));
        }

        // Match the no-file `mdo-open.exe` onboarding path: prefer Windows
        // Terminal for a styled, centered tour, then fall back to a plain new
        // console if `wt` cannot be started.
        mdo_cli::windows_tour::spawn_terminal_tour(&mdo)
    }

    pub fn error_dialog(title: &str, main_instruction: &str, content: &str) {
        let _ = message_box(
            title,
            &format!("{main_instruction}\n\n{content}"),
            MB_OK | MB_ICONERROR,
        );
    }

    fn message_box(title: &str, text: &str, style: MESSAGEBOX_STYLE) -> io::Result<i32> {
        let title = wide(title);
        let text = wide(text);
        let button = unsafe { MessageBoxW(ptr::null_mut(), text.as_ptr(), title.as_ptr(), style) };

        if button == 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(button)
    }

    fn sibling_binary(name: &str) -> io::Result<PathBuf> {
        let mut path = std::env::current_exe()?;
        path.pop();
        Ok(path.join(name))
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(iter::once(0)).collect()
    }
}
