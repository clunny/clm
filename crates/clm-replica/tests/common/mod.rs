#![allow(dead_code)]
//! Fixtures shared by the replica's tests: repositories as maps of path to bytes, the
//! changes a source would report for them, and replicas built from them.

use std::collections::{BTreeMap, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

use clm_replica::{Answer, Change, Entry, File, Pull, Replica, Store};
use serde_json::Value;

pub type FileMap = BTreeMap<String, Vec<u8>>;

pub fn files(entries: &[(&str, &str)]) -> FileMap {
    entries
        .iter()
        .map(|(p, t)| (p.to_string(), t.as_bytes().to_vec()))
        .collect()
}

pub fn node(deps: &str, reqs: &[&str], status: &str) -> String {
    let mut s = format!(
        "---\ntitle: T\nstatus: {status}\nowners: []\ndepends_on: [{deps}]\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n"
    );
    for r in reqs {
        s.push_str(&format!(
            "\n### Requirement: {r}\nIt SHALL {r}.\n\n#### Scenario: s\n- ok\n"
        ));
    }
    s
}

/// A small CLM repo: root, `a` (active, requirement `p`) and `b` (specd, depends on `a`,
/// requirement `q`).
pub fn clm_repo() -> FileMap {
    files(&[
        ("ARCHITECTURE.md", "# Arch\n"),
        ("specs/SPEC.md", &node("", &[], "active")),
        ("specs/a/SPEC.md", &node("", &["p"], "active")),
        ("specs/b/SPEC.md", &node("a", &["q"], "specd")),
    ])
}

pub fn write_checkout(dir: &Path, files: &FileMap) {
    for (p, b) in files {
        let path = dir.join(p);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b).unwrap();
    }
}

/// The first 8 hex digits of requirement `id`'s hash in `files`: its version.
pub fn version_of(files: &FileMap, id: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    write_checkout(dir.path(), files);
    let (graph, _) = clm_model::load(dir.path());
    let (node, _) = id.split_once('/').unwrap();
    graph.nodes[node]
        .requirements
        .iter()
        .find(|r| r.id == id)
        .unwrap()
        .hash[..8]
        .to_string()
}

/// A test file citing `id` at `version`.
pub fn cites(id: &str, version: &str) -> String {
    format!(
        "#[test]\nfn t() {{\n    // {}{id}@{version}\n}}\n",
        concat!("req", ":")
    )
}

pub fn sha_of(bytes: &[u8]) -> String {
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    format!("{:016x}", h.finish())
}

pub fn entry(path: &str, bytes: &[u8]) -> Entry {
    Entry {
        path: path.into(),
        sha: sha_of(bytes),
        size: bytes.len() as u64,
        mode: "100644".into(),
    }
}

/// `path` as a source reports it: its text, or binary when the bytes aren't UTF-8.
pub fn file(path: &str, bytes: &[u8]) -> File {
    let text = String::from_utf8(bytes.to_vec()).ok();
    File {
        entry: entry(path, bytes),
        binary: text.is_none(),
        text,
    }
}

pub fn tree(sha: &str, files: &FileMap) -> Change {
    Change::Tree {
        sha: sha.into(),
        entries: files.iter().map(|(p, b)| entry(p, b)).collect(),
    }
}

/// Applies the files the replica still needs, as the runtime does in slices.
pub fn fetch(r: &mut Replica, files: &FileMap, sha: &str) {
    let pending = r.pending_fetch();
    r.apply(Change::Tip {
        sha: sha.into(),
        files: pending.iter().map(|p| file(p, &files[p])).collect(),
        removed: Vec::new(),
    });
}

/// A replica built from scratch at `sha`: one `Tree` change and every file's text.
pub fn scratch(files: &FileMap, sha: &str) -> Replica {
    let mut r = Replica::default();
    r.apply(tree(sha, files));
    fetch(&mut r, files, sha);
    r
}

pub fn ready(files: &FileMap, sha: &str, now: u64) -> Replica {
    let mut r = scratch(files, sha);
    r.recompute(now);
    r
}

pub fn view(r: &Replica, name: &str) -> Value {
    match r.view(name).unwrap() {
        Answer::Ready { value, .. } => value,
        Answer::Preparing { done } => panic!("{name} is preparing at {done}"),
    }
}

pub fn pull(number: u64, head: &str, head_sha: &str) -> Pull {
    Pull {
        number,
        title: format!("PR {number}"),
        head: head.into(),
        base: "main".into(),
        head_sha: head_sha.into(),
        url: format!("https://example.test/pull/{number}"),
        fork: false,
        open: true,
    }
}

/// A store in a map, which remembers the largest value it was given.
#[derive(Default)]
pub struct MapStore {
    pub values: HashMap<String, Vec<u8>>,
    pub largest: usize,
}

impl Store for MapStore {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.values.get(key).cloned()
    }
    fn put(&mut self, key: &str, bytes: Vec<u8>) {
        self.largest = self.largest.max(bytes.len());
        self.values.insert(key.into(), bytes);
    }
    fn delete_all(&mut self) {
        self.values.clear();
    }
}
