//! Native first-run setup window for mdo.
//!
//! `mdo` stays the normal CLI. This companion binary gives desktop users a
//! no-terminal onboarding path for installing file-manager integration and
//! opening the welcome sample.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::process::ExitCode;

#[cfg(target_os = "windows")]
fn main() -> ExitCode {
    match windows_setup::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            windows_setup::error_dialog(
                "mdo setup failed",
                "mdo setup could not finish",
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
                "mdo setup could not finish",
                &e.to_string(),
            );
            ExitCode::from(1)
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn main() -> ExitCode {
    eprintln!("mdo-setup is only needed for native Windows and Linux onboarding windows.");
    eprintln!("Run `mdo --tour` for the command-line tour on this platform.");
    ExitCode::from(2)
}

#[cfg(target_os = "linux")]
mod linux_setup {
    use std::io;
    use std::path::PathBuf;
    use std::process::{Command, ExitStatus};

    use mdo_cli::{file_manager, open_tour_sample};

    pub fn run() -> io::Result<()> {
        let dialogs = Dialogs::detect()?;

        dialogs.info(
            "Welcome to mdo",
            "Read Markdown as HTML from your file manager",
            "mdo remains the command-line tool. This setup window can add the reversible Open as HTML action for Markdown files without opening a terminal.",
        )?;

        if dialogs.yes_no(
            "Install Open as HTML?",
            "Add mdo to your Linux file manager?",
            "This per-user install writes an XDG desktop entry and icon. It does not need admin rights and does not change your default Markdown app. You can remove it later with mdo --uninstall-file-manager.",
        )? {
            let mdo_exe = sibling_binary("mdo")?;
            if !mdo_exe.exists() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("expected mdo next to mdo-setup at {}", mdo_exe.display()),
                ));
            }

            file_manager::install_linux_for_exe(&mdo_exe, false)?;
            dialogs.info(
                "Open as HTML installed",
                "File-manager integration is ready",
                "Open With should now offer Open as HTML for .md files. To make it the default, run mdo --install-file-manager --set-default.",
            )?;
        }

        if dialogs.yes_no(
            "Open welcome sample?",
            "Verify the browser-opening flow?",
            "mdo will render a short Markdown sample to a private temp path and open it in your default browser.",
        )? {
            open_tour_sample()?;
        }

        Ok(())
    }

    pub fn error_dialog(title: &str, main_instruction: &str, content: &str) {
        if let Ok(dialogs) = Dialogs::detect() {
            let _ = dialogs.error(title, main_instruction, content);
        } else {
            eprintln!("{title}: {main_instruction}: {content}");
        }
    }

    #[derive(Clone, Copy)]
    enum Dialogs {
        Zenity,
        KDialog,
        Yad,
    }

    impl Dialogs {
        fn detect() -> io::Result<Self> {
            for (program, dialogs) in [
                ("zenity", Dialogs::Zenity),
                ("kdialog", Dialogs::KDialog),
                ("yad", Dialogs::Yad),
            ] {
                if command_on_path(program) {
                    return Ok(dialogs);
                }
            }

            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "could not find zenity, kdialog, or yad; install one of them or run `mdo --tour` in a terminal",
            ))
        }

        fn info(&self, title: &str, main_instruction: &str, content: &str) -> io::Result<()> {
            self.message(MessageKind::Info, title, main_instruction, content)
        }

        fn error(&self, title: &str, main_instruction: &str, content: &str) -> io::Result<()> {
            self.message(MessageKind::Error, title, main_instruction, content)
        }

        fn message(
            &self,
            kind: MessageKind,
            title: &str,
            main_instruction: &str,
            content: &str,
        ) -> io::Result<()> {
            let body = dialog_body(main_instruction, content);
            let mut command = match self {
                Dialogs::Zenity => {
                    let mut command = Command::new("zenity");
                    command
                        .arg(match kind {
                            MessageKind::Info => "--info",
                            MessageKind::Error => "--error",
                        })
                        .arg("--title")
                        .arg(title)
                        .arg("--text")
                        .arg(&body)
                        .arg("--no-wrap");
                    command
                }
                Dialogs::KDialog => {
                    let mut command = Command::new("kdialog");
                    command.arg("--title").arg(title).arg(match kind {
                        MessageKind::Info => "--msgbox",
                        MessageKind::Error => "--error",
                    });
                    command.arg(&body);
                    command
                }
                Dialogs::Yad => {
                    let mut command = Command::new("yad");
                    command
                        .arg(match kind {
                            MessageKind::Info => "--info",
                            MessageKind::Error => "--error",
                        })
                        .arg("--title")
                        .arg(title)
                        .arg("--text")
                        .arg(&body)
                        .arg("--button=OK:0");
                    command
                }
            };

            ensure_success(command.status()?, "dialog")
        }

        fn yes_no(&self, title: &str, main_instruction: &str, content: &str) -> io::Result<bool> {
            let body = dialog_body(main_instruction, content);
            let status = match self {
                Dialogs::Zenity => Command::new("zenity")
                    .arg("--question")
                    .arg("--title")
                    .arg(title)
                    .arg("--text")
                    .arg(&body)
                    .arg("--no-wrap")
                    .status()?,
                Dialogs::KDialog => Command::new("kdialog")
                    .arg("--title")
                    .arg(title)
                    .arg("--yesno")
                    .arg(&body)
                    .status()?,
                Dialogs::Yad => Command::new("yad")
                    .arg("--question")
                    .arg("--title")
                    .arg(title)
                    .arg("--text")
                    .arg(&body)
                    .arg("--button=Yes:0")
                    .arg("--button=No:1")
                    .status()?,
            };

            Ok(status.success())
        }
    }

    enum MessageKind {
        Info,
        Error,
    }

    fn command_on_path(program: &str) -> bool {
        std::env::var_os("PATH")
            .map(|path| std::env::split_paths(&path).any(|dir| dir.join(program).is_file()))
            .unwrap_or(false)
    }

    fn ensure_success(status: ExitStatus, action: &str) -> io::Result<()> {
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "{action} exited with status {status}"
            )))
        }
    }

    fn dialog_body(main_instruction: &str, content: &str) -> String {
        format!("{main_instruction}\n\n{content}")
    }

    fn sibling_binary(name: &str) -> io::Result<PathBuf> {
        let mut path = std::env::current_exe()?;
        path.pop();
        Ok(path.join(name))
    }
}

#[cfg(target_os = "windows")]
mod windows_setup {
    use std::io;
    use std::iter;
    use std::path::PathBuf;
    use std::ptr;

    use mdo_cli::{file_manager, open_tour_sample};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, IDYES, MB_DEFBUTTON1, MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MB_YESNO,
        MESSAGEBOX_STYLE,
    };

    pub fn run() -> io::Result<()> {
        info_dialog(
            "Welcome to mdo",
            "Read Markdown as HTML from Explorer",
            "mdo.exe remains the command-line tool. This native setup window can add the reversible Open as HTML action for Markdown files without opening a terminal.",
        )?;

        if yes_no_dialog(
            "Install Open as HTML?",
            "Add mdo to Windows Explorer?",
            "This per-user install does not need admin rights and does not change your default Markdown app. You can remove it later with mdo --uninstall-file-manager.",
        )? {
            let mdo_exe = sibling_binary("mdo.exe")?;
            if !mdo_exe.exists() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("expected mdo.exe next to mdo-setup.exe at {}", mdo_exe.display()),
                ));
            }

            file_manager::install_windows_for_exe(&mdo_exe, false)?;
            info_dialog(
                "Open as HTML installed",
                "Explorer integration is ready",
                "Right-click any .md file and choose Open as HTML. To make it the default, use Windows' Open with -> Choose another app -> Always flow.",
            )?;
        }

        if yes_no_dialog(
            "Open welcome sample?",
            "Verify the browser-opening flow?",
            "mdo will render a short Markdown sample to a private temp path and open it in your default browser.",
        )? {
            open_tour_sample()?;
        }

        Ok(())
    }

    pub fn error_dialog(title: &str, main_instruction: &str, content: &str) {
        let _ = message_box(
            title,
            &dialog_body(main_instruction, content),
            MB_OK | MB_ICONERROR,
        );
    }

    fn info_dialog(title: &str, main_instruction: &str, content: &str) -> io::Result<()> {
        message_box(
            title,
            &dialog_body(main_instruction, content),
            MB_OK | MB_ICONINFORMATION,
        )?;
        Ok(())
    }

    fn yes_no_dialog(title: &str, main_instruction: &str, content: &str) -> io::Result<bool> {
        let button = message_box(
            title,
            &dialog_body(main_instruction, content),
            MB_YESNO | MB_ICONINFORMATION | MB_DEFBUTTON1,
        )?;
        Ok(button == IDYES)
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

    fn dialog_body(main_instruction: &str, content: &str) -> String {
        format!("{main_instruction}\n\n{content}")
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
