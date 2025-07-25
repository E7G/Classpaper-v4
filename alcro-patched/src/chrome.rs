use std::{
    fmt::Display,
    sync::{Arc, Mutex},
};

use crossbeam_channel::{bounded, Sender};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::atomic::{AtomicI32, Ordering};

mod devtools;
use devtools::{readloop, recv_msg, send, send_msg};
mod os;
#[cfg(target_family = "windows")]
use os::close_process_handle;
#[cfg(target_family = "unix")]
use os::kill_proc;
use os::{exited, new_process, wait_proc, PipeReader, PipeWriter, Process};

/// A JS object. It is an alias for `serde_json::Value`. See it's documentation for how to use it.
pub type JSObject = serde_json::Value;
/// The result of a JS function.
///
/// The Err variant will be returned if
/// * There is an exception
/// * An error type is returned
pub type JSResult = Result<JSObject, JSObject>;

/// An error from chrome in JSON format
#[derive(Debug)]
pub struct JSError(JSObject);
impl JSError {
    pub fn source(self) -> JSObject {
        self.0
    }
}
impl std::error::Error for JSError {}
impl Display for JSError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl From<JSObject> for JSError {
    fn from(o: JSObject) -> Self {
        Self(o)
    }
}

trait ToResultOfJSError {
    fn to_result_of_jserror(self) -> Result<(), JSError>;
}
impl ToResultOfJSError for JSResult {
    fn to_result_of_jserror(self) -> Result<(), JSError> {
        match self {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

/// Context for an async binding function.
pub struct BindingContext {
    active: Option<ActiveBindingContext>,
}

impl BindingContext {
    fn new(active: ActiveBindingContext) -> Self {
        Self {
            active: Some(active),
        }
    }

    /// The arguments from JS.
    pub fn args(&self) -> &[JSObject] {
        match &self.active {
            None => &[],
            Some(active) => active.payload["args"].as_array().expect("Expected array"),
        }
    }

    /// Completes the JS function successfully. Equivalent to `complete(Ok(result))`
    pub fn done(self, result: JSObject) {
        self.complete(Ok(result))
    }

    /// Completes the JS function with an error. Equivalent to `complete(Err(error))`
    pub fn err(self, error: JSObject) {
        self.complete(Err(error))
    }

    /// Completes the JS function, either successfully or not. Takes the [`BindingContext`] by
    /// value as it releases the outstanding call on the Chrome(ium) side.
    pub fn complete(mut self, result: JSResult) {
        if let Some(incomplete) = self.active.take() {
            complete_binding(incomplete, result)
        }
    }
}

impl Drop for BindingContext {
    fn drop(&mut self) {
        if let Some(incomplete) = self.active.take() {
            complete_binding(incomplete, Ok(JSObject::Null))
        }
    }
}

struct ActiveBindingContext {
    chrome: Arc<Chrome>,
    payload: JSObject,
    context_id: i64,
}

type BindingFunc = Arc<dyn Fn(BindingContext) + Sync + Send>;

pub struct Chrome {
    id: AtomicI32,
    #[cfg(target_family = "unix")]
    pid: Process,
    #[cfg(target_family = "windows")]
    pid: usize,
    psend: Mutex<PipeWriter>,
    precv: Mutex<PipeReader>,
    target: String,
    session: String,
    _kill_send: Sender<()>,
    pending: dashmap::DashMap<i32, Sender<JSResult>>,
    window: AtomicI32,
    bindings: dashmap::DashMap<String, BindingFunc>,
}

/// A struct that stores the size, position and window state of the browser window.

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Bounds {
    /// x coordinate of the window
    pub left: i32,
    /// y coordinate of the window
    pub top: i32,
    /// width of the window
    pub width: i32,
    /// height of the window
    pub height: i32,
    pub window_state: WindowState,
}

/// The state of the window
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum WindowState {
    Normal,
    Maximized,
    Minimized,
    Fullscreen,
}

impl WindowState {
    /// Convert to Bounds struct
    pub fn to_bounds(self) -> Bounds {
        Bounds {
            height: 0,
            width: 0,
            top: 0,
            left: 0,
            window_state: self,
        }
    }
}

impl Chrome {
    pub fn new_with_args(chrome_binary: &str, args: &[&str]) -> Result<Arc<Chrome>, JSError> {
        println!("\n========== [alcro-debug] 启动浏览器 ==========");
        println!("[alcro-debug] 浏览器路径: {}", chrome_binary);
        println!("[alcro-debug] 启动参数:");
        for arg in args.iter() {
            println!("    {}", arg);
        }
        println!("==========================================\n");
        let (pid, precv, psend) =
            new_process(chrome_binary, &args).expect("Unable to launch chrome");
        let (_kill_send, _kill_recv) = bounded(1);

        let mut c = Chrome {
            id: AtomicI32::new(2),
            precv: Mutex::new(precv),
            psend: Mutex::new(psend),
            target: String::new(),
            session: String::new(),
            _kill_send,
            pending: dashmap::DashMap::new(),
            bindings: dashmap::DashMap::new(),
            window: AtomicI32::new(0),
            #[cfg(target_family = "windows")]
            pid: pid as usize,
            #[cfg(target_family = "unix")]
            pid: pid,
        };

        // 只保留关键日志
        // println!("[alcro-debug] => 调用 find_target...");
        c.target = c.find_target();
        println!("[alcro-debug] Target found: {}", c.target);
        // println!("[alcro-debug] => 调用 start_session...");
        c.session = c.start_session()?;
        println!("[alcro-debug] Session started: {}", c.session);

        let c_arc = Arc::new(c);

        #[cfg(target_family = "unix")]
        std::thread::spawn(move || {
            _kill_recv.recv().unwrap();
            kill_proc(pid).expect("Unable to kill process");
        });

        let c_arc_clone = c_arc.clone();
        std::thread::spawn(move || readloop(c_arc_clone));

        for (method, args) in [
            ("Page.enable", JSObject::Null),
            (
                "Target.setAutoAttach",
                json!({"autoAttach": true, "waitForDebuggerOnStart": false}),
            ),
            ("Network.enable", JSObject::Null),
            ("Runtime.enable", JSObject::Null),
            ("Security.enable", JSObject::Null),
            ("Performance.enable", JSObject::Null),
            ("Log.enable", JSObject::Null),
            ("DOM.enable", JSObject::Null),
            ("CSS.enable", JSObject::Null),
        ]
        .iter()
        {
            // 只保留 Target.attachToTarget 关键日志
            if *method == "Target.attachToTarget" {
                println!("[alcro-debug] 发送方法: {}", method);
            }
            send(Arc::clone(&c_arc), method, args)?;
        }

        if !args.contains(&"--headless") {
            // 只保留 Target.targetDestroyed 关键日志
            println!("[alcro-debug] 获取窗口ID...");
            let win_id = get_window_for_target(Arc::clone(&c_arc))?;
            Arc::clone(&c_arc).window.store(win_id, Ordering::Relaxed);
            println!("[alcro-debug] Window id: {}", win_id);
        }
        Ok(c_arc)
    }

    fn find_target(&self) -> String {
        // 只保留 Target.setDiscoverTargets 关键日志
        println!("[alcro-debug] 发送 Target.setDiscoverTargets...");
        send_msg(
            &self.psend,
            json!(
            {
            "id": 0,
            "method": "Target.setDiscoverTargets",
            "params": { "discover": true }
            }
            )
            .to_string(),
        );

        loop {
            let raw = recv_msg(&self.precv);
            // 只保留 Target.targetDestroyed 关键日志
            // println!("    [alcro-debug] find_target 收到: {}", raw);
            let pmsg: JSObject = match serde_json::from_str(&raw) {
                Ok(obj) => obj,
                Err(_) => {
                    break;
                }
            };
            if pmsg["method"] == "Target.targetCreated" {
                let params = &pmsg["params"];
                if params["targetInfo"]["type"] == "page" {
                    return params["targetInfo"]["targetId"]
                        .as_str()
                        .expect("Value not of string datatype")
                        .to_string();
                }
            }
        }
        String::new()
    }

    fn start_session(&self) -> Result<String, JSError> {
        // 只保留 Target.attachToTarget 关键日志
        println!("[alcro-debug] 发送 Target.attachToTarget...");
        send_msg(
            &self.psend,
            json!(
            {
            "id": 1,
            "method": "Target.attachToTarget",
            "params": {"targetId": self.target}
            }
            )
            .to_string(),
        );

        loop {
            let raw = recv_msg(&self.precv);
            let pmsg: JSObject = match serde_json::from_str(&raw) {
                Ok(obj) => obj,
                Err(_) => {
                    break;
                }
            };
            if pmsg["id"] == 1 {
                if pmsg["error"] != JSObject::Null {
                    return Err(pmsg["error"].clone().into());
                }
                let session = &pmsg["result"];
                return Ok(session["sessionId"]
                    .as_str()
                    .expect("Value not of string datatype")
                    .to_string());
            }
        }
        Err(JSError(json!({"error": "start_session failed"})))
    }

    pub fn done(&self) -> bool {
        exited(self.pid as Process).expect("Error in getting process state")
    }

    pub fn wait_finish(&self) {
        wait_proc(self.pid as Process).expect("Error in waiting for process")
    }
}

fn get_window_for_target(c: Arc<Chrome>) -> Result<i32, JSObject> {
    match send(
        Arc::clone(&c),
        "Browser.getWindowForTarget",
        &json!({
            "targetId": c.target
        }),
    ) {
        Ok(v) => Ok(v["windowId"].as_i64().expect("Value not i64") as i32),
        Err(e) => Err(e),
    }
}

pub fn load(c: Arc<Chrome>, url: &str) -> Result<(), JSError> {
    send(Arc::clone(&c), "Page.navigate", &json!({ "url": url })).to_result_of_jserror()
}

pub fn eval(c: Arc<Chrome>, expr: &str) -> JSResult {
    send(
        c,
        "Runtime.evaluate",
        &json!({
            "expression": expr, "awaitPromise": true, "returnByValue": true
        }),
    )
}

pub fn set_bounds(c: Arc<Chrome>, b: Bounds) -> Result<(), JSError> {
    let param = json!({
        "windowId": c.window,
        "bounds": if b.window_state != WindowState::Normal {
            json!({
                "windowState":b.window_state
            })
        }else {
            serde_json::to_value(b).unwrap()
        }
    });
    send(c, "Browser.setWindowBounds", &param).to_result_of_jserror()
}

pub fn bounds(c: Arc<Chrome>) -> Result<Bounds, JSObject> {
    match send(
        Arc::clone(&c),
        "Browser.getWindowBounds",
        &json!({
            "windowId": c.window.load(Ordering::Relaxed)
        }),
    ) {
        Err(e) => Err(e),
        Ok(result) => {
            let ret: Bounds = serde_json::from_value(result["bounds"].clone())
                .expect("Value not of bounds datatype");
            Ok(ret)
        }
    }
}

pub fn load_js(c: Arc<Chrome>, script: &str) -> Result<(), JSError> {
    if let Err(e) = send(
        Arc::clone(&c),
        "Page.addScriptToEvaluateOnNewDocument",
        &json!({ "source": script }),
    ) {
        return Err(e.into());
    }
    eval(Arc::clone(&c), &script).to_result_of_jserror()
}

pub fn load_css(c: Arc<Chrome>, css: &str) -> Result<(), JSError> {
    let frame_tree = match send(
        Arc::clone(&c),
        "Page.getFrameTree",
        &json!({ "targetId": c.target }),
    ) {
        Ok(ft) => ft,
        Err(e) => return Err(e.into()),
    };
    let frame_id = frame_tree["frameTree"]["frame"]["id"].as_str().unwrap();
    let style_sheet = match send(
        Arc::clone(&c),
        "CSS.createStyleSheet",
        &json!({ "frameId": frame_id }),
    ) {
        Ok(ss) => ss,
        Err(e) => return Err(e.into()),
    };
    let style_sheet_id = style_sheet["styleSheetId"].as_str().unwrap();
    send(
        Arc::clone(&c),
        "CSS.setStyleSheetText",
        &json!({ "styleSheetId": style_sheet_id, "text": css }),
    )
    .to_result_of_jserror()
}

pub fn bind(c: Arc<Chrome>, name: &str, f: BindingFunc) -> Result<(), JSError> {
    c.bindings.insert(name.to_string(), f);

    if let Err(e) = send(
        Arc::clone(&c),
        "Runtime.addBinding",
        &json!({ "name": name }),
    ) {
        return Err(e.into());
    }

    let script = format!(
        r"(()=>{{
        const bindingName = '{name}';
        const binding = window[bindingName];
        window[bindingName] = async (...args) => {{
            const me = window[bindingName];
            let errors = me['errors'];
            let callbacks = me['callbacks'];
            if (!callbacks) {{
                callbacks = new Map();
                me['callbacks'] = callbacks;
            }}
            if (!errors) {{
                errors = new Map();
                me['errors'] = errors;
            }}
            const seq = (me['lastSeq'] || 0) + 1;
            me['lastSeq'] = seq;
            const promise = new Promise((resolve, reject) => {{
                callbacks.set(seq, resolve);
                errors.set(seq, reject);
            }});
            binding(JSON.stringify({{name: bindingName, seq, args}}));
            return promise;
        }}}})();
   ",
        name = name
    );

    if let Err(e) = send(
        Arc::clone(&c),
        "Page.addScriptToEvaluateOnNewDocument",
        &json!({ "source": script }),
    ) {
        return Err(e.into());
    }
    eval(Arc::clone(&c), &script).to_result_of_jserror()
}

fn complete_binding(context: ActiveBindingContext, result: JSResult) {
    let (r, e) = match result {
        Ok(x) => (x.to_string(), r#""""#.to_string()),
        Err(e) => ("".to_string(), e.to_string()),
    };

    let expr = format!(
        r"
        if ({error}) {{
            window['{name}']['errors'].get({seq})({error});
        }} else {{
            window['{name}']['callbacks'].get({seq})({result});
        }}
        window['{name}']['callbacks'].delete({seq});
        window['{name}']['errors'].delete({seq});
        ",
        name = context.payload["name"].as_str().expect("Expected string"),
        seq = context.payload["seq"].as_i64().expect("Expected i64"),
        result = r,
        error = e
    );

    if let Err(e) = send(
        context.chrome,
        "Runtime.evaluate",
        &json!({
            "expression":expr,
            "contextId":context.context_id
        }),
    ) {
        eprintln!("{}", e);
    }
}

pub fn close(c: Arc<Chrome>) {
    std::thread::spawn(move || {
        if let Err(e) = send(c, "Browser.close", &json!({})) {
            eprintln!("{}", e);
        }
    });
}

#[cfg(target_family = "windows")]
pub fn close_handle(c: Arc<Chrome>) {
    close_process_handle(c.pid as Process).expect("Unable to close handle")
}
