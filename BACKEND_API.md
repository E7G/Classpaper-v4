# 🦀 后端开发文档 - Classpaper v4

## 📋 项目概述

Classpaper v4 后端是一个基于 Rust 的系统托盘应用，负责管理 Web 前端窗口、系统托盘交互、桌面壁纸穿透等核心功能。使用 alcro 框架实现 WebView 窗口管理。

## 🏗️ 架构设计

### 整体架构
```
┌─────────────────────────────────────────┐
│              Rust Backend              │
│  ┌─────────────────────────────────────┐ │
│  │          main.rs                    │ │
│  │  ┌─────────────┐  ┌───────────────┐ │ │
│  │  │ 系统托盘    │  │ 窗口管理      │ │ │
│  │  │ TrayItem    │  │ alcro::UI     │ │ │
│  │  └─────────────┘  └───────────────┘ │ │
│  │  ┌─────────────────────────────────┐ │ │
│  │  │  Windows API 集成               │ │ │
│  │  │  winapi.rs                      │ │ │
│  │  │  ┌──────────┐  ┌──────────────┐ │ │ │
│  │  │  │ 桌面穿透 │  │ 配置管理     │ │ │ │
│  │  │  │ Win32 API│  │ TOML解析     │ │ │ │
│  │  │  └──────────┘  └──────────────┘ │ │ │
│  │  └─────────────────────────────────┘ │ │
│  └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

### 模块结构
```
src/
├── main.rs      # 主程序入口，系统托盘逻辑
├── winapi.rs    # Windows API 集成，桌面穿透实现
└── lib.rs       # 库文件（预留）
```

## 🔧 核心功能模块

### 🎯 系统托盘管理 (main.rs)

#### 托盘菜单结构
```rust
// 托盘菜单项
- "打开设置" → 打开设置窗口
- "重新加载" → 刷新主窗口
- "退出" → 关闭所有窗口并退出
```

#### 配置管理
```rust
#[derive(Debug, Serialize, Deserialize)]
struct AppConfig {
    default: DefaultConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct DefaultConfig {
    url: String,           // 主窗口URL
    browser_path: String,  // 自定义浏览器路径
}
```

#### 核心函数
```rust
// 配置解析
fn parse_config() -> AppConfig

// 窗口创建
fn create_window(url: &str, window_name: &str, browser_path: &str) -> Result<UI, Box<dyn Error>>

// 设置窗口
fn open_settings_window(app_state: Arc<Mutex<AppState>>)

// 优雅退出
fn close_all_and_exit(app_state: &Arc<Mutex<AppState>>) -> !
```

### 🪟 Windows API 集成 (winapi.rs)

#### 桌面穿透系统

##### 兼容性方案
- **Win10/11 24H2+**: 新版本桌面穿透
- **Win7/8/10 早期版本**: 传统桌面穿透
- **自动检测**: 根据Windows版本选择合适方案

##### 核心API
```rust
// 桌面穿透主函数
pub fn setup_wallpaper_new(window_title: &str) -> bool

// 传统桌面穿透
pub fn set_desktop(hwnd: HWND)

// 窗口查找
pub fn find_window_by_title(title: &str) -> HWND

// 样式设置
pub fn setup_window_style(hwnd: HWND) -> bool
pub fn setup_transparency(hwnd: HWND) -> bool
pub fn setup_fullscreen_position(hwnd: HWND) -> bool
```

#### 系统信息获取
```rust
// 屏幕信息
pub fn get_screen_width() -> i32
pub fn get_screen_height() -> i32
pub fn set_dpi_aware() -> bool

// Windows版本检测
pub fn get_windows_version() -> (u32, u32, u32)
```

#### 错误处理
```rust
// 错误通知
pub fn show_error_notification(message: &str)
pub fn handle_window_creation_error(error: &str)
```

## 🔄 进程间通信 (IPC)

### JavaScript ↔ Rust 绑定

#### 前端可调用的Rust函数
```javascript
// 系统信息
getWidth() → number    // 屏幕宽度
getHeight() → number   // 屏幕高度

// 文件系统操作
readFile(path: string) → string     // 读取文件
writeFile(path: string, content: string) → boolean  // 写入文件
readDir(dir: string) → string[]     // 读取目录

// 设置相关
readConfig() → object    // 读取配置
saveConfig(config: object) → boolean  // 保存配置
scanWallpaperDir() → string[]  // 扫描壁纸
reloadMainWindow() → boolean   // 重新加载主窗口
openURLInBrowser(url: string) → boolean  // 用浏览器打开URL
```

#### 绑定实现示例
```rust
// 在 create_window 中绑定
ui.bind("getWidth", |_| Ok(winapi::get_screen_width().into()));
ui.bind("readFile", |args| {
    if let Some(path) = args.get(0).and_then(|v| v.as_str()) {
        match std::fs::read_to_string(path) {
            Ok(content) => Ok(serde_json::Value::String(content)),
            Err(e) => Err(format!("读取文件失败: {}", e).into()),
        }
    } else {
        Err("参数错误".into())
    }
});
```

## 📊 配置系统

### TOML 配置文件 (config.toml)
```toml
[Default]
URL = "./res/index.html"
BrowserPath = ""
```

### 配置自动迁移
- 支持旧格式自动升级到新格式
- 小写字段名自动转换为大写
- 缺失字段使用默认值

### 配置验证
- URL路径规范化
- 中文字符自动转码
- 本地文件路径处理

## 🛠️ 开发指南

### 快速开始

#### 1. 环境准备
```bash
# 安装Rust
rustup update stable

# 安装依赖
cargo build --release
```

#### 2. 配置修改
```toml
# config.toml
[Default]
URL = "./res/index.html"           # 主窗口路径
BrowserPath = "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe"  # 自定义浏览器
```

#### 3. 添加新功能

##### 添加新的托盘菜单项
```rust
// 在 main.rs 的 main() 函数中
let mut tray = TrayItem::new("ClassPaper", icon_path)?;

// 添加新菜单项
tray.add_menu_item("新功能", move || {
    // 你的逻辑
    println!("新功能被点击");
    Ok::<_, Box<dyn Error>>(())
})?;
```

##### 添加新的IPC函数
```rust
// 在 create_window 或 open_settings_window 中
ui.bind("myNewFunction", |args| {
    // 参数处理
    if let Some(param) = args.get(0).and_then(|v| v.as_str()) {
        // 你的逻辑
        Ok(format!("处理结果: {}", param).into())
    } else {
        Err("参数错误".into())
    }
});
```

### 调试技巧

#### 日志系统
```rust
// 初始化日志
CombinedLogger::init(vec![
    TermLogger::new(
        LevelFilter::Debug,
        ConfigBuilder::new().build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    ),
])
.expect("初始化日志失败");

// 使用日志
log::info!("应用启动");
log::warn!("配置文件读取失败: {}", e);
log::error!("严重错误: {}", error);
```

#### Windows调试
```rust
// 控制台输出
println!("[debug] 窗口句柄: {:?}", hwnd);

// Windows消息调试
unsafe {
    println!(
        "[winapi debug] 查找窗口标题: '{}', HWND: {:?}", 
        title, hwnd
    );
}
```

## 🎯 高级功能

### 自定义窗口行为

#### 窗口参数配置
```rust
let mut builder = UIBuilder::new();
builder
    .content(Content::Url(url))
    .custom_args(&[
        "--kiosk",
        "--autoplay-policy=no-user-gesture-required",
        "--disable-application-cache",
        "--disable-cache",
    ]);
```

#### 多窗口管理
```rust
// 管理多个设置窗口
struct AppState {
    window: Option<Arc<UI>>,           // 主窗口
    settings_windows: Vec<Arc<UI>>,    // 设置窗口列表
}

// 关闭所有窗口
fn close_all_and_exit(app_state: &Arc<Mutex<AppState>>) -> ! {
    let state = app_state.lock().unwrap();
    if let Some(ref window) = state.window {
        window.close_blocking(3000);
    }
    for win in &state.settings_windows {
        win.close_blocking(3000);
    }
    std::process::exit(0);
}
```

### 错误处理最佳实践

#### 用户友好的错误提示
```rust
// 显示系统通知
winapi::show_error_notification(&format!(
    "配置保存失败\n\n无法保存配置文件 config.toml\n\n错误信息: {}\n\n可能原因：\n• 文件权限不足\n• 文件被其他程序占用\n• 磁盘空间不足\n\n请检查文件权限或重启程序后再试。", 
    e
));
```

#### 错误恢复机制
```rust
// 配置文件读取失败时使用默认值
match std::fs::read_to_string("config.toml") {
    Ok(content) => parse_config_content(&content),
    Err(e) => {
        log::warn!("配置文件读取失败，使用默认值: {}", e);
        AppConfig::default()
    }
}
```

## 📦 构建与部署

### 构建命令
```bash
# 调试构建
cargo build

# 发布构建
cargo build --release

# 运行
cargo run --release
```

### 依赖管理
```toml
[dependencies]
# WebView框架
alcro = { path = "alcro-patched", version = "0.5.4" }

# 系统托盘
tray-item = "0.10.0"

# Windows API
winapi = { version = "0.3.9", features = ["winuser", "dwmapi"] }

# 配置解析
toml = "0.8"
serde = { version = "1.0", features = ["derive"] }

# 日志系统
log = "0.4"
simplelog = "0.12"
```

### Windows兼容性
```toml
# Windows 7+ 支持
[target.'cfg(target_family = "windows")'.dependencies]
oldwin-targets = { version = "0.1.1", features = ["win7", "yy-thunks", "vc-ltl5"] }
```

## 🔍 调试工具

### 系统信息检测
```rust
// 获取Windows版本
let (major, minor, build) = winapi::get_windows_version();
println!("Windows版本: {}.{}.{}", major, minor, build);

// 检测桌面穿透方案
let compatible = match (major, minor, build) {
    (10, 0, 19041..) => "Win10 24H2+ 新版本",
    (10, 0, _) => "Win10 传统版本",
    (6, 1, _) => "Windows 7",
    _ => "未知版本",
};
```

### 性能监控
```rust
// 内存使用监控
use std::process;
let memory_usage = process::id();
println!("进程ID: {}", memory_usage);

// 窗口句柄监控
let hwnd = winapi::find_window_by_title("ClassPaper");
if !hwnd.is_null() {
    println!("窗口句柄: {:?}", hwnd);
}
```

## 📞 技术支持

### 常见问题排查
1. **窗口不显示**: 检查URL路径是否正确
2. **桌面穿透失败**: 检查Windows版本兼容性
3. **托盘图标不显示**: 检查图标文件路径
4. **配置文件错误**: 检查TOML格式是否正确

### 调试环境
- 操作系统: Windows 7/8/10/11
- Rust版本: 1.70+
- 构建工具: cargo
- 调试工具: Visual Studio Code + Rust插件

---

💡 **提示**: 所有后端API都通过IPC暴露给前端，可在浏览器控制台直接调用测试！