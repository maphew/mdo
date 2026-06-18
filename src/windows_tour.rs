//! Windows helpers for opening the first-run terminal tour.
//!
//! Windows desktop entry points (`mdo-open.exe` and `mdo-setup.exe`) are
//! windows-subsystem binaries, so they cannot draw the interactive tour in
//! their own process. Prefer Windows Terminal for a styled, centered tour, and
//! fall back to a plain new console when `wt` is unavailable.

use std::ffi::OsString;
use std::io;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

pub const WINDOWS_TERMINAL_BIN: &str = "wt";
pub const WINDOWS_TERMINAL_COLOR_SCHEME: &str = "One Half Light";

const WINDOWS_TERMINAL_WINDOW_FIND_ATTEMPTS: usize = 20;
const WINDOWS_TERMINAL_WINDOW_FIND_DELAY: Duration = Duration::from_millis(100);

// CREATE_NEW_CONSOLE: give the console-subsystem mdo.exe its own visible
// console window when Windows Terminal is not available. Desktop launchers are
// GUI-subsystem processes with no console of their own, so without this the tour
// would have nowhere reliable to draw.
const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;

pub fn spawn_terminal_tour(mdo: &Path) -> io::Result<()> {
    let active_display = active_display_work_area();
    let title = format!("mdo tour {}", std::process::id());
    let args = windows_terminal_tour_args(mdo, &title, active_display);

    match Command::new(WINDOWS_TERMINAL_BIN).args(args).spawn() {
        Ok(_) => {
            if let Some(work_area) = active_display {
                center_windows_terminal_tour(&title, work_area);
            }
            Ok(())
        }
        Err(_) => spawn_tour_in_new_console(mdo),
    }
}

fn spawn_tour_in_new_console(mdo: &Path) -> io::Result<()> {
    Command::new(mdo)
        .arg("--tour")
        .creation_flags(CREATE_NEW_CONSOLE)
        .spawn()
        .map(|_| ())
}

pub fn windows_terminal_tour_args(
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorkArea {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl WorkArea {
    fn width(self) -> i32 {
        self.right - self.left
    }

    fn height(self) -> i32 {
        self.bottom - self.top
    }
}

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

fn center_windows_terminal_tour(title: &str, work_area: WorkArea) {
    for _ in 0..WINDOWS_TERMINAL_WINDOW_FIND_ATTEMPTS {
        if let Some(hwnd) = find_visible_window_by_title(title) {
            center_window_on_display(hwnd, work_area);
            return;
        }
        std::thread::sleep(WINDOWS_TERMINAL_WINDOW_FIND_DELAY);
    }
}

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

#[cfg(test)]
mod tests {
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
