//! A `Replica` object over the fake GitHub: an in-memory store and alarm the test can keep
//! across a reload, and helpers that run the alarm as Cloudflare would.

#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use clm_replica::Store;
use clm_web_platform::fake::{Fake, FakeRepo, FileMap, clm_repo, files};
use clm_web_replica::{Alarm, Object};
use futures::executor::block_on;
use serde_json::{Value, json};

#[derive(Clone, Default)]
pub struct Mem(pub Rc<RefCell<BTreeMap<String, Vec<u8>>>>);

impl Store for Mem {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.0.borrow().get(key).cloned()
    }
    fn put(&mut self, key: &str, bytes: Vec<u8>) {
        self.0.borrow_mut().insert(key.into(), bytes);
    }
    fn delete_all(&mut self) {
        self.0.borrow_mut().clear();
    }
}

#[derive(Clone, Default)]
pub struct Slot(pub Rc<Cell<Option<u64>>>);

impl Alarm for Slot {
    fn get(&self) -> Option<u64> {
        self.0.get()
    }
    fn set(&mut self, at: u64) {
        self.0.set(Some(at));
    }
    fn clear(&mut self) {
        self.0.set(None);
    }
}

pub type Obj = Object<Mem, Slot>;

pub fn object() -> (Obj, Mem, Slot) {
    let (mem, slot) = (Mem::default(), Slot::default());
    (Object::load("o/a", mem.clone(), slot.clone()), mem, slot)
}

/// `o/a` at `sha` with `files`, and the fake that serves it.
pub fn fake_with(sha: &str, files: FileMap) -> Fake {
    let fake = Fake::default();
    fake.gh.add_repo("o/a", FakeRepo::new(sha, files));
    fake
}

/// The repo's files, with `extra` added.
pub fn repo_files(extra: &[(&str, &str)]) -> FileMap {
    let mut all = clm_repo();
    all.extend(files(extra));
    all
}

/// Commits `files` to `o/a` as `sha` and moves `main` to it.
pub fn commit(fake: &Fake, sha: &str, files: FileMap) {
    let mut repos = fake.gh.repos.borrow_mut();
    let repo = repos.get_mut("o/a").unwrap();
    repo.commits.insert(sha.into(), files);
    repo.branches.insert("main".into(), sha.into());
}

/// The webhook body of a push of `branch` from `before` to `after`.
pub fn push(branch: &str, before: &str, after: &str) -> Value {
    json!({
        "ref": format!("refs/heads/{branch}"),
        "before": before,
        "after": after,
        "created": false,
        "deleted": false,
        "pusher": {"name": "alice"},
        "repository": {"full_name": "o/a", "default_branch": "main"},
    })
}

pub fn event(name: &str, delivery: &str, body: Value) -> Value {
    json!({"event": name, "delivery": delivery, "installation": 9, "body": body})
}

/// Runs the alarm, as Cloudflare does at the time it was set, until it re-arms no more.
/// Answers the subrequests each invocation made.
pub fn run_alarms(fake: &Fake, object: &mut Obj, slot: &Slot) -> Vec<usize> {
    let mut invocations = Vec::new();
    for _ in 0..200 {
        let Some(at) = slot.0.get() else {
            return invocations;
        };
        // The pages stay open: a read keeps them so until the test says otherwise.
        fake.now.set(fake.now.get().max(at));
        let before = fake.gh.calls.borrow().len();
        block_on(object.on_alarm(fake));
        invocations.push(fake.gh.calls.borrow().len() - before);
    }
    panic!("the alarm never stopped");
}

/// Builds the replica of `fake`'s `o/a` from nothing, through the first read and the alarms.
pub fn built(fake: &Fake) -> (Obj, Mem, Slot) {
    let (mut object, mem, slot) = object();
    let first = block_on(object.view(fake, "snapshot", false));
    assert_eq!(first.status, 503);
    run_alarms(fake, &mut object, &slot);
    (object, mem, slot)
}

pub fn view(fake: &Fake, object: &mut Obj, name: &str) -> Value {
    let answer = block_on(object.view(fake, name, false));
    assert_eq!(answer.status, 200, "{}", answer.body);
    serde_json::from_str(&answer.body).unwrap()
}

/// The paths of the files fetched from GitHub as blobs since `from` calls.
pub fn graphql_calls(fake: &Fake, from: usize) -> Vec<String> {
    fake.gh.calls.borrow()[from..]
        .iter()
        .filter(|c| c.url.ends_with("/graphql"))
        .map(|c| c.body.clone().unwrap_or_default())
        .collect()
}

pub fn urls(fake: &Fake, from: usize) -> Vec<String> {
    fake.gh.calls.borrow()[from..]
        .iter()
        .map(|c| c.url.clone())
        .collect()
}
