//! Volatile script ownership. Keep the lock until all daemonized descendants exit.
//! An interrupted supervisor is quarantined until a cold boot, never guessed dead.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};

pub struct Lease {
    file: File,
    generation: String,
}
impl Lease {
    pub fn acquire(path: &Path, generation: &str) -> io::Result<Option<Self>> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        match file.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => return Ok(None),
            Err(e) => return Err(io::Error::other(e)),
        }
        let previous = fs::read_to_string(path)?;
        if previous.starts_with("running ") || previous.starts_with("failed ") {
            return Err(io::Error::other(
                "interrupted or failed script; cold boot required",
            ));
        }
        if previous == format!("done {generation}\n") {
            return Ok(None);
        }
        let mut lease = Self {
            file,
            generation: generation.to_owned(),
        };
        lease.record("running")?;
        Ok(Some(lease))
    }
    fn record(&mut self, state: &str) -> io::Result<()> {
        use std::io::{Seek, SeekFrom};
        self.file.set_len(0)?;
        self.file.seek(SeekFrom::Start(0))?;
        writeln!(self.file, "{state} {}", self.generation)?;
        self.file.sync_all()
    }
    pub fn finish(&mut self, success: bool) -> io::Result<()> {
        self.record(if success { "done" } else { "failed" })
    }
}

pub fn adopt_descendants() -> io::Result<()> {
    if unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
pub fn wait_descendants() -> io::Result<()> {
    loop {
        let rc = unsafe { libc::waitpid(-1, std::ptr::null_mut(), 0) };
        if rc >= 0 {
            continue;
        }
        let e = io::Error::last_os_error();
        match e.raw_os_error() {
            Some(libc::ECHILD) => return Ok(()),
            Some(libc::EINTR) => {}
            _ => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn daemonized_descendants_keep_ownership() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("owner");
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &format!(
                    "{}::supervisor_fixture",
                    module_path!().split_once("::").unwrap().1
                ),
                "--test-threads=1",
            ])
            .env("KSU_TEST_LEASE", &p)
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !dir.path().join("ready").exists() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(Lease::acquire(&p, "second").unwrap().is_none());
        assert!(child.wait().unwrap().success());
        assert!(Lease::acquire(&p, "first").unwrap().is_none());
        assert!(Lease::acquire(&p, "second").unwrap().is_some());
    }
    #[test]
    fn supervisor_fixture() {
        let Ok(p) = std::env::var("KSU_TEST_LEASE") else {
            return;
        };
        let p = Path::new(&p);
        let mut lease = Lease::acquire(p, "first").unwrap().unwrap();
        adopt_descendants().unwrap();
        let status = std::process::Command::new("sh")
            .args(["-c", "setsid sh -c 'sleep 0.5' >/dev/null 2>&1 &"])
            .status()
            .unwrap();
        assert!(status.success());
        fs::write(p.parent().unwrap().join("ready"), "").unwrap();
        wait_descendants().unwrap();
        lease.finish(true).unwrap();
    }
    #[test]
    fn same_generation_once_new_generation_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("lock");
        let mut a = Lease::acquire(&p, "one").unwrap().unwrap();
        assert!(Lease::acquire(&p, "two").unwrap().is_none());
        a.finish(true).unwrap();
        drop(a);
        assert!(Lease::acquire(&p, "one").unwrap().is_none());
        assert!(Lease::acquire(&p, "two").unwrap().is_some());
    }
    #[test]
    fn interrupted_supervisor_never_duplicates_survivors() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("lock");
        drop(Lease::acquire(&p, "one").unwrap());
        assert!(Lease::acquire(&p, "two").is_err());
    }
    #[test]
    fn failure_survives_soft_generation_but_not_cold_storage() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("lock");
        let mut a = Lease::acquire(&p, "one").unwrap().unwrap();
        a.finish(false).unwrap();
        drop(a);
        assert!(Lease::acquire(&p, "two").is_err());
        let cold = tempfile::tempdir().unwrap();
        assert!(
            Lease::acquire(&cold.path().join("lock"), "three")
                .unwrap()
                .is_some()
        );
    }
}
