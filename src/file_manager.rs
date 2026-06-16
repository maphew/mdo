use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(target_os = "windows")]
use std::process::Stdio;

const APP_DISPLAY_NAME: &str = "Open as HTML";
#[cfg(target_os = "linux")]
const DESKTOP_FILE_NAME: &str = "mdo.desktop";
#[cfg(target_os = "windows")]
const WINDOWS_ICON_BYTES: &[u8] = include_bytes!("../assets/mdo.ico");
#[cfg(target_os = "linux")]
const SVG_ICON: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256">
  <rect width="256" height="256" rx="48" fill="#f7f7f7"/>
  <text x="128" y="151"
        text-anchor="middle"
        dominant-baseline="middle"
        font-family="Segoe UI Symbol, Noto Sans Symbols 2, Noto Sans Symbols, DejaVu Sans, sans-serif"
        font-size="176"
        font-weight="700"
        fill="#1e66e2">Ⓜ</text>
</svg>
"##;

#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::fs;

pub fn install(set_default: bool) -> io::Result<()> {
    install_impl(set_default)
}

pub fn uninstall() -> io::Result<()> {
    uninstall_impl()
}

#[cfg(target_os = "linux")]
fn install_impl(set_default: bool) -> io::Result<()> {
    let exe = std::env::current_exe()?;
    let data_home = xdg_data_home()?;
    let desktop_dir = data_home.join("applications");
    let desktop_file = desktop_dir.join(DESKTOP_FILE_NAME);
    let icon_root = data_home.join("icons").join("hicolor");
    let icon_dir = icon_root.join("scalable").join("apps");
    let icon_file = icon_dir.join("mdo.svg");

    fs::create_dir_all(&desktop_dir)?;
    fs::create_dir_all(&icon_dir)?;
    fs::write(&icon_file, SVG_ICON)?;

    let desktop_entry = linux_desktop_entry(&exe);
    fs::write(&desktop_file, desktop_entry)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&desktop_file, fs::Permissions::from_mode(0o644))?;
    }

    run_optional("update-desktop-database", &[desktop_dir.as_os_str()]);
    run_optional(
        "gtk-update-icon-cache",
        &[std::ffi::OsStr::new("-q"), icon_root.as_os_str()],
    );

    if set_default {
        run_optional(
            "xdg-mime",
            &[
                std::ffi::OsStr::new("default"),
                std::ffi::OsStr::new(DESKTOP_FILE_NAME),
                std::ffi::OsStr::new("text/markdown"),
            ],
        );
        run_optional(
            "xdg-mime",
            &[
                std::ffi::OsStr::new("default"),
                std::ffi::OsStr::new(DESKTOP_FILE_NAME),
                std::ffi::OsStr::new("text/x-markdown"),
            ],
        );
    }

    remove_legacy_nautilus_scripts(&data_home)?;

    println!("Installed desktop entry: {}", desktop_file.display());
    println!("Installed icon: {}", icon_file.display());
    if set_default {
        println!("{APP_DISPLAY_NAME} is now the default Markdown handler.");
    } else {
        println!(
            "{APP_DISPLAY_NAME} is available from Open With without changing your default Markdown handler."
        );
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn uninstall_impl() -> io::Result<()> {
    let data_home = xdg_data_home()?;
    let desktop_dir = data_home.join("applications");
    let desktop_file = desktop_dir.join(DESKTOP_FILE_NAME);
    let icon_root = data_home.join("icons").join("hicolor");
    let icon_file = icon_root.join("scalable").join("apps").join("mdo.svg");

    remove_file_if_present(&desktop_file)?;
    remove_file_if_present(&icon_file)?;
    remove_legacy_nautilus_scripts(&data_home)?;

    for mimeapps in mimeapps_paths(&data_home) {
        remove_desktop_from_mimeapps(&mimeapps)?;
    }

    run_optional("update-desktop-database", &[desktop_dir.as_os_str()]);
    run_optional(
        "gtk-update-icon-cache",
        &[std::ffi::OsStr::new("-q"), icon_root.as_os_str()],
    );

    println!("Done.");
    Ok(())
}

#[cfg(target_os = "windows")]
fn install_impl(_set_default: bool) -> io::Result<()> {
    let current_exe = std::env::current_exe()?;
    let handler = windows_handler_for(&current_exe);
    let command = windows_registry_command(&handler.path, handler.is_wrapper);
    let icon_file = install_windows_icon()?;
    let icon_ref = format!("\"{}\"", icon_file.display());
    let current_exe_command = windows_registry_command(&current_exe, false);

    register_windows_application("mdo.exe", &current_exe_command, &icon_ref)?;
    if handler.is_wrapper {
        register_windows_application("mdo-open.exe", &command, &icon_ref)?;
    }

    reg_add_value(r"HKCU\Software\Classes\.md\OpenWithProgids", "mdo.md", "")?;

    reg_add_default(
        r"HKCU\Software\Classes\mdo.md",
        "Markdown document (Open as HTML)",
    )?;
    reg_add_default(r"HKCU\Software\Classes\mdo.md\shell\open\command", &command)?;
    reg_add_default(r"HKCU\Software\Classes\mdo.md\DefaultIcon", &icon_ref)?;

    for old_verb in [
        r"HKCU\Software\Classes\SystemFileAssociations\.md\shell\Preview with mdo",
        r"HKCU\Software\Classes\SystemFileAssociations\.md\shell\Render with mdo",
    ] {
        let _ = reg_delete_key(old_verb);
    }

    let verb = r"HKCU\Software\Classes\SystemFileAssociations\.md\shell\Open as HTML";
    reg_add_default(verb, APP_DISPLAY_NAME)?;
    reg_add_default(&format!(r"{verb}\command"), &command)?;
    reg_add_value(verb, "Icon", &icon_ref)?;

    println!("Using handler: {}", handler.path.display());
    println!("Using icon: {}", icon_file.display());
    if !handler.is_wrapper {
        println!(
            "Note: mdo-open.exe was not found next to mdo.exe, so Explorer will launch mdo.exe directly."
        );
    }
    println!("{APP_DISPLAY_NAME} is registered for Markdown files.");
    println!("To make it the default, use Open with -> Choose another app -> {APP_DISPLAY_NAME} -> Always.");

    Ok(())
}

#[cfg(target_os = "windows")]
fn uninstall_impl() -> io::Result<()> {
    for key in [
        r"HKCU\Software\Classes\Applications\mdo.exe",
        r"HKCU\Software\Classes\Applications\mdo-open.exe",
        r"HKCU\Software\Classes\mdo.md",
        r"HKCU\Software\Classes\SystemFileAssociations\.md\shell\Open as HTML",
        r"HKCU\Software\Classes\SystemFileAssociations\.md\shell\Preview with mdo",
        r"HKCU\Software\Classes\SystemFileAssociations\.md\shell\Render with mdo",
        r"HKCU\Software\Classes\Applications\md2htmlx.exe",
        r"HKCU\Software\Classes\md2htmlx.md",
        r"HKCU\Software\Classes\SystemFileAssociations\.md\shell\Render with md2htmlx",
    ] {
        let _ = reg_delete_key(key);
    }

    let _ = reg_delete_value(r"HKCU\Software\Classes\.md\OpenWithProgids", "mdo.md");
    let _ = reg_delete_value(r"HKCU\Software\Classes\.md\OpenWithProgids", "md2htmlx.md");
    let _ = remove_windows_icon();

    println!("Done.");
    println!(
        "If {APP_DISPLAY_NAME} was set as the default handler for .md, Windows will prompt you to pick a new default the next time you open a .md file."
    );
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn install_impl(_set_default: bool) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "built-in file-manager installation is currently supported on Windows and Linux",
    ))
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn uninstall_impl() -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "built-in file-manager uninstallation is currently supported on Windows and Linux",
    ))
}

#[cfg(target_os = "linux")]
fn xdg_data_home() -> io::Result<PathBuf> {
    if let Some(value) = std::env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(value));
    }

    let home = std::env::var_os("HOME").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "HOME is not set; cannot locate ~/.local/share",
        )
    })?;
    Ok(PathBuf::from(home).join(".local").join("share"))
}

#[cfg(target_os = "linux")]
fn linux_desktop_entry(exe: &Path) -> String {
    let quoted_exe = quote_desktop_exec_arg(&exe.to_string_lossy());
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name={APP_DISPLAY_NAME}\n\
         GenericName=Markdown HTML Opener\n\
         Comment=Open Markdown as HTML in the default browser\n\
         Exec={quoted_exe} --open %f\n\
         Icon=mdo\n\
         Terminal=false\n\
         NoDisplay=true\n\
         MimeType=text/markdown;text/x-markdown;\n\
         Categories=Utility;TextTools;\n\
         StartupNotify=false\n"
    )
}

#[cfg(target_os = "linux")]
fn quote_desktop_exec_arg(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for ch in value.chars() {
        match ch {
            '\\' | '"' | '$' | '`' => {
                quoted.push('\\');
                quoted.push(ch);
            }
            _ => quoted.push(ch),
        }
    }
    quoted.push('"');
    quoted
}

#[cfg(target_os = "linux")]
fn run_optional(program: &str, args: &[&std::ffi::OsStr]) {
    let _ = Command::new(program).args(args).status();
}

#[cfg(target_os = "linux")]
fn remove_legacy_nautilus_scripts(data_home: &Path) -> io::Result<()> {
    let nautilus_dir = data_home.join("nautilus").join("scripts");
    for name in ["Open as HTML", "Preview with mdo", "Render with mdo"] {
        remove_file_if_present(&nautilus_dir.join(name))?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn remove_file_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => {
            println!("Removed: {}", path.display());
            Ok(())
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            println!("Skip   : {} (not present)", path.display());
            Ok(())
        }
        Err(e) => Err(e),
    }
}

#[cfg(target_os = "linux")]
fn mimeapps_paths(data_home: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        paths.push(PathBuf::from(config_home).join("mimeapps.list"));
    } else if let Some(home) = std::env::var_os("HOME") {
        paths.push(PathBuf::from(home).join(".config").join("mimeapps.list"));
    }
    paths.push(data_home.join("applications").join("mimeapps.list"));
    paths
}

#[cfg(target_os = "linux")]
fn remove_desktop_from_mimeapps(path: &Path) -> io::Result<()> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };

    let updated = strip_mdo_from_mimeapps(&contents);
    if updated != contents {
        fs::write(path, updated)?;
        println!("Updated: {}", path.display());
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn strip_mdo_from_mimeapps(contents: &str) -> String {
    let mut section = "";
    let mut out = Vec::new();

    for line in contents.lines() {
        match line {
            "[Default Applications]" => {
                section = "default";
                out.push(line.to_string());
                continue;
            }
            "[Added Associations]" => {
                section = "added";
                out.push(line.to_string());
                continue;
            }
            _ if line.starts_with('[') => {
                section = "";
                out.push(line.to_string());
                continue;
            }
            _ => {}
        }

        if section == "default"
            && (line == "text/markdown=mdo.desktop" || line == "text/x-markdown=mdo.desktop")
        {
            continue;
        }

        if section == "added"
            && (line.starts_with("text/markdown=") || line.starts_with("text/x-markdown="))
        {
            if let Some((mime, apps)) = line.split_once('=') {
                let apps = apps
                    .split(';')
                    .filter(|app| !app.is_empty() && *app != DESKTOP_FILE_NAME)
                    .collect::<Vec<_>>();
                if apps.is_empty() {
                    continue;
                }
                out.push(format!("{mime}={};", apps.join(";")));
                continue;
            }
        }

        out.push(line.to_string());
    }

    let mut result = out.join("\n");
    if contents.ends_with('\n') {
        result.push('\n');
    }
    result
}

#[cfg(target_os = "windows")]
struct WindowsHandler {
    path: PathBuf,
    is_wrapper: bool,
}

#[cfg(target_os = "windows")]
fn windows_handler_for(current_exe: &Path) -> WindowsHandler {
    let sibling_wrapper = current_exe.with_file_name("mdo-open.exe");
    if sibling_wrapper.exists() {
        WindowsHandler {
            path: sibling_wrapper,
            is_wrapper: true,
        }
    } else {
        WindowsHandler {
            path: current_exe.to_path_buf(),
            is_wrapper: false,
        }
    }
}

#[cfg(target_os = "windows")]
fn windows_registry_command(handler: &Path, is_wrapper: bool) -> String {
    if is_wrapper {
        format!("\"{}\" \"%1\"", handler.display())
    } else {
        format!("\"{}\" --open \"%1\"", handler.display())
    }
}

#[cfg(target_os = "windows")]
fn register_windows_application(exe_name: &str, command: &str, icon_ref: &str) -> io::Result<()> {
    let key = format!(r"HKCU\Software\Classes\Applications\{exe_name}");

    reg_add_value(&key, "FriendlyAppName", APP_DISPLAY_NAME)?;
    reg_add_value(&key, "ApplicationName", APP_DISPLAY_NAME)?;
    reg_add_default(&format!(r"{key}\DefaultIcon"), icon_ref)?;
    reg_add_default(&format!(r"{key}\shell\open\command"), command)?;
    reg_add_value(&format!(r"{key}\SupportedTypes"), ".md", "")?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn install_windows_icon() -> io::Result<PathBuf> {
    let icon_file = windows_icon_file()?;
    if let Some(parent) = icon_file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&icon_file, WINDOWS_ICON_BYTES)?;
    Ok(icon_file)
}

#[cfg(target_os = "windows")]
fn remove_windows_icon() -> io::Result<()> {
    let icon_file = match windows_icon_file() {
        Ok(path) => path,
        Err(_) => return Ok(()),
    };

    match fs::remove_file(&icon_file) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }

    if let Some(parent) = icon_file.parent() {
        match fs::remove_dir(parent) {
            Ok(()) => {}
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::DirectoryNotEmpty
                ) => {}
            Err(e) => return Err(e),
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_icon_file() -> io::Result<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("APPDATA"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "LOCALAPPDATA and APPDATA are not set; cannot install icon",
            )
        })?;

    Ok(PathBuf::from(base).join("mdo").join("mdo.ico"))
}

#[cfg(target_os = "windows")]
fn reg_add_default(key: &str, value: &str) -> io::Result<()> {
    reg_status(
        Command::new("reg").args(["add", key, "/ve", "/d", value, "/f"]),
        "reg add",
    )
}

#[cfg(target_os = "windows")]
fn reg_add_value(key: &str, name: &str, value: &str) -> io::Result<()> {
    reg_status(
        Command::new("reg").args(["add", key, "/v", name, "/t", "REG_SZ", "/d", value, "/f"]),
        "reg add",
    )
}

#[cfg(target_os = "windows")]
fn reg_delete_key(key: &str) -> io::Result<()> {
    reg_status(
        Command::new("reg").args(["delete", key, "/f"]),
        "reg delete",
    )
}

#[cfg(target_os = "windows")]
fn reg_delete_value(key: &str, name: &str) -> io::Result<()> {
    reg_status(
        Command::new("reg").args(["delete", key, "/v", name, "/f"]),
        "reg delete",
    )
}

#[cfg(target_os = "windows")]
fn reg_status(command: &mut Command, action: &str) -> io::Result<()> {
    let output = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let details = stderr.trim();
        let details = if details.is_empty() {
            stdout.trim()
        } else {
            details
        };

        let suffix = if details.is_empty() {
            String::new()
        } else {
            format!(": {details}")
        };

        Err(io::Error::other(format!(
            "{action} failed with status {}{suffix}",
            output.status
        )))
    }
}

#[cfg(all(test, target_os = "linux"))]
mod linux_tests {
    use super::*;

    #[test]
    fn desktop_exec_arg_escapes_special_chars() {
        assert_eq!(
            quote_desktop_exec_arg(r#"/tmp/a\b"c$d`e/mdo"#),
            r#""/tmp/a\\b\"c\$d\`e/mdo""#
        );
    }

    #[test]
    fn mimeapps_uninstall_removes_only_mdo_entries() {
        let input = "\
[Default Applications]\n\
text/markdown=mdo.desktop\n\
text/plain=code.desktop\n\
[Added Associations]\n\
text/markdown=code.desktop;mdo.desktop;other.desktop;\n\
text/x-markdown=mdo.desktop;\n\
image/png=viewer.desktop;\n";

        let output = strip_mdo_from_mimeapps(input);

        assert_eq!(
            output,
            "\
[Default Applications]\n\
text/plain=code.desktop\n\
[Added Associations]\n\
text/markdown=code.desktop;other.desktop;\n\
image/png=viewer.desktop;\n"
        );
    }
}

#[cfg(all(test, target_os = "windows"))]
mod windows_tests {
    use super::*;

    #[test]
    fn registry_command_uses_open_flag_without_wrapper() {
        let command = windows_registry_command(Path::new(r"C:\Tools\mdo.exe"), false);
        assert_eq!(command, r#""C:\Tools\mdo.exe" --open "%1""#);
    }

    #[test]
    fn registry_command_omits_open_flag_for_wrapper() {
        let command = windows_registry_command(Path::new(r"C:\Tools\mdo-open.exe"), true);
        assert_eq!(command, r#""C:\Tools\mdo-open.exe" "%1""#);
    }
}
