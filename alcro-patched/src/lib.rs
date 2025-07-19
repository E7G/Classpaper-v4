//! # Alcro
//!
//! Alcro is a library to create desktop apps using rust and modern web technologies.
//! It uses the existing chrome installation for the UI.
//!
//! # Example
//!
//! ```no_run
//! #![windows_subsystem = "windows"]
//! use alcro::{UIBuilder, Content};
//! use serde_json::to_value;
//!
//! let ui = UIBuilder::new().content(Content::Html("<html><body>Close Me!</body></html>")).run().expect("Unable to launch");
//! assert_eq!(ui.eval("document.body.innerText").unwrap(), "Close Me!");
//!
//! //Expose rust function to js
//! ui.bind("product",|args| {
//!     let mut product = 1;
//!     for arg in args {
//!         match arg.as_i64() {
//!             Some(i) => product*=i,
//!             None => return Err(to_value("Not number").unwrap())
//!         }
//!     }
//!     Ok(to_value(product).unwrap())
//! }).expect("Unable to bind function");
//!
//! assert_eq!(ui.eval("(async () => await product(1,2,3))();").unwrap(), 6);
//! assert!(ui.eval("(async () => await product(1,2,'hi'))();").is_err());
//! ui.wait_finish();
//! ```
//!
//! To change the path of the browser launched set the ALCRO_BROWSER_PATH environment variable. Only Chromium based browsers work.
//!

mod chrome;
#[cfg(target_family = "windows")]
use chrome::close_handle;
use chrome::{bind, bounds, close, eval, load, load_css, load_js, set_bounds, Chrome};
pub use chrome::{BindingContext, Bounds, JSError, JSObject, JSResult, WindowState};
mod locate;
pub use locate::tinyfiledialogs as dialog;
use locate::{locate_chrome, LocateChromeError};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tempfile;

const DEFAULT_CHROME_ARGS: &[&str] = &[
    "--disable-background-networking",
    "--disable-background-timer-throttling",
    "--disable-backgrounding-occluded-windows",
    "--disable-breakpad",
    "--disable-client-side-phishing-detection",
    "--disable-default-apps",
    "--disable-dev-shm-usage",
    "--disable-infobars",
    "--disable-extensions",
    "--disable-features=site-per-process",
    "--disable-hang-monitor",
    "--disable-ipc-flooding-protection",
    "--disable-popup-blocking",
    "--disable-prompt-on-repost",
    "--disable-renderer-backgrounding",
    "--disable-sync",
    "--disable-translate",
    "--disable-windows10-custom-titlebar",
    "--metrics-recording-only",
    "--no-first-run",
    "--no-default-browser-check",
    "--safebrowsing-disable-auto-update",
    "--password-store=basic",
    "--use-mock-keychain",
];

/// The browser window
pub struct UI {
    chrome: Arc<Chrome>,
    _tmpdir: Option<tempfile::TempDir>,
    waited: AtomicBool,
}

/// Error in launching a UI window
#[derive(Debug, thiserror::Error)]
pub enum UILaunchError {
    /// Cannot create temporary directory
    #[error("Cannot create temporary directory: {0}")]
    TempDirectoryCreationError(#[from] std::io::Error),
    /// The path specified by ALCRO_BROWSER_PATH does not exist
    #[error("The path {0} specified by ALCRO_BROWSER_PATH does not exist")]
    BrowserPathInvalid(String),
    /// Error in locating chrome
    #[error("Error in locating chrome: {0}")]
    LocateChromeError(#[from] LocateChromeError),
    /// Error when initializing chrome
    #[error("Error when initializing chrome: {0}")]
    ChromeInitError(#[from] JSError),
}

impl UI {
    /// Returns true if the browser is closed
    pub fn done(&self) -> bool {
        self.chrome.done()
    }

    /// Wait for the browser to be closed
    pub fn wait_finish(&self) {
        self.chrome.wait_finish();
        self.waited.store(true, Ordering::Relaxed);
    }

    /// Close the browser window
    pub fn close(&self) {
        close(self.chrome.clone())
    }

    /// 关闭窗口并阻塞直到窗口真正关闭（或超时）
    pub fn close_blocking(&self, timeout_ms: u64) {
        use std::time::{Duration, Instant};
        println!("[alcro] close_blocking: 开始关闭窗口，超时时间 {}ms", timeout_ms);
        self.close();
        let start = Instant::now();
        while !self.done() {
            println!("[alcro] close_blocking: 检查窗口是否关闭... (elapsed {}ms)", start.elapsed().as_millis());
            if start.elapsed() > Duration::from_millis(timeout_ms) {
                eprintln!("[alcro] close_blocking 超时: {}ms", timeout_ms);
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        if self.done() {
            println!("[alcro] close_blocking: 窗口已关闭 (耗时 {}ms)", start.elapsed().as_millis());
        } else {
            eprintln!("[alcro] close_blocking: 关闭失败，窗口未关闭 (耗时 {}ms)", start.elapsed().as_millis());
        }
    }

    /// Load content in the browser. It returns Err if it fails.
    pub fn load(&self, content: Content) -> Result<(), JSError> {
        let html: String;
        let url = match content {
            Content::Url(u) => u,
            Content::Html(h) => {
                html = format!("data:text/html,{}", h);
                &html
            }
        };
        load(self.chrome.clone(), url)
    }

    /// Bind a rust function so that JS code can use it. It returns Err if it fails.
    /// The rust function will be executed in a new thread and can be called asynchronously from Javascript
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the function
    /// * `f` - The function. It should take a list of `JSObject` as argument and return a `JSResult`
    ///
    /// # Examples
    ///
    /// ```
    /// #![windows_subsystem = "windows"]
    /// use alcro::UIBuilder;
    /// use serde_json::to_value;
    ///
    /// let ui = UIBuilder::new().custom_args(&["--headless"]).run().expect("Unable to launch");
    /// ui.bind("add",|args| {
    ///     let mut sum = 0;
    ///     for arg in args {
    ///         match arg.as_i64() {
    ///             Some(i) => sum+=i,
    ///             None => return Err(to_value("Not number").unwrap())
    ///         }
    ///     }
    ///     Ok(to_value(sum).unwrap())
    /// }).expect("Unable to bind function");
    /// assert_eq!(ui.eval("(async () => await add(1,2,3))();").unwrap(), 6);
    /// assert!(ui.eval("(async () => await add(1,2,'hi'))();").is_err());
    /// ```
    pub fn bind<F>(&self, name: &str, f: F) -> Result<(), JSError>
    where
        F: Fn(&[JSObject]) -> JSResult + Sync + Send + 'static,
    {
        let f = Arc::new(f);
        bind(
            self.chrome.clone(),
            name,
            Arc::new(move |context| {
                let f = f.clone();
                std::thread::spawn(move || {
                    let result = f(context.args());
                    context.complete(result);
                });
            }),
        )
    }

    /// Bind a rust function callable from JS that can complete asynchronously. If you are using
    /// [`tokio`], you will probably want to be using [`Self::bind_tokio()`] instead.
    ///
    /// Unlike `bind()`, this passes ownership of the arguments to the callback function `f`, and
    /// allows completing the javascript implementation after returning from `f`. This makes async
    /// behavior much simpler to implement.
    ///
    /// For efficency, `f` will be executed in the message processing loop, and therefore should
    /// avoid blocking by moving work onto another thread, for example with an async runtime
    /// spawn method.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the function
    /// * `f` - The function. It should take a [`BindingContext`] that gives access to the
    ///         arguments and allows returning results.
    ///
    /// # Examples
    ///
    /// `bind()` approximately performs the following:
    ///
    /// ```
    /// #![windows_subsystem = "windows"]
    /// use alcro::UIBuilder;
    /// use serde_json::to_value;
    ///
    /// let ui = UIBuilder::new().custom_args(&["--headless"]).run().expect("Unable to launch");
    /// ui.bind_async("add", |context| {
    ///     std::thread::spawn(|| {
    ///         // imagine this is very expensive, or hits a network...
    ///         let mut sum = 0;
    ///         for arg in context.args() {
    ///             match arg.as_i64() {
    ///                 Some(i) => sum+=i,
    ///                 None => return context.err(to_value("Not number").unwrap())
    ///             }
    ///         }
    ///
    ///         context.complete(Ok(to_value(sum).unwrap()));
    ///     });
    /// }).expect("Unable to bind function");
    /// assert_eq!(ui.eval("(async () => await add(1,2,3))();").unwrap(), 6);
    /// assert!(ui.eval("(async () => await add(1,2,'hi'))();").is_err());
    /// ```
    pub fn bind_async<F>(&self, name: &str, f: F) -> Result<(), JSError>
    where
        F: Fn(BindingContext) + Sync + Send + 'static,
    {
        bind(self.chrome.clone(), name, Arc::new(f))
    }

    /// Bind a rust function callable from JS that can complete asynchronously, using the [`tokio`]
    /// runtime to wrap `bind_async()`, making usage more ergonomic for `tokio` users.
    ///
    /// The callback is closer to `bind()` than `bind_async()` in that you take the JS arguments
    /// and return the JS result, the main difference is that the arguments are passed by value
    /// and the result is a [`Future`].
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the function
    /// * `f` - The function. It should take a [`Vec`] of [`JSObject`] arguments by value, and
    ///         return a [`Future`] for the [`JSResult`] (generally, by using an `async move`
    ///         block body)
    ///
    /// # Examples
    ///
    /// ```
    /// #![windows_subsystem = "windows"]
    /// use alcro::UIBuilder;
    /// use serde_json::to_value;
    ///
    /// # fn main() {
    /// #   // Ensure a tokio runtime is active for the test. A user will probably be using
    /// #   // #[tokio::main] instead, which doesn't work in doctests.
    /// #   let rt = tokio::runtime::Runtime::new().unwrap();
    /// #   let _guard = rt.enter();
    /// let ui = UIBuilder::new().custom_args(&["--headless"]).run().expect("Unable to launch");
    /// ui.bind_tokio("add", |args| async move {
    ///     // imagine this is very expensive, or hits a network...
    ///     let mut sum = 0;
    ///     for arg in &args {
    ///         match arg.as_i64() {
    ///             Some(i) => sum+=i,
    ///             None => return Err(to_value("Not number").unwrap())
    ///         }
    ///     }
    ///
    ///     Ok(to_value(sum).unwrap())
    /// }).expect("Unable to bind function");
    /// assert_eq!(ui.eval("(async () => await add(1,2,3))();").unwrap(), 6);
    /// assert!(ui.eval("(async () => await add(1,2,'hi'))();").is_err());
    /// # }
    /// ```
    ///
    /// [`Future`]: std::future::Future
    #[cfg(feature = "tokio")]
    pub fn bind_tokio<F, R>(&self, name: &str, f: F) -> Result<(), JSError>
    where
        F: Fn(Vec<JSObject>) -> R + Send + Sync + 'static,
        R: std::future::Future<Output = JSResult> + Send + 'static,
    {
        // Capture the callers runtime, as using tokio::spawn() inside the binding function
        // will fail as the message processing loop does not have a runtime registered.
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|err| JSError::from(JSObject::String(err.to_string())))?;

        self.bind_async(name, move |context| {
            // Create future outside the spawn, avoiding the async block capturing `f`, which
            // would require cloning it. This is fine as futures must not have side effects until
            // polled. For async fn, this means no user code gets run until the await.
            let fut = f(context.args().to_vec());
            runtime.spawn(async move {
                let result = fut.await;
                context.complete(result);
            });
        })
    }

    /// Evaluates js code and returns the result.
    ///
    /// # Examples
    ///
    /// ```
    /// #![windows_subsystem = "windows"]
    /// use alcro::UIBuilder;
    /// let ui = UIBuilder::new().custom_args(&["--headless"]).run().expect("Unable to launch");
    /// assert_eq!(ui.eval("1+1").unwrap(), 2);
    /// assert_eq!(ui.eval("'Hello'+' World'").unwrap(), "Hello World");
    /// assert!(ui.eval("xfgch").is_err());
    /// ```

    pub fn eval(&self, js: &str) -> JSResult {
        eval(self.chrome.clone(), js)
    }

    /// Evaluates js code and adds functions before document loads. Loaded js is unloaded on reload.
    ///
    /// # Arguments
    ///
    /// * `script` - Javascript that should be loaded
    ///
    /// # Examples
    ///
    /// ```
    /// #![windows_subsystem = "windows"]
    /// use alcro::UIBuilder;
    /// let ui = UIBuilder::new().custom_args(&["--headless"]).run().expect("Unable to launch");
    /// ui.load_js("function loadedFunction() { return 'This function was loaded from rust'; }").expect("Unable to load js");
    /// assert_eq!(ui.eval("loadedFunction()").unwrap(), "This function was loaded from rust");
    /// ```

    pub fn load_js(&self, script: &str) -> Result<(), JSError> {
        load_js(self.chrome.clone(), script)
    }

    /// Loads CSS into current window. Loaded CSS is unloaded on reload.
    ///
    /// # Arguments
    ///
    /// * `css` - CSS that should be loaded
    ///
    /// # Examples
    ///
    /// ```
    /// #![windows_subsystem = "windows"]
    /// use alcro::UIBuilder;
    /// let ui = UIBuilder::new().custom_args(&["--headless"]).run().expect("Unable to launch");
    /// ui.load_css("body {display: none;}").expect("Unable to load css");
    /// ```

    pub fn load_css(&self, css: &str) -> Result<(), JSError> {
        load_css(self.chrome.clone(), css)
    }

    /// It changes the size, position or state of the browser window specified by the `Bounds` struct. It returns Err if it fails.
    ///
    /// To change the window state alone use `WindowState::to_bounds()`
    pub fn set_bounds(&self, b: Bounds) -> Result<(), JSError> {
        set_bounds(self.chrome.clone(), b)
    }

    /// It gets the size, position and state of the browser window. It returns Err if it fails.
    pub fn bounds(&self) -> Result<Bounds, JSObject> {
        bounds(self.chrome.clone())
    }
}

/// Closes the browser window
impl Drop for UI {
    fn drop(&mut self) {
        if !self.waited.load(Ordering::Relaxed) && !self.done() {
            self.close();
            self.wait_finish();
        }
        #[cfg(target_family = "windows")]
        close_handle(self.chrome.clone());
        if let Some(tmpdir) = self._tmpdir.take() {
            let _ = tmpdir.close();
        }
    }
}

/// Specifies the type of content shown by the browser
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Content<'a> {
    /// The URL
    Url(&'a str),
    /// HTML text
    Html(&'a str),
}

/// Builder for constructing a UI instance.
pub struct UIBuilder<'a> {
    content: Content<'a>,
    dir: Option<&'a std::path::Path>,
    width: i32,
    height: i32,
    custom_args: &'a [&'a str],
    browser_path: Option<&'a str>,
}

impl<'a> Default for UIBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> UIBuilder<'a> {
    /// Default UI
    pub fn new() -> Self {
        UIBuilder {
            content: Content::Html(""),
            dir: None,
            width: 0, // 0表示不指定
            height: 0,
            custom_args: &[],
            browser_path: None,
        }
    }

    pub fn browser_path(&mut self, path: &'a str) -> &mut Self {
        self.browser_path = Some(path);
        self
    }

    /// Return the UI instance. It returns the Err variant if any error occurs.
    pub fn run(&self) -> Result<UI, UILaunchError> {
        let html: String;
        let url = match self.content {
            Content::Url(u) => u,
            Content::Html(h) => {
                html = format!("data:text/html,{}", h);
                &html
            }
        };
        // 修改chrome_path获取逻辑
        let chrome_path = if let Some(path) = self.browser_path {
            if std::fs::metadata(path).is_ok() {
                path.to_string()
            } else {
                return Err(UILaunchError::BrowserPathInvalid(path.to_string()));
            }
        } else {
            match std::env::var("ALCRO_BROWSER_PATH") {
                Ok(path) => {
                    if std::fs::metadata(&path).is_ok() {
                        path
                    } else {
                        return Err(UILaunchError::BrowserPathInvalid(path));
                    }
                }
                Err(_) => locate_chrome()?,
            }
        };
        let mut args: Vec<String> = DEFAULT_CHROME_ARGS.iter().map(|s| s.to_string()).collect();
        // 自动生成 user-data-dir
        let tmpdir = tempfile::TempDir::new()?;
        let user_data_arg = format!("--user-data-dir={}", tmpdir.path().to_string_lossy());
        args.push(user_data_arg);
        // 只有width和height都大于0时才加--window-size参数
        if self.width > 0 && self.height > 0 {
            let window_size_arg = format!("--window-size={},{}", self.width, self.height);
            args.push(window_size_arg);
        }
        for arg in self.custom_args {
            args.push(arg.to_string())
        }
        args.push("--remote-debugging-pipe".to_string());
        // kiosk模式：不加--app，直接加url
        if self.custom_args.iter().any(|&a| a == "--kiosk") {
            args.push(url.to_string());
        } else if self.custom_args.contains(&"--headless") {
            args.push(url.to_string());
        } else {
            let app_arg = format!("--app={}", url);
            args.push(app_arg);
        }
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        // println!("\n========== [alcro] 启动浏览器 ==========");
        // println!("[alcro] chrome_path: {}", chrome_path);
        // println!("[alcro] chrome args:");
        // for arg in args_ref.iter() {
        //     println!("    {}", arg);
        // }
        // println!("========================================\n");
        let chrome = Chrome::new_with_args(&chrome_path, &args_ref)?;
        Ok(UI {
            chrome,
            _tmpdir: Some(tmpdir),
            waited: AtomicBool::new(false),
        })
    }

    /// Set the content (url or html text)
    pub fn content(&mut self, content: Content<'a>) -> &mut Self {
        self.content = content;
        self
    }

    /// Set the user data directory. By default it is a temporary directory.
    pub fn user_data_dir(&mut self, dir: &'a std::path::Path) -> &mut Self {
        self.dir = Some(dir);
        self
    }

    /// Set the window size
    pub fn size(&mut self, width: i32, height: i32) -> &mut Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Add custom arguments to spawn chrome with
    pub fn custom_args(&mut self, custom_args: &'a [&'a str]) -> &mut Self {
        self.custom_args = custom_args;
        self
    }
}
