//! What the system gives: load averages, memory and a filesystem's free space.

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use crate::{Readings, Space};

/// The machine's readings, with the free space of the filesystem holding `disk_path`.
pub(crate) fn read_system(disk_path: &Path) -> Readings {
    // A reading the system refuses is unknown.
    let memory = sysinfo::System::new().ok().and_then(|mut system| {
        system.refresh_memory();
        let total = system.total_memory();
        (total > 0).then(|| Space {
            total,
            available: system.available_memory().min(total),
        })
    });
    let load = sysinfo::System::load_average().ok();
    Readings {
        cores: std::thread::available_parallelism().map_or(1, |n| n.get() as u64),
        load1: load.as_ref().map(|l| l.one),
        load5: load.as_ref().map(|l| l.five),
        memory,
        disk_path: disk_path.to_path_buf(),
        disk: filesystem(disk_path),
    }
}

/// `statvfs`'s total and the space an unprivileged process can still write.
#[allow(clippy::unnecessary_cast)]
fn filesystem(path: &Path) -> Option<Space> {
    let path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat = std::mem::MaybeUninit::<libc::statvfs>::zeroed();
    // SAFETY: `path` is a NUL-terminated string and `stat` points to writable storage of the
    // type `statvfs` fills.
    let status = unsafe { libc::statvfs(path.as_ptr(), stat.as_mut_ptr()) };
    if status != 0 {
        return None;
    }
    // SAFETY: a zero status means `statvfs` filled the structure.
    let stat = unsafe { stat.assume_init() };
    let unit = stat.f_frsize as u64;
    Some(Space {
        total: stat.f_blocks as u64 * unit,
        available: stat.f_bavail as u64 * unit,
    })
}
