use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM},
    UI::WindowsAndMessaging::{GetWindowTextW, IsWindowVisible},
};

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use windows::Win32::System::Threading::{
    AttachThreadInput, OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GWL_EXSTYLE, GetForegroundWindow, GetWindow, GetWindowLongW,
    GetWindowThreadProcessId, IsIconic, SW_RESTORE, SetForegroundWindow, ShowWindow,
    WS_EX_TOOLWINDOW,
};
use windows::core::PWSTR;

unsafe fn exe_from_pid(pid: u32) -> Option<PathBuf> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()? };
    let mut buf = [0u16; 260];
    let mut size = buf.len() as u32;

    unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            PWSTR(buf.as_mut_ptr()),
            &mut size,
        )
        .ok()?;
    }

    Some(PathBuf::from(OsString::from_wide(&buf[..size as usize])))
}

unsafe fn filter_windows_apps(hwnd: HWND) -> bool {
    let is_window_visible = unsafe { IsWindowVisible(hwnd).as_bool() };
    if !is_window_visible {
        return false;
    }

    let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 };
    if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
        return false;
    }

    let window_owner = unsafe { GetWindow(hwnd, GW_OWNER) };
    if window_owner.0 != 0 {
        return false;
    }

    let mut title_buf = [0u16; 256];
    let len = unsafe { GetWindowTextW(hwnd, &mut title_buf) };
    if len == 0 {
        return false;
    }

    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };

    let exe = unsafe { exe_from_pid(pid) };
    let exe = match exe {
        Some(exe) => exe,
        None => return false,
    };

    let exe_name = exe.file_name().unwrap().to_string_lossy().to_lowercase();

    const BLACKLIST: [&str; 3] = [
        "nvcontainer.exe",
        "applicationframehost.exe",
        "textinputhost.exe",
    ];

    if BLACKLIST.iter().any(|b| exe_name.contains(b)) {
        return false;
    }

    true
}

unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        return filter_windows_apps(hwnd).into();
    }
}

#[derive(Debug)]
struct FocusData<'a> {
    name: &'a str,
    focused: bool,
}

pub unsafe fn focus_window_from_hwnd(hwnd: HWND) {
    unsafe {
        if IsIconic(hwnd).as_bool() {
            ShowWindow(hwnd, SW_RESTORE);
        }

        let fg = GetForegroundWindow();

        let mut fg_tid = 0;
        let mut target_tid = 0;

        GetWindowThreadProcessId(fg, Some(&mut fg_tid));
        GetWindowThreadProcessId(hwnd, Some(&mut target_tid));

        // podpinamy wątki, tylko jeśli są różne
        if fg_tid != target_tid {
            AttachThreadInput(fg_tid, target_tid, true);
            SetForegroundWindow(hwnd);
            AttachThreadInput(fg_tid, target_tid, false);
        } else {
            SetForegroundWindow(hwnd);
        }
    }
}

pub unsafe extern "system" fn focus_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let data = unsafe { &mut *(lparam.0 as *mut FocusData) };

    let is_window_visible = unsafe { filter_windows_apps(hwnd) };
    if !is_window_visible {
        return true.into();
    }

    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };

    let exe = unsafe { exe_from_pid(pid) };

    let exe = match exe {
        Some(exe) => exe,
        None => return true.into(),
    };

    let exe_name = exe.file_name().unwrap().to_string_lossy().to_lowercase();
    let name_match = exe_name.contains(&data.name.to_lowercase());

    if !name_match {
        return true.into();
    }

    dbg!("found");
    unsafe { focus_window_from_hwnd(hwnd) };
    data.focused = true;

    false.into()
}

pub fn focus_window(name: &str) -> Option<()> {
    let mut data = FocusData {
        name,
        focused: false,
    };

    unsafe {
        let _ = EnumWindows(
            Some(focus_window_callback),
            LPARAM(&mut data as *mut _ as isize),
        );
    }

    data.focused.then_some(())
}
fn get_running_windows_apps() {
    unsafe {
        EnumWindows(Some(enum_proc), LPARAM(0)).expect("cannot read windows applications");
    }
}

#[test]
fn test_focus_window() {
    focus_window("explorer.exe");
}

#[test]
fn test_get_running_windows_apps() {
    let _ = get_running_windows_apps();
}
