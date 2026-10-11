#![allow(dead_code)]
use std::cell::{Cell, RefCell};
thread_local! {
 static FAIL: Cell<&'static str> = const {Cell::new("")};
 static CALLS: RefCell<Vec<String>> = const {RefCell::new(Vec::new())};
}
fn step(name: &str) -> anyhow::Result<()> {
    CALLS.with(|calls| calls.borrow_mut().push(name.into()));
    anyhow::ensure!(
        !FAIL.with(|fail| fail.get() == name),
        "injected ENOENT: {name}"
    );
    Ok(())
}
mod defs {
    pub const MODULE_DIR: &str = "/tmp/ksu-fixture";
    pub const LOG_DIR: &str = "/tmp/ksu-fixture";
    pub const BOOTLOG_TIMEOUT: &str = "1";
    pub const BOOT_STAGE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);
}
mod utils {
    pub fn umask(_: u32) {}
    pub fn is_safe_mode() -> bool {
        false
    }
    pub fn has_magisk() -> bool {
        false
    }
    pub fn switch_cgroups() {}
    pub fn ensure_dir_exists(_: &std::path::Path) -> anyhow::Result<()> {
        Ok(())
    }
}
mod assets {
    pub fn ensure_binaries(_: bool) -> anyhow::Result<()> {
        Ok(())
    }
}
mod module_config {
    pub fn clear_all_temp_configs() -> anyhow::Result<()> {
        Ok(())
    }
}
mod restorecon {
    pub fn restorecon() -> anyhow::Result<()> {
        Ok(())
    }
}
mod profile {
    pub fn apply_sepolies() -> anyhow::Result<()> {
        Ok(())
    }
}
mod feature {
    pub fn init_features() -> anyhow::Result<()> {
        Ok(())
    }
}
mod ksucalls {
    pub fn ensure_uapi_version_matched() -> anyhow::Result<()> {
        crate::step("uapi")
    }
    pub fn report_post_fs_data() {}
    pub fn report_services() -> anyhow::Result<bool> {
        crate::step("report-services")?;
        Ok(true)
    }
    pub fn report_boot_complete() {}
}
mod restart_stage {
    pub fn blocked() -> bool {
        false
    }
    pub fn participants() -> anyhow::Result<Vec<std::path::PathBuf>> {
        Ok(vec![])
    }
}
mod soft_reboot {
    pub fn soft_reboot() -> anyhow::Result<()> {
        Ok(())
    }
}
mod module {
    #[derive(Clone, Copy)]
    pub enum ScriptWait {
        NoWait,
        Until(std::time::Instant),
    }
    pub fn handle_updated_modules() -> anyhow::Result<()> {
        Ok(())
    }
    pub fn prune_modules() -> anyhow::Result<()> {
        Ok(())
    }
    pub fn disable_all_modules() -> anyhow::Result<()> {
        Ok(())
    }
    pub fn regenerate_preinit_rc() -> anyhow::Result<()> {
        Ok(())
    }
    pub fn load_sepolicy_rule() -> anyhow::Result<()> {
        Ok(())
    }
    pub fn load_system_prop() -> anyhow::Result<()> {
        Ok(())
    }
    pub fn exec_common_scripts(s: &str, _: ScriptWait) -> anyhow::Result<()> {
        crate::step(&format!("common:{s}"))
    }
    pub fn exec_stage_script(s: &str, _: ScriptWait) -> anyhow::Result<()> {
        crate::step(&format!("module:{s}"))
    }
}
mod metamodule {
    pub fn exec_stage_script(s: &str, _: crate::module::ScriptWait) -> anyhow::Result<()> {
        crate::step(&format!("meta:{s}"))
    }
    pub fn exec_mount_script(_: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
#[path = "../../../userspace/ksud/src/init_event.rs"]
mod init_event;
#[test]
fn service_dispatch_failure_reaches_caller() {
    FAIL.set("module:service");
    let error = init_event::on_services().unwrap_err();
    assert!(format!("{error:#}").contains("injected ENOENT: module:service"));
}
#[test]
fn boot_dispatch_failure_reaches_caller() {
    FAIL.set("module:boot-completed");
    assert!(init_event::on_boot_completed().is_err());
}
#[test]
fn common_error_stops_later_dispatch() {
    FAIL.set("common:service.d");
    assert!(init_event::on_services().is_err());
    assert!(!CALLS.with(|c| c.borrow().iter().any(|s| s == "module:service")));
}
#[test]
fn metamodule_error_reaches_caller() {
    FAIL.set("meta:service");
    assert!(init_event::on_services().is_err());
}
#[test]
fn uapi_failure_is_not_success() {
    FAIL.set("uapi");
    assert!(init_event::on_services().is_err());
    assert!(init_event::on_boot_completed().is_err());
}
#[test]
fn successful_dispatch_preserves_standard_order() {
    init_event::on_services().unwrap();
    assert_eq!(
        CALLS.with(|c| c.borrow().clone()),
        vec![
            "uapi",
            "report-services",
            "common:service.d",
            "meta:service",
            "module:service"
        ]
    );
}
