//! An uninstall deletes the storage of each repo's object.

mod common;

use clm_web_replica::{Storage, on_removal};
use common::*;
use futures::executor::block_on;
use serde_json::json;

#[test]
fn an_installation_deleted_through_the_object() {
    // req:web.worker.replica/uninstall-deletes@5353d3fd
    let fake = fake_with("111", repo_files(&[]));
    let deleted = |action: &str| {
        json!({"action": action, "repositories": [{"full_name": "o/a"}],
            "installation": {"id": 9}})
    };

    // A suspension changes nothing the object holds.
    let (mut object, mem, _slot) = built(&fake);
    let held = mem.0.borrow().clone();
    let answer = block_on(object.event(&fake, &event("installation", "d1", deleted("suspend"))));
    assert_eq!(answer.status, 202);
    assert!(!object.removed());
    assert_eq!(*mem.0.borrow(), held, "it holds what it held");
    assert_eq!(view(&fake, &mut object, "snapshot")["sha"], "111");

    // A deletion empties it, and a read then answers as an object that holds nothing.
    let answer = block_on(object.event(&fake, &event("installation", "d2", deleted("deleted"))));
    assert_eq!(answer.status, 202);
    assert!(object.removed());
    assert!(mem.0.borrow().is_empty(), "the object holds nothing");
    let read = block_on(object.view(&fake, "snapshot", false));
    assert_eq!(
        (read.status, read.body.as_str()),
        (503, r#"{"preparing":0}"#)
    );
}

/// An object's storage, which holds `name`'s data until it is deleted.
struct Object(Option<&'static str>);

impl Storage for Object {
    fn delete_all(&mut self) {
        self.0 = None;
    }
}

#[test]
fn a_repo_removed() {
    // req:web.worker.replica/uninstall-deletes@5353d3fd
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
    // req:web.worker.replica/uninstall-deletes@5353d3fd
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
    // req:web.worker.replica/uninstall-deletes@5353d3fd
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
