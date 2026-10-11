//! Same-boot restart serialization and a kernel-expiring suspend blocker.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
    time::{Duration, Instant},
};

pub const WAKE_TIMEOUT: Duration = Duration::from_secs(180);
const WAKE_NAME: &str = "ksu_soft_reboot";

pub struct RestartGuard {
    // flock follows the open file description through daemonization. Never
    // unlink this file: another caller could otherwise lock a different inode.
    _transaction: File,
    unlock: File,
    active: bool,
    acquired: Instant,
}

impl RestartGuard {
    pub fn acquire() -> io::Result<Self> {
        Self::acquire_at(
            Path::new("/dev/ksu-soft-reboot.lock"),
            Path::new("/sys/power/wake_lock"),
            Path::new("/sys/power/wake_unlock"),
        )
    }

    fn acquire_at(transaction: &Path, wake: &Path, unlock: &Path) -> io::Result<Self> {
        let transaction = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(transaction)?;
        transaction.try_lock().map_err(|error| {
            io::Error::other(format!(
                "soft restart is already running or lock failed: {error}"
            ))
        })?;
        // Open both existing sysfs endpoints before acquiring. No creation,
        // truncation, persistent setting or modification of an original file.
        let mut wake_writer = OpenOptions::new().write(true).open(wake)?;
        let unlock = OpenOptions::new().write(true).open(unlock)?;
        let mut guard = Self {
            _transaction: transaction,
            unlock,
            active: false,
            acquired: Instant::now(),
        };
        // Linux wake_lock takes a nonzero duration in NANOSECONDS. This finite
        // kernel timer is the fallback even for SIGKILL, abort or a hung daemon.
        let request = format!("{WAKE_NAME} {}\n", WAKE_TIMEOUT.as_nanos());
        guard.active = true;
        let written = wake_writer.write(request.as_bytes())?;
        if written != request.len() {
            return Err(io::Error::other("incomplete wake-lock request"));
        }
        if !fs::read_to_string(wake)?
            .split_whitespace()
            .any(|s| s == WAKE_NAME)
        {
            return Err(io::Error::other("restart wake lock did not become active"));
        }
        Ok(guard)
    }

    pub fn remaining(&self) -> Duration {
        WAKE_TIMEOUT.saturating_sub(self.acquired.elapsed())
    }

    pub fn release(&mut self) -> io::Result<()> {
        if self.active {
            let request = format!("{WAKE_NAME}\n");
            if self.unlock.write(request.as_bytes())? != request.len() {
                return Err(io::Error::other("incomplete wake-unlock request"));
            }
            self.active = false;
        }
        Ok(())
    }
}

impl Drop for RestartGuard {
    fn drop(&mut self) {
        if let Err(error) = self.release() {
            log::warn!("restart wake-lock release failed: {error}; kernel timeout remains armed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoints() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let wake = dir.path().join("wake");
        let unlock = dir.path().join("unlock");
        fs::write(&wake, "").unwrap();
        fs::write(&unlock, "").unwrap();
        (dir, wake, unlock)
    }

    #[test]
    fn finite_timeout_and_explicit_release() {
        let (dir, wake, unlock) = endpoints();
        let mut guard = RestartGuard::acquire_at(&dir.path().join("lock"), &wake, &unlock).unwrap();
        assert_eq!(
            fs::read_to_string(wake).unwrap(),
            "ksu_soft_reboot 180000000000\n"
        );
        assert!(guard.remaining() <= WAKE_TIMEOUT);
        guard.release().unwrap();
        guard.release().unwrap();
        drop(guard);
        assert_eq!(fs::read_to_string(unlock).unwrap(), "ksu_soft_reboot\n");
    }

    #[test]
    fn error_unwind_releases_and_allows_retry() {
        let (dir, wake, unlock) = endpoints();
        let lock = dir.path().join("lock");
        let fail = || -> io::Result<()> {
            let _guard = RestartGuard::acquire_at(&lock, &wake, &unlock)?;
            Err(io::Error::other("stage failed"))
        };
        assert!(fail().is_err());
        assert_eq!(fs::read_to_string(&unlock).unwrap(), "ksu_soft_reboot\n");
        assert!(RestartGuard::acquire_at(&lock, &wake, &unlock).is_ok());
    }

    #[test]
    fn duplicate_does_not_touch_wake_lock() {
        let (dir, wake, unlock) = endpoints();
        let lock = dir.path().join("lock");
        let first = RestartGuard::acquire_at(&lock, &wake, &unlock).unwrap();
        assert!(RestartGuard::acquire_at(&lock, &wake, &unlock).is_err());
        assert_eq!(fs::read_to_string(&unlock).unwrap(), "");
        drop(first);
        assert!(RestartGuard::acquire_at(&lock, &wake, &unlock).is_ok());
    }

    #[test]
    fn missing_unlock_fails_before_acquisition() {
        let (dir, wake, unlock) = endpoints();
        fs::remove_file(&unlock).unwrap();
        assert!(RestartGuard::acquire_at(&dir.path().join("lock"), &wake, &unlock).is_err());
        assert_eq!(fs::read_to_string(wake).unwrap(), "");
    }

    #[test]
    fn symlink_transaction_is_rejected() {
        let (dir, wake, unlock) = endpoints();
        let lock = dir.path().join("lock");
        std::os::unix::fs::symlink(&wake, &lock).unwrap();
        assert!(RestartGuard::acquire_at(&lock, &wake, &unlock).is_err());
        assert_eq!(fs::read_to_string(wake).unwrap(), "");
    }
}
