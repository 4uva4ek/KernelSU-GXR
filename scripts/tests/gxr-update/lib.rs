#![allow(dead_code)]
mod utils {
    pub fn getprop(_: &str) -> Option<String> { Some("1".to_owned()) }
}
#[path = "../../../userspace/ksud/src/gxr_update.rs"]
mod gxr_update;
