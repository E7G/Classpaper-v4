// use std::env;
extern crate embed_resource;

fn main() {
    #[cfg(target_family = "windows")] {
        // oldwin::overwrite_subsystem(oldwin::Subsystem::Console);
        oldwin::inject();
    }
    if cfg!(target_os = "windows") {
        let _ = embed_resource::compile("app.rc", embed_resource::NONE);
    }
} 