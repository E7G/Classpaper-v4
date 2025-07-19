use std::ffi::CString;
// use std::ffi::OsStr;
use std::mem;
// use std::os::windows::ffi::OsStrExt;
use std::ptr;
use winapi::shared::windef::HWND;
use winapi::um::{
    dwmapi::DwmSetWindowAttribute,
    winuser::{
        EnumWindows, FindWindowA, FindWindowExA, GetSystemMetrics, GetWindowTextW, SetParent, ShowWindow, SetWindowPos,
        SendMessageTimeoutA, HWND_BOTTOM, SW_HIDE, SWP_NOMOVE, SWP_NOSIZE, SM_CXSCREEN, SM_CYSCREEN, SMTO_NORMAL, SetProcessDPIAware,
    },
};
// use winapi::um::winnt::KEY_READ;
// use winapi::um::winreg::{RegOpenKeyExW, RegQueryValueExW, RegCloseKey, HKEY_CURRENT_USER};
use std::sync::atomic::{AtomicUsize, Ordering};
use once_cell::sync::Lazy;
use std::sync::Mutex;

static WORKER_W: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(ptr::null_mut::<HWND>() as usize));
static TARGET: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(ptr::null_mut::<HWND>() as usize));
static SEARCH_TITLE: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::new()));
static LAST_FAIL_TITLE: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

pub fn get_screen_width() -> i32 {
    unsafe { GetSystemMetrics(SM_CXSCREEN) }
}

pub fn get_screen_height() -> i32 {
    unsafe { GetSystemMetrics(SM_CYSCREEN) }
}

pub fn set_dpi_aware() -> bool {
    unsafe { SetProcessDPIAware() != 0 }
}

// pub fn get_windows_dark_mode() -> bool {
//     let reg_path = OsStr::new(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
//         .encode_wide().chain(Some(0)).collect::<Vec<u16>>();
//     let mut hkey = ptr::null_mut();
//     let status = unsafe {
//         RegOpenKeyExW(HKEY_CURRENT_USER, reg_path.as_ptr(), 0, KEY_READ, &mut hkey)
//     };
//     if status != 0 { return false; }
//     let mut data = [0u8; 4];
//     let mut data_len = 4u32;
//     let value_name = OsStr::new("AppsUseLightTheme").encode_wide().chain(Some(0)).collect::<Vec<u16>>();
//     let status = unsafe {
//         RegQueryValueExW(hkey, value_name.as_ptr(), ptr::null_mut(), ptr::null_mut(), data.as_mut_ptr(), &mut data_len)
//     };
//     unsafe { RegCloseKey(hkey); }
//     if status != 0 { return false; }
//     data[0] == 0
// }

unsafe extern "system" fn enum_windows_proc1(hwnd: HWND, _: isize) -> i32 {
    let defview = FindWindowExA(
        hwnd,
        ptr::null_mut(),
        CString::new("SHELLDLL_DefView").unwrap().as_ptr(),
        ptr::null_mut(),
    );
    if !defview.is_null() {
        WORKER_W.store(
            FindWindowExA(
                ptr::null_mut(),
                hwnd,
                CString::new("WorkerW").unwrap().as_ptr(),
                ptr::null_mut(),
            ) as usize,
            Ordering::SeqCst,
        );
    }
    1
}

unsafe extern "system" fn enum_windows_proc2(hwnd: HWND, _: isize) -> i32 {
    let mut title = [0u16; 256];
    GetWindowTextW(hwnd, title.as_mut_ptr(), 256);
    let window_title = String::from_utf16_lossy(&title)
        .trim_matches('\0')
        .to_string();
    let search_title = SEARCH_TITLE.lock().unwrap().clone();
    if !search_title.is_empty() && window_title.contains(&search_title) {
        TARGET.store(hwnd as usize, Ordering::SeqCst);
        return 0;
    }
    1
}

pub fn find_window_by_title(title: &str) -> HWND {
    TARGET.store(ptr::null_mut::<HWND>() as usize, Ordering::SeqCst);
    *SEARCH_TITLE.lock().unwrap() = title.to_string();
    unsafe { EnumWindows(Some(enum_windows_proc2), 0); }
    let hwnd = TARGET.load(Ordering::SeqCst) as HWND;
    println!("[winapi debug] 查找窗口标题: '{}', HWND: {:?}", title, hwnd);
    hwnd
}

pub fn set_desktop(hwnd: HWND) {
    let progman = unsafe {
        FindWindowA(
            CString::new("Progman").unwrap().as_ptr(),
            ptr::null_mut(),
        )
    };
    println!("[winapi debug] Progman HWND: {:?}, 目标窗口 HWND: {:?}", progman, hwnd);
    unsafe {
        SendMessageTimeoutA(
            progman,
            0x052C,
            0,
            0,
            SMTO_NORMAL,
            1000,
            ptr::null_mut(),
        );
        EnumWindows(Some(enum_windows_proc1), 0);
    }
    let worker_w = WORKER_W.load(Ordering::SeqCst) as HWND;
    if !worker_w.is_null() {
        println!("[winapi debug] WorkerW HWND: {:?}, 执行 SW_HIDE", worker_w);
        unsafe { ShowWindow(worker_w, SW_HIDE); }
    }
    let set_parent_result = unsafe { SetParent(hwnd, progman) };
    println!("[winapi debug] SetParent 返回: {:?}", set_parent_result);
}

pub fn setup_wallpaper(window_title: &str) -> bool {
    let hwnd = find_window_by_title(window_title);
    if !hwnd.is_null() {
        println!("[winapi debug] 找到窗口 HWND: {:?}, 开始设置桌面穿透", hwnd);
        set_desktop(hwnd);
        unsafe {
            let val: u32 = 1;
            let dwm_result = DwmSetWindowAttribute(
                hwnd,
                19, // DWMWA_CLOAK
                &val as *const u32 as *const _,
                mem::size_of::<u32>() as u32,
            );
            println!("[winapi debug] DwmSetWindowAttribute 返回: {}", dwm_result);
            let swp_result = SetWindowPos(
                hwnd,
                HWND_BOTTOM,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE,
            );
            println!("[winapi debug] SetWindowPos 返回: {}", swp_result);
        }
        true
    } else {
        let mut last = LAST_FAIL_TITLE.lock().unwrap();
        if last.as_ref().map_or(true, |t| t != window_title) {
            println!("[winapi debug] 未找到窗口包含 '{}', 桌面穿透失败", window_title);
            *last = Some(window_title.to_string());
        }
        false
    }
} 