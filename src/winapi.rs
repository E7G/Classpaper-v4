use once_cell::sync::Lazy;
use winapi::um::winuser::WS_CHILD;
use std::ffi::{CString, OsStr};
use std::mem;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use winapi::shared::minwindef::{DWORD, FALSE, TRUE};
use winapi::shared::windef::{HWND, RECT};
use winapi::um::{
    dwmapi::{
        DwmEnableBlurBehindWindow, DwmExtendFrameIntoClientArea, DwmSetWindowAttribute,
        DWM_BB_BLURREGION, DWM_BB_ENABLE, DWM_BLURBEHIND,
    },
    handleapi::CloseHandle,
    processthreadsapi::OpenProcess,
    psapi::GetModuleBaseNameW,
    uxtheme::MARGINS,
    wingdi::CreateRectRgn,
    winnt::{PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    winuser::{
        AdjustWindowRect, EnumWindows, FindWindowA, FindWindowExA, FindWindowExW, FindWindowW,
        GetClassNameW, GetSystemMetrics, GetWindow, GetWindowLongPtrW, GetWindowTextW,
        GetWindowThreadProcessId, MoveWindow, SendMessageTimeoutA, SendMessageTimeoutW,
        SetLayeredWindowAttributes, SetParent, SetProcessDPIAware, SetWindowLongPtrW,
        SetWindowPlacement, SetWindowPos, ShowWindow, GWL_EXSTYLE, GWL_STYLE, GW_HWNDPREV,
        HWND_BOTTOM, HWND_TOP, LWA_ALPHA, SMTO_NORMAL, SM_CXSCREEN, SM_CYSCREEN, SWP_DRAWFRAME,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE, SW_SHOW, SW_SHOWNORMAL, WINDOWPLACEMENT,
        WPF_SETMINPOSITION, WS_BORDER, WS_CAPTION, WS_CHILDWINDOW, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
        WS_OVERLAPPED, WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
    },
};

static WORKER_W: Lazy<AtomicUsize> =
    Lazy::new(|| AtomicUsize::new(ptr::null_mut::<HWND>() as usize));
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
    unsafe {
        EnumWindows(Some(enum_windows_proc2), 0);
    }
    let hwnd = TARGET.load(Ordering::SeqCst) as HWND;
    println!("[winapi debug] 查找窗口标题: '{}', HWND: {:?}", title, hwnd);
    hwnd
}

pub fn set_desktop(hwnd: HWND) {
    if hwnd.is_null() {
        println!("[winapi debug] 目标窗口句柄为空");
        return;
    }

    let progman =
        unsafe { FindWindowA(CString::new("Progman").unwrap().as_ptr(), ptr::null_mut()) };

    if progman.is_null() {
        println!("[winapi debug] 未找到 Progman 窗口");
        return;
    }

    println!(
        "[winapi debug] Progman HWND: {:?}, 目标窗口 HWND: {:?}",
        progman, hwnd
    );

    unsafe {
        SendMessageTimeoutA(progman, 0x052C, 0, 0, SMTO_NORMAL, 1000, ptr::null_mut());
        EnumWindows(Some(enum_windows_proc1), 0);
    }

    let worker_w = WORKER_W.load(Ordering::SeqCst) as HWND;
    if !worker_w.is_null() {
        println!("[winapi debug] WorkerW HWND: {:?}, 执行 SW_HIDE", worker_w);
        unsafe {
            ShowWindow(worker_w, SW_HIDE);
        }
    }

    // 在调用 SetParent 之前，确保目标窗口没有其他父窗口
    unsafe {
        // 获取当前窗口样式
        let mut style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        // 清除 WS_CHILD 位并设置 WS_POPUP 样式
        style &= !WS_CHILD;
        style |= WS_POPUP;
        SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize);

        let mut ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        ex_style &= !WS_EX_TOOLWINDOW;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style as isize);

        // 尝试将目标窗口的父窗口设置为 NULL
        SetParent(hwnd, ptr::null_mut());
    }

    // 在调用 SetParent(hwnd, progman) 之前，设置正确的窗口样式
    unsafe {
        // 获取当前窗口样式
        let mut style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        // 清除 WS_POPUP 样式并设置 WS_CHILD 样式
        style &= !WS_POPUP;
        style |= WS_CHILD;
        SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize);
    }

    let set_parent_result = unsafe { SetParent(hwnd, progman) };
    println!("[winapi debug] SetParent 返回: {:?}", set_parent_result);

    if set_parent_result.is_null() {
        println!("[winapi debug] SetParent 调用失败");
    }
}

// 新版本Windows兼容性方案 (Win10/11)
pub fn raise_desktop(h_progman_wnd: HWND) -> bool {
    if h_progman_wnd.is_null() {
        return false;
    }

    unsafe {
        let mut res0: usize = usize::MAX;
        let mut res1: usize = usize::MAX;
        let mut res2: usize = usize::MAX;
        let mut res3: usize = usize::MAX;

        // Call CDesktopBrowser::_IsDesktopWallpaperInitialized
        let result0 = SendMessageTimeoutW(
            h_progman_wnd,
            0x052C,
            0xA,
            0,
            SMTO_NORMAL,
            1000,
            &mut res0 as *mut usize,
        );
        if result0 == 0 || res0 != 0 {
            println!(
                "[winapi debug] 桌面初始化检查失败: result={}, res={}",
                result0, res0
            );
            return false;
        }

        // Prepare to generate wallpaper window
        SendMessageTimeoutW(
            h_progman_wnd,
            0x052C,
            0xD,
            0,
            SMTO_NORMAL,
            1000,
            &mut res1 as *mut usize,
        );
        SendMessageTimeoutW(
            h_progman_wnd,
            0x052C,
            0xD,
            1,
            SMTO_NORMAL,
            1000,
            &mut res2 as *mut usize,
        );
        // "Animate desktop", which will make sure the wallpaper window is there
        SendMessageTimeoutW(
            h_progman_wnd,
            0x052C,
            0,
            0,
            SMTO_NORMAL,
            1000,
            &mut res3 as *mut usize,
        );

        println!(
            "[winapi debug] RaiseDesktop 结果: res1={}, res2={}, res3={}",
            res1, res2, res3
        );
        res1 == 0 && res2 == 0 && res3 == 0
    }
}

pub fn is_explorer_worker(hwnd: HWND) -> bool {
    if hwnd.is_null() {
        return false;
    }

    unsafe {
        let mut class_name = [0u16; 256];
        GetClassNameW(hwnd, class_name.as_mut_ptr(), 256);
        let class_str = String::from_utf16_lossy(&class_name)
            .trim_matches('\0')
            .to_string();

        if class_str != "WorkerW" {
            return false;
        }

        let mut pid: DWORD = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let h_proc = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, FALSE, pid);
        if h_proc.is_null() {
            return false;
        }

        let mut exe_name = [0u16; 260]; // MAX_PATH
        let result = GetModuleBaseNameW(h_proc, ptr::null_mut(), exe_name.as_mut_ptr(), 260);
        CloseHandle(h_proc);

        if result > 0 {
            let exe_str = String::from_utf16_lossy(&exe_name)
                .trim_matches('\0')
                .to_lowercase();
            exe_str.contains("explorer.exe")
        } else {
            false
        }
    }
}

pub fn ensure_embed_window_below(h_shell_def_view: HWND, h_embed_wnd: HWND) -> (bool, HWND) {
    unsafe {
        let prev = GetWindow(h_embed_wnd, GW_HWNDPREV);
        if prev == h_shell_def_view {
            // 顺序已正确，无需操作
            return (true, ptr::null_mut());
        }

        println!("[winapi debug] 修正Z顺序: 将嵌入窗口移动到ShellDefView下方");
        SetWindowPos(
            h_embed_wnd,
            h_shell_def_view,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );

        if is_explorer_worker(prev) {
            (true, ptr::null_mut())
        } else {
            (false, prev)
        }
    }
}

pub fn setup_window_style(hwnd: HWND) -> bool {
    if hwnd.is_null() {
        return false;
    }

    unsafe {
        let mut style_tw = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let mut exstyle_tw = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;

        // 设置扩展样式
        if (exstyle_tw & WS_EX_LAYERED) == 0 {
            exstyle_tw |= WS_EX_LAYERED;
        }
        if (exstyle_tw & WS_EX_TOOLWINDOW) != 0 {
            exstyle_tw &= !WS_EX_TOOLWINDOW;
        }

        // 设置窗口样式
        style_tw &= !(WS_CHILDWINDOW
            | WS_POPUP
            | WS_OVERLAPPED
            | WS_CAPTION
            | WS_BORDER
            | WS_SYSMENU
            | WS_THICKFRAME);

        SetWindowLongPtrW(hwnd, GWL_STYLE, style_tw as isize);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, exstyle_tw as isize);

        println!(
            "[winapi debug] 窗口样式设置完成: style={:x}, exstyle={:x}",
            style_tw, exstyle_tw
        );
        true
    }
}

pub fn setup_transparency(hwnd: HWND) -> bool {
    if hwnd.is_null() {
        return false;
    }

    unsafe {
        // 扩展框架到客户区
        let margins = MARGINS {
            cxLeftWidth: 0,
            cxRightWidth: 0,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        };
        DwmExtendFrameIntoClientArea(hwnd, &margins);

        // 设置毛玻璃效果
        let h_rgn = CreateRectRgn(0, 0, -1, -1);
        let bb = DWM_BLURBEHIND {
            dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
            fEnable: TRUE,
            hRgnBlur: h_rgn,
            fTransitionOnMaximized: FALSE,
        };
        DwmEnableBlurBehindWindow(hwnd, &bb);

        // 设置分层窗口属性
        SetLayeredWindowAttributes(hwnd, 0, 0xFF, LWA_ALPHA);

        println!("[winapi debug] 透明度设置完成");
        true
    }
}

pub fn setup_fullscreen_position(hwnd: HWND) -> bool {
    if hwnd.is_null() {
        return false;
    }

    unsafe {
        let rcx = GetSystemMetrics(SM_CXSCREEN);
        let rcy = GetSystemMetrics(SM_CYSCREEN);

        if rcx <= 0 || rcy <= 0 {
            println!("[winapi debug] 获取屏幕尺寸失败");
            return false;
        }

        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let mut rc_full_screen = RECT {
            left: 0,
            top: 0,
            right: rcx,
            bottom: rcy,
        };

        AdjustWindowRect(&mut rc_full_screen, style, FALSE);

        let rcfx = (rc_full_screen.right - rc_full_screen.left) as i32;
        let rcfy = (rc_full_screen.bottom - rc_full_screen.top) as i32;

        MoveWindow(
            hwnd,
            rc_full_screen.left,
            rc_full_screen.top,
            rcfx,
            rcfy,
            TRUE,
        );

        let wp = WINDOWPLACEMENT {
            length: mem::size_of::<WINDOWPLACEMENT>() as u32,
            flags: WPF_SETMINPOSITION,
            showCmd: SW_SHOWNORMAL as u32,
            ptMinPosition: winapi::shared::windef::POINT {
                x: rc_full_screen.left,
                y: rc_full_screen.top,
            },
            ptMaxPosition: winapi::shared::windef::POINT {
                x: rc_full_screen.left,
                y: rc_full_screen.top,
            },
            rcNormalPosition: rc_full_screen,
        };

        SetWindowPlacement(hwnd, &wp);

        println!("[winapi debug] 全屏位置设置完成: {}x{}", rcfx, rcfy);
        true
    }
}

// 新版本桌面穿透方案 (Win10/11 24H2兼容)
pub fn setup_wallpaper_new(window_title: &str) -> bool {
    let hwnd = find_window_by_title(window_title);
    if hwnd.is_null() {
        println!("[winapi debug] 未找到窗口包含 '{}'", window_title);
        return false;
    }

    println!(
        "[winapi debug] 找到窗口 HWND: {:?}, 开始新版本桌面穿透设置",
        hwnd
    );

    // 1. 设置窗口样式
    if !setup_window_style(hwnd) {
        return false;
    }

    // 2. 查找桌面窗口
    let h_top_desk_wnd = unsafe {
        FindWindowW(
            OsStr::new("Progman")
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<u16>>()
                .as_ptr(),
            OsStr::new("Program Manager")
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<u16>>()
                .as_ptr(),
        )
    };

    if h_top_desk_wnd.is_null() {
        println!("[winapi debug] 未找到桌面顶级父窗口");
        return false;
    }

    // 3. 激活桌面
    if !raise_desktop(h_top_desk_wnd) {
        println!("[winapi debug] 激活桌面失败");
        return false;
    }

    // 4. 查找ShellDefView
    let mut h_shell_def_view = unsafe {
        FindWindowExW(
            h_top_desk_wnd,
            ptr::null_mut(),
            OsStr::new("SHELLDLL_DefView")
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<u16>>()
                .as_ptr(),
            OsStr::new("")
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<u16>>()
                .as_ptr(),
        )
    };

    let mut _h_worker1 = ptr::null_mut();
    let mut h_worker2 = ptr::null_mut();
    let mut is_version_1_2 = false;

    if h_shell_def_view.is_null() {
        // 回退到23H2搜索模式
        unsafe {
            let h_worker_p1 = GetWindow(h_top_desk_wnd, GW_HWNDPREV);
            if !h_worker_p1.is_null() {
                h_shell_def_view = FindWindowExW(
                    h_worker_p1,
                    ptr::null_mut(),
                    OsStr::new("SHELLDLL_DefView")
                        .encode_wide()
                        .chain(Some(0))
                        .collect::<Vec<u16>>()
                        .as_ptr(),
                    OsStr::new("")
                        .encode_wide()
                        .chain(Some(0))
                        .collect::<Vec<u16>>()
                        .as_ptr(),
                );

                if h_shell_def_view.is_null() {
                    h_worker2 = h_worker_p1;
                    let h_worker_p2 = GetWindow(h_worker_p1, GW_HWNDPREV);
                    if !h_worker_p2.is_null() {
                        h_shell_def_view = FindWindowExW(
                            h_worker_p2,
                            ptr::null_mut(),
                            OsStr::new("SHELLDLL_DefView")
                                .encode_wide()
                                .chain(Some(0))
                                .collect::<Vec<u16>>()
                                .as_ptr(),
                            OsStr::new("")
                                .encode_wide()
                                .chain(Some(0))
                                .collect::<Vec<u16>>()
                                .as_ptr(),
                        );
                        if !h_shell_def_view.is_null() {
                            _h_worker1 = h_worker_p2;
                        }
                    }
                } else {
                    _h_worker1 = h_worker_p1;
                }
            }
        }
    }

    if h_shell_def_view.is_null() {
        println!("[winapi debug] 未找到桌面shell defview窗口");
        return false;
    }

    // 5. 查找Worker窗口
    let mut h_worker = unsafe {
        FindWindowExW(
            h_top_desk_wnd,
            ptr::null_mut(),
            OsStr::new("WorkerW")
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<u16>>()
                .as_ptr(),
            OsStr::new("")
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<u16>>()
                .as_ptr(),
        )
    };

    if h_worker.is_null() {
        h_worker = unsafe {
            FindWindowExW(
                h_top_desk_wnd,
                ptr::null_mut(),
                OsStr::new("WorkerA")
                    .encode_wide()
                    .chain(Some(0))
                    .collect::<Vec<u16>>()
                    .as_ptr(),
                OsStr::new("")
                    .encode_wide()
                    .chain(Some(0))
                    .collect::<Vec<u16>>()
                    .as_ptr(),
            )
        };
    }

    if h_worker.is_null() {
        h_worker = if !h_worker2.is_null() {
            h_worker2
        } else {
            h_top_desk_wnd
        };
        is_version_1_2 = true;
        println!("[winapi debug] 使用23H2兼容模式");
    }

    // 6. 设置父窗口和透明度
    unsafe {
        SetParent(hwnd, ptr::null_mut());
    }

    setup_transparency(hwnd);

    // 7. 设置父子关系和窗口位置
    unsafe {
        SetParent(
            hwnd,
            if is_version_1_2 {
                h_worker
            } else {
                h_top_desk_wnd
            },
        );

        SetWindowPos(
            hwnd,
            HWND_TOP,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_DRAWFRAME,
        );
        SetWindowPos(
            h_shell_def_view,
            HWND_TOP,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
        SetWindowPos(
            h_worker,
            HWND_BOTTOM,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_DRAWFRAME,
        );
    }

    // 8. 设置全屏位置
    setup_fullscreen_position(hwnd);

    // 9. 显示窗口
    unsafe {
        ShowWindow(h_top_desk_wnd, SW_SHOW);
        ShowWindow(hwnd, SW_SHOW);
        ShowWindow(h_worker, SW_SHOW);
    }

    // 10. 启动Z顺序监控 (仅24H2需要)
    if !is_version_1_2 {
        start_z_order_monitor(h_shell_def_view, hwnd);
    }

    println!("[winapi debug] 新版本桌面穿透设置完成");
    true
}

pub fn start_z_order_monitor(h_shell_def_view: HWND, h_embed_wnd: HWND) {
    let shell_def_view_ptr = h_shell_def_view as usize;
    let embed_wnd_ptr = h_embed_wnd as usize;

    thread::spawn(move || {
        println!("[winapi debug] 开始监控Z顺序...");
        let max_consecutive_fixes = 5;
        let mut consecutive_fix_count = 0;
        let mut last_conflict_hwnd = ptr::null_mut();

        let h_shell_def_view = shell_def_view_ptr as HWND;
        let h_embed_wnd = embed_wnd_ptr as HWND;

        loop {
            let (ok, conflict_hwnd) = ensure_embed_window_below(h_shell_def_view, h_embed_wnd);

            if !ok {
                if conflict_hwnd.is_null() {
                    consecutive_fix_count = 0;
                } else if conflict_hwnd == last_conflict_hwnd {
                    consecutive_fix_count += 1;
                } else {
                    last_conflict_hwnd = conflict_hwnd;
                    consecutive_fix_count = 1;
                }

                if !conflict_hwnd.is_null() {
                    println!(
                        "[winapi debug] 检测到冲突窗口: {:?}, 连续修复次数: {}",
                        conflict_hwnd, consecutive_fix_count
                    );
                }

                if consecutive_fix_count >= max_consecutive_fixes {
                    println!("[winapi debug] 检测到重复的Z顺序冲突! 退出监控");
                    break;
                }
            } else {
                consecutive_fix_count = 0;
                last_conflict_hwnd = ptr::null_mut();
            }

            thread::sleep(Duration::from_millis(500));
        }
    });
}

// 旧版本桌面穿透方案 (保留兼容性)
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
            let swp_result = SetWindowPos(hwnd, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
            println!("[winapi debug] SetWindowPos 返回: {}", swp_result);
        }
        true
    } else {
        let mut last = LAST_FAIL_TITLE.lock().unwrap();
        if last.as_ref().map_or(true, |t| t != window_title) {
            println!(
                "[winapi debug] 未找到窗口包含 '{}', 桌面穿透失败",
                window_title
            );
            *last = Some(window_title.to_string());
        }
        false
    }
}

// 错误通知功能
pub fn show_error_notification(message: &str) {
    use winapi::um::winuser::{MessageBoxW, MB_ICONERROR, MB_OK};
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    let title = "ClassPaper 错误";
    let title_wide: Vec<u16> = OsStr::new(title)
        .encode_wide()
        .chain(Some(0))
        .collect();
    let message_wide: Vec<u16> = OsStr::new(message)
        .encode_wide()
        .chain(Some(0))
        .collect();

    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message_wide.as_ptr(),
            title_wide.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

// 显示托盘通知（使用Windows通知API）
// 注：需要添加windows-notification特性到Cargo.toml才能启用
// #[cfg(feature = "windows-notification")]
pub fn show_toast_notification(message: &str) {
    // 这里可以实现Windows 10+的Toast通知
    // 需要额外的依赖如 windows-rs
    // 暂时使用MessageBox作为fallback
    show_error_notification(message);
}

// 通用的错误处理函数
pub fn handle_window_creation_error(error: &str) {
    let error_message = format!(
        "无法创建alcro窗口:\n{}\n\n可能的原因:\n- Chrome/Edge浏览器未安装\n- 浏览器路径配置错误\n- 系统资源不足\n- 权限问题",
        error
    );
    
    log::error!("[ClassPaper] {}", error_message);
    show_error_notification(&error_message);
}