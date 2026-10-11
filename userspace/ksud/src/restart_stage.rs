//! Generic opt-in restart phases; no device, framebuffer or module IDs here.
use crate::{assets, defs, module, utils};
use anyhow::{Context, Result, ensure};
use std::{
    fs,
    io::{Read, Write},
    os::{
        fd::AsRawFd,
        unix::{fs::OpenOptionsExt, process::CommandExt},
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub const STATE: &str = "/dev/ksu-restart-stage";

fn nonblocking(stream: &impl AsRawFd) -> Result<()> {
    let fd = stream.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    ensure!(
        flags >= 0 && unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } >= 0,
        "hook output pipe setup failed"
    );
    Ok(())
}

fn drain(stream: &mut impl Read, log: &mut fs::File, budget: &mut usize) -> Result<()> {
    // Bound each poll and total stored output. Keep draining after truncation so
    // verbose hooks cannot deadlock the coordinator. Never wait for descendant EOF.
    let mut buf = [0u8; 4096];
    for _ in 0..16 {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let keep = n.min(*budget);
                log.write_all(&buf[..keep])?;
                *budget -= keep;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
pub fn begin() -> Result<()> {
    fs::create_dir_all(STATE)?;
    let _ = fs::remove_file(format!("{STATE}/error"));
    let generation = fs::read_to_string("/proc/sys/kernel/random/uuid")?;
    fs::write(format!("{STATE}/generation"), generation)?;
    fs::write(format!("{STATE}/busy"), "restart in progress\n")?;
    let _ = fs::remove_file(format!("{STATE}/ready"));
    Ok(())
}
pub fn ready() -> Result<()> {
    fs::write(format!("{STATE}/ready"), "ready\n")?;
    fs::remove_file(format!("{STATE}/busy"))?;
    Ok(())
}
pub fn blocked() -> bool {
    Path::new(&format!("{STATE}/busy")).exists()
}
pub fn generation() -> Result<String> {
    match fs::read_to_string(format!("{STATE}/generation")) {
        Ok(s) => Ok(s.trim().to_owned()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(fs::read_to_string("/proc/sys/kernel/random/boot_id")?
                .trim()
                .to_owned())
        }
        Err(e) => Err(e.into()),
    }
}
pub fn participants() -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    module::foreach_active_module(|path| {
        if path.join("restart-prepare.sh").is_file() {
            for phase in ["prepare", "stopped", "started", "abort"] {
                ensure!(
                    path.join(format!("restart-{phase}.sh")).is_file(),
                    "incomplete restart hooks: {}",
                    path.display()
                );
            }
            paths.push(path.to_owned());
        }
        Ok(())
    })?;
    paths.sort();
    Ok(paths)
}
pub fn run(paths: &[PathBuf], phase: &str, deadline: Instant) -> Result<()> {
    run_with(
        paths,
        phase,
        deadline,
        &generation()?,
        Path::new(assets::BUSYBOX_PATH),
    )
}

fn run_with(
    paths: &[PathBuf],
    phase: &str,
    deadline: Instant,
    generation: &str,
    shell: &Path,
) -> Result<()> {
    let mut abort_errors = Vec::new();
    for path in paths {
        let result = run_one(path, phase, deadline, generation, shell);
        if phase == "abort" {
            if let Err(e) = result {
                abort_errors.push(format!("{}: {e:#}", path.display()));
            }
        } else {
            result?;
        }
    }
    ensure!(
        abort_errors.is_empty(),
        "abort failures: {}",
        abort_errors.join("; ")
    );
    Ok(())
}

fn run_one(
    path: &Path,
    phase: &str,
    deadline: Instant,
    generation: &str,
    shell: &Path,
) -> Result<()> {
    // Abort addresses the original snapshot, including a now-disabled participant.
    if phase != "abort" {
        ensure!(
            !path.join("disable").exists() && !path.join("remove").exists(),
            "participant disabled during restart"
        );
    }
    ensure!(Instant::now() < deadline, "restart hook deadline exhausted");
    let id = path
        .file_name()
        .and_then(|s| s.to_str())
        .context("module ID")?;
    module::validate_module_id(id)?;
    let script = path.join(format!("restart-{phase}.sh"));
    let phase_limit = match phase {
        "stopped" => 45,
        "started" => 70,
        _ => 30,
    };
    let deadline = deadline.min(Instant::now() + Duration::from_secs(phase_limit));
    #[cfg(not(test))]
    let log_dir = PathBuf::from(format!("{STATE}/logs"));
    #[cfg(test)]
    let log_dir = path.join(".restart-logs");
    fs::create_dir_all(&log_dir)?;
    let log_path = log_dir.join(format!("{id}-{phase}.log"));
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&log_path)?;
    writeln!(output, "generation={generation} module={id} phase={phase}")?;
    let began = Instant::now();
    let mut budget = 64 * 1024;
    let mut child = Command::new(shell)
        .arg("sh")
        .arg(&script)
        .current_dir(path)
        .envs(module::get_common_script_envs(Some(id)))
        .env("KSU_RESTART_GENERATION", generation)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()?;
    let mut stdout = child.stdout.take().context("hook stdout")?;
    let mut stderr = child.stderr.take().context("hook stderr")?;
    let result = (|| -> Result<()> {
        nonblocking(&stdout)?;
        nonblocking(&stderr)?;
        loop {
            drain(&mut stdout, &mut output, &mut budget)?;
            drain(&mut stderr, &mut output, &mut budget)?;
            if let Some(status) = child.try_wait()? {
                drain(&mut stdout, &mut output, &mut budget)?;
                drain(&mut stderr, &mut output, &mut budget)?;
                writeln!(
                    output,
                    "\nexit={status} elapsed_ms={} truncated={}",
                    began.elapsed().as_millis(),
                    budget == 0
                )?;
                ensure!(
                    status.success(),
                    "{} exited {status}; log={}",
                    script.display(),
                    log_path.display()
                );
                break;
            }
            if Instant::now() >= deadline {
                // Only the known hook's new process group. A participant's
                // independent recovery guardian must use a separate session.
                unsafe {
                    libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
                }
                let _ = child.wait();
                writeln!(
                    output,
                    "\ntimeout elapsed_ms={}",
                    began.elapsed().as_millis()
                )?;
                anyhow::bail!(
                    "{} timed out; hook group stopped; abort required",
                    script.display()
                );
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(())
    })();
    if result.is_err() {
        // Includes pipe/log I/O errors: do not leave a mutation hook running
        // concurrently with abort after the coordinator returns an error.
        unsafe {
            libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
        }
        let _ = child.wait();
    }
    result
}
pub fn launch(path: &Path) -> Result<()> {
    Command::new(std::env::current_exe()?)
        .arg("supervise-script")
        .arg(path)
        .arg(generation()?)
        .spawn()?;
    Ok(())
}
pub fn supervise(path: &Path, generation: &str) -> Result<()> {
    ensure!(unsafe { libc::geteuid() } == 0, "root required");
    let path = fs::canonicalize(path)?;
    ensure!(
        path.starts_with(defs::ADB_DIR),
        "script outside adb directory"
    );
    fs::create_dir_all(format!("{STATE}/scripts"))?;
    let key = sha256::digest(path.to_string_lossy().as_bytes());
    let lock = PathBuf::from(format!("{STATE}/scripts/{key}"));
    let Some(mut lease) = crate::service_guard::Lease::acquire(&lock, generation)? else {
        return Ok(());
    };
    crate::service_guard::adopt_descendants()?;
    utils::switch_cgroups();
    let result = module::exec_script_direct(&path, module::ScriptWait::Forever);
    crate::service_guard::wait_descendants()?;
    lease.finish(result.is_ok())?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_hook_output_is_saved_and_large_output_is_bounded() {
        let (_d, m, s) = fixture();
        fs::write(
            m.join("restart-started.sh"),
            "echo exact-reason >&2\nexit 7\n",
        )
        .unwrap();
        assert!(
            run_with(
                &[m.clone()],
                "started",
                Instant::now() + Duration::from_secs(2),
                "log-generation",
                &s
            )
            .is_err()
        );
        let log = m.join(".restart-logs/module-started.log");
        let text = fs::read_to_string(&log).unwrap();
        assert!(
            text.contains("exact-reason")
                && text.contains("log-generation")
                && text.contains("exit=")
        );
        fs::write(
            m.join("restart-started.sh"),
            "head -c 200000 /dev/zero\necho end >&2\n",
        )
        .unwrap();
        run_with(
            &[m.clone()],
            "started",
            Instant::now() + Duration::from_secs(3),
            "large",
            &s,
        )
        .unwrap();
        let data = fs::read(&log).unwrap();
        assert!(data.len() < 66000);
        assert!(String::from_utf8_lossy(&data).contains("truncated=true"));
    }

    #[test]
    fn inherited_output_pipe_does_not_wait_for_background_eof() {
        let (_d, m, s) = fixture();
        fs::write(m.join("restart-started.sh"), "sleep 1 &\necho ready\n").unwrap();
        let began = Instant::now();
        run_with(
            &[m],
            "started",
            began + Duration::from_secs(2),
            "background",
            &s,
        )
        .unwrap();
        assert!(began.elapsed() < Duration::from_millis(700));
    }
    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let module = dir.path().join("module");
        fs::create_dir(&module).unwrap();
        // Production invokes BusyBox with argv[1]=sh. Adapter preserves args.
        let shell = dir.path().join("busybox");
        fs::write(&shell, "#!/bin/sh\nshift\nexec /bin/sh \"$@\"\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&shell, fs::Permissions::from_mode(0o755)).unwrap();
        (dir, module, shell)
    }
    #[test]
    fn phase_order_and_generation() {
        let (_d, m, s) = fixture();
        for p in ["prepare", "stopped", "started"] {
            fs::write(
                m.join(format!("restart-{p}.sh")),
                format!("echo {p}:$KSU_RESTART_GENERATION >> history\n"),
            )
            .unwrap();
            run_with(
                &[m.clone()],
                p,
                Instant::now() + Duration::from_secs(2),
                "first",
                &s,
            )
            .unwrap();
        }
        assert_eq!(
            fs::read_to_string(m.join("history")).unwrap(),
            "prepare:first\nstopped:first\nstarted:first\n"
        );
    }
    #[test]
    fn disabled_participant_refuses_start_but_receives_abort() {
        let (_d, m, s) = fixture();
        fs::write(m.join("disable"), "").unwrap();
        fs::write(m.join("restart-prepare.sh"), "touch unexpected\n").unwrap();
        fs::write(m.join("restart-abort.sh"), "touch aborted\n").unwrap();
        assert!(
            run_with(
                &[m.clone()],
                "prepare",
                Instant::now() + Duration::from_secs(2),
                "g",
                &s
            )
            .is_err()
        );
        run_with(
            &[m.clone()],
            "abort",
            Instant::now() + Duration::from_secs(2),
            "g",
            &s,
        )
        .unwrap();
        assert!(m.join("aborted").exists());
        assert!(!m.join("unexpected").exists());
    }
    #[test]
    fn failed_hook_and_timeout_are_not_success() {
        let (_d, m, s) = fixture();
        fs::write(m.join("restart-stopped.sh"), "exit 7\n").unwrap();
        assert!(
            run_with(
                &[m.clone()],
                "stopped",
                Instant::now() + Duration::from_secs(2),
                "g",
                &s
            )
            .is_err()
        );
        fs::write(
            m.join("restart-stopped.sh"),
            "sleep 1\ntouch unsafe-late-write\n",
        )
        .unwrap();
        assert!(
            run_with(
                &[m.clone()],
                "stopped",
                Instant::now() + Duration::from_millis(100),
                "g",
                &s
            )
            .is_err()
        );
        std::thread::sleep(Duration::from_millis(1100));
        assert!(!m.join("unsafe-late-write").exists());
    }
    #[test]
    fn abort_failure_does_not_skip_other_participants() {
        let (d, m, s) = fixture();
        let n = d.path().join("other");
        fs::create_dir(&n).unwrap();
        fs::write(m.join("restart-abort.sh"), "exit 1\n").unwrap();
        fs::write(n.join("restart-abort.sh"), "touch aborted\n").unwrap();
        assert!(
            run_with(
                &[m, n.clone()],
                "abort",
                Instant::now() + Duration::from_secs(2),
                "g",
                &s
            )
            .is_err()
        );
        assert!(n.join("aborted").exists());
    }
}
