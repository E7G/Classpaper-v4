// use std::env;
extern crate embed_resource;

fn main() {
    // 设置编译日期环境变量
    let build_date = chrono::Local::now().format("%Y-%m-%d").to_string();
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);
    
    #[cfg(target_family = "windows")] {
        // oldwin::overwrite_subsystem(oldwin::Subsystem::Console);
        oldwin::inject();
    }
    if cfg!(target_os = "windows") {
        let _ = embed_resource::compile("app.rc", embed_resource::NONE);
    }
} 