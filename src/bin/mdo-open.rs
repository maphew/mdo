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
//!   3. If launched directly with no file args, opens onboarding: Windows
//!      starts the terminal tour in Windows Terminal when available; Linux
//!      opens `mdo-setup` for native first-run onboarding.
//!   4. Exits immediately. The child runs detached and pops the browser.
//!
//! Net result: registering `mdo-open.exe "%1"` in the Explorer file
//! association gives a flash-free double-click experience without changing
//! how the regular CLI behaves in a terminal.
//!
//! On Windows, no-file launches open the terminal tour in a fresh `wt` window
//! with the One Half Light color scheme and then centers that window on the
//! active display. If `wt` cannot be started, mdo falls back to the sibling
//! `mdo-setup.exe` desktop onboarding flow. On Linux, no-file launches open the
//! sibling `mdo-setup` desktop onboarding flow. Other non-Windows targets remain
//! a passthrough to the sibling `mdo` binary; nothing about the subsystem flag
//! applies there.

// Mark this binary as windows-subsystem on Windows so Windows itself does not
// allocate a console for it. The attribute is a no-op (and the cfg_attr makes
// it absent) on other platforms.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::env;
use std::process::{Command, ExitCode};

#[cfg(target_os = "windows")]
use std::ffi::OsString;
#[cfg(target_os = "windows")]
use std::io;
#[cfg(target_os = "windows")]
use std::path::Path;
#[cfg(target_os = "windows")]
use std::time::Duration;

#[cfg(target_os = "windows")]
const MDO_BIN: &str = "mdo.exe";
#[cfg(target_os = "windows")]
const SETUP_BIN: &str = "mdo-setup.exe";
#[cfg(target_os = "windows")]
const WINDOWS_TERMINAL_BIN: &str = "wt";
#[cfg(target_os = "windows")]
const WINDOWS_TERMINAL_COLOR_SCHEME: &str = "One Half Light";
#[cfg(target_os = "windows")]
const WINDOWS_TERMINAL_WINDOW_FIND_ATTEMPTS: usize = 20;
#[cfg(target_os = "windows")]
const WINDOWS_TERMINAL_WINDOW_FIND_DELAY: Duration = Duration::from_millis(100);

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

    #[cfg(target_os = "windows")]
    {
        let result = if args.is_empty() {
            spawn_windows_onboarding(&exe_path)
        } else {
            spawn_windows_open(&exe_path, args)
        };

        match result {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::from(1),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        #[cfg(target_os = "linux")]
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

        #[cfg(not(target_os = "linux"))]
        let mut cmd = {
            let mut cmd = Command::new(exe_path.join(MDO_BIN));
            cmd.arg("--open");
            cmd.args(args);
            cmd
        };

        match cmd.spawn() {
            Ok(_) => ExitCode::SUCCESS,
            Err(_) => ExitCode::from(1),
        }
    }
}

#[cfg(target_os = "windows")]
fn spawn_windows_open(exe_dir: &Path, args: Vec<std::ffi::OsString>) -> io::Result<()> {
    let mut cmd = Command::new(exe_dir.join(MDO_BIN));
    cmd.arg("--open");
    cmd.args(args);
    spawn_without_console(cmd)
}

#[cfg(target_os = "windows")]
fn spawn_windows_onboarding(exe_dir: &Path) -> io::Result<()> {
    let mdo = exe_dir.join(MDO_BIN);
    if !mdo.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "expected {MDO_BIN} next to mdo-open.exe at {}",
                mdo.display()
            ),
        ));
    }

    let active_display = active_display_work_area();
    let title = format!("mdo tour {}", std::process::id());
    let args = windows_terminal_tour_args(&mdo, &title, active_display);

    match Command::new(WINDOWS_TERMINAL_BIN).args(args).spawn() {
        Ok(_) => {
            if let Some(work_area) = active_display {
                center_windows_terminal_tour(&title, work_area);
            }
            Ok(())
        }
        Err(_) => spawn_windows_setup(exe_dir),
    }
}

#[cfg(target_os = "windows")]
fn spawn_windows_setup(exe_dir: &Path) -> io::Result<()> {
    let setup = exe_dir.join(SETUP_BIN);
    if !setup.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "expected {SETUP_BIN} next to mdo-open.exe at {}",
                setup.display()
            ),
        ));
    }

    spawn_without_console(Command::new(setup))
}

#[cfg(target_os = "windows")]
fn spawn_without_console(mut cmd: Command) -> io::Result<()> {
    use std::os::windows::process::CommandExt;

    // CREATE_NO_WINDOW (0x0800_0000) tells Windows not to give the child a
    // console of its own. Without it, even though *we* have no console, the
    // child (a console-subsystem exe) would briefly get one allocated —
    // exactly the flash we are trying to avoid.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.spawn().map(|_| ())
}

#[cfg(target_os = "windows")]
fn windows_terminal_tour_args(
    mdo: &Path,
    title: &str,
    active_display: Option<WorkArea>,
) -> Vec<OsString> {
    let mut args = vec![
        OsString::from("-w"),
        OsString::from("new"),
        OsString::from("new-tab"),
        OsString::from("--title"),
        OsString::from(title),
        OsString::from("--suppressApplicationTitle"),
        OsString::from("--colorScheme"),
        OsString::from(WINDOWS_TERMINAL_COLOR_SCHEME),
        mdo.as_os_str().to_os_string(),
        OsString::from("--tour"),
    ];

    if let Some(work_area) = active_display {
        // `--pos` does not center by itself, but it tells Windows Terminal to
        // open on the active display immediately. We then center the real
        // window after it appears.
        let pos = OsString::from(format!("{},{}", work_area.left, work_area.top));
        args.insert(2, pos);
        args.insert(2, OsString::from("--pos"));
    }

    args
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct WorkArea {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[cfg(target_os = "windows")]
impl WorkArea {
    fn width(self) -> i32 {
        self.right - self.left
    }

    fn height(self) -> i32 {
        self.bottom - self.top
    }
}

#[cfg(target_os = "windows")]
fn active_display_work_area() -> Option<WorkArea> {
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    unsafe {
        let foreground = GetForegroundWindow();
        let monitor = MonitorFromWindow(foreground, MONITOR_DEFAULTTONEAREST);
        if monitor.is_null() {
            return None;
        }

        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: std::mem::zeroed(),
            rcWork: std::mem::zeroed(),
            dwFlags: 0,
        };

        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return None;
        }

        Some(WorkArea {
            left: info.rcWork.left,
            top: info.rcWork.top,
            right: info.rcWork.right,
            bottom: info.rcWork.bottom,
        })
    }
}

#[cfg(target_os = "windows")]
fn center_windows_terminal_tour(title: &str, work_area: WorkArea) {
    for _ in 0..WINDOWS_TERMINAL_WINDOW_FIND_ATTEMPTS {
        if let Some(hwnd) = find_visible_window_by_title(title) {
            center_window_on_display(hwnd, work_area);
            return;
        }
        std::thread::sleep(WINDOWS_TERMINAL_WINDOW_FIND_DELAY);
    }
}

#[cfg(target_os = "windows")]
fn find_visible_window_by_title(title: &str) -> Option<windows_sys::Win32::Foundation::HWND> {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible,
    };

    struct SearchState {
        needle: String,
        hwnd: HWND,
    }

    unsafe extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> i32 {
        let state = unsafe { &mut *(lparam as *mut SearchState) };

        if unsafe { IsWindowVisible(hwnd) } == 0 {
            return 1;
        }

        let len = unsafe { GetWindowTextLengthW(hwnd) };
        if len <= 0 {
            return 1;
        }

        let mut buffer = vec![0u16; len as usize + 1];
        let copied = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
        if copied <= 0 {
            return 1;
        }

        let window_title = String::from_utf16_lossy(&buffer[..copied as usize]);
        if window_title.contains(&state.needle) {
            state.hwnd = hwnd;
            return 0;
        }

        1
    }

    let mut state = SearchState {
        needle: title.to_string(),
        hwnd: std::ptr::null_mut(),
    };

    unsafe {
        EnumWindows(Some(enum_window), &mut state as *mut SearchState as LPARAM);
    }

    if state.hwnd.is_null() {
        None
    } else {
        Some(state.hwnd)
    }
}

#[cfg(target_os = "windows")]
fn center_window_on_display(hwnd: windows_sys::Win32::Foundation::HWND, work_area: WorkArea) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowRect, SetWindowPos, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
    };

    unsafe {
        let mut rect = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return;
        }

        let window_width = rect.right - rect.left;
        let window_height = rect.bottom - rect.top;
        if window_width <= 0 || window_height <= 0 {
            return;
        }

        let x = work_area.left + ((work_area.width() - window_width) / 2).max(0);
        let y = work_area.top + ((work_area.height() - window_height) / 2).max(0);

        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            x,
            y,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOZORDER,
        );
    }
}

#[cfg(all(test, target_os = "windows"))]
mod windows_tests {
    use super::*;

    #[test]
    fn tour_args_use_windows_terminal_with_light_scheme() {
        let args = windows_terminal_tour_args(
            Path::new(r"C:\Tools\mdo.exe"),
            "mdo tour test",
            Some(WorkArea {
                left: 100,
                top: 50,
                right: 2020,
                bottom: 1130,
            }),
        )
        .into_iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();

        assert_eq!(
            args,
            vec![
                "-w",
                "new",
                "--pos",
                "100,50",
                "new-tab",
                "--title",
                "mdo tour test",
                "--suppressApplicationTitle",
                "--colorScheme",
                "One Half Light",
                r"C:\Tools\mdo.exe",
                "--tour",
            ]
        );
    }

    #[test]
    fn tour_args_do_not_force_position_without_active_display() {
        let args =
            windows_terminal_tour_args(Path::new(r"C:\Tools\mdo.exe"), "mdo tour test", None)
                .into_iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>();

        assert!(!args.iter().any(|arg| arg == "--pos"));
        assert!(args.contains(&"One Half Light".to_string()));
        assert!(args.contains(&"--tour".to_string()));
    }
}
