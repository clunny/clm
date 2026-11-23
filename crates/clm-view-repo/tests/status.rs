//! `clm status` reads the working tree and the local refs only: the checkout, the claims and
//! the sizes under the budgets it is passed, never a fetch or a ledger
//! (`reads-the-repository`).

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use common::{Output, clm};
use tempfile::TempDir;

const REQ: &str = concat!("req", ":");

/// `clm status --root <root>`, through `clm-view-repo`'s call, with the budgets of the
/// repository's `clm.toml`.
fn clm_status(root: &Path) -> Output {
    clm(root, &["status"])
}

fn status(root: &Path) -> String {
    let out = clm_status(root);
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(0), "{text}");
    text
}

/// The summary: the lines up to the first blank one.
fn summary(text: &str) -> String {
    text.split("\n\n").next().unwrap().to_string()
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";

fn node(status: &str, reqs: &[&str]) -> String {
    let mut s = format!("---\ntitle: A\nstatus: {status}\nowners: []\n---\n\n## Requirements\n");
    for r in reqs {
        s.push_str(&format!(
            "\n### Requirement: {r}\n{}\n\n#### Scenario: s\n- s\n",
            r.to_uppercase()
        ));
    }
    s
}

/// Node `a` with `a/x` (cited at its current version) and `a/y` (uncited).
fn base() -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/a/SPEC.md", &node("active", &["x", "y"]));
    cite(dir.path(), "t", &["a/x"]);
    dir
}

/// `tests/<file>.rs` citing `ids` at their current versions.
fn cite(root: &Path, file: &str, ids: &[&str]) {
    let (graph, _) = clm_model::load(root);
    let mut text: String = ids
        .iter()
        .map(|id| {
            format!(
                "// {REQ}{id}@{}\n",
                graph.requirement(id).unwrap().version()
            )
        })
        .collect();
    text.push_str("#[test]\nfn t() {}\n");
    write(root, &format!("tests/{file}.rs"), &text);
}

#[test]
fn invalid_graph_prints_violations() {
    // req:view.repo/arguments-and-exit@c4b91292
    let dir = base();
    // Valid, `clm status` exits 0.
    status(dir.path());
    write(dir.path(), "specs/b/SPEC.md", "---\ntitle: B\n---\n");
    let out = clm_status(dir.path());
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(text.contains("model/valid-frontmatter"), "{text}");
    assert!(!text.contains("CLM status"), "{text}");
}

fn git_env(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> String {
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .envs(env.iter().copied())
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

/// Makes `root` a repository with a remote-tracking ref `origin/impl/<node>` per
/// `(node, hours ago)`, as a fetch would leave them, and `origin/main` at its commit.
fn claim_refs(root: &Path, claims: &[(&str, u64)]) {
    let git = |args: &[&str]| git_env(root, args, &[]);
    git(&["init", "-q", "-b", "main"]);
    git(&["add", "-A"]);
    git(&["commit", "-qm", "base"]);
    git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    for (node, hours) in claims {
        remote_ref(root, &format!("impl/{node}"), *hours);
    }
}

/// A remote-tracking ref `origin/<name>` on a commit on `HEAD` made `hours` hours ago.
fn remote_ref(root: &Path, name: &str, hours: u64) {
    let git = |args: &[&str]| git_env(root, args, &[]);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let date = format!("@{} +0000", now - hours * 3600);
    let tree = git(&["rev-parse", "HEAD^{tree}"]);
    let head = git(&["rev-parse", "HEAD"]);
    let sha = git_env(
        root,
        &[
            "commit-tree",
            tree.trim(),
            "-p",
            head.trim(),
            "-m",
            &format!("Claim {name}"),
        ],
        &[("GIT_COMMITTER_DATE", &date), ("GIT_AUTHOR_DATE", &date)],
    );
    git(&[
        "update-ref",
        &format!("refs/remotes/origin/{name}"),
        sha.trim(),
    ]);
}

#[test]
fn status_and_view_never_fetch() {
    // req:view.repo/reads-the-repository@dd91d681
    let dir = base();
    let d = dir.path();
    write(d, "specs/z/SPEC.md", &node("specd", &["x"]));
    claim_refs(d, &[("a", 1), ("z", 1)]);
    // `origin` holds a claim on `z` that the last fetch didn't bring: a fetch would show it.
    let origin = TempDir::new().unwrap();
    git_env(origin.path(), &["init", "-q", "--bare", "-b", "main"], &[]);
    let git = |args: &[&str]| git_env(d, args, &[]);
    git(&["remote", "add", "origin", origin.path().to_str().unwrap()]);
    git(&[
        "push",
        "-q",
        "origin",
        "refs/remotes/origin/impl/z:refs/heads/impl/z",
    ]);
    git(&["update-ref", "-d", "refs/remotes/origin/impl/z"]);
    let text = status(d);
    assert!(text.contains("\n  claims 1 live\n"), "{text}");
    assert!(text.ends_with("\nclaims\n  a  live\n\n"), "{text}");
    let out = clm(d, &["view", "flight", "--format", "json"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let z = v["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["node"] == "z")
        .unwrap();
    assert_eq!(z["state"], "ready", "{v:#}");
}

/// A repository whose `clm.toml` sets a code budget of 100 tokens, with node `a` owning
/// `src/a/**` (`frontmatter` added to its own), a tracked `src/a/x.rs` of 200 tokens, and
/// node `b` within its budgets.
fn sized(frontmatter: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(
        d,
        "specs/a/SPEC.md",
        &node("active", &[]).replace(
            "owners: []\n",
            &format!("owners: []\npaths: [\"src/a/**\"]\n{frontmatter}"),
        ),
    );
    write(d, "specs/b/SPEC.md", &node("active", &[]));
    write(d, "clm.toml", "[component_size]\ncode_tokens = 100\n");
    write(d, "src/a/x.rs", &"a".repeat(800));
    claim_refs(d, &[]);
    dir
}

/// The row of `node` in the `components` section.
fn row<'a>(text: &'a str, node: &str) -> &'a str {
    let section = text.split("\n\ncomponents\n").nth(1).unwrap();
    section
        .lines()
        .find(|l| l.starts_with(&format!("  {node} ")))
        .unwrap_or_else(|| panic!("no {node} row in {text}"))
}

/// The summary's `components` line.
fn nodes_line(text: &str) -> String {
    summary(text)
        .lines()
        .find(|l| l.starts_with("  components "))
        .unwrap()
        .to_string()
}

#[test]
fn scenario_oversized_node() {
    // req:view.repo/reads-the-repository@dd91d681
    let dir = sized("");
    // `c`'s code is over the repository's budget but within the `size_budget` it sets.
    write(
        dir.path(),
        "specs/c/SPEC.md",
        &node("active", &[]).replace(
            "owners: []\n",
            "owners: []\npaths: [\"src/c/**\"]\nsize_budget: { code_tokens: 1000 }\n",
        ),
    );
    write(dir.path(), "src/c/x.rs", &"c".repeat(800));
    git_env(dir.path(), &["add", "-A"], &[]);
    let text = status(dir.path());
    let line = nodes_line(&text);
    assert!(line.ends_with(", 1 oversized"), "{text}");
    assert!(row(&text, "a").ends_with("  oversized"), "{text}");
    assert!(!row(&text, "b").contains("oversized"), "{text}");
    assert!(!row(&text, "c").contains("oversized"), "{text}");
}

#[test]
fn scenario_budget_accepted() {
    // req:view.repo/reads-the-repository@dd91d681
    let dir = sized("size_budget: { code_tokens: 1000 }\n");
    let text = status(dir.path());
    assert!(!nodes_line(&text).contains("oversized"), "{text}");
    assert!(!row(&text, "a").contains("oversized"), "{text}");
}

#[test]
fn sizes_that_cant_be_checked_mark_nothing() {
    // req:view.repo/reads-the-repository@dd91d681
    let dir = sized("");
    write(
        dir.path(),
        "clm.toml",
        "[component_size]\ncode_tokens = \"many\"\n",
    );
    let out = clm_status(dir.path());
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let stderr = String::from_utf8(out.stderr.clone()).unwrap();
    assert!(stderr.starts_with("size not checked: "), "{stderr}");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(!text.contains("oversized"), "{text}");
}

/// A `landed` record for change `y` as an older `clm` wrote it, `tier` `SOFT`, which then
/// awaited an after-merge review.
const SOFT_LAND: &str = "{\"id\":\"r1\",\"writer\":\"a/1\",\"at\":\"2026-10-01T10:00:00.000Z\",\"kind\":\"landed\",\"commit\":\"1111111111111111111111111111111111111111\",\"branch\":\"spec/y\",\"land_kind\":\"spec\",\"tier\":\"SOFT\",\"fingerprint\":\"f-r1\",\"changes\":[\"y\"]}\n";

/// The remote-tracking ledger ref `origin/clm/ledger/<writer>` holding `records`, as a fetch
/// would leave it.
fn ledger_ref(root: &Path, writer: &str, records: &str) {
    let git = |args: &[&str]| git_env(root, args, &[]);
    let file = root.join("records.jsonl.tmp");
    fs::write(&file, records).unwrap();
    let blob = git(&["hash-object", "-w", file.to_str().unwrap()]);
    fs::remove_file(&file).unwrap();
    let index = root.join(".git/ledger-index");
    let env = [("GIT_INDEX_FILE", index.to_str().unwrap())];
    let cacheinfo = format!("100644,{},records.jsonl", blob.trim());
    git_env(
        root,
        &["update-index", "--add", "--cacheinfo", &cacheinfo],
        &env,
    );
    let tree = git_env(root, &["write-tree"], &env);
    fs::remove_file(&index).unwrap();
    let commit = git(&["commit-tree", tree.trim(), "-m", "ledger"]);
    git(&[
        "update-ref",
        &format!("refs/remotes/origin/clm/ledger/{writer}"),
        commit.trim(),
    ]);
}

#[test]
fn status_has_no_review_section() {
    // req:view.repo/reads-the-repository@dd91d681
    let dir = base();
    let d = dir.path();
    claim_refs(d, &[]);
    ledger_ref(d, "a/1", &format!("{SOFT_LAND}not a record\n"));
    let out = clm_status(d);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8(out.stderr).unwrap(), "");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(!text.contains("review"), "{text}");
    assert!(text.contains("\n  waiting on you 0\n"), "{text}");
    assert!(text.ends_with("\nclaims\n  none\n\n"), "{text}");
}

/// `HEAD`'s hash cut to its first 7 hex digits.
fn head7(root: &Path) -> String {
    git_env(root, &["rev-parse", "HEAD"], &[])[..7].to_string()
}

/// `n` more commits on `HEAD`, then `HEAD` moved back to where it was: the commits stay
/// on `origin/main`, which `HEAD` is then `n` behind.
fn origin_ahead(root: &Path, n: usize) {
    let git = |args: &[&str]| git_env(root, args, &[]);
    let base = git(&["rev-parse", "HEAD"]);
    for i in 0..n {
        git(&["commit", "-q", "--allow-empty", "-m", &format!("later {i}")]);
    }
    git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(&["reset", "-q", "--hard", base.trim()]);
}

#[test]
fn the_commit_is_cut_to_seven_digits_whatever_git_abbreviates_to() {
    // req:view.repo/reads-the-repository@dd91d681
    let dir = base();
    let d = dir.path();
    claim_refs(d, &[]);
    // `--short` would follow this and print 12.
    git_env(d, &["config", "core.abbrev", "12"], &[]);
    let text = status(d);
    assert_eq!(
        text.lines().nth(1),
        Some(format!("  commit {}", head7(d)).as_str()),
        "{text}"
    );
}

#[test]
fn scenario_checkout_named() {
    // req:view.repo/reads-the-repository@dd91d681
    let dir = base();
    let d = dir.path();
    claim_refs(d, &[]);
    git_env(
        d,
        &[
            "remote",
            "add",
            "origin",
            "https://127.0.0.1:9/unreachable.git",
        ],
        &[],
    );
    let text = status(d);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "CLM status", "{text}");
    assert_eq!(lines[1], format!("  commit {}", head7(d)), "{text}");
}

#[test]
fn scenario_checkout_behind() {
    // req:view.repo/reads-the-repository@dd91d681
    let dir = base();
    let d = dir.path();
    claim_refs(d, &[]);
    origin_ahead(d, 2);
    git_env(
        d,
        &[
            "remote",
            "add",
            "origin",
            "https://127.0.0.1:9/unreachable.git",
        ],
        &[],
    );
    let text = status(d);
    let line = text.lines().nth(1).unwrap();
    assert!(line.starts_with("  commit "), "{text}");
    assert!(line.ends_with("⚠ 2 behind origin/main"), "{text}");
    assert_eq!(
        line,
        format!("  commit {}  ⚠ 2 behind origin/main", head7(d))
    );
}

#[test]
fn scenario_no_origin() {
    // req:view.repo/reads-the-repository@dd91d681
    let dir = base();
    let d = dir.path();
    git_env(d, &["init", "-q", "-b", "main"], &[]);
    git_env(d, &["add", "-A"], &[]);
    git_env(d, &["commit", "-qm", "base"], &[]);
    git_env(
        d,
        &[
            "remote",
            "add",
            "origin",
            "https://127.0.0.1:9/unreachable.git",
        ],
        &[],
    );
    let text = status(d);
    let line = text.lines().nth(1).unwrap();
    assert!(line.starts_with("  commit "), "{text}");
    assert!(!line.contains("behind"), "{text}");
}

#[test]
fn no_repository_names_no_commit() {
    // req:view.repo/reads-the-repository@dd91d681
    let dir = base();
    let out = clm_status(dir.path());
    // Outside a repository the sizes can't be measured: said on stderr, and nothing marked.
    let stderr = String::from_utf8(out.stderr.clone()).unwrap();
    assert!(stderr.starts_with("size not checked: "), "{stderr}");
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert!(!text.contains("oversized"), "{text}");
    assert!(
        text.lines().nth(1).unwrap().starts_with("  components "),
        "{text}"
    );
    // A repository with no commit yet has no `HEAD` to name either.
    git_env(dir.path(), &["init", "-q", "-b", "main"], &[]);
    let text = status(dir.path());
    assert!(
        text.lines().nth(1).unwrap().starts_with("  components "),
        "{text}"
    );
}

#[test]
fn scenario_a_main_it_cant_compare() {
    // req:view.repo/reads-the-repository@dd91d681
    let dir = base();
    let d = dir.path();
    claim_refs(d, &[]);
    // `origin/main` names a commit the repository's objects don't hold.
    fs::write(
        d.join(".git/refs/remotes/origin/main"),
        format!("{}\n", "d".repeat(40)),
    )
    .unwrap();
    let stderr = |out: &Output| String::from_utf8(out.stderr.clone()).unwrap();
    let not_compared = |out: &Output| {
        assert_eq!(out.status.code(), Some(0), "{out:?}");
        assert!(
            stderr(out).starts_with("checkout not compared: "),
            "{}",
            stderr(out)
        );
    };
    let out = clm_status(d);
    not_compared(&out);
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        text.lines().nth(1),
        Some(format!("  commit {}", head7(d)).as_str()),
        "{text}"
    );
    let out = clm(d, &["view", "flight", "--format", "json"]);
    not_compared(&out);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["checkout"]["commit"], head7(d), "{v:#}");
    assert_eq!(v["checkout"].get("behind"), None, "{v:#}");
    not_compared(&clm(d, &["pending"]));
}
