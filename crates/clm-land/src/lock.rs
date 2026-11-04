//! One check at a time on the clone's shared `.clm/land` (`land/land-target-lock`): an
//! exclusive advisory lock of the operating system on `.clm/land.lock`, released when the
//! file is dropped, or by the system when the process dies.

use std::fs::{File, TryLockError};
use std::path::Path;

use crate::Error;

/// Locks `<target>.lock`, creating it and its directory when missing. When another land
/// holds it, says so on stderr once and waits without a time limit. Also says whether it
/// waited.
pub(crate) fn lock(target: &Path) -> Result<(File, bool), Error> {
    let mut path = target.as_os_str().to_owned();
    path.push(".lock");
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)?;
    let waited = match file.try_lock() {
        Ok(()) => false,
        Err(TryLockError::WouldBlock) => {
            eprintln!("waiting for another land's check on {}", target.display());
            file.lock()?;
            true
        }
        Err(TryLockError::Error(e)) => return Err(e.into()),
    };
    Ok((file, waited))
}
