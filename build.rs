// use std::env;
extern crate embed_resource;

fn main() {
    if cfg!(target_os = "windows") {
        let _ = embed_resource::compile("app.rc", embed_resource::NONE);
    }
} 