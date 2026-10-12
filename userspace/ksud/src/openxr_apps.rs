//! Close verified application clients before an intentional Android restart.
//! No system services, module preferences, or failure guards are modified.
use std::{
    collections::BTreeSet,
    fs,
    io::{self, Read},
    os::{fd::AsRawFd, unix::process::CommandExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail, ensure};
use log::info;

const LIMIT: Duration = Duration::from_secs(20);
const OUTPUT_LIMIT: usize = 1024 * 1024;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Package {
    uid: u32,
    name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Client {
    pid: u32,
    start: u64,
    package: Package,
}

fn packages(output: &str) -> Result<BTreeSet<Package>> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            ensure!(fields.len() == 2, "unrecognized PackageManager response");
            let name = fields[0].strip_prefix("package:").context("package name")?;
            ensure!(
                !name.is_empty()
                    && name.split('.').all(|part| {
                        !part.is_empty()
                            && part.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
                    }),
                "invalid package name"
            );
            let uid = fields[1]
                .strip_prefix("uid:")
                .context("package UID")?
                .parse()?;
            Ok(Package {
                uid,
                name: name.into(),
            })
        })
        .collect()
}

fn runtime_loaded(maps: &str) -> bool {
    maps.lines().any(|line| {
        let fields: Vec<_> = line.split_whitespace().collect();
        fields.len() >= 6
            && fields[1].as_bytes().get(2) == Some(&b'x')
            && matches!(
                fields[5],
                "/system/lib64/libopenxr_android.so" | "/system/lib/libopenxr_android.so"
            )
    })
}

fn start_time(stat: &str) -> Result<u64> {
    // comm can contain spaces and parentheses; field 22 follows its last ')'.
    stat.rsplit_once(')')
        .context("process stat comm")?
        .1
        .split_whitespace()
        .nth(19)
        .context("process start time")?
        .parse()
        .context("invalid process start time")
}

fn client(
    proc: &Path,
    pid: u32,
    apps: &BTreeSet<Package>,
    system_uids: &BTreeSet<u32>,
) -> Result<Option<Client>> {
    let dir = proc.join(pid.to_string());
    let identity = fs::read_to_string(dir.join("stat"))?;
    let start = start_time(&identity)?;
    let status = fs::read_to_string(dir.join("status"))?;
    let uid_line = status
        .lines()
        .find_map(|line| line.strip_prefix("Uid:"))
        .context("process UID")?;
    let uids = uid_line
        .split_whitespace()
        .map(str::parse::<u32>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(uids.len() == 4, "incomplete process credentials");
    let uid = uids[0];
    if !(10_000..20_000).contains(&(uid % 100_000))
        || system_uids.contains(&uid)
        || uids.iter().any(|value| *value != uid)
    {
        return Ok(None);
    }
    let owners: Vec<_> = apps.iter().filter(|app| app.uid == uid).collect();
    if owners.is_empty() {
        return Ok(None);
    }
    if !runtime_loaded(&fs::read_to_string(dir.join("maps"))?) {
        return Ok(None);
    }
    let cmdline = fs::read(dir.join("cmdline"))?;
    let name = std::str::from_utf8(cmdline.split(|c| *c == 0).next().unwrap_or_default())?;
    let named = name.split(':').next().unwrap_or_default();
    let package = owners
        .iter()
        .find(|app| app.name == named)
        .copied()
        .or_else(|| (owners.len() == 1).then_some(owners[0]))
        .context("ambiguous OpenXR app ownership; refusing to stop a shared UID")?;
    // Exclude native helpers running under an app UID, even with a matching name.
    let exe = fs::read_link(dir.join("exe"))?;
    if !matches!(
        exe.to_str(),
        Some("/system/bin/app_process64" | "/system/bin/app_process32")
    ) {
        return Ok(None);
    }
    if start_time(&fs::read_to_string(dir.join("stat"))?)? != start {
        return Ok(None);
    }
    Ok(Some(Client {
        pid,
        start,
        package: package.clone(),
    }))
}

fn clients(
    proc: &Path,
    apps: &BTreeSet<Package>,
    system_uids: &BTreeSet<u32>,
) -> Result<Vec<Client>> {
    let mut result = Vec::new();
    for entry in fs::read_dir(proc)? {
        let entry = entry?;
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        match client(proc, pid, apps, system_uids) {
            Ok(Some(found)) => result.push(found),
            Ok(None) => (),
            Err(error)
                if error
                    .downcast_ref::<io::Error>()
                    .is_some_and(|e| e.kind() == io::ErrorKind::NotFound) => {}
            Err(error) => return Err(error.context(format!("inspect process {pid}"))),
        }
    }
    result.sort_by_key(|item| item.pid);
    Ok(result)
}

fn drain(pipe: &mut impl Read, output: &mut Vec<u8>) -> Result<()> {
    let mut buffer = [0_u8; 8192];
    loop {
        match pipe.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(n) => {
                ensure!(
                    output.len() + n <= OUTPUT_LIMIT,
                    "Android command output limit exceeded"
                );
                output.extend_from_slice(&buffer[..n]);
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => (),
            Err(error) => return Err(error.into()),
        }
    }
}

fn bounded_command(command: &mut Command, deadline: Instant) -> Result<String> {
    ensure!(
        Instant::now() < deadline,
        "OpenXR app shutdown deadline exhausted"
    );
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .context("start Android package command")?;
    let result = (|| -> Result<String> {
        let mut stdout = child.stdout.take().context("command stdout")?;
        let mut stderr = child.stderr.take().context("command stderr")?;
        for fd in [stdout.as_raw_fd(), stderr.as_raw_fd()] {
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            ensure!(
                flags >= 0
                    && unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } >= 0,
                "command pipe nonblocking setup failed"
            );
        }
        let (mut output, mut errors) = (Vec::new(), Vec::new());
        loop {
            drain(&mut stdout, &mut output)?;
            drain(&mut stderr, &mut errors)?;
            if let Some(status) = child.try_wait()? {
                drain(&mut stdout, &mut output)?;
                drain(&mut stderr, &mut errors)?;
                ensure!(
                    status.success(),
                    "Android command failed: {status}: {}",
                    String::from_utf8_lossy(&errors)
                );
                ensure!(
                    errors.is_empty(),
                    "Android command reported: {}",
                    String::from_utf8_lossy(&errors)
                );
                return String::from_utf8(output).context("Android command output encoding");
            }
            ensure!(
                Instant::now() < deadline,
                "Android command timed out before app shutdown completed"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    })();
    if result.is_err() {
        // Only the subprocess group we created, never a guessed application PID.
        unsafe {
            libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
        }
        let _ = child.wait();
    }
    result
}

struct PatchTransaction(PathBuf);
impl PatchTransaction {
    fn acquire(path: &Path, deadline: Instant) -> Result<Self> {
        loop {
            match fs::create_dir(path) {
                Ok(()) => return Ok(Self(path.to_owned())),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    ensure!(
                        fs::symlink_metadata(path)?.file_type().is_dir(),
                        "unsafe OpenXR transaction path"
                    );
                    ensure!(
                        Instant::now() < deadline,
                        "OpenXR patch operation is still busy; restart cancelled"
                    );
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(error) => return Err(error.into()),
            }
        }
    }
}
impl Drop for PatchTransaction {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir(&self.0) {
            log::error!("OpenXR transaction release failed: {error}");
        }
    }
}

/// Caller has notified module workers and set boot_completed=0. PackageManager
/// is still alive. The shared lock lets any in-flight app patch finish first.
pub fn close_before_restart(deadline: Instant) -> Result<()> {
    let deadline = deadline.min(Instant::now() + LIMIT);
    let _patch = PatchTransaction::acquire(Path::new("/dev/gxr_openxr_transaction"), deadline)?;
    close_with(Path::new("/proc"), deadline, |args, deadline| {
        bounded_command(Command::new("/system/bin/cmd").args(args), deadline)
    })
}

fn close_with(
    proc: &Path,
    deadline: Instant,
    mut command: impl FnMut(&[&str], Instant) -> Result<String>,
) -> Result<()> {
    let apps = packages(&command(
        &["package", "list", "packages", "-3", "-U", "--user", "all"],
        deadline,
    )?)?;
    let system = packages(&command(
        &["package", "list", "packages", "-s", "-U", "--user", "all"],
        deadline,
    )?)?;
    ensure!(
        !system.is_empty(),
        "empty system package inventory; cannot establish safe app targets"
    );
    let system_uids = system.into_iter().map(|app| app.uid).collect();
    let mut stopped = BTreeSet::new();
    loop {
        ensure!(
            Instant::now() < deadline,
            "OpenXR clients did not stop before deadline"
        );
        let found = clients(proc, &apps, &system_uids)?;
        if found.is_empty() {
            info!(
                "OpenXR apps closed before Android stop: {} package(s); no eligible runtime clients remain",
                stopped.len()
            );
            return Ok(());
        }
        for target in found {
            if stopped.contains(&target.package) {
                continue;
            }
            // Re-read process identity and mappings immediately before the
            // package operation. Never signal an unverified/recycled PID.
            match client(proc, target.pid, &apps, &system_uids) {
                Ok(Some(current)) if current == target => (),
                Ok(_) => continue,
                Err(error)
                    if error
                        .downcast_ref::<io::Error>()
                        .is_some_and(|e| e.kind() == io::ErrorKind::NotFound) =>
                {
                    continue;
                }
                Err(error) => return Err(error),
            }
            let user = (target.package.uid / 100_000).to_string();
            info!(
                "OpenXR planned force-stop: {} user={} pid={} start={}",
                target.package.name, user, target.pid, target.start
            );
            let output = command(
                &[
                    "activity",
                    "force-stop",
                    "--user",
                    &user,
                    &target.package.name,
                ],
                deadline,
            )?;
            // AOSP success is silent; do not accept a printed Error with code 0.
            if !output.trim().is_empty() {
                bail!("force-stop reported: {}", output.trim());
            }
            stopped.insert(target.package);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    const APP: &str = "package:com.valvesoftware.steamlinkvr uid:10274\npackage:logger.com.valvesoftware.steamlinkvr uid:10255\npackage:other.xr uid:1010274\npackage:system.shared uid:10211\n";
    const SYSTEM: &str = "package:android uid:1000\npackage:com.android.systemui uid:10211\n";
    const MAPS: &str = "1000-2000 r-xp 0 01:01 1 /system/lib64/libopenxr_android.so\n";

    fn process(root: &Path, pid: u32, uid: u32, name: &str, maps: &str) {
        let dir = root.join(pid.to_string());
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("stat"),
            format!("{pid} (name with ) space) S {}42 0\n", "0 ".repeat(18)),
        )
        .unwrap();
        fs::write(
            dir.join("status"),
            format!("Uid:\t{uid}\t{uid}\t{uid}\t{uid}\n"),
        )
        .unwrap();
        fs::write(dir.join("cmdline"), format!("{name}\0")).unwrap();
        fs::write(dir.join("maps"), maps).unwrap();
        if !dir.join("exe").symlink_metadata().is_ok() {
            symlink("/system/bin/app_process64", dir.join("exe")).unwrap();
        }
    }

    fn inventory(args: &[&str]) -> Option<String> {
        if args.contains(&"-3") {
            Some(APP.into())
        } else if args.contains(&"-s") {
            Some(SYSTEM.into())
        } else {
            None
        }
    }

    #[test]
    fn closes_both_links_and_secondary_user_once_but_preserves_system_and_non_xr() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path();
        process(p, 10, 10274, "com.valvesoftware.steamlinkvr", MAPS);
        process(p, 11, 10255, "logger.com.valvesoftware.steamlinkvr", MAPS);
        process(
            p,
            12,
            10255,
            "logger.com.valvesoftware.steamlinkvr:render",
            MAPS,
        );
        process(p, 13, 1010274, "other.xr", MAPS);
        process(p, 14, 10211, "system.shared", MAPS);
        process(
            p,
            15,
            10274,
            "com.valvesoftware.steamlinkvr",
            "1000-2000 r-xp 0 01:01 1 /other/lib.so\n",
        );
        process(p, 16, 1000, "system_server", MAPS);
        process(p, 17, 10255, "helper", MAPS);
        fs::remove_file(p.join("17/exe")).unwrap();
        symlink("/data/local/tmp/helper", p.join("17/exe")).unwrap();
        let mut calls = Vec::new();
        close_with(p, Instant::now() + Duration::from_secs(2), |args, _| {
            if let Some(output) = inventory(args) {
                return Ok(output);
            }
            assert_eq!(&args[..3], &["activity", "force-stop", "--user"]);
            calls.push((args[3].to_owned(), args[4].to_owned()));
            let targets: &[u32] = match args[4] {
                "com.valvesoftware.steamlinkvr" => &[10, 15],
                "logger.com.valvesoftware.steamlinkvr" => &[11, 12],
                "other.xr" => &[13],
                _ => panic!("unsafe package selected"),
            };
            for pid in targets {
                fs::remove_dir_all(p.join(pid.to_string())).unwrap();
            }
            Ok(String::new())
        })
        .unwrap();
        assert_eq!(calls.len(), 3);
        assert!(calls.contains(&("10".into(), "other.xr".into())));
        for pid in [14, 16, 17] {
            assert!(p.join(pid.to_string()).exists());
        }
    }

    #[test]
    fn no_runtime_means_no_force_stop() {
        let d = tempfile::tempdir().unwrap();
        process(d.path(), 10, 10274, "com.valvesoftware.steamlinkvr", "");
        close_with(
            d.path(),
            Instant::now() + Duration::from_secs(1),
            |args, _| Ok(inventory(args).expect("must not force-stop")),
        )
        .unwrap();
    }

    #[test]
    fn package_manager_errors_do_not_turn_into_empty_success() {
        let d = tempfile::tempdir().unwrap();
        for output in [
            "Error: service unavailable",
            "package:bad;name uid:12345",
            "package:bad uid:unknown",
            "package:ok uid:10255 extra",
        ] {
            assert!(
                close_with(
                    d.path(),
                    Instant::now() + Duration::from_secs(1),
                    |_, _| Ok(output.into())
                )
                .is_err()
            );
        }
        assert!(
            close_with(
                d.path(),
                Instant::now() + Duration::from_secs(1),
                |_, _| Ok(String::new())
            )
            .is_err()
        );
    }

    #[test]
    fn unknown_shared_uid_owner_is_refused() {
        let d = tempfile::tempdir().unwrap();
        process(d.path(), 10, 10255, "custom.process", MAPS);
        let apps = packages("package:one.app uid:10255\npackage:two.app uid:10255\n").unwrap();
        assert!(clients(d.path(), &apps, &BTreeSet::new()).is_err());
    }

    #[test]
    fn map_matching_requires_exact_runtime_and_executable_mapping() {
        assert!(runtime_loaded(MAPS));
        assert!(runtime_loaded(&MAPS.replace("lib64", "lib")));
        assert!(!runtime_loaded(&MAPS.replace("r-xp", "r--p")));
        assert!(!runtime_loaded(&MAPS.replace(".so", ".so.fake")));
        assert!(!runtime_loaded(&MAPS.replace("/system/", "/data/")));
    }

    #[test]
    fn failure_and_surviving_client_never_report_shutdown_complete() {
        let d = tempfile::tempdir().unwrap();
        process(d.path(), 10, 10274, "com.valvesoftware.steamlinkvr", MAPS);
        for output in ["Error: denied", ""] {
            let mut stops = 0;
            let result = close_with(
                d.path(),
                Instant::now() + Duration::from_millis(90),
                |args, _| {
                    if let Some(out) = inventory(args) {
                        return Ok(out);
                    }
                    stops += 1;
                    Ok(output.into())
                },
            );
            assert!(result.is_err());
            assert_eq!(stops, 1);
        }
    }

    #[test]
    fn a_new_runtime_client_is_found_on_verification_pass() {
        let d = tempfile::tempdir().unwrap();
        process(d.path(), 10, 10274, "com.valvesoftware.steamlinkvr", MAPS);
        let mut stops = 0;
        close_with(
            d.path(),
            Instant::now() + Duration::from_secs(1),
            |args, _| {
                if let Some(out) = inventory(args) {
                    return Ok(out);
                }
                stops += 1;
                if stops == 1 {
                    fs::remove_dir_all(d.path().join("10"))?;
                    process(
                        d.path(),
                        11,
                        10255,
                        "logger.com.valvesoftware.steamlinkvr",
                        MAPS,
                    );
                } else {
                    fs::remove_dir_all(d.path().join("11"))?;
                }
                Ok(String::new())
            },
        )
        .unwrap();
        assert_eq!(stops, 2);
    }

    #[test]
    fn bounded_commands_fail_on_error_timeout_or_excess_output() {
        let run = |script: &str| {
            bounded_command(
                Command::new("/bin/sh").args(["-c", script]),
                Instant::now() + Duration::from_millis(150),
            )
        };
        assert_eq!(run("printf 'ok'").unwrap(), "ok");
        assert!(run("echo denied >&2; exit 7").is_err());
        assert!(run("echo error >&2").is_err());
        assert!(run("head -c 1100000 /dev/zero").is_err());
        let began = Instant::now();
        assert!(run("sleep 5").is_err());
        assert!(began.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn patch_lock_waits_without_removing_existing_owner_or_symlink() {
        let d = tempfile::tempdir().unwrap();
        let lock = d.path().join("transaction");
        let first =
            PatchTransaction::acquire(&lock, Instant::now() + Duration::from_secs(1)).unwrap();
        assert!(
            PatchTransaction::acquire(&lock, Instant::now() + Duration::from_millis(30)).is_err()
        );
        assert!(lock.is_dir());
        drop(first);
        assert!(!lock.exists());
        symlink(d.path(), &lock).unwrap();
        assert!(
            PatchTransaction::acquire(&lock, Instant::now() + Duration::from_millis(30)).is_err()
        );
        assert!(lock.symlink_metadata().unwrap().file_type().is_symlink());
    }
}
