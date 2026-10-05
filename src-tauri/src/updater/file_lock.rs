use chrono::Utc;
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime},
};

pub(crate) const UPDATE_STATE_LOCK_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const UPDATE_STATE_LOCK_POLL_INTERVAL: Duration = Duration::from_millis(50);
pub(crate) const STALE_UPDATE_STATE_LOCK_AGE: Duration = Duration::from_secs(5 * 60);

#[derive(Debug)]
pub(crate) struct UpdateStateLock {
    path: PathBuf,
}

impl Drop for UpdateStateLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub(crate) fn acquire_update_state_lock(state_path: &Path) -> io::Result<UpdateStateLock> {
    let lock_path = state_path.with_extension("lock");
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let deadline = Instant::now() + UPDATE_STATE_LOCK_TIMEOUT;
    loop {
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(mut file) => {
                writeln!(file, "{} {}", std::process::id(), Utc::now().to_rfc3339())?;
                return Ok(UpdateStateLock { path: lock_path });
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                remove_stale_update_state_lock(&lock_path);
                if Instant::now() >= deadline {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!("timed out waiting for state lock {}", lock_path.display()),
                    ));
                }
                thread::sleep(UPDATE_STATE_LOCK_POLL_INTERVAL);
            }
            Err(error) => return Err(error),
        }
    }
}

fn remove_stale_update_state_lock(lock_path: &Path) {
    let Ok(metadata) = fs::metadata(lock_path) else {
        return;
    };
    let Ok(modified) = metadata.modified() else {
        return;
    };
    let Ok(age) = SystemTime::now().duration_since(modified) else {
        return;
    };
    if age >= STALE_UPDATE_STATE_LOCK_AGE {
        let _ = fs::remove_file(lock_path);
    }
}
