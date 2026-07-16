use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(target_os = "windows")]
use std::process::Stdio;

const APP_DISPLAY_NAME: &str = "Open as HTML";
#[cfg(target_os = "linux")]
const DESKTOP_FILE_NAME: &str = "mdo.desktop";
#[cfg(target_os = "linux")]
const SETUP_DESKTOP_FILE_NAME: &str = "mdo-setup.desktop";
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
    install_linux_for_exe(&exe, set_default)
}

#[cfg(target_os = "linux")]
pub fn install_linux_for_exe(current_exe: &Path, set_default: bool) -> io::Result<()> {
    let data_home = xdg_data_home()?;
    let desktop_dir = data_home.join("applications");
    let desktop_file = desktop_dir.join(DESKTOP_FILE_NAME);
    let icon_root = data_home.join("icons").join("hicolor");
    let icon_dir = icon_root.join("scalable").join("apps");
    let icon_file = icon_dir.join("mdo.svg");

    fs::create_dir_all(&desktop_dir)?;
    fs::create_dir_all(&icon_dir)?;
    fs::write(&icon_file, SVG_ICON)?;

    let desktop_entry = linux_desktop_entry(current_exe);
    fs::write(&desktop_file, desktop_entry)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&desktop_file, fs::Permissions::from_mode(0o644))?;
    }

    run_optional("update-desktop-database", &[desktop_dir.as_os_str()]);

    // A scalable SVG under hicolor/scalable/apps is resolved by name, so it
    // needs no icon-cache refresh; gtk-update-icon-cache on a user theme dir
    // without an index.theme just fails. Skip it.

    let default_set = if set_default {
        let mut ok = false;
        for mime in ["text/markdown", "text/x-markdown"] {
            ok |= run_optional_status(
                "xdg-mime",
                &[
                    std::ffi::OsStr::new("default"),
                    std::ffi::OsStr::new(DESKTOP_FILE_NAME),
                    std::ffi::OsStr::new(mime),
                ],
            );
        }
        ok
    } else {
        false
    };

    remove_legacy_nautilus_scripts(&data_home)?;

    println!("Installed desktop entry: {}", desktop_file.display());
    println!("Installed icon: {}", icon_file.display());
    if set_default {
        if default_set {
            println!("{APP_DISPLAY_NAME} is now the default Markdown handler.");
        } else {
            println!(
                "Could not set {APP_DISPLAY_NAME} as the default automatically (is xdg-mime installed?)."
            );
            println!("To set it manually, run: xdg-mime default {DESKTOP_FILE_NAME} text/markdown");
            println!("(and the same for text/x-markdown)");
        }
    } else {
        println!(
            "{APP_DISPLAY_NAME} is available from Open With without changing your default Markdown handler."
        );
    }

    Ok(())
}

/// Install the application-menu entry for the native setup launcher.
///
/// This is deliberately separate from [`install_linux_for_exe`]: the latter
/// owns the hidden Markdown file-handler entry and may be uninstalled without
/// making setup disappear from the application menu.
#[cfg(target_os = "linux")]
pub fn install_linux_setup_launcher_for_exe(setup_exe: &Path) -> io::Result<PathBuf> {
    let data_home = xdg_data_home()?;
    let desktop_file = write_linux_setup_launcher(setup_exe, &data_home)?;
    let desktop_dir = data_home.join("applications");

    run_optional("update-desktop-database", &[desktop_dir.as_os_str()]);
    Ok(desktop_file)
}

#[cfg(target_os = "linux")]
fn write_linux_setup_launcher(setup_exe: &Path, data_home: &Path) -> io::Result<PathBuf> {
    let desktop_dir = data_home.join("applications");
    let desktop_file = desktop_dir.join(SETUP_DESKTOP_FILE_NAME);

    fs::create_dir_all(&desktop_dir)?;
    fs::write(&desktop_file, linux_setup_desktop_entry(setup_exe))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&desktop_file, fs::Permissions::from_mode(0o644))?;
    }

    Ok(desktop_file)
}

#[cfg(target_os = "linux")]
fn uninstall_impl() -> io::Result<()> {
    let data_home = xdg_data_home()?;
    let desktop_dir = data_home.join("applications");
    remove_linux_handler_files(&data_home)?;

    for mimeapps in mimeapps_paths(&data_home) {
        remove_desktop_from_mimeapps(&mimeapps)?;
    }

    run_optional("update-desktop-database", &[desktop_dir.as_os_str()]);

    println!("Done.");
    Ok(())
}

#[cfg(target_os = "linux")]
fn remove_linux_handler_files(data_home: &Path) -> io::Result<()> {
    let desktop_file = data_home.join("applications").join(DESKTOP_FILE_NAME);
    let icon_file = data_home
        .join("icons")
        .join("hicolor")
        .join("scalable")
        .join("apps")
        .join("mdo.svg");

    remove_file_if_present(&desktop_file)?;
    remove_file_if_present(&icon_file)?;
    remove_legacy_nautilus_scripts(data_home)
}

#[cfg(target_os = "windows")]
fn install_impl(set_default: bool) -> io::Result<()> {
    let current_exe = std::env::current_exe()?;
    install_windows_for_exe(&current_exe, set_default)
}

#[cfg(target_os = "windows")]
pub fn install_windows_for_exe(current_exe: &Path, _set_default: bool) -> io::Result<()> {
    let handler = windows_handler_for(current_exe);
    let command = windows_registry_command(&handler.path, handler.is_wrapper);
    let icon_file = install_windows_icon()?;
    let icon_ref = format!("\"{}\",0", icon_file.display());
    let current_exe_command = windows_registry_command(current_exe, false);

    // Registry mutation is split into a pure planning step (`install_windows_plan`,
    // unit-tested below without touching the registry) and a thin executor
    // (`apply_registry_plan`) that shells out to `reg.exe`. This keeps the
    // "which keys/values would be written, in what order, with which
    // failure semantics" logic testable in CI without mutating the real
    // registry on the CI machine.
    let plan = install_windows_plan(
        handler.is_wrapper,
        &command,
        &current_exe_command,
        &icon_ref,
    );
    apply_registry_plan(&plan)?;

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
    // Uninstall was already best-effort (every op ignores failures), so this
    // reduces to applying the plan; `apply_registry_plan` returns `Ok(())`
    // whenever every op in the plan is best-effort, which is always true here.
    apply_registry_plan(&uninstall_windows_plan())?;
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
fn linux_setup_desktop_entry(setup_exe: &Path) -> String {
    let quoted_exe = quote_desktop_exec_arg(&setup_exe.to_string_lossy());
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=mdo Setup\n\
         GenericName=Markdown Opener Setup\n\
         Comment=Set up mdo and open the first-run tour\n\
         Exec={quoted_exe}\n\
         Icon=utilities-terminal\n\
         Terminal=false\n\
         NoDisplay=false\n\
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
    let _ = run_optional_status(program, args);
}

#[cfg(target_os = "linux")]
fn run_optional_status(program: &str, args: &[&std::ffi::OsStr]) -> bool {
    Command::new(program)
        .args(args)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
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

/// A single registry write/delete, described data-first so installation
/// logic can be composed and unit-tested without shelling out to `reg.exe`.
///
/// `best_effort` mirrors the original inline `let _ = reg_delete_key(...)`
/// pattern: some ops (e.g. clearing legacy verbs that may not exist) should
/// not abort the rest of the plan if they fail.
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistryOp {
    action: RegistryAction,
    best_effort: bool,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, PartialEq, Eq)]
enum RegistryAction {
    SetDefault {
        key: String,
        value: String,
    },
    SetValue {
        key: String,
        name: String,
        value: String,
    },
    DeleteKey {
        key: String,
    },
    DeleteValue {
        key: String,
        name: String,
    },
}

#[cfg(target_os = "windows")]
impl RegistryOp {
    fn required(action: RegistryAction) -> Self {
        Self {
            action,
            best_effort: false,
        }
    }

    fn best_effort(action: RegistryAction) -> Self {
        Self {
            action,
            best_effort: true,
        }
    }
}

/// Registry ops that register `exe_name` as an "Applications" handler for
/// Open With, mirroring the shape `register_windows_application` used to
/// write directly. Pure: takes already-computed strings and returns a plan.
#[cfg(target_os = "windows")]
fn application_registration_ops(exe_name: &str, command: &str, icon_ref: &str) -> Vec<RegistryOp> {
    let key = format!(r"HKCU\Software\Classes\Applications\{exe_name}");

    vec![
        RegistryOp::required(RegistryAction::SetValue {
            key: key.clone(),
            name: "FriendlyAppName".to_string(),
            value: APP_DISPLAY_NAME.to_string(),
        }),
        RegistryOp::required(RegistryAction::SetValue {
            key: key.clone(),
            name: "ApplicationName".to_string(),
            value: APP_DISPLAY_NAME.to_string(),
        }),
        RegistryOp::required(RegistryAction::SetDefault {
            key: format!(r"{key}\DefaultIcon"),
            value: icon_ref.to_string(),
        }),
        RegistryOp::required(RegistryAction::SetDefault {
            key: format!(r"{key}\shell\open\command"),
            value: command.to_string(),
        }),
        RegistryOp::required(RegistryAction::SetValue {
            key: format!(r"{key}\SupportedTypes"),
            name: ".md".to_string(),
            value: String::new(),
        }),
    ]
}

/// Pure planning step for [`install_windows_for_exe`]: given the already
/// pure-computed handler command, current-exe command, and icon reference,
/// produce the ordered list of registry writes install would perform.
/// Unit-tested directly (see `windows_tests`) so the composition — which
/// keys, which values, wrapper-vs-direct branching, best-effort cleanup of
/// legacy verbs — is covered without mutating the real registry.
#[cfg(target_os = "windows")]
fn install_windows_plan(
    is_wrapper: bool,
    command: &str,
    current_exe_command: &str,
    icon_ref: &str,
) -> Vec<RegistryOp> {
    let mut ops = application_registration_ops("mdo.exe", current_exe_command, icon_ref);
    if is_wrapper {
        ops.extend(application_registration_ops(
            "mdo-open.exe",
            command,
            icon_ref,
        ));
    }

    ops.push(RegistryOp::required(RegistryAction::SetValue {
        key: r"HKCU\Software\Classes\.md\OpenWithProgids".to_string(),
        name: "mdo.md".to_string(),
        value: String::new(),
    }));

    ops.push(RegistryOp::required(RegistryAction::SetDefault {
        key: r"HKCU\Software\Classes\mdo.md".to_string(),
        value: "Markdown document (Open as HTML)".to_string(),
    }));
    ops.push(RegistryOp::required(RegistryAction::SetDefault {
        key: r"HKCU\Software\Classes\mdo.md\shell\open\command".to_string(),
        value: command.to_string(),
    }));
    ops.push(RegistryOp::required(RegistryAction::SetDefault {
        key: r"HKCU\Software\Classes\mdo.md\DefaultIcon".to_string(),
        value: icon_ref.to_string(),
    }));

    for old_verb in [
        r"HKCU\Software\Classes\SystemFileAssociations\.md\shell\Preview with mdo",
        r"HKCU\Software\Classes\SystemFileAssociations\.md\shell\Render with mdo",
    ] {
        ops.push(RegistryOp::best_effort(RegistryAction::DeleteKey {
            key: old_verb.to_string(),
        }));
    }

    let verb = r"HKCU\Software\Classes\SystemFileAssociations\.md\shell\Open as HTML";
    ops.push(RegistryOp::required(RegistryAction::SetDefault {
        key: verb.to_string(),
        value: APP_DISPLAY_NAME.to_string(),
    }));
    ops.push(RegistryOp::required(RegistryAction::SetDefault {
        key: format!(r"{verb}\command"),
        value: command.to_string(),
    }));
    ops.push(RegistryOp::required(RegistryAction::SetValue {
        key: verb.to_string(),
        name: "Icon".to_string(),
        value: icon_ref.to_string(),
    }));

    ops
}

/// Pure planning step for [`uninstall_impl`]: every op here is best-effort,
/// matching the original `let _ = reg_delete_*(...)` calls — uninstall
/// should remove everything it can rather than stop at the first missing
/// key.
#[cfg(target_os = "windows")]
fn uninstall_windows_plan() -> Vec<RegistryOp> {
    let mut ops = Vec::new();

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
        ops.push(RegistryOp::best_effort(RegistryAction::DeleteKey {
            key: key.to_string(),
        }));
    }

    ops.push(RegistryOp::best_effort(RegistryAction::DeleteValue {
        key: r"HKCU\Software\Classes\.md\OpenWithProgids".to_string(),
        name: "mdo.md".to_string(),
    }));
    ops.push(RegistryOp::best_effort(RegistryAction::DeleteValue {
        key: r"HKCU\Software\Classes\.md\OpenWithProgids".to_string(),
        name: "md2htmlx.md".to_string(),
    }));

    ops
}

/// The only impure step of the install/uninstall registry seam: apply an
/// already-built plan by shelling out to `reg.exe` for each op, propagating
/// the first failure among the `required` ops and swallowing failures on
/// `best_effort` ops (mirroring the original `let _ = ...` cleanup calls).
#[cfg(target_os = "windows")]
fn apply_registry_plan(ops: &[RegistryOp]) -> io::Result<()> {
    for op in ops {
        let result = apply_registry_action(&op.action);
        if !op.best_effort {
            result?;
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn apply_registry_action(action: &RegistryAction) -> io::Result<()> {
    match action {
        RegistryAction::SetDefault { key, value } => reg_add_default(key, value),
        RegistryAction::SetValue { key, name, value } => reg_add_value(key, name, value),
        RegistryAction::DeleteKey { key } => reg_delete_key(key),
        RegistryAction::DeleteValue { key, name } => reg_delete_value(key, name),
    }
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
    fn setup_launcher_is_visible_and_not_a_file_handler() {
        let entry = linux_setup_desktop_entry(Path::new(r#"/opt/mdo tools/a\b"c$d`e/mdo-setup"#));

        assert!(entry.contains("Name=mdo Setup\n"));
        assert!(entry.contains(r#"Exec="/opt/mdo tools/a\\b\"c\$d\`e/mdo-setup""#));
        assert!(entry.contains("Terminal=false\n"));
        assert!(entry.contains("NoDisplay=false\n"));
        assert!(!entry.contains("MimeType="));
        assert!(!entry.contains("%f"));
    }

    #[test]
    fn setup_launcher_has_separate_handler_ownership() {
        assert_ne!(SETUP_DESKTOP_FILE_NAME, DESKTOP_FILE_NAME);
        let entry = linux_setup_desktop_entry(Path::new("/usr/bin/mdo-setup"));
        assert!(!entry.contains("Icon=mdo\n"));
    }

    #[test]
    fn handler_uninstall_preserves_setup_launcher() {
        let data_home = std::env::temp_dir().join(format!(
            "mdo-setup-launcher-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let applications = data_home.join("applications");
        fs::create_dir_all(&applications).unwrap();
        fs::write(applications.join(DESKTOP_FILE_NAME), "handler").unwrap();

        let setup_file =
            write_linux_setup_launcher(Path::new("/opt/mdo/bin/mdo-setup"), &data_home).unwrap();
        // Registration is idempotent and rewrites stale content.
        fs::write(&setup_file, "stale").unwrap();
        write_linux_setup_launcher(Path::new("/opt/mdo/bin/mdo-setup"), &data_home).unwrap();

        remove_linux_handler_files(&data_home).unwrap();

        assert!(!applications.join(DESKTOP_FILE_NAME).exists());
        assert!(setup_file.exists());
        assert!(fs::read_to_string(&setup_file)
            .unwrap()
            .contains("Exec=\"/opt/mdo/bin/mdo-setup\"\n"));
        fs::remove_dir_all(data_home).unwrap();
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

    // The exe path is only ever wrapped in quotes, never parsed or
    // re-escaped, so spaces and cmd.exe metacharacters (& ^ ( ) ...), which
    // are all legal in Windows filenames, must survive unmodified inside the
    // quoted segment. Quoting (rather than sanitizing) is what neutralizes
    // them for the shell that later reads the registry value.
    #[test]
    fn registry_command_preserves_spaces_in_exe_path() {
        let command =
            windows_registry_command(Path::new(r"C:\Program Files\mdo tools\mdo.exe"), false);
        assert_eq!(
            command,
            r#""C:\Program Files\mdo tools\mdo.exe" --open "%1""#
        );
    }

    #[test]
    fn registry_command_preserves_cmd_metacharacters_in_exe_path() {
        let command =
            windows_registry_command(Path::new(r"C:\Program Files (x86)\mdo & co^\mdo.exe"), true);
        assert_eq!(
            command,
            r#""C:\Program Files (x86)\mdo & co^\mdo.exe" "%1""#
        );
    }

    #[test]
    fn install_plan_registers_wrapper_and_direct_targets_when_wrapper_present() {
        let handler_command =
            windows_registry_command(Path::new(r"C:\Program Files\mdo & co\mdo-open.exe"), true);
        let current_exe_command =
            windows_registry_command(Path::new(r"C:\Program Files\mdo & co\mdo.exe"), false);
        let icon_ref = r#""C:\Users\me\AppData\Local\mdo\mdo.ico",0"#;

        let plan = install_windows_plan(true, &handler_command, &current_exe_command, icon_ref);

        // mdo.exe is always registered as an Applications handler using the
        // *direct* (--open) command, even when a wrapper exists, because
        // some surfaces (e.g. "Open With" listing before mdo-open exists)
        // may still reference mdo.exe directly.
        assert!(
            plan.contains(&RegistryOp::required(RegistryAction::SetDefault {
                key: r"HKCU\Software\Classes\Applications\mdo.exe\shell\open\command".to_string(),
                value: current_exe_command.clone(),
            }))
        );

        // The wrapper is also registered as its own Applications handler,
        // using the wrapper (no --open) command.
        assert!(
            plan.contains(&RegistryOp::required(RegistryAction::SetDefault {
                key: r"HKCU\Software\Classes\Applications\mdo-open.exe\shell\open\command"
                    .to_string(),
                value: handler_command.clone(),
            }))
        );

        // The .md ProgID's own open command must point at the *handler*
        // command (wrapper, if present) so double-click uses it.
        assert!(
            plan.contains(&RegistryOp::required(RegistryAction::SetDefault {
                key: r"HKCU\Software\Classes\mdo.md\shell\open\command".to_string(),
                value: handler_command.clone(),
            }))
        );

        // Legacy verb cleanup must be best-effort so a first-time install
        // (where those keys never existed) does not fail the whole plan.
        assert!(
            plan.contains(&RegistryOp::best_effort(RegistryAction::DeleteKey {
                key: r"HKCU\Software\Classes\SystemFileAssociations\.md\shell\Preview with mdo"
                    .to_string(),
            }))
        );
        assert!(
            plan.contains(&RegistryOp::best_effort(RegistryAction::DeleteKey {
                key: r"HKCU\Software\Classes\SystemFileAssociations\.md\shell\Render with mdo"
                    .to_string(),
            }))
        );

        // Every op besides the two legacy-verb deletes must be required
        // (install should fail loudly on a real registry error).
        let best_effort_count = plan.iter().filter(|op| op.best_effort).count();
        assert_eq!(best_effort_count, 2);
    }

    #[test]
    fn install_plan_skips_wrapper_registration_without_wrapper() {
        let command = windows_registry_command(Path::new(r"C:\Tools\mdo.exe"), false);
        let icon_ref = r#""C:\Tools\mdo.ico",0"#;

        let plan = install_windows_plan(false, &command, &command, icon_ref);

        assert!(!plan.iter().any(|op| matches!(
            &op.action,
            RegistryAction::SetDefault { key, .. } | RegistryAction::SetValue { key, .. }
                if key.contains("mdo-open.exe")
        )));
    }

    #[test]
    fn uninstall_plan_is_entirely_best_effort() {
        let plan = uninstall_windows_plan();

        assert!(!plan.is_empty());
        assert!(plan.iter().all(|op| op.best_effort));

        // Legacy md2htmlx keys (an earlier project name) must still be
        // cleaned up so upgrading users do not keep a stale handler.
        assert!(
            plan.contains(&RegistryOp::best_effort(RegistryAction::DeleteKey {
                key: r"HKCU\Software\Classes\Applications\md2htmlx.exe".to_string(),
            }))
        );
        assert!(
            plan.contains(&RegistryOp::best_effort(RegistryAction::DeleteValue {
                key: r"HKCU\Software\Classes\.md\OpenWithProgids".to_string(),
                name: "md2htmlx.md".to_string(),
            }))
        );
    }
}
