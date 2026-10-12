//! Direct, disabled installation for GXR packages satisfying the lifecycle contract.
//! No installer/customize/metamodule/preinit code is executed. Backups are inert.
use anyhow::{Context, Result, bail, ensure};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions, Permissions},
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const LIMIT: u64 = 64 * 1024 * 1024;
type Payload = BTreeMap<String, Vec<u8>>;

fn valid_gxr_id(id: &str) -> bool {
    id.starts_with("gxr_") && id.len() > 4 && !id.contains('/') && safe_name(id)
}

/// Serialize updates with Manager checkbox/actions and the complete soft restart.
/// This is a volatile lock only, never a saved activation preference.
pub fn control_lock() -> Result<File> {
    lock_at(Path::new("/dev/gxr-module-control.lock"))
}

fn lock_at(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    file.try_lock()
        .context("Another module operation or soft restart is running; retry later")?;
    Ok(file)
}

fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        })
}

fn props(bytes: &[u8]) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for line in std::str::from_utf8(bytes)?.lines() {
        if let Some((key, value)) = line.split_once('=') {
            ensure!(
                out.insert(key.to_owned(), value.to_owned()).is_none(),
                "Duplicate property"
            );
        }
    }
    Ok(out)
}

fn checksums(bytes: &[u8]) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for line in std::str::from_utf8(bytes)?.lines() {
        let (hash, name) = line.split_once("  ").context("Invalid SHA256SUMS line")?;
        ensure!(
            safe_name(name) && hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
            "Invalid checksum entry"
        );
        ensure!(
            out.insert(name.to_owned(), hash.to_owned()).is_none(),
            "Duplicate checksum"
        );
    }
    Ok(out)
}

fn validate_payload(files: &Payload) -> Result<BTreeMap<String, String>> {
    for marker in ["disable", "skip_mount"] {
        ensure!(
            files.get(marker).is_some_and(Vec::is_empty),
            "Missing empty {marker}"
        );
    }
    let properties = props(files.get("module.prop").context("Missing module.prop")?)?;
    ensure!(
        properties.get("id").is_some_and(|id| valid_gxr_id(id)),
        "Invalid GXR module ID"
    );
    ensure!(
        !properties.contains_key("metamodule"),
        "Metamodules are unsupported"
    );
    version_code(&properties)?;
    for name in files.keys() {
        ensure!(safe_name(name), "Unsafe payload path");
        ensure!(
            !name.contains('/') || name.starts_with("bin/"),
            "Unexpected payload directory"
        );
        ensure!(
            !matches!(
                name.as_str(),
                "post-fs-data.sh"
                    | "post-mount.sh"
                    | "system.prop"
                    | "sepolicy.rule"
                    | "boot-completed.sh"
                    | "remove"
                    | "update"
                    | "guard-boot-id"
                    | "auto.pending"
                    | "failure"
            ),
            "Unexpected startup/state payload"
        );
    }
    let sums = checksums(files.get("SHA256SUMS").context("Missing SHA256SUMS")?)?;
    ensure!(
        sums.len() + 2 == files.len(),
        "Incomplete checksum manifest"
    );
    for (name, hash) in &sums {
        ensure!(
            name != "disable" && name != "SHA256SUMS",
            "Invalid checksum manifest"
        );
        ensure!(
            sha256::digest(files.get(name).context("Missing payload")?) == *hash,
            "Checksum mismatch: {name}"
        );
    }
    ensure!(
        files.contains_key("action.sh") && files.contains_key("service.sh"),
        "Missing lifecycle scripts"
    );
    Ok(properties)
}

fn version_code(properties: &BTreeMap<String, String>) -> Result<i32> {
    let code: i32 = properties
        .get("versionCode")
        .context("Missing versionCode")?
        .parse()?;
    ensure!(code > 0, "Invalid versionCode");
    Ok(code)
}

fn read_archive(path: &Path) -> Result<Payload> {
    let file = File::open(path)?;
    ensure!(file.metadata()?.len() <= LIMIT, "ZIP too large");
    let mut archive = zip::ZipArchive::new(file)?;
    ensure!(archive.len() <= 128, "Too many ZIP entries");
    let mut files = Payload::new();
    let mut total = 0;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name().to_owned();
        let kind = entry.unix_mode().unwrap_or(0) & 0o170_000;
        ensure!(
            safe_name(&name) && (kind == 0 || kind == 0o100_000),
            "Unsafe ZIP entry"
        );
        total += entry.size();
        ensure!(total <= LIMIT, "Unpacked ZIP too large");
        let mut bytes = Vec::new();
        (&mut entry).take(LIMIT + 1).read_to_end(&mut bytes)?;
        ensure!(bytes.len() as u64 == entry.size(), "Invalid ZIP entry size");
        ensure!(files.insert(name, bytes).is_none(), "Duplicate ZIP entry");
    }
    Ok(files)
}

fn regular_tree(path: &Path, depth: u32, total: &mut u64) -> Result<()> {
    ensure!(depth < 8, "Module tree is too deep");
    let meta = fs::symlink_metadata(path)?;
    ensure!(
        !meta.file_type().is_symlink(),
        "Unsafe symlink: {}",
        path.display()
    );
    if meta.is_dir() {
        for entry in fs::read_dir(path)? {
            regular_tree(&entry?.path(), depth + 1, total)?;
        }
    } else {
        ensure!(
            meta.is_file() && meta.len() <= LIMIT,
            "Unsupported module entry"
        );
        ensure!(meta.nlink() == 1, "Hard-linked module file is unsupported");
        *total += meta.len();
        ensure!(*total <= LIMIT, "Installed module too large");
    }
    Ok(())
}

fn verify_installed(path: &Path) -> Result<BTreeMap<String, String>> {
    regular_tree(path, 0, &mut 0)?;
    let sums = checksums(&fs::read(path.join("SHA256SUMS"))?)?;
    for required in ["action.sh", "auto.sh", "module.prop"] {
        ensure!(
            sums.contains_key(required),
            "Unverified installed lifecycle script"
        );
    }
    for (name, hash) in sums {
        ensure!(
            sha256::digest(fs::read(path.join(name))?) == hash,
            "Installed checksum mismatch"
        );
    }
    props(&fs::read(path.join("module.prop"))?)
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    if from.is_dir() {
        fs::create_dir_all(to)?;
        fs::set_permissions(to, Permissions::from_mode(0o700))?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else {
        fs::copy(from, to)?;
        File::open(to)?.sync_all()?;
    }
    Ok(())
}

fn run_action(path: &Path, mode: &str) -> Result<String> {
    // File output avoids pipe-buffer deadlocks; no shell interpolation.
    let output = tempfile::tempfile()?;
    let mut child = Command::new("/system/bin/sh")
        .arg(path.join("action.sh"))
        .arg(mode)
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(output.try_clone()?)
        .spawn()?;
    let deadline = Instant::now() + Duration::from_secs(45);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!(
                "Restoration timed out; old module retained DISABLED. Fully reboot before retrying."
            );
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let mut output = output;
    output.seek(SeekFrom::Start(0))?;
    let mut text = String::new();
    output.take(128 * 1024).read_to_string(&mut text)?;
    print!("{text}");
    ensure!(
        status.success(),
        "{mode} failed; old module retained DISABLED. Use Action for recovery."
    );
    Ok(text)
}

fn restored_report(text: &str) -> bool {
    let mut verified = false;
    for line in text.lines() {
        if line.starts_with("STOCK ")
            || line == "GXR_UPDATE_STATE=STOCK"
            || line == "DISABLED - Stock instructions verified"
            || line == "WAITING: target not running; no live patch"
            || line == "WAITING: no third-party main app process has OpenXR loaded"
            || line == "DISABLED - Waiting: controller service is absent; no live target"
        {
            verified = true;
        }
        if line.starts_with("MAIN ")
            || line.starts_with("CORRECTED ")
            || line.starts_with("EXTRAPOLATION ")
            || line.starts_with("ENABLED")
            || line.contains("ACTIVE")
            || line.contains("Error:")
            || line.contains("ERROR:")
            || line.contains("Stopping:")
        {
            return false;
        }
    }
    verified
}

fn sync_dir(path: &Path) -> Result<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}

fn sync_tree(path: &Path) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let path = entry?.path();
        if path.is_dir() {
            sync_tree(&path)?;
        }
    }
    sync_dir(path)
}

/// Return false for ordinary modules. GXR packages use this path regardless of
/// their ID or repository; incompatible payloads never fall through to installer scripts.
pub fn try_install(zip: &str) -> Result<bool> {
    // Inspect only module.prop before deciding the route, to leave ordinary ZIPs
    // (including directory entries) on their existing installer path.
    let mut archive = zip::ZipArchive::new(File::open(zip)?)?;
    let mut data = Vec::new();
    archive
        .by_name("module.prop")?
        .take(16 * 1024)
        .read_to_end(&mut data)?;
    let properties = props(&data)?;
    let id = properties.get("id").context("Missing module ID")?;
    if !id.starts_with("gxr_") {
        return Ok(false);
    }
    ensure!(valid_gxr_id(id), "Invalid GXR module ID");
    ensure!(
        crate::utils::getprop("sys.boot_completed").as_deref() == Some("1"),
        "Android is restarting"
    );
    let files = read_archive(Path::new(zip))?;
    let properties = validate_payload(&files)?;
    ensure!(
        properties.get("id") == Some(id),
        "ZIP changed during inspection"
    );
    install_at(
        &files,
        &properties,
        Path::new("/data/adb/modules"),
        Path::new("/data/adb/modules_update"),
        Path::new("/data/adb/gxr-updates"),
        |old| {
            run_action(old, "off")?;
            ensure!(
                restored_report(&run_action(old, "status")?),
                "Live restoration not verified; old module retained DISABLED"
            );
            Ok(())
        },
    )?;
    Ok(true)
}

fn install_at(
    files: &Payload,
    properties: &BTreeMap<String, String>,
    modules: &Path,
    pending: &Path,
    backups: &Path,
    stop: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    let id = properties.get("id").context("Missing ID")?;
    for root in [modules, backups] {
        if root.symlink_metadata().is_ok() {
            ensure!(
                root.symlink_metadata()?.is_dir(),
                "Unsafe module/update root"
            );
        } else {
            fs::create_dir(root)?;
        }
    }
    if pending.symlink_metadata().is_ok() {
        ensure!(pending.symlink_metadata()?.is_dir(), "Unsafe pending root");
    }
    let live = modules.join(id);
    ensure!(
        pending.join(id).symlink_metadata().is_err(),
        "A staged update already exists"
    );
    let exists = live.symlink_metadata().is_ok();
    if exists {
        let old = verify_installed(&live)?;
        ensure!(old.get("id") == Some(id), "Installed identity mismatch");
        ensure!(
            version_code(properties)? > version_code(&old)?,
            "Version must increase; no downgrade/reinstall"
        );
        for marker in ["remove", "update"] {
            ensure!(
                !live.join(marker).exists(),
                "Resolve pending module changes first"
            );
        }
    }
    let transaction = tempfile::Builder::new()
        .prefix(&format!("{id}-"))
        .tempdir_in(backups)?
        .keep();
    let stage = transaction.join("new");
    fs::create_dir(&stage)?;
    fs::set_permissions(&stage, Permissions::from_mode(0o700))?;
    // Keep the entire old tree as an inert backup. Preserve guard/config files
    // in the new tree, but drop all previous package-owned files before overlay.
    if exists {
        File::create(live.join("disable"))?.sync_all()?;
        sync_dir(&live)?;
        stop(&live)?;
        regular_tree(&live, 0, &mut 0)?;
        copy_tree(&live, &stage)?;
        for name in checksums(&fs::read(live.join("SHA256SUMS"))?)?.keys() {
            fs::remove_file(stage.join(name))?;
        }
        fs::remove_file(stage.join("SHA256SUMS"))?;
    }
    for (name, data) in files {
        let dest = stage.join(name);
        fs::create_dir_all(dest.parent().context("Missing parent")?)?;
        let mut file = File::create(&dest)?;
        file.write_all(data)?;
        file.set_permissions(Permissions::from_mode(
            if Path::new(name).extension().is_some_and(|ext| ext == "sh")
                || name.starts_with("bin/")
            {
                0o755
            } else {
                0o600
            },
        ))?;
        file.sync_all()?;
    }
    verify_installed(&stage)?;
    sync_tree(&stage)?;
    sync_dir(&transaction)?;
    if exists {
        fs::rename(&live, transaction.join("old"))?;
        sync_dir(modules)?;
        sync_dir(&transaction)?;
    }
    if let Err(error) = fs::rename(&stage, &live) {
        if exists {
            fs::rename(transaction.join("old"), &live)
                .context("Replacement AND backup restore failed; module absent. Inspect /data/adb/gxr-updates")?;
            sync_dir(modules)?;
        }
        return Err(error.into());
    }
    sync_dir(modules)?;
    sync_dir(&transaction)?;
    println!(
        "DISABLED - Installed {id}. Backup/transaction: {}",
        transaction.display()
    );
    println!(
        "Enable separately when intended, then use the documented soft restart. Action reports runtime status."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const ID: &str = "gxr_haptic_main";
    #[test]
    #[ignore = "requires GXR_RELEASE_FIXTURES with downloaded public ZIPs"]
    fn public_release_archives_match_installer_contract() {
        let root = std::path::PathBuf::from(std::env::var("GXR_RELEASE_FIXTURES").unwrap());
        let mut checked = 0;
        for module in fs::read_dir(root).unwrap() {
            for file in fs::read_dir(module.unwrap().path()).unwrap() {
                let path = file.unwrap().path();
                if path.extension().is_some_and(|e| e == "zip") {
                    validate_payload(&read_archive(&path).unwrap()).unwrap();
                    checked += 1;
                }
            }
        }
        assert!(checked > 0, "No release fixtures supplied");
    }
    fn package(code: i32) -> Payload {
        package_for(ID, code)
    }
    fn package_for(id: &str, code: i32) -> Payload {
        let mut files = Payload::from([
            (
                "module.prop".into(),
                format!("id={id}\nversionCode={code}\nversion=1.0.{code}\n").into_bytes(),
            ),
            ("disable".into(), vec![]),
            ("skip_mount".into(), vec![]),
            ("action.sh".into(), b"exit 99\n".to_vec()),
            ("auto.sh".into(), b"exit 99\n".to_vec()),
            ("service.sh".into(), b"exit 99\n".to_vec()),
            ("customize.sh".into(), b"exit 99\n".to_vec()),
        ]);
        let sums = files
            .iter()
            .filter(|(n, _)| n.as_str() != "disable")
            .map(|(n, b)| format!("{}  {n}\n", sha256::digest(b)))
            .collect::<String>();
        files.insert("SHA256SUMS".into(), sums.into_bytes());
        files
    }
    fn install(root: &Path, code: i32, stop: impl FnOnce(&Path) -> Result<()>) -> Result<()> {
        let files = package(code);
        let p = validate_payload(&files)?;
        install_at(
            &files,
            &p,
            &root.join("modules"),
            &root.join("pending"),
            &root.join("backups"),
            stop,
        )
    }
    #[test]
    fn arbitrary_catalogue_ids_install_and_update_without_a_builtin_list() {
        let root = tempfile::tempdir().unwrap();
        for index in 0..12 {
            let id = format!("gxr_repository_module_{index}");
            for code in [1, 2] {
                let files = package_for(&id, code);
                let props = validate_payload(&files).unwrap();
                install_at(
                    &files,
                    &props,
                    &root.path().join("modules"),
                    &root.path().join("pending"),
                    &root.path().join("backups"),
                    |old| {
                        assert!(old.join("disable").exists());
                        Ok(())
                    },
                )
                .unwrap();
                assert!(
                    root.path()
                        .join("modules")
                        .join(&id)
                        .join("disable")
                        .exists()
                );
            }
        }
        assert_eq!(
            fs::read_dir(root.path().join("modules")).unwrap().count(),
            12
        );
        assert!(restored_report("GXR_UPDATE_STATE=STOCK\n"));
        assert!(!restored_report(
            "GXR_UPDATE_STATE=STOCK\nERROR: restore failed\n"
        ));
        for id in ["gxr_", "gxr_../escape", "gxr_bad id"] {
            assert!(validate_payload(&package_for(id, 1)).is_err());
        }
    }
    #[test]
    fn fresh_install_is_disabled_and_never_runs_package_scripts() {
        let root = tempfile::tempdir().unwrap();
        install(root.path(), 1, |_| panic!("No old module exists")).unwrap();
        assert!(
            root.path()
                .join("modules")
                .join(ID)
                .join("disable")
                .is_file()
        );
    }
    #[test]
    fn replacement_stops_old_preserves_guards_and_retains_backup() {
        let root = tempfile::tempdir().unwrap();
        install(root.path(), 1, |_| Ok(())).unwrap();
        let live = root.path().join("modules").join(ID);
        fs::remove_file(live.join("disable")).unwrap();
        fs::create_dir(live.join("state")).unwrap();
        fs::write(live.join("state/failure"), "same-boot failure").unwrap();
        install(root.path(), 2, |old| {
            assert!(old.join("disable").exists());
            assert_eq!(version_code(&verify_installed(old)?).unwrap(), 1);
            Ok(())
        })
        .unwrap();
        assert_eq!(
            fs::read_to_string(live.join("state/failure")).unwrap(),
            "same-boot failure"
        );
        assert_eq!(version_code(&verify_installed(&live).unwrap()).unwrap(), 2);
        let old = fs::read_dir(root.path().join("backups"))
            .unwrap()
            .map(|e| e.unwrap().path().join("old"))
            .find(|p| p.is_dir())
            .unwrap();
        assert!(old.join("disable").exists());
        assert_eq!(version_code(&verify_installed(&old).unwrap()).unwrap(), 1);
    }
    #[test]
    fn restore_failure_leaves_old_disabled_and_unchanged() {
        let root = tempfile::tempdir().unwrap();
        install(root.path(), 1, |_| Ok(())).unwrap();
        assert!(install(root.path(), 2, |_| bail!("Restore failed")).is_err());
        let old = root.path().join("modules").join(ID);
        assert!(old.join("disable").exists());
        assert_eq!(version_code(&verify_installed(&old).unwrap()).unwrap(), 1);
    }
    #[test]
    fn downgrade_and_same_version_do_not_stop_live_module() {
        let root = tempfile::tempdir().unwrap();
        install(root.path(), 2, |_| Ok(())).unwrap();
        for code in [1, 2] {
            assert!(install(root.path(), code, |_| panic!("Must reject before stop")).is_err());
        }
    }
    #[test]
    fn symlink_or_pending_update_cannot_replace_old() {
        let root = tempfile::tempdir().unwrap();
        install(root.path(), 1, |_| Ok(())).unwrap();
        let link = root.path().join("modules").join(ID).join("trap");
        std::os::unix::fs::symlink("/etc", &link).unwrap();
        assert!(install(root.path(), 2, |_| panic!("Unsafe tree")).is_err());
        fs::remove_file(link).unwrap();
        fs::create_dir_all(root.path().join("pending").join(ID)).unwrap();
        assert!(install(root.path(), 2, |_| panic!("Pending update")).is_err());
    }
    #[test]
    fn tampered_or_incomplete_payload_rejected() {
        let mut files = package(1);
        files.get_mut("auto.sh").unwrap().push(0);
        assert!(validate_payload(&files).is_err());
        files = package(1);
        files.remove("disable");
        assert!(validate_payload(&files).is_err());
        files = package(1);
        files.insert("system/evil".into(), vec![]);
        assert!(validate_payload(&files).is_err());
    }
    #[test]
    fn concurrent_controls_and_symlink_locks_fail_closed() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("lock");
        let first = lock_at(&path).unwrap();
        assert!(lock_at(&path).is_err());
        drop(first);
        assert!(lock_at(&path).is_ok());
        let link = root.path().join("link");
        std::os::unix::fs::symlink(path, &link).unwrap();
        assert!(lock_at(&link).is_err());
    }
    #[test]
    fn remaining_active_patch_is_not_a_disabled_status() {
        assert!(restored_report("STOCK verified\n"));
        assert!(!restored_report(
            "DISABLED - Stopping: owned patch is still active; restoration pending\n"
        ));
        assert!(!restored_report("STOCK first\nCORRECTED second\n"));
        assert!(!restored_report("Autostart disabled\n"));
        assert!(!restored_report(
            "DISABLED - External or unowned whitening patch is ACTIVE\n"
        ));
    }
    #[test]
    fn unsafe_archive_paths_rejected() {
        for name in [
            "../outside",
            "/absolute",
            "a/../b",
            "a\\b",
            "bin//file",
            "./action.sh",
        ] {
            assert!(!safe_name(name), "{name}");
        }
    }
}
