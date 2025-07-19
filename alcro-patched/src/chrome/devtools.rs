use super::{ActiveBindingContext, BindingContext, Chrome, JSObject, JSResult};
use super::{PipeReader, PipeWriter};
use crossbeam_channel::{bounded, Sender};
use serde_json::json;
use std::sync::{atomic::Ordering, Arc, Mutex};
use std::collections::HashSet;
use once_cell::sync::Lazy;
use std::io::ErrorKind;

static DESTROYED_TARGETS: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

pub fn send_msg(p: &Mutex<PipeWriter>, message: String) {
    let result = p.lock()
        .expect("Unable to lock")
        .write(message);
    if let Err(e) = result {
        // 特殊处理 BrokenPipe 错误，防止 panic
        match e {
            super::os::PipeWriteError::IOError(ref ioerr) if ioerr.kind() == ErrorKind::BrokenPipe => {
                eprintln!("[alcro-devtools] 管道已关闭 (BrokenPipe)，忽略写入: {}", ioerr);
                return;
            }
            _ => panic!("Unable to write to pipe: {}", e),
        }
    }
}

pub fn recv_msg(p: &Mutex<PipeReader>) -> String {
    let msg = p.lock()
        .expect("Unable to lock")
        .read()
        .unwrap_or_else(|e| {
            eprintln!("[alcro-debug] 管道读取错误: {:?}", e);
            String::new()
        });
    // 只在出错时显示
    msg
}

fn beautify_nested_json(json_value: &serde_json::Value) -> String {
    let mut pretty_json = serde_json::to_string_pretty(json_value).unwrap();
    
    // 尝试格式化 params.message 中的 JSON
    if let Some(params) = json_value.get("params") {
        if let Some(message) = params.get("message") {
            if let Some(message_str) = message.as_str() {
                // 尝试解析嵌套的 JSON
                if let Ok(nested_json) = serde_json::from_str::<serde_json::Value>(message_str) {
                    let nested_pretty = serde_json::to_string_pretty(&nested_json).unwrap();
                    // 使用正则表达式或更精确的替换方法
                    // 找到 "message": "..." 的模式并替换
                    let message_pattern = format!("\"message\": \"{}\"", message_str.replace("\"", "\\\""));
                    let replacement = format!("\"message\": \n{}", nested_pretty);
                    pretty_json = pretty_json.replace(&message_pattern, &replacement);
                }
            }
        }
    }
    
    pretty_json
}

pub fn readloop(c: Arc<Chrome>) {
    loop {
        let pmsg = recv_msg(&c.precv);
        if pmsg.is_empty() {
            eprintln!("[alcro-debug] 收到空消息，浏览器可能已退出或管道已关闭");
            break;
        }
        // 只打印关键事件，Target.targetDestroyed 去重
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&pmsg) {
            let method = json_value.get("method").and_then(|m| m.as_str()).unwrap_or("");
            match method {
                "Target.targetDestroyed" => {
                    if let Some(params) = json_value.get("params") {
                        if let Some(target_id) = params.get("targetId").and_then(|v| v.as_str()) {
                            let mut set = DESTROYED_TARGETS.lock().unwrap();
                            if set.insert(target_id.to_string()) {
                                println!("[alcro-debug] 事件: {}", method);
                                println!("{}", beautify_nested_json(&json_value));
                            }
                        }
                    }
                }
                "Target.targetCreated" | "Runtime.exceptionThrown" | "Runtime.consoleAPICalled" => {
                    println!("[alcro-debug] 事件: {}", method);
                    println!("{}", beautify_nested_json(&json_value));
                }
                _ => {}
            }
        }
        let pmsg: JSObject = match serde_json::from_str(&pmsg) {
            Ok(obj) => obj,
            Err(e) => {
                eprintln!("[alcro-debug] JSON 解析失败: {:?}", e);
                break;
            }
        };

        if pmsg["method"] == "Target.targetDestroyed" {
            #[cfg(target_family = "unix")]
            if pmsg["params"]["targetId"] == c.target {
                let _ = c.kill_send.send(());
                return;
            }
        } else if pmsg["method"] == "Target.receivedMessageFromTarget" {
            let params = &pmsg["params"];
            if params["sessionId"] != c.session {
                continue;
            }

            let message = params["message"]
                .as_str()
                .expect("message should be a string");
            let res: JSObject = serde_json::from_str(message).expect("Invalid JSON");

            if res["id"] == JSObject::Null && res["method"] == "Runtime.consoleAPICalled"
                || res["method"] == "Runtime.exceptionThrown"
            {
                let tag = if res["method"] == "Runtime.exceptionThrown" {
                    "[JS Exception]"
                } else if res["method"] == "Runtime.consoleAPICalled" {
                    "[JS Console]"
                } else {
                    "[JS Message]"
                };
                println!("{}\n{}", tag, serde_json::to_string_pretty(&res).unwrap());
            } else if res["id"] == JSObject::Null && res["method"] == "Runtime.bindingCalled" {
                let payload: JSObject = serde_json::from_str(
                    res["params"]["payload"]
                        .as_str()
                        .expect("payload should be a string"),
                )
                .expect("Invalid JSON");
                binding_called(
                    c.clone(),
                    res["params"]["name"].as_str().expect("Expected string"),
                    payload,
                    res["params"]["executionContextId"]
                        .as_i64()
                        .expect("Expected i64"),
                );
                continue;
            } else if res["id"].is_i64() {
                let res_id = res["id"].as_i64().expect("Expected i64") as i32;

                match c.pending.get(&res_id) {
                    None => continue,
                    Some(reschan) => {
                        send_result(&*reschan, &res);
                    }
                }
                c.pending.remove(&res_id);
            }
        }
    }
}

pub fn send(c: Arc<Chrome>, method: &str, params: &JSObject) -> JSResult {
    let id = c.id.fetch_add(1, Ordering::Relaxed) + 1;
    let json_msg = json!({
        "id":id,
        "method":method,
        "params":params
    });
    let (s, r) = bounded(1);
    c.pending.insert(id, s);

    send_msg(
        &c.psend,
        json!({
            "id":id,
            "method":"Target.sendMessageToTarget",
            "params":json!({
                "message":json_msg.to_string(),
                "sessionId":c.session
            })
        })
        .to_string(),
    );
    r.recv().unwrap()
}

fn send_result(reschan: &Sender<JSResult>, res: &JSObject) {
    if res["error"]["message"] != JSObject::Null {
        reschan.send(Err(res["error"]["message"].clone())).unwrap();
    } else if res["result"]["exceptionDetails"]["exception"]["value"] != JSObject::Null {
        reschan
            .send(Err(
                res["result"]["exceptionDetails"]["exception"]["value"].clone()
            ))
            .unwrap();
    } else if res["result"]["result"]["type"] == "object"
        && res["result"]["result"]["subtype"] == "error"
    {
        reschan
            .send(Err(res["result"]["result"]["description"].clone()))
            .unwrap();
    } else if res["result"]["result"]["type"] != JSObject::Null {
        reschan
            .send(Ok(res["result"]["result"]["value"].clone()))
            .unwrap();
    } else {
        reschan.send(Ok(res["result"].clone())).unwrap();
    }
}

fn binding_called(c: Arc<Chrome>, name: &str, payload: JSObject, context_id: i64) {
    let binding = match c.bindings.get(name) {
        Some(b) => Some(Arc::clone(&*b)),
        None => None,
    };
    if let Some(binding) = binding {
        binding(BindingContext::new(ActiveBindingContext {
            chrome: c,
            payload,
            context_id,
        }))
    }
}
