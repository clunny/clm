//! An uninstall deletes the storage of each repo's object.

use clm_web_replica::{Storage, on_removal};
use serde_json::json;

/// An object's storage, which holds `name`'s data until it is deleted.
struct Object(Option<&'static str>);

impl Storage for Object {
    fn delete_all(&mut self) {
        self.0 = None;
    }
}

#[test]
fn a_repo_removed() {
    // req:web.worker.replica/uninstall-deletes@b0ac190f
    let event = json!({
        "action": "removed",
        "repositories_removed": [{"full_name": "o/a"}],
        "repositories_added": [{"full_name": "o/b"}],
    });
    let (mut a, mut b) = (Object(Some("data a")), Object(Some("data b")));
    assert!(on_removal(
        &mut a,
        "o/a",
        "installation_repositories",
        &event
    ));
    assert!(!on_removal(
        &mut b,
        "o/b",
        "installation_repositories",
        &event
    ));
    assert_eq!(a.0, None);
    assert_eq!(b.0, Some("data b"));
}

#[test]
fn an_installation_deleted() {
    // req:web.worker.replica/uninstall-deletes@b0ac190f
    let event = json!({
        "action": "deleted",
        "repositories": [{"full_name": "o/a"}, {"full_name": "o/b"}],
    });
    let (mut a, mut c) = (Object(Some("data a")), Object(Some("data c")));
    assert!(on_removal(&mut a, "o/a", "installation", &event));
    assert!(!on_removal(&mut c, "o/c", "installation", &event));
    assert_eq!(a.0, None);
    assert_eq!(c.0, Some("data c"));
}

#[test]
fn other_actions_delete_nothing() {
    // req:web.worker.replica/uninstall-deletes@b0ac190f
    let mut a = Object(Some("data a"));
    let added = json!({"action": "added", "repositories_removed": [{"full_name": "o/a"}]});
    assert!(!on_removal(
        &mut a,
        "o/a",
        "installation_repositories",
        &added
    ));
    let created = json!({"action": "created", "repositories": [{"full_name": "o/a"}]});
    assert!(!on_removal(&mut a, "o/a", "installation", &created));
    let push = json!({"action": "removed", "repositories_removed": [{"full_name": "o/a"}]});
    assert!(!on_removal(&mut a, "o/a", "push", &push));
    assert_eq!(a.0, Some("data a"));
}
