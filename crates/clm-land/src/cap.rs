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

/// What [`cap_directory`] returns when it did something: the line for a directory it removed,
/// or the warning for one it could not measure or remove.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capped {
    /// `cleared <name> (<n> GB, cap <cap> GB)`.
    Cleared(String),
    /// A warning naming the path; the directory is left as it was.
    Warning(String),
}

impl Capped {
    /// The line to print.
    pub fn line(&self) -> &str {
        match self {
            Capped::Cleared(line) | Capped::Warning(line) => line,
        }
    }
}

/// Removes `dir` when its regular files total more than `cap_gb` gigabytes (10^9 bytes each),
/// following no symbolic link, and returns the line saying so as `name` (`shared-target-cap`).
/// A directory that doesn't exist gives nothing; one that can't be measured or removed gives a
/// warning naming the path and is left alone.
pub fn cap_directory(dir: &Path, name: &str, cap_gb: u64) -> Option<Capped> {
    let sum = match measure(dir) {
        Ok(sum) => sum,
        // A check that built nothing leaves nothing to cap.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && !dir.exists() => return None,
        Err(e) => {
            return Some(Capped::Warning(format!(
                "didn't measure {}: {e}",
                dir.display()
            )));
        }
    };
    if sum <= cap_gb.saturating_mul(GB) {
        return None;
    }
    match fs::remove_dir_all(dir) {
        Ok(()) => Some(Capped::Cleared(format!(
            "cleared {name} ({} GB, cap {cap_gb} GB)",
            sum / GB
        ))),
        Err(e) => Some(Capped::Warning(format!(
            "didn't remove {}: {e}",
            dir.display()
        ))),
    }
}

/// The cap on the land's own target, `.clm/land` (`land/land-target-cap`), through the shared
/// call; its line goes to the land's warnings.
pub(crate) fn apply(target: &Path, cap_gb: u64, stderr: &mut Vec<String>) {
    if let Some(capped) = cap_directory(target, ".clm/land", cap_gb) {
        stderr.push(capped.line().to_string());
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

    fn sparse(path: &Path, len: u64) {
        fs::File::create(path).unwrap().set_len(len).unwrap();
    }

    #[test]
    fn over_the_cap_clears_and_keeps_link_targets() {
        // req:land/shared-target-cap@07ebae61
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let target = dir.path().join("x");
        fs::create_dir(&target).unwrap();
        sparse(&target.join("big"), 2_000_000_000);
        sparse(&outside.path().join("other"), 2_000_000_000);
        std::os::unix::fs::symlink(outside.path(), target.join("link")).unwrap();
        let got = cap_directory(&target, ".clm/x", 1);
        assert_eq!(
            got,
            Some(Capped::Cleared("cleared .clm/x (2 GB, cap 1 GB)".into()))
        );
        assert!(!target.exists());
        assert!(outside.path().join("other").exists());
    }

    #[test]
    fn under_the_cap_returns_nothing() {
        // req:land/shared-target-cap@07ebae61
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("f"), [0u8; 10]).unwrap();
        assert_eq!(cap_directory(dir.path(), ".clm/x", 1), None);
        assert!(dir.path().join("f").exists());
    }

    #[test]
    fn no_directory_returns_nothing() {
        // req:land/shared-target-cap@07ebae61
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(cap_directory(&dir.path().join("none"), ".clm/x", 1), None);
    }

    #[test]
    fn unreadable_entry_warns_and_removes_nothing() {
        // req:land/shared-target-cap@07ebae61
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        sparse(&dir.path().join("big"), 2_000_000_000);
        fs::set_permissions(&sub, fs::Permissions::from_mode(0o000)).unwrap();
        let got = cap_directory(dir.path(), ".clm/x", 1);
        fs::set_permissions(&sub, fs::Permissions::from_mode(0o755)).unwrap();
        if fs::read_dir(&sub).is_ok() {
            return; // running as a user who reads anything: nothing to observe
        }
        match got {
            Some(Capped::Warning(w)) => assert!(w.contains(&dir.path().display().to_string())),
            other => panic!("{other:?}"),
        }
        assert!(dir.path().join("big").exists());
    }
}
