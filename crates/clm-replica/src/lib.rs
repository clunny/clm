//! A repository's derived state, kept up to date from the changes a source reports, and the
//! views computed from it (`web-repo-replica`). Host-neutral and runtime-neutral: it knows
//! nothing of a host, a network, a browser or a filesystem (`replica/host-neutral`).
//!
//! A runtime gives it the changes a source fetched ([`Replica::apply`]), a [`Store`] to keep
//! it in ([`Replica::save`], [`Replica::load`]) and asks for the views
//! ([`Replica::recompute`], [`Replica::view`]). Every view is a JSON value, shaped as the
//! web app's routes answer it. `snapshot` has no `repo` field: the runtime knows the repo's
//! name and adds it.

mod compute;
mod snapshot;
mod write;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use clm_coverage::Marker;

pub use write::{Refusal, Validated, Write};

/// The largest value `save` writes to a store.
const CHUNK: usize = 1 << 20;
/// The events kept for the activity feed.
const EVENTS: usize = 100;
/// Bytes from the start of a file whose NUL decides it is binary, as `clm size` does.
const BINARY_PROBE: usize = 8000;
/// The store key of the chunk count; the chunks are `<KEY>/<n>`.
const KEY: &str = "replica";

/// Where a replica keeps its state, in values of at most 1 MiB each.
pub trait Store {
    fn get(&self, key: &str) -> Option<Vec<u8>>;
    fn put(&mut self, key: &str, bytes: Vec<u8>);
    fn delete_all(&mut self);
}

/// One file of a commit's tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub path: String,
    /// The blob's SHA.
    pub sha: String,
    pub size: u64,
    /// The git mode, such as `100644`.
    pub mode: String,
}

/// A file a source fetched: its tree entry and, for a spec file or a non-Markdown one, its
/// text, or for a binary asset its bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct File {
    pub entry: Entry,
    /// The source says the blob is binary.
    pub binary: bool,
    /// `None` for a binary blob.
    pub text: Option<String>,
    /// A binary asset's bytes, of which the replica keeps only the hash (`asset-hashes`).
    #[serde(default)]
    pub bytes: Option<Vec<u8>>,
}

impl File {
    /// A file the source read as text (`files-a-source-gives`): held binary when its text
    /// has a NUL byte near its start, as `git` judges it.
    pub fn text(entry: Entry, text: impl Into<String>) -> File {
        File {
            entry,
            binary: false,
            text: Some(text.into()),
            bytes: None,
        }
    }

    /// A blob the source says is binary: held with no text and no markers.
    pub fn binary(entry: Entry) -> File {
        File {
            entry,
            binary: true,
            text: None,
            bytes: None,
        }
    }

    /// A binary blob given with its bytes: held binary, with the hash the model gives them
    /// as an asset when `keeps_hash` accepts its path, and none of the bytes
    /// (`replica/asset-hashes`).
    pub fn asset(entry: Entry, bytes: Vec<u8>) -> File {
        File {
            entry,
            binary: true,
            text: None,
            bytes: Some(bytes),
        }
    }

    /// A file the source read nothing of: held in the tree only, so `pending_fetch` still
    /// lists it.
    pub fn unfetched(entry: Entry) -> File {
        File {
            entry,
            binary: false,
            text: None,
            bytes: None,
        }
    }
}

/// What a source fetched for a ref it reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefChange {
    /// A spec ref's `changes/<id>/change.md` text and the names under its `deltas/`, or the
    /// archived ones when its tip has no live change (`orchestrator/archived-spec-work`).
    Spec {
        change: Option<String>,
        deltas: Vec<String>,
    },
    /// An `impl/*` head's code key.
    Code { key: String },
}

/// A pull request, as `live::pulls` reads one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pull {
    pub number: u64,
    pub title: String,
    pub head: String,
    pub base: String,
    pub head_sha: String,
    pub url: String,
    pub fork: bool,
    /// `false` once it is closed.
    pub open: bool,
}

/// A repository event the activity feed shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    Push {
        branch: String,
        at: u64,
        actor: String,
    },
    /// A branch created: `impl/<component>` is a claim.
    Create {
        branch: String,
        at: u64,
        actor: String,
    },
}

impl Event {
    fn at(&self) -> u64 {
        match self {
            Event::Push { at, .. } | Event::Create { at, .. } => *at,
        }
    }
}

/// What a source reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Change {
    /// The default branch at a new commit: the files that changed, and the paths removed.
    Tip {
        sha: String,
        files: Vec<File>,
        removed: Vec<String>,
    },
    /// The whole listing at a commit, to rebuild from or to recover after a force-push.
    Tree {
        sha: String,
        entries: Vec<Entry>,
    },
    /// A ledger branch's `records.jsonl` at its tip, or `None` once it is deleted.
    Ledger {
        branch: String,
        text: Option<String>,
    },
    /// A ref the views read, moved or created; `sha` is `None` once it is deleted. `time` is
    /// its tip's commit time in Unix seconds.
    Ref {
        name: String,
        sha: Option<String>,
        time: u64,
        change: Option<RefChange>,
    },
    Pull(Pull),
    Event(Event),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Blob {
    pub sha: String,
    pub size: u64,
    pub mode: String,
}

/// One marker of a scanned file, as `clm_coverage::markers_in` found it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Found {
    pub id: String,
    pub version: Option<String>,
    pub line: usize,
}

/// What the replica holds of a blob it fetched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Held {
    pub sha: String,
    pub binary: bool,
    /// Kept for the files the model and the sizes read.
    pub text: Option<String>,
    pub markers: Vec<Found>,
    /// A binary asset's hash, for the snapshot's `assets`; absent in a store saved before
    /// hashes were held.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RefState {
    pub sha: String,
    pub time: u64,
    pub change: Option<RefChange>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct State {
    pub tip: Option<String>,
    /// A `Tree` change has listed the commit.
    pub listed: bool,
    pub tree: BTreeMap<String, Blob>,
    pub held: BTreeMap<String, Held>,
    pub ledgers: BTreeMap<String, String>,
    pub refs: BTreeMap<String, RefState>,
    pub pulls: BTreeMap<u64, Pull>,
    /// Newest first.
    pub events: Vec<Event>,
}

/// The paths the model reads.
fn is_spec_path(path: &str) -> bool {
    path == "ARCHITECTURE.md"
        || path == "CAPABILITIES.md"
        || ["specs/", "changes/", "backlog/", "docs/decisions/"]
            .iter()
            .any(|d| path.starts_with(d))
}

/// The files the snapshot reads besides the model's: `clm.toml`, and every `.gitattributes`.
fn is_size_path(path: &str) -> bool {
    path == "clm.toml" || path == ".gitattributes" || path.ends_with("/.gitattributes")
}

/// Whether the replica keeps a file's text: the files the model reads, `clm.toml` and every
/// `.gitattributes` (`replica/files-it-fetches`).
pub fn keeps_text(path: &str) -> bool {
    is_spec_path(path) || is_size_path(path)
}

/// Whether the replica reads a file at all: one whose text it keeps, or one that can hold
/// markers (`replica/files-it-fetches`).
pub fn wanted(path: &str) -> bool {
    keeps_text(path) || scannable(path)
}

/// The prefixes of the refs the views read (`replica/refs-the-views-read`).
pub const REF_PREFIXES: [&str; 4] = ["impl/", "spec/", "autosync/spec/", "propose/"];

/// Whether the views read a ref: one of `REF_PREFIXES` followed by a non-empty name with no
/// `/` (`replica/refs-the-views-read`).
pub fn reads_ref(name: &str) -> bool {
    REF_PREFIXES.iter().any(|prefix| {
        name.strip_prefix(prefix)
            .is_some_and(|rest| !rest.is_empty() && !rest.contains('/'))
    })
}

/// Whether the replica keeps a binary file's hash for the snapshot's `assets`: a path under
/// `specs/` with an `assets` directory in it, as the model reads a component's assets
/// (`replica/asset-hashes`).
pub fn keeps_hash(path: &str) -> bool {
    path.strip_prefix("specs/")
        .and_then(|rest| rest.rsplit_once('/'))
        .is_some_and(|(dir, _)| dir.split('/').any(|c| c == "assets"))
}

/// Whether a file can hold markers that count: Markdown can't.
fn scannable(path: &str) -> bool {
    !path.ends_with(".md")
}

pub(crate) fn regular(mode: &str) -> bool {
    matches!(mode, "100644" | "100755")
}

impl State {
    fn put(&mut self, file: File) {
        let File {
            entry,
            binary,
            text,
            bytes,
        } = file;
        let Entry {
            path,
            sha,
            size,
            mode,
        } = entry;
        self.tree.insert(
            path.clone(),
            Blob {
                sha: sha.clone(),
                size,
                mode: mode.clone(),
            },
        );
        self.held.remove(&path);
        let wanted = regular(&mode) && wanted(&path);
        // A blob the source read nothing of stays to fetch.
        if !wanted || (text.is_none() && !binary) {
            return;
        }
        let nul = text
            .as_deref()
            .is_some_and(|t| t.bytes().take(BINARY_PROBE).any(|b| b == 0));
        let binary = binary || nul;
        let markers = match &text {
            Some(t) if scannable(&path) && !binary => {
                clm_coverage::markers_in(std::path::Path::new(&path), t)
                    .into_iter()
                    .map(|m: Marker| Found {
                        id: m.id,
                        version: m.version,
                        line: m.line,
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        let text = if keeps_text(&path) { text } else { None };
        let hash = bytes
            .filter(|_| keeps_hash(&path))
            .map(|b| clm_model::asset_hash(&b));
        self.held.insert(
            path,
            Held {
                sha,
                binary,
                text,
                markers,
                hash,
            },
        );
    }

    /// Whether the replica holds the blob the tree lists at `path`.
    fn holds(&self, path: &str, blob: &Blob) -> bool {
        self.held.get(path).is_some_and(|h| h.sha == blob.sha)
    }

    /// The paths in the tree the replica still needs, spec files first.
    fn missing(&self) -> Vec<String> {
        let mut paths: Vec<(bool, &String)> = self
            .tree
            .iter()
            .filter(|(p, b)| regular(&b.mode) && wanted(p) && !self.holds(p, b))
            .map(|(p, _)| (!keeps_text(p), p))
            .collect();
        paths.sort_by_key(|(later, _)| *later);
        paths.into_iter().map(|(_, p)| p.clone()).collect()
    }

    /// The share of the listed spec files whose text is held; 0 before a listing.
    fn spec_done(&self) -> f64 {
        if !self.listed {
            return 0.0;
        }
        let spec: Vec<_> = self
            .tree
            .iter()
            .filter(|(p, b)| regular(&b.mode) && keeps_text(p))
            .collect();
        if spec.is_empty() {
            return 1.0;
        }
        let held = spec.iter().filter(|(p, b)| self.holds(p, b)).count();
        held as f64 / spec.len() as f64
    }

    /// The share of the listed scannable files that are scanned; 1 when there are none.
    pub(crate) fn index_done(&self) -> f64 {
        let all: Vec<_> = self
            .tree
            .iter()
            .filter(|(p, b)| regular(&b.mode) && scannable(p))
            .collect();
        if all.is_empty() {
            return 1.0;
        }
        let held = all.iter().filter(|(p, b)| self.holds(p, b)).count();
        held as f64 / all.len() as f64
    }

    /// Every scannable file is scanned, so the markers are complete.
    pub(crate) fn index_complete(&self) -> bool {
        self.listed
            && self
                .tree
                .iter()
                .filter(|(p, b)| regular(&b.mode) && scannable(p))
                .all(|(p, b)| self.holds(p, b))
    }

    /// The markers of every scanned file.
    pub(crate) fn markers(&self) -> Vec<Marker> {
        self.held
            .iter()
            .flat_map(|(path, h)| {
                h.markers.iter().map(move |f| Marker {
                    id: f.id.clone(),
                    version: f.version.clone(),
                    path: path.into(),
                    line: f.line,
                })
            })
            .collect()
    }

    /// The model's files, or `None` while a spec file's text is still to fetch. A binary
    /// file `keeps_hash` accepts is given with no bytes, so the model sees its path;
    /// `held_assets` then sets its hash.
    pub(crate) fn spec_files(&self) -> Option<clm_model::Files> {
        if !self.listed || self.spec_done() < 1.0 {
            return None;
        }
        Some(
            self.tree
                .keys()
                .filter(|p| keeps_text(p))
                .filter_map(|p| {
                    let held = self.held.get(p)?;
                    let bytes = match &held.text {
                        Some(text) => text.clone().into_bytes(),
                        None if held.binary && keeps_hash(p) => Vec::new(),
                        None => return None,
                    };
                    Some((p.into(), bytes))
                })
                .collect(),
        )
    }

    /// Gives each binary asset the model loaded the hash the replica holds for it, and
    /// leaves out one held with none (`replica/asset-hashes`).
    pub(crate) fn held_assets(&self, graph: &mut clm_model::Graph) {
        for node in graph.nodes.values_mut() {
            let Some(dir) = node.path.parent().and_then(|d| d.to_str()) else {
                continue;
            };
            node.assets.retain(|name, hash| {
                let path = format!("{dir}/assets/{name}");
                match self.held.get(&path) {
                    Some(held) if held.binary && held.text.is_none() => match &held.hash {
                        Some(h) => {
                            hash.clone_from(h);
                            true
                        }
                        None => false,
                    },
                    _ => true,
                }
            });
        }
    }
}

/// The views one `recompute` made, with the commit they describe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Views {
    sha: String,
    at: u64,
    snapshot: Value,
    implementation: Value,
    capabilities: Value,
    architecture: Value,
}

/// What `recompute` returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recomputed {
    /// The time the implementation view next changes without a change: the earliest time a
    /// live claim lapses; `None` when none is live.
    pub next: Option<u64>,
}

/// What `view` answers.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    Ready {
        /// The commit the view describes.
        sha: String,
        /// When it was computed, in Unix seconds.
        computed_at: u64,
        value: Value,
    },
    /// Before the first recompute with the spec files: the share of them fetched.
    Preparing { done: f64 },
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Replica {
    state: State,
    views: Option<Views>,
    stale: bool,
}

impl Replica {
    /// The replica `save` wrote to `store`, or a new one when it holds none that reads.
    pub fn load(store: &impl Store) -> Replica {
        let read = || -> Option<Replica> {
            let count: usize = String::from_utf8(store.get(KEY)?).ok()?.parse().ok()?;
            let mut bytes = Vec::new();
            for n in 0..count {
                bytes.extend(store.get(&format!("{KEY}/{n}"))?);
            }
            serde_json::from_slice(&bytes).ok()
        };
        read().unwrap_or_default()
    }

    /// Writes the replica to `store`, in values of at most 1 MiB.
    pub fn save(&self, store: &mut impl Store) {
        let bytes = serde_json::to_vec(self).unwrap_or_default();
        store.delete_all();
        let mut count = 0;
        for chunk in bytes.chunks(CHUNK) {
            store.put(&format!("{KEY}/{count}"), chunk.to_vec());
            count += 1;
        }
        store.put(KEY, count.to_string().into_bytes());
    }

    /// Takes in what a source fetched and marks the views stale; it never recomputes
    /// (`replica/batched-recompute`).
    pub fn apply(&mut self, change: Change) {
        self.stale = true;
        let s = &mut self.state;
        match change {
            Change::Tip {
                sha,
                files,
                removed,
            } => {
                s.tip = Some(sha);
                for path in removed {
                    s.tree.remove(&path);
                    s.held.remove(&path);
                }
                for file in files {
                    s.put(file);
                }
            }
            Change::Tree { sha, entries } => {
                s.tip = Some(sha);
                s.listed = true;
                s.tree = entries
                    .into_iter()
                    .map(|e| {
                        let blob = Blob {
                            sha: e.sha,
                            size: e.size,
                            mode: e.mode,
                        };
                        (e.path, blob)
                    })
                    .collect();
                let tree = &s.tree;
                s.held
                    .retain(|p, h| tree.get(p).is_some_and(|b| b.sha == h.sha));
            }
            Change::Ledger { branch, text } => match text {
                Some(text) => {
                    s.ledgers.insert(branch, text);
                }
                None => {
                    s.ledgers.remove(&branch);
                }
            },
            Change::Ref {
                name,
                sha,
                time,
                change,
            } if reads_ref(&name) => match sha {
                Some(sha) => {
                    s.refs.insert(name, RefState { sha, time, change });
                }
                None => {
                    s.refs.remove(&name);
                }
            },
            // A ref the views don't read is held by none (`replica/refs-the-views-read`),
            // and one a store saved before the rule held goes.
            Change::Ref { name, .. } => {
                s.refs.remove(&name);
            }
            Change::Pull(pull) => {
                if pull.open {
                    s.pulls.insert(pull.number, pull);
                } else {
                    s.pulls.remove(&pull.number);
                }
            }
            Change::Event(event) => {
                let feed = match &event {
                    Event::Push { branch, .. } => !branch.starts_with("clm/ledger/"),
                    Event::Create { branch, .. } => compute::branch(branch, "impl/").is_some(),
                };
                if feed {
                    s.events.push(event);
                    s.events.sort_by_key(|e| std::cmp::Reverse(e.at()));
                    s.events.truncate(EVENTS);
                }
            }
        }
    }

    /// Whether a change has come since the last recompute.
    pub fn stale(&self) -> bool {
        self.stale
    }

    /// What the replica still needs before its views are complete: tree paths whose text it
    /// doesn't hold, the spec files first.
    pub fn pending_fetch(&self) -> Vec<String> {
        self.state.missing()
    }

    /// Computes every view from the state, at once. While a spec file's text is still to
    /// fetch it changes nothing.
    pub fn recompute(&mut self, now: u64) -> Recomputed {
        let Some(files) = self.state.spec_files() else {
            return Recomputed { next: None };
        };
        self.views = Some(compute::views(&self.state, &files, now));
        self.stale = false;
        Recomputed {
            next: compute::next_lapse(&self.state, now),
        }
    }

    /// The last computed view `name` (`snapshot`, `implementation`, `capabilities` or
    /// `architecture`), `None` for another name.
    pub fn view(&self, name: &str) -> Option<Answer> {
        if !["snapshot", "implementation", "capabilities", "architecture"].contains(&name) {
            return None;
        }
        let Some(v) = &self.views else {
            return Some(Answer::Preparing {
                done: self.state.spec_done(),
            });
        };
        let value = match name {
            "snapshot" => &v.snapshot,
            "implementation" => &v.implementation,
            "capabilities" => &v.capabilities,
            _ => &v.architecture,
        };
        Some(Answer::Ready {
            sha: v.sha.clone(),
            computed_at: v.at,
            value: value.clone(),
        })
    }

    /// Checks a web write against the state (`replica/validate-writes`).
    pub fn validate(&self, write: &Write) -> Result<Validated, Refusal> {
        write::validate(&self.state, write)
    }
}
