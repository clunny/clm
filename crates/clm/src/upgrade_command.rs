//! `clm upgrade`: which `clm-upgrade` call each flag makes (`upgrade-command`).

use std::path::Path;

/// What `clm upgrade` was asked to do: at most one of its flags is set.
pub(crate) struct Request {
    pub apply: bool,
    pub note: Option<String>,
    pub decline: Option<String>,
    pub lock: Option<String>,
}

/// The text to print, or the refusal to print with exit status 1. Every flag works over the
/// migrations `clm-init` holds, `upgrade`'s own and its own (`init/all-migrations`).
pub(crate) fn run(root: &Path, request: Request) -> Result<String, String> {
    let list = clm_init::migrations();
    let supersedes = clm_init::supersedes();
    if let Some(id) = request.note {
        clm_upgrade::note_in(&list, &supersedes, root, &id)
    } else if let Some(id) = request.decline {
        clm_upgrade::decline_in(&list, &supersedes, root, &id)
    } else if let Some(version) = request.lock {
        clm_upgrade::list_in(&list, &supersedes, root)?;
        let upgrade_ids: Vec<&str> = clm_upgrade::migrations().iter().map(|m| m.id).collect();
        let init_ids: Vec<&str> = clm_init::own_migrations().iter().map(|m| m.id).collect();
        lock(root, &version, &upgrade_ids, &init_ids)
    } else if request.apply {
        clm_upgrade::apply_in(&list, &supersedes, root)
    } else {
        clm_upgrade::list_in(&list, &supersedes, root)
    }
}

/// `--lock`'s step: appends to `crates/clm-upgrade/migrations.lock` for `upgrade_ids` and to
/// `crates/clm-init/migrations.lock` for `init_ids`, creating the second only when `init_ids`
/// holds a migration, and returns `locked <id>` for each line it added.
pub(crate) fn lock(
    root: &Path,
    version: &str,
    upgrade_ids: &[&str],
    init_ids: &[&str],
) -> Result<String, String> {
    let mut added =
        clm_upgrade::append_lock(&root.join("crates/clm-upgrade"), version, upgrade_ids)?;
    if !init_ids.is_empty() {
        added.extend(clm_upgrade::append_lock(
            &root.join("crates/clm-init"),
            version,
            init_ids,
        )?);
    }
    Ok(added.iter().map(|id| format!("locked {id}\n")).collect())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn migration(root: &Path, krate: &str, id: &str) {
        let dir = root
            .join("crates")
            .join(krate)
            .join("src/migrations")
            .join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("mod.rs"), id).unwrap();
    }

    // req:clm/upgrade-command@0552ef64
    #[test]
    fn a_migration_of_inits_is_locked_in_inits_own_lock() {
        let dir = TempDir::new().unwrap();
        migration(dir.path(), "clm-upgrade", "a");
        migration(dir.path(), "clm-init", "z");
        let upgrade_lock = dir.path().join("crates/clm-upgrade/migrations.lock");
        fs::write(&upgrade_lock, "a 0.1.0 x\n").unwrap();
        let out = lock(dir.path(), "0.9.0", &["a"], &["z"]).unwrap();
        assert_eq!(out, "locked z\n");
        let init = fs::read_to_string(dir.path().join("crates/clm-init/migrations.lock")).unwrap();
        assert_eq!(init.lines().count(), 1);
        assert!(init.starts_with("z 0.9.0 "), "{init}");
        assert_eq!(
            fs::read_to_string(upgrade_lock).unwrap(),
            "a 0.1.0 x\n",
            "upgrade's lock gains no line"
        );
    }

    // req:clm/upgrade-command@0552ef64
    #[test]
    fn no_lock_is_created_for_an_init_with_no_migration() {
        let dir = TempDir::new().unwrap();
        migration(dir.path(), "clm-upgrade", "a");
        let out = lock(dir.path(), "0.9.0", &["a"], &[]).unwrap();
        assert_eq!(out, "locked a\n");
        assert!(!dir.path().join("crates/clm-init/migrations.lock").exists());
    }
}
