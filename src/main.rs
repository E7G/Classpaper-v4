#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    path::Path,
    sync::{Arc, Mutex},
    thread,
};

mod winapi;

use alcro::{Content, UIBuilder, UI};
use ctrlc;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use serde_json;
use simplelog::{ConfigBuilder, WriteLogger, CombinedLogger, TermLogger, TerminalMode, ColorChoice, LevelFilter, ThreadLogMode, LevelPadding};
use std::process::Command;
use toml::Value as TomlValue;
use tray_item::TrayItem;

#[derive(Debug, Serialize, Deserialize)]
struct AppConfig {
    #[serde(default, rename = "Default")]
    default: DefaultConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct DefaultConfig {
    #[serde(default = "default_url", rename = "URL")]
    url: String,
    #[serde(default, rename = "BrowserPath")]
    browser_path: String,
}

fn default_url() -> String {
    "./res/index.html".to_string()
}

impl Default for DefaultConfig {
    fn default() -> Self {
        Self {
            url: default_url(),
            browser_path: String::new(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default: DefaultConfig::default(),
        }
    }
}

struct AppState {
    window: Option<Arc<UI>>,
    window_name: String,
    settings_windows: Vec<Arc<UI>>,
}

fn generate_random_string(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

fn encode_chinese_characters(path: &str) -> String {
    // 只对中文字符做转码
    path.chars()
        .map(|c| {
            if c >= '\u{4e00}' && c <= '\u{9fa5}' {
                utf8_percent_encode(&c.to_string(), NON_ALPHANUMERIC).to_string()
            } else {
                c.to_string()
            }
        })
        .collect::<String>()
}

fn normalize_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        // 以当前工作目录为基准
        let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
        let abs_path = cwd
            .join(url)
            .canonicalize()
            .unwrap_or_else(|_| cwd.join(url));
        let mut abs_path_str = abs_path.to_string_lossy().to_string();
        // 修正 Windows 下的 \\?\ 前缀
        if abs_path_str.starts_with(r"\\?\") {
            abs_path_str = abs_path_str[4..].to_string();
        }
        // 去除路径中的"./"或".\\"前缀
        let abs_path_str = if abs_path_str.starts_with("./") {
            &abs_path_str[2..]
        } else if abs_path_str.starts_with(".\\") {
            &abs_path_str[2..]
        } else {
            &abs_path_str
        };
        // Windows下去除多余的斜杠
        #[cfg(windows)]
        let abs_path_str = {
            let mut s = abs_path_str.replace("\\", "/");
            // 去除 "/./" 片段
            while let Some(idx) = s.find("/./") {
                s.replace_range(idx..idx + 3, "/");
            }
            // 去除 "D:/" 前的多余斜杠
            if s.starts_with("/") && s.chars().nth(2) == Some(':') {
                s = s[1..].to_string();
            }
            s
        };
        #[cfg(not(windows))]
        let abs_path_str = abs_path_str.replace("\\", "/");
        let encoded = encode_chinese_characters(&abs_path_str);
        let file_url = format!("file:///{}", encoded);
        file_url
    }
}

fn parse_config() -> AppConfig {
    match std::fs::read_to_string("config.toml") {
        Ok(content) => {
            // 先尝试直接解析为新格式
            if let Ok(cfg) = toml::from_str::<AppConfig>(&content) {
                return cfg;
            }
            // 兼容旧格式：小写段名和字段名
            if let Ok(toml_value) = content.parse::<TomlValue>() {
                // 检查是否有 [default] 段
                if let Some(table) = toml_value.get("default").and_then(|v| v.as_table()) {
                    // 兼容小写字段
                    let url = table
                        .get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let browser_path = table
                        .get("browser_path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    // 构造新格式
                    let new = AppConfig {
                        default: DefaultConfig { url, browser_path },
                    };
                    // 自动迁移为新格式
                    if let Ok(new_content) = toml::to_string(&new) {
                        let _ = std::fs::write("config.toml", new_content);
                    }
                    return new;
                }
            }
            // 兜底：返回默认
            let config = AppConfig::default();
            if let Ok(content) = toml::to_string(&config) {
                let _ = std::fs::write("config.toml", content);
            }
            config
        }
        Err(_) => {
            let config = AppConfig::default();
            if let Ok(content) = toml::to_string(&config) {
                let _ = std::fs::write("config.toml", content);
            }
            config
        }
    }
}

fn create_window(url: &str, window_name: &str, browser_path: &str) -> UI {
    let mut builder = UIBuilder::new();
    builder.content(Content::Url(url));
    if !browser_path.is_empty() {
        builder.browser_path(browser_path);
    }
    // 根据 URL 类型智能决定是否禁用缓存
    let mut chrome_args = vec![
        "--kiosk",
        "--autoplay-policy=no-user-gesture-required",
        // 其他参数
    ];
    if !url.starts_with("http://") && !url.starts_with("https://") {
        // 本地文件禁用缓存
        chrome_args.extend_from_slice(&[
            "--disable-application-cache",
            "--disk-cache-size=1",
            "--media-cache-size=1",
            "--disable-cache",
            "--disable-offline-load-stale-cache",
            "--disable-gpu-program-cache",
            "--aggressive-cache-discard",
        ]);
    }
    println!("\n========== [ClassPaper] 启动参数 ==========");
    println!("[ClassPaper] chrome_args:");
    for arg in chrome_args.iter() {
        println!("    {}", arg);
    }
    println!("==========================================\n");
    builder.custom_args(&chrome_args);
    let ui = builder.run().expect("无法创建alcro窗口");
    let _ = ui.eval(&format!("document.title = '{}';", window_name));
    // // 自动全屏JS
    // let fullscreen_js = r#"
    //     (function() {
    //         function launchFullscreen(element) {
    //             if(element.requestFullscreen) {
    //                 element.requestFullscreen();
    //             } else if(element.mozRequestFullScreen) {
    //                 element.mozRequestFullScreen();
    //             } else if(element.webkitRequestFullscreen) {
    //                 element.webkitRequestFullscreen();
    //             } else if(element.msRequestFullscreen) {
    //                 element.msRequestFullscreen();
    //             }
    //         }
    //         window.addEventListener('load', function() {
    //             setTimeout(function() {
    //                 launchFullscreen(document.documentElement);
    //             }, 300);
    //         });
    //     })();
    // "#;
    // let _ = ui.eval(fullscreen_js);
    // 绑定Rust函数到JS
    let _ = ui.bind("getWidth", |_| Ok(winapi::get_screen_width().into()));
    let _ = ui.bind("getHeight", |_| Ok(winapi::get_screen_height().into()));
    let _ = ui.bind("readFile", |args| {
        if let Some(path) = args.get(0).and_then(|v| v.as_str()) {
            match std::fs::read_to_string(path) {
                Ok(content) => Ok(serde_json::Value::String(content)),
                Err(e) => Err(format!("读取文件失败: {}", e).into()),
            }
        } else {
            Err("参数错误".into())
        }
    });
    let _ = ui.bind("writeFile", |args| {
        if let (Some(path), Some(content)) = (
            args.get(0).and_then(|v| v.as_str()),
            args.get(1).and_then(|v| v.as_str()),
        ) {
            match std::fs::write(path, content) {
                Ok(_) => Ok(true.into()),
                Err(e) => Err(format!("写入文件失败: {}", e).into()),
            }
        } else {
            Err("参数错误".into())
        }
    });
    let _ = ui.bind("readDir", |args| {
        if let Some(dir) = args.get(0).and_then(|v| v.as_str()) {
            match std::fs::read_dir(dir) {
                Ok(entries) => {
                    let names: Vec<_> = entries
                        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().to_string()))
                        .collect();
                    Ok(names.into())
                }
                Err(e) => Err(format!("读取目录失败: {}", e).into()),
            }
        } else {
            Err("参数错误".into())
        }
    });
    ui
}

// cleanup_profile_dir 已移除

fn open_settings_window(app_state: Arc<Mutex<AppState>>) {
    let settings_path = normalize_url("./res/settings.html");
    let settings_ui = Arc::new(
        UIBuilder::new()
            .content(Content::Url(&settings_path))
            .run()
            .expect("无法创建设置窗口"),
    );

    // 绑定 readConfig
    let _ = settings_ui.bind("readConfig", |_| {
        match std::fs::read_to_string("config.toml") {
            Ok(content) => {
                println!(
                    "[settings][readConfig] 读取到的 config.toml 内容:\n{}",
                    content
                );
                // 先用 toml 解析为 serde_json::Value
                match toml::from_str::<serde_json::Value>(&content) {
                    Ok(json) => {
                        // 强制转换为 { Default: { URL, BrowserPath } } 格式，字段名大写
                        // 兼容 [default] 或 [Default] 段
                        let mut result = serde_json::Map::new();
                        let default_obj = json.get("Default").or_else(|| json.get("default"));
                        if let Some(default) = default_obj {
                            let mut default_map = serde_json::Map::new();
                            if let Some(url) = default.get("url").or_else(|| default.get("URL")) {
                                default_map.insert("URL".to_string(), url.clone());
                            }
                            if let Some(browser_path) = default
                                .get("browser_path")
                                .or_else(|| default.get("BrowserPath"))
                            {
                                default_map.insert("BrowserPath".to_string(), browser_path.clone());
                            }
                            result.insert(
                                "Default".to_string(),
                                serde_json::Value::Object(default_map),
                            );
                        }
                        let json_result = serde_json::Value::Object(result);
                        println!("[settings][readConfig] 兼容后的 JSON: {}", json_result);
                        Ok(json_result)
                    }
                    Err(e) => {
                        println!("[settings][readConfig] TOML 解析失败: {}", e);
                        Err(format!("TOML 解析失败: {}", e).into())
                    }
                }
            }
            Err(e) => {
                println!("[settings][readConfig] 读取配置失败: {}", e);
                Err(format!("读取配置失败: {}", e).into())
            }
        }
    });
    // 绑定 saveConfig
    let _ = settings_ui.bind("saveConfig", |args| {
        if let Some(config_json) = args.get(0).and_then(|v| v.as_str()) {
            // 先解析为 serde_json::Value
            let value: serde_json::Value = match serde_json::from_str(config_json) {
                Ok(v) => v,
                Err(e) => return Err(format!("解析配置JSON失败: {}", e).into()),
            };
            // 只序列化 Default 字段为 [Default] 段，字段名大写
            let mut toml_map = toml::map::Map::new();
            if let Some(default) = value.get("Default") {
                toml_map.insert(
                    "Default".to_string(),
                    toml::Value::try_from(default.clone())
                        .unwrap_or(toml::Value::String(String::new())),
                );
            }
            let toml_value = toml::Value::Table(toml_map);
            let toml_str = match toml::to_string(&toml_value) {
                Ok(s) => s,
                Err(e) => return Err(format!("序列化TOML失败: {}", e).into()),
            };
            // 写入文件
            match std::fs::write("config.toml", toml_str) {
                Ok(_) => Ok(true.into()),
                Err(e) => Err(format!("写入配置失败: {}", e).into()),
            }
        } else {
            Err("参数错误".into())
        }
    });
    // 绑定 writeFile
    let _ = settings_ui.bind("writeFile", |args| {
        if let (Some(path), Some(content)) = (
            args.get(0).and_then(|v| v.as_str()),
            args.get(1).and_then(|v| v.as_str()),
        ) {
            match std::fs::write(path, content) {
                Ok(_) => Ok(true.into()),
                Err(e) => Err(format!("写入文件失败: {}", e).into()),
            }
        } else {
            Err("参数错误".into())
        }
    });
    // 绑定 scanWallpaperDir
    let _ = settings_ui.bind("scanWallpaperDir", |_| {
        let mut wallpapers = Vec::new();
        if let Ok(entries) = std::fs::read_dir("res/wallpaper") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        let ext = ext.to_lowercase();
                        if ext == "jpg" || ext == "jpeg" || ext == "png" || ext == "gif" {
                            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                wallpapers.push(format!("wallpaper/{}", name));
                            }
                        }
                    }
                }
            }
        }
        Ok(serde_json::Value::Array(
            wallpapers
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        ))
    });
    // 绑定 reloadMainWindow
    let app_state_reload = Arc::clone(&app_state);
    let _ = settings_ui.bind("reloadMainWindow", move |_| {
        let state = app_state_reload.lock().unwrap();
        if let Some(ref window) = state.window {
            let _ = window.eval("location.reload(true)");
        }
        Ok(true.into())
    });
    // 绑定 openURLInBrowser
    let _ = settings_ui.bind("openURLInBrowser", |args| {
        if let Some(url) = args.get(0).and_then(|v| v.as_str()) {
            #[cfg(target_os = "windows")]
            let result = Command::new("cmd").args(["/c", "start", url]).spawn();
            #[cfg(target_os = "macos")]
            let result = Command::new("open").arg(url).spawn();
            #[cfg(target_os = "linux")]
            let result = Command::new("xdg-open").arg(url).spawn();
            match result {
                Ok(_) => Ok(true.into()),
                Err(e) => Err(format!("打开浏览器失败: {}", e).into()),
            }
        } else {
            Err("参数错误".into())
        }
    });
    // 设置窗口标题
    let _ = settings_ui.eval("document.title = 'ClassPaper 设置';");
    // 最后 push Arc 到 AppState
    {
        let mut state = app_state.lock().unwrap();
        state.settings_windows.push(settings_ui);
    }
}

fn close_all_and_exit(app_state: &Arc<Mutex<AppState>>) -> ! {
    let state = app_state.lock().unwrap();
    if let Some(ref window) = state.window {
        window.close_blocking(3000);
    }
    for win in &state.settings_windows {
        win.close_blocking(3000);
    }
    // 让后台线程有机会检测到管道关闭
    std::thread::sleep(std::time::Duration::from_millis(500));
    std::process::exit(0);
}

fn main() -> std::io::Result<()> {
    // 日志初始化增强（美化格式/本地时间/分级/彩色/线程/文件/行号）
    let mut builder = ConfigBuilder::new();
    builder.set_thread_mode(ThreadLogMode::Both);
    builder.set_thread_level(LevelFilter::Info);
    builder.set_location_level(LevelFilter::Debug);
    builder.set_level_padding(LevelPadding::Right);
    builder.set_time_offset_to_local().ok();
    let log_config = builder.build();
    let log_file = std::fs::OpenOptions::new().create(true).append(true).open("app.log").unwrap_or_else(|e| {
        eprintln!("[日志] 无法打开 app.log: {}，日志将输出到 stderr/nul", e);
        #[cfg(windows)]
        { std::fs::OpenOptions::new().write(true).open("nul").unwrap() }
        #[cfg(not(windows))]
        { std::fs::File::create("/dev/stderr").unwrap() }
    });
    let log_level = std::env::var("RUST_LOG").ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(LevelFilter::Info);
    CombinedLogger::init(vec![
        WriteLogger::new(log_level, log_config.clone(), log_file),
        #[cfg(debug_assertions)]
        TermLogger::new(LevelFilter::Debug, log_config, TerminalMode::Mixed, ColorChoice::Auto),
    ]).expect("无法初始化日志");
    log::info!("[ClassPaper] 日志系统初始化完成，日志级别: {:?}", log_level);
    // DPI感知
    winapi::set_dpi_aware();
    log::info!("[ClassPaper] DPI 感知已设置");
    let config = parse_config();
    let url = normalize_url(&config.default.url);
    log::info!("[ClassPaper][加载配置URL] {}", url);
    let window_name = format!("classpaper{}", generate_random_string(6));
    let app_state = Arc::new(Mutex::new(AppState {
        window: None,
        window_name: window_name.clone(),
        settings_windows: Vec::new(),
    }));
    // ctrlc 优雅退出
    let app_state_ctrlc = Arc::clone(&app_state);
    let app_state_ctrlc2 = Arc::clone(&app_state_ctrlc);
    ctrlc::set_handler(move || {
        log::warn!("[ClassPaper] 收到 Ctrl+C 信号，准备优雅退出");
        close_all_and_exit(&app_state_ctrlc2);
    })
    .expect("设置 ctrlc 失败");
    let mut tray = TrayItem::new("ClassPaper", tray_item::IconSource::Resource("IDI_ICON1"))
        .expect("无法创建系统托盘");
    log::info!("[ClassPaper] 托盘已创建");
    let app_state_reload = Arc::clone(&app_state);
    tray.add_menu_item("重载网页", move || {
        log::info!("[托盘] 点击了重载网页");
        let state = app_state_reload.lock().unwrap();
        if let Some(ref window) = state.window {
            let _ = window.eval("location.reload(true)");
            log::debug!("[托盘] 已请求主窗口重载");
        }
    })
    .expect("无法添加重载菜单项");
    let app_state_penetration = Arc::clone(&app_state);
    tray.add_menu_item("设置程序桌面穿透", move || {
        log::info!("[托盘] 点击了桌面穿透");
        let state = app_state_penetration.lock().unwrap();
        winapi::setup_wallpaper(&state.window_name);
        log::debug!("[托盘] 已请求设置桌面穿透");
    })
    .expect("无法添加穿透菜单项");
    let app_state_restart = Arc::clone(&app_state);
    tray.add_menu_item("重启网页显示程序", move || {
        log::info!("[托盘] 点击了重启网页显示程序");
        let config = parse_config();
        let url = normalize_url(&config.default.url);
        let browser_path = config.default.browser_path.clone();
        let mut state = app_state_restart.lock().unwrap();
        if let Some(ref window) = state.window {
            log::info!("[ClassPaper] 关闭旧主窗口");
            window.close_blocking(3000);
        }
        let new_window = Arc::new(create_window(&url, &state.window_name, &browser_path));
        log::info!("[ClassPaper] 新主窗口已创建");
        state.window = Some(new_window.clone());
        winapi::setup_wallpaper(&state.window_name);
        log::debug!("[托盘] 已请求重启网页显示程序并设置桌面穿透");
    })
    .expect("无法添加重启菜单项");
    let app_state_settings = Arc::clone(&app_state);
    tray.add_menu_item("设置", move || {
        log::info!("[托盘] 点击了设置");
        open_settings_window(app_state_settings.clone());
        log::debug!("[托盘] 已请求打开设置窗口");
    })
    .expect("无法添加设置菜单项");
    let _app_state_restart_app = Arc::clone(&app_state);
    tray.add_menu_item("重启程序", move || {
        log::warn!("[托盘] 点击了重启主程序");
        if let Ok(exec_path) = std::env::current_exe() {
            let _ = Command::new(exec_path).spawn();
            log::info!("[托盘] 已请求重启主程序");
        }
        std::process::exit(0);
    }).expect("无法添加重启程序菜单项");
    let app_state_quit = Arc::clone(&app_state);
    let app_state_quit2 = Arc::clone(&app_state_quit);
    tray.add_menu_item("退出程序", move || {
        log::warn!("[托盘] 点击了退出程序");
        close_all_and_exit(&app_state_quit2);
    })
    .expect("无法添加退出菜单项");
    let window = Arc::new(create_window(
        &url,
        &window_name,
        &config.default.browser_path,
    ));
    log::info!("[ClassPaper] 主窗口已创建: {}", window_name);
    let mut state = app_state.lock().unwrap();
    state.window = Some(window);
    drop(state);
    thread::sleep(std::time::Duration::from_millis(300));
    let state = app_state.lock().unwrap();
    winapi::setup_wallpaper(&state.window_name);
    drop(state);
    log::info!("[ClassPaper] 桌面穿透已设置");
    std::thread::park();
    log::info!("[ClassPaper] 主线程即将退出，准备关闭所有窗口");
    close_all_and_exit(&app_state);
}

// 日志轮转说明：
// simplelog 不支持日志轮转，如需自动分割日志、保留天数、压缩等，请用 flexi_logger 替换。
// 示例：
// use flexi_logger::{Logger, Criterion, Naming, Cleanup};
// Logger::try_with_str("info")?.log_to_file().directory("log_files").rotate(
//     Criterion::Size(10_000_000), Naming::Numbers, Cleanup::KeepLogFiles(7),
// ).start()?;
// ---
