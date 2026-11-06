//! Which migrations a project still needs, and whether commands may run.

use std::path::Path;

use crate::{Kind, Level, Migration, migrations};

/// The project's `clm.toml` names a migration this binary doesn't hold, so a newer `clm`
/// upgraded it.
#[derive(Debug, PartialEq, Eq)]
pub struct TooNew(pub String);

/// The top-level table of a TOML file, or none when it is missing or doesn't parse.
fn table(path: &Path) -> Option<toml::Table> {
    std::fs::read_to_string(path).ok()?.parse().ok()
}

fn strings(table: &toml::Table, key: &str) -> Vec<String> {
    match table.get(key) {
        Some(toml::Value::Array(items)) => items
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

/// The migrations of `list` the project at `root` still needs, in list order.
pub(crate) fn pending_in<'a>(
    list: &'a [Migration],
    root: &Path,
) -> Result<Vec<&'a Migration>, TooNew> {
    let config = table(&root.join("clm.toml")).unwrap_or_default();
    let declined = strings(&config, "declined_migrations");
    // How many migrations the stamp covers: none without one.
    let stamped = match config.get("upgraded").and_then(|v| v.as_str()) {
        Some(id) => Some(
            list.iter()
                .position(|m| m.id == id)
                .ok_or_else(|| TooNew(id.to_string()))?
                + 1,
        ),
        None => None,
    };
    Ok(list
        .iter()
        .enumerate()
        .filter(|(i, m)| {
            !(m.done)(root)
                && match m.kind {
                    Kind::Apply(_) => *i >= stamped.unwrap_or(0),
                    Kind::Note(_) => !declined.iter().any(|d| d == m.id),
                }
        })
        .map(|(_, m)| m)
        .collect())
}

/// The migrations the project at `root` still needs, in list order, or the migration it names
/// that this binary lacks.
pub fn pending(root: &Path) -> Result<Vec<&'static Migration>, TooNew> {
    pending_in(migrations(), root)
}

pub(crate) fn require_current_in(list: &[Migration], root: &Path) -> Result<(), String> {
    let pending = match pending_in(list, root) {
        Ok(pending) => pending,
        Err(TooNew(id)) => {
            return Err(format!(
                "this project was upgraded by a newer `clm` (migration `{id}`); install a newer `clm`"
            ));
        }
    };
    let skipped = clm_config::local_settings_path(root)
        .and_then(|path| table(&path))
        .map(|local| strings(&local, "skip_migrations"))
        .unwrap_or_default();
    let blocking: Vec<&str> = pending
        .iter()
        .filter(|m| m.level == Level::Required && !skipped.iter().any(|s| s == m.id))
        .map(|m| m.id)
        .collect();
    if blocking.is_empty() {
        return Ok(());
    }
    Err(format!(
        "required migration{} pending: {}; run `/clm:upgrade` or `clm upgrade`",
        if blocking.len() == 1 { "" } else { "s" },
        blocking
            .iter()
            .map(|id| format!("`{id}`"))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

/// `Ok` when no required migration is pending outside the clone's `skip_migrations`; otherwise an
/// error naming each such migration and saying to run `/clm:upgrade` or `clm upgrade`.
pub fn require_current(root: &Path) -> Result<(), String> {
    require_current_in(migrations(), root)
}
