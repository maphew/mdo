//! Native Windows first-run setup window for mdo.
//!
//! `mdo.exe` stays a console-subsystem CLI. This companion binary is a
//! windows-subsystem app so Explorer users can install the file-manager
//! integration and open the welcome sample without seeing a terminal window.

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

#[cfg(not(target_os = "windows"))]
fn main() -> ExitCode {
    eprintln!("mdo-setup is only needed for the native Windows onboarding window.");
    eprintln!("Run `mdo --tour` for the command-line tour on this platform.");
    ExitCode::from(2)
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
