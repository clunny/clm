//! `replica/snapshot-matches-clm` and `replica/snapshot-size`: the snapshot reports what
//! `clm status`, `clm coverage`, `clm backlog` and `clm size` report on a checkout.

mod common;

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use common::*;
use serde_json::{Value, json};

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect()
}

/// An unbuilt and an active component, a test citing `a/p` at its current version and
/// `b/q` at an older one, a Markdown file citing `b/r`, an applied change and a backlog
/// item blocking `b/r`.
fn fixture() -> FileMap {
    let mut f = clm_repo();
    f.insert(
        "specs/b/SPEC.md".into(),
        node("a", &["q", "r"], "specd").into_bytes(),
    );
    let current = version_of(&f, "a/p");
    let test = format!(
        "#[test]\nfn t() {{\n    // {}@{current}\n    // {}@00000000\n}}\n",
        concat!("req", ":a/p"),
        concat!("req", ":b/q")
    );
    let doc = format!("Cites {}.\n", concat!("req", ":b/r"));
    f.extend(files(&[
        ("tests/t.rs", &test),
        ("docs/notes.md", &doc),
        ("changes/c1/change.md", "---\nstatus: applied\n---\n\n## Intent\nI.\n"),
        (
            "changes/c1/deltas/b.md",
            "---\ncomponent: b\n---\n## ADDED Requirements\n### Requirement: q\nIt SHALL q.\n\n#### Scenario: s\n- ok\n\n### Requirement: r\nIt SHALL r.\n\n#### Scenario: s\n- ok\n",
        ),
        (
            "backlog/thing.md",
            "---\ntitle: A thing\nkind: human\ncomponents: [b]\nblocks: [b/r]\n---\nBody.\n",
        ),
    ]));
    f
}

#[test]
fn fixture_repo_matches_clm() {
    // req:replica/snapshot-matches-clm@98d112e9
    let f = fixture();
    let dir = tempfile::tempdir().unwrap();
    write_checkout(dir.path(), &f);
    let r = ready(&f, "111", 100);
    let snap = view(&r, "snapshot");
    assert_eq!(snap["sha"], "111");
    assert_eq!(snap["coverage"]["state"], "complete");

    // Components and statuses, requirements current of total, as `clm status` reports them.
    let status = clm_check::status(dir.path()).unwrap();
    let nodes = snap["nodes"].as_array().unwrap();
    let (graph, _) = clm_model::load(dir.path());
    assert_eq!(nodes.len(), graph.nodes.len());
    for n in nodes {
        let reqs = n["requirements"].as_array().unwrap();
        let current = reqs.iter().filter(|r| r["state"] == "current").count();
        let row = vec![
            n["id"].as_str().unwrap().to_string(),
            n["status"].as_str().unwrap().to_string(),
            format!("{current}/{}", reqs.len()),
        ];
        assert!(status.nodes.contains(&row), "{row:?} in\n{status}");
    }
    let a_row = status.nodes.iter().find(|r| r[0] == "a").unwrap();
    assert_eq!(a_row[2], "1/1");

    // Coverage, requirement by requirement, as `clm coverage` reports it.
    let coverage = clm_check::coverage(dir.path()).unwrap();
    let by_id: BTreeMap<&str, &Value> = nodes
        .iter()
        .flat_map(|n| n["requirements"].as_array().unwrap())
        .map(|r| (r["id"].as_str().unwrap(), r))
        .collect();
    assert_eq!(by_id.len(), coverage.len());
    for c in &coverage {
        let r = by_id[c.id.as_str()];
        let markers: Vec<String> = c.markers.iter().map(|m| m.to_string()).collect();
        assert_eq!(strings(&r["markers"]), markers, "{}", c.id);
        assert_eq!(r["state"], c.state.to_string(), "{}", c.id);
        assert_eq!(r["version"], json!(c.version), "{}", c.id);
    }
    assert_eq!(by_id["a/p"]["state"], "current");
    assert_eq!(by_id["b/q"]["state"], "behind");
    assert_eq!(by_id["b/r"]["state"], "unbuilt", "Markdown doesn't count");

    // The applied change, with requirements current of those it adds, as `clm status`.
    let change = &snap["changes"][0];
    assert_eq!(change["id"], "c1");
    assert_eq!(change["status"], "applied");
    assert_eq!(change["current"], 0);
    assert_eq!(change["total"], 2);
    assert_eq!(change["complete"], false);
    assert_eq!(status.changes[0].1[0][0], "c1");
    assert_eq!(status.changes[0].1[0][1], "0/2 requirements");

    // The backlog, in `clm backlog`'s order.
    let backlog = clm_check::backlog(dir.path()).unwrap();
    let listed = snap["backlog"].as_array().unwrap();
    assert_eq!(listed.len(), backlog.len());
    for (item, s) in backlog.iter().zip(listed) {
        assert_eq!(s["slug"], item.slug.as_str());
        assert_eq!(s["kind"], item.frontmatter.kind.to_string());
        assert_eq!(s["title"], item.frontmatter.title.as_str());
        assert_eq!(strings(&s["nodes"]), item.frontmatter.nodes);
        assert_eq!(strings(&s["blocks"]), item.frontmatter.blocks);
    }
    assert_eq!(strings(&listed[0]["blocks"]), ["b/r"]);

    // The `spec` model, as `clm view spec --format json` prints it, sizes included.
    let none = std::collections::BTreeSet::new();
    let spec = clm_view::spec_view(&graph, &clm_check::scan(dir.path()).req, Some(&none));
    assert_eq!(snap["spec"], serde_json::to_value(&spec).unwrap());
}

#[test]
fn a_binary_asset_hashes_text_only_when_it_is_text() {
    // req:replica/snapshot-matches-clm@98d112e9
    // A binary file outside the spec directories never blocks completeness, and a text
    // asset hashes as the checkout's does.
    let mut f = fixture();
    f.insert("specs/b/assets/note.txt".into(), b"a note\n".to_vec());
    f.insert("img/logo.bin".into(), vec![0xff, 0x00, 0xfe, 0x01]);
    let dir = tempfile::tempdir().unwrap();
    write_checkout(dir.path(), &f);
    let snap = view(&ready(&f, "111", 100), "snapshot");
    assert_eq!(snap["coverage"]["state"], "complete");
    let (graph, _) = clm_model::load(dir.path());
    let b = snap["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "b")
        .unwrap();
    assert_eq!(
        b["assets"]["note.txt"],
        graph.nodes["b"].assets["note.txt"].as_str()
    );
}

#[test]
fn a_complete_change_reports_complete() {
    // req:replica/snapshot-matches-clm@98d112e9
    let mut f = fixture();
    // `tests/t.rs` now cites `a/p`, `b/q` and `b/r` at their versions (archive-on-sync).
    let cited: String = ["a/p", "b/q", "b/r"]
        .iter()
        .map(|id| cites(id, &version_of(&f, id)))
        .collect();
    f.insert("tests/t.rs".into(), cited.into_bytes());
    let dir = tempfile::tempdir().unwrap();
    write_checkout(dir.path(), &f);
    let snap = view(&ready(&f, "111", 100), "snapshot");
    let status = clm_check::status(dir.path()).unwrap();
    assert_eq!(snap["changes"][0]["complete"], true);
    assert_eq!(snap["changes"][0]["current"], 2);
    assert!(status.changes[0].1[0][1].starts_with("2/2"), "{status}");
}

// ---- sizes ----

/// A component with `paths` and, when given, a `size_budget`.
fn sized_node(paths: &str, budget: Option<&str>) -> String {
    let budget = budget.map_or(String::new(), |b| format!("size_budget: {b}\n"));
    format!(
        "---\ntitle: T\nstatus: active\nowners: []\ndepends_on: []\npaths: [\"{paths}\"]\n{budget}---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n"
    )
}

/// `n` bytes of text.
fn text(n: usize) -> Vec<u8> {
    let mut t = "x".repeat(n - 1).into_bytes();
    t.push(b'\n');
    t
}

/// The `same as clm size` fixture: `clm.toml` sets `code_tokens = 100`; `a` has 600 bytes
/// of code, `b` 600 bytes and a budget of 200 tokens, `c` 200 bytes, a binary file, a text
/// file with a NUL near its start and a `linguist-generated` lockfile, each over 400 bytes.
fn sized() -> FileMap {
    let mut f = files(&[
        ("ARCHITECTURE.md", "# Arch\n"),
        ("clm.toml", "[component_size]\ncode_tokens = 100\n"),
        ("specs/SPEC.md", &node("", &[], "active")),
        ("specs/a/SPEC.md", &sized_node("a/**", None)),
        (
            "specs/b/SPEC.md",
            &sized_node("b/**", Some("{ code_tokens: 200 }")),
        ),
        ("specs/c/SPEC.md", &sized_node("c/**", None)),
        ("c/.gitattributes", "Cargo.lock linguist-generated\n"),
    ]);
    f.insert("a/lib.txt".into(), text(600));
    f.insert("b/lib.txt".into(), text(600));
    let attributes = f["c/.gitattributes"].len();
    f.insert("c/lib.txt".into(), text(200 - attributes));
    let mut binary = vec![0xff, 0x00, 0xfe];
    binary.resize(500, 0x01);
    f.insert("c/blob.bin".into(), binary);
    let mut nul = b"\0".to_vec();
    nul.extend(text(499));
    f.insert("c/nul.dat".into(), nul);
    f.insert("c/Cargo.lock".into(), text(500));
    f
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "core.autocrlf=false",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A git checkout of `files`, the paths in `links` symlinks to their bytes.
fn checkout(files: &FileMap, links: &[&str]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let plain: FileMap = files
        .iter()
        .filter(|(p, _)| !links.contains(&p.as_str()))
        .map(|(p, b)| (p.clone(), b.clone()))
        .collect();
    write_checkout(dir.path(), &plain);
    for link in links {
        let target = std::str::from_utf8(&files[*link]).unwrap();
        std::os::unix::fs::symlink(target, dir.path().join(link)).unwrap();
    }
    git(dir.path(), &["init", "-q", "-b", "main"]);
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-q", "-m", "fixture"]);
    dir
}

/// What `clm size` says of each component on the checkout at `dir`: oversized or not.
fn clm_size(dir: &Path) -> BTreeMap<String, bool> {
    let n = clm_config::load(Some(dir), &|_| None)
        .unwrap()
        .node_size
        .value;
    let defaults = clm_check::Budgets {
        spec_tokens: n.spec_tokens,
        code_tokens: n.code_tokens,
    };
    let (graph, _) = clm_model::load(dir);
    clm_check::size(dir, &graph, &clm_check::scan(dir).req, defaults)
        .unwrap()
        .into_iter()
        .map(|s| (s.node.clone(), s.oversized()))
        .collect()
}

/// Each component of the snapshot's `spec` model with its `oversized`, `Null` when absent.
fn oversized(snap: &Value) -> BTreeMap<String, Value> {
    snap["spec"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| {
            let o = n.get("oversized").cloned().unwrap_or(Value::Null);
            (n["node"].as_str().unwrap().to_string(), o)
        })
        .collect()
}

fn as_bools(o: &BTreeMap<String, Value>) -> BTreeMap<String, bool> {
    o.iter()
        .map(|(k, v)| (k.clone(), v.as_bool().unwrap_or_else(|| panic!("{k}: {v}"))))
        .collect()
}

#[test]
fn same_as_clm_size() {
    // req:replica/snapshot-size@5c56339e
    let f = sized();
    let dir = checkout(&f, &[]);
    let snap = view(&ready(&f, "111", 100), "snapshot");
    assert_eq!(snap["sizes"], "checked");
    let marks = as_bools(&oversized(&snap));
    assert_eq!(marks, clm_size(dir.path()));
    assert!(marks["a"]);
    assert!(!marks["b"]);
    assert!(!marks["c"]);
}

#[test]
fn still_indexing() {
    // req:replica/snapshot-size@5c56339e
    let f = sized();
    let mut r = Replica::default();
    r.apply(tree("111", &f));
    // Only the spec files, so the marker index still has files to fetch.
    let spec = r.pending_fetch().into_iter().filter(|p| {
        p.starts_with("specs/")
            || p == "clm.toml"
            || p.ends_with(".gitattributes")
            || p == "ARCHITECTURE.md"
    });
    let spec: Vec<String> = spec.collect();
    r.apply(Change::Tip {
        sha: "111".into(),
        files: spec.iter().map(|p| file(p, &f[p])).collect(),
        removed: Vec::new(),
    });
    r.recompute(100);
    let snap = view(&r, "snapshot");
    assert_eq!(snap["coverage"]["state"], "indexing");
    assert_eq!(snap["sizes"], "indexing");
    assert!(oversized(&snap).values().all(Value::is_null), "{snap}");
}

use clm_replica::{Change, Replica};

#[test]
fn unreadable_settings_say_why() {
    // req:replica/snapshot-size@5c56339e
    let mut f = sized();
    f.insert(
        "clm.toml".into(),
        b"[component_size]\nspec_tokens = 0\n".to_vec(),
    );
    let snap = view(&ready(&f, "111", 100), "snapshot");
    let sizes = snap["sizes"].as_str().unwrap();
    assert!(sizes.starts_with("clm.toml: "), "{sizes}");
    assert!(oversized(&snap).values().all(Value::is_null), "{snap}");
}

#[test]
fn a_symlink_never_counts() {
    // req:replica/snapshot-size@5c56339e
    // `a` holds 400 bytes, exactly its budget; the symlink's listed size, 10,000, would tip
    // it over if it counted.
    let mut f = sized();
    f.insert("a/lib.txt".into(), text(400));
    f.insert("big.txt".into(), text(10_000));
    f.insert("a/link".into(), b"../big.txt".to_vec());
    let dir = checkout(&f, &["a/link"]);
    let mut r = Replica::default();
    let mut entries: Vec<_> = f.iter().map(|(p, b)| entry(p, b)).collect();
    for e in &mut entries {
        if e.path == "a/link" {
            e.mode = "120000".into();
            e.size = 10_000;
        }
    }
    r.apply(Change::Tree {
        sha: "111".into(),
        entries,
    });
    // The link's blob is never fetched.
    assert!(!r.pending_fetch().contains(&"a/link".to_string()));
    fetch(&mut r, &f, "111");
    r.recompute(100);
    let snap = view(&r, "snapshot");
    assert_eq!(snap["sizes"], "checked");
    let marks = as_bools(&oversized(&snap));
    assert_eq!(marks, clm_size(dir.path()));
    assert!(!marks["a"]);
}

#[test]
fn a_backlog_item_in_full() {
    // req:replica/snapshot-backlog-text@990be0ee
    let mut f = clm_repo();
    f.extend(files(&[
        (
            "backlog/q.md",
            "---\ntitle: Q\nkind: question\npriority: P2\nchoices: [yes, no]\n---\n\n\nShould we?\n\n",
        ),
        ("backlog/i.md", "---\ntitle: I\nkind: idea\n---\nAn idea.\n"),
    ]));
    let r = ready(&f, "111", 100);
    let snap = view(&r, "snapshot");
    let item = |slug: &str| {
        snap["backlog"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["slug"] == slug)
            .unwrap()
            .clone()
    };
    let q = item("q");
    assert_eq!(q["body"], "Should we?");
    assert_eq!(q["priority"], "P2");
    assert_eq!(q["choices"], json!(["yes", "no"]));
    let i = item("i");
    assert_eq!(i["body"], "An idea.");
    assert_eq!(i["priority"], "P5");
    assert_eq!(i["choices"], json!([]));
}
