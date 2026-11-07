//! The clone's two advisory locks, exclusive locks of the operating system released when the
//! file is dropped, or by the system when the process dies: one check at a time on the shared
//! `.clm/land` (`land/land-target-lock`, `.clm/land.lock`) and one `[land] check` at a time
//! on the clone (`land/check-lock`, `.clm/check.lock`).

use std::fs::{File, TryLockError};
use std::path::Path;

use crate::Error;

/// Locks the file at `path`, creating it and its directory when missing. When another
/// process holds it, passes `waiting` to `say` once and waits without a time limit. Also
/// says whether it waited.
fn lock(path: &Path, waiting: &str, say: impl FnOnce(&str)) -> Result<(File, bool), Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)?;
    let waited = match file.try_lock() {
        Ok(()) => false,
        Err(TryLockError::WouldBlock) => {
            say(waiting);
            file.lock()?;
            true
        }
        Err(TryLockError::Error(e)) => return Err(e.into()),
    };
    Ok((file, waited))
}

/// Locks `<target>.lock`, the land lock. Also says whether it waited.
pub(crate) fn land(target: &Path) -> Result<(File, bool), Error> {
    let mut path = target.as_os_str().to_owned();
    path.push(".lock");
    let waiting = format!("waiting for another land's check on {}", target.display());
    lock(Path::new(&path), &waiting, |line| eprintln!("{line}"))
}

/// Locks `check.lock` in `places`, the check lock, passing the waiting line to `say` once
/// when another check holds it.
pub(crate) fn check(places: &Path, say: impl FnOnce(&str)) -> Result<File, Error> {
    let path = places.join("check.lock");
    let waiting = format!("waiting for another check on {}", path.display());
    lock(&path, &waiting, say).map(|(file, _)| file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn a_held_lock_is_waited_for_and_named_once() {
        // req:land/check-lock@069468cd
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".clm/check.lock");
        let (held, waited) = lock(&path, "unsaid", |_| panic!("said")).unwrap();
        assert!(!waited);
        let releaser = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            drop(held);
        });
        let mut said = Vec::new();
        let start = Instant::now();
        let (_file, waited) = lock(&path, "waiting for another check", |l| {
            said.push(l.to_string())
        })
        .unwrap();
        assert!(waited);
        assert!(start.elapsed() >= Duration::from_millis(200));
        assert_eq!(said, ["waiting for another check"]);
        releaser.join().unwrap();
    }
}
