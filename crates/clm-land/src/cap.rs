//! The cap on the shared target `.clm/land` (`land/land-target-cap`): after a push, under the
//! lock, a target above `[land] target_cap_gb` is removed, since Cargo prunes nothing there.

use std::fs;
use std::path::Path;

const GB: u64 = 1_000_000_000;

/// The sum of the lengths of the regular files under `dir`, following no symbolic link; an
/// entry that can't be read fails the measure.
fn measure(dir: &Path) -> std::io::Result<u64> {
    let mut sum = 0;
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let meta = fs::symlink_metadata(&path)?;
        if meta.is_dir() {
            sum += measure(&path)?;
        } else if meta.is_file() {
            sum += meta.len();
        }
    }
    Ok(sum)
}

/// Removes `target` when it holds more than `cap_gb` gigabytes and says so on `stderr`
/// (the land's warnings); a failure to measure or remove only adds a warning naming the path.
pub(crate) fn apply(target: &Path, cap_gb: u64, stderr: &mut Vec<String>) {
    let sum = match measure(target) {
        Ok(sum) => sum,
        // A check that built nothing leaves nothing to cap.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && !target.exists() => return,
        Err(e) => {
            stderr.push(format!("didn't measure {}: {e}", target.display()));
            return;
        }
    };
    if sum <= cap_gb.saturating_mul(GB) {
        return;
    }
    match fs::remove_dir_all(target) {
        Ok(()) => stderr.push(format!(
            "cleared .clm/land ({} GB, cap {cap_gb} GB)",
            sum / GB
        )),
        Err(e) => stderr.push(format!("didn't remove {}: {e}", target.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_follows_no_symlink() {
        // req:land/land-target-cap@69c8919e
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("big"), vec![0u8; 1000]).unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/a"), [0u8; 10]).unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("link")).unwrap();
        std::os::unix::fs::symlink(outside.path().join("big"), dir.path().join("flink")).unwrap();
        assert_eq!(measure(dir.path()).unwrap(), 10);
    }
}
