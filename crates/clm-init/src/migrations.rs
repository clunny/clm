//! The migrations the binary holds: `upgrade`'s own, then this crate's in the order they landed
//! (`all-migrations`). Each of this crate's is a directory `src/migrations/<id>/` holding `mod.rs`
//! and any text it embeds, pulled in here with `#[path]` since an ID such as
//! `20261114-x` isn't a Rust identifier.

use clm_upgrade::Migration;

/// This crate's own migrations, oldest first.
static OWN: &[Migration] = &[];

/// `(migration, note it supersedes)` for this crate's own migrations.
static OWN_SUPERSEDES: &[(&str, &str)] = &[];

/// This crate's own migrations alone, which the lock step reads.
pub fn own_migrations() -> &'static [Migration] {
    OWN
}

/// `upgrade`'s migrations followed by `own`.
fn list_with(own: &[Migration]) -> Vec<Migration> {
    let mut list = clm_upgrade::migrations().to_vec();
    list.extend_from_slice(own);
    list
}

/// Every migration the binary holds: `upgrade`'s own, then this crate's.
pub fn migrations() -> Vec<Migration> {
    list_with(OWN)
}

/// `upgrade`'s pairs followed by this crate's.
pub fn supersedes() -> Vec<(&'static str, &'static str)> {
    let mut pairs = clm_upgrade::supersedes().to_vec();
    pairs.extend_from_slice(OWN_SUPERSEDES);
    pairs
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use clm_upgrade::{Kind, Level, pending_in};

    use super::*;

    fn done_when_z_exists(root: &Path) -> bool {
        root.join("z").exists()
    }

    fn apply_z(root: &Path) -> Result<(), String> {
        fs::write(root.join("z"), "").map_err(|e| e.to_string())
    }

    fn migration(id: &'static str) -> Migration {
        Migration {
            id,
            level: Level::Advised,
            done: done_when_z_exists,
            kind: Kind::Apply(apply_z, ""),
        }
    }

    /// The test's check of a list: no two migrations share an ID, and none of the crate's own is
    /// `required` with a note.
    fn check_list(list: &[Migration], own: &[Migration]) -> Result<(), String> {
        for (i, m) in list.iter().enumerate() {
            if list[..i].iter().any(|n| n.id == m.id) {
                return Err(format!("duplicate migration ID `{}`", m.id));
            }
        }
        for m in own {
            if m.level == Level::Required && matches!(m.kind, Kind::Note(_)) {
                return Err(format!("migration `{}` is required with a note", m.id));
            }
        }
        Ok(())
    }

    // req:init/all-migrations@acba0c68
    #[test]
    fn the_list_today_is_upgrades_own_and_its_pairs() {
        assert!(own_migrations().is_empty());
        let ids: Vec<&str> = migrations().iter().map(|m| m.id).collect();
        let upgrades: Vec<&str> = clm_upgrade::migrations().iter().map(|m| m.id).collect();
        assert_eq!(upgrades.len(), 13);
        assert_eq!(ids, upgrades);
        assert_eq!(supersedes(), clm_upgrade::supersedes());
        check_list(&migrations(), own_migrations()).unwrap();
    }

    // req:init/all-migrations@acba0c68
    #[test]
    fn a_migration_of_its_own_comes_last_and_is_pending() {
        let own = [migration("z")];
        let list = list_with(&own);
        assert_eq!(list.len(), 14);
        assert_eq!(list.last().unwrap().id, "z");
        assert_eq!(clm_upgrade::latest_in(&list), Some("z"));
        check_list(&list, &own).unwrap();

        let dir = tempfile::TempDir::new().unwrap();
        let last_of_upgrade = clm_upgrade::migrations().last().unwrap().id;
        fs::write(
            dir.path().join("clm.toml"),
            format!("upgraded = \"{last_of_upgrade}\"\n"),
        )
        .unwrap();
        let pending = pending_in(&list, &supersedes(), dir.path()).unwrap();
        assert!(pending.iter().any(|m| m.id == "z"), "z is pending");
    }

    // req:init/all-migrations@acba0c68
    #[test]
    fn a_duplicate_id_fails_the_check_naming_it() {
        let taken = clm_upgrade::migrations()[3].id;
        let own = [migration(taken)];
        let err = check_list(&list_with(&own), &own).unwrap_err();
        assert!(err.contains(taken), "{err}");
    }

    // req:init/all-migrations@acba0c68
    #[test]
    fn a_required_note_of_its_own_fails_the_check() {
        let own = [Migration {
            id: "n",
            level: Level::Required,
            done: done_when_z_exists,
            kind: Kind::Note(clm_upgrade::Note {
                text: "t",
                file: "f",
                old: &[],
                new: None,
            }),
        }];
        assert!(check_list(&list_with(&own), &own).is_err());
    }
}
