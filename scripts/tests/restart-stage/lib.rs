#![allow(dead_code)]
mod assets { pub const BUSYBOX_PATH:&str="/bin/sh"; }
mod defs { pub const ADB_DIR:&str="/tmp"; }
mod utils {pub fn switch_cgroups() {}}
mod module {
 use std::path::Path;
 pub enum ScriptWait {Forever}
 pub fn foreach_active_module(_f:impl FnMut(&Path)->anyhow::Result<()>) -> anyhow::Result<()> {Ok(())}
 pub fn validate_module_id(id:&str)->anyhow::Result<()> {anyhow::ensure!(!id.contains('/'),"bad ID");Ok(())}
 pub fn get_common_script_envs(_id:Option<&str>)->Vec<(&'static str,String)> {vec![]}
 pub fn exec_script_direct(_p:&Path,_w:ScriptWait)->anyhow::Result<()> {Ok(())}
}
#[path="../../../userspace/ksud/src/service_guard.rs"] mod service_guard;
#[path="../../../userspace/ksud/src/restart_stage.rs"] mod restart_stage;
