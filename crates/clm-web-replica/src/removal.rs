//! An uninstall deletes what the repo's object holds (`uninstall-deletes`).

use serde_json::Value;

/// The object's storage, as far as an uninstall needs it.
pub trait Storage {
    fn delete_all(&mut self);
}

/// Handles `event` as the object named `name` (`<owner>/<repo>`): an
/// `installation_repositories` event with action `removed`, or an `installation` event with
/// action `deleted`, that names `name` deletes the storage. Answers whether it did.
pub fn on_removal<S: Storage>(store: &mut S, name: &str, event: &str, body: &Value) -> bool {
    let named = match (event, body["action"].as_str()) {
        ("installation_repositories", Some("removed")) => &body["repositories_removed"],
        ("installation", Some("deleted")) => &body["repositories"],
        _ => return false,
    };
    let hit = named
        .as_array()
        .into_iter()
        .flatten()
        .any(|r| r["full_name"].as_str() == Some(name));
    if hit {
        store.delete_all();
    }
    hit
}
