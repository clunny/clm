//! `clm sync`'s regeneration, stale-delta merges and `index.lock` retries (`sync.regenerate`),
//! against a bare `origin` and a work clone.

use clm_git::Options;
use clm_regenerate::{Error, Regenerated, git as lgit, regenerated, run};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
#[path = "../../clm-git/tests/exec/mod.rs"]
mod exec;
use exec::write_exec;

/// What `sync` found: the lines it printed, and whether nothing is left for the caller.
#[derive(Debug)]
struct Report {
    lines: Vec<String>,
    ok: bool,
}

/// The parts of `clm sync` this crate serves, in the order `sync`'s Design runs them: fetch,
/// rebase with the keep-`specs/` driver, regenerate the added applied changes
/// (`regenerated`), mirror the tree and commit `Re-apply`, merge the other changes' deltas,
/// and check. `clm-sync` is not a dependency of this node, so the run is driven here.
fn sync(root: &Path) -> Result<Report, Error> {
    clm_git::fetch(root)?;
    let onto = lgit(root, &["rev-parse", "origin/main"])?;
    let files = |rev: &str| -> Result<BTreeSet<String>, Error> {
        Ok(lgit(
            root,
            &["ls-tree", "-r", "--name-only", rev, "--", "changes"],
        )?
        .lines()
        .filter(|f| f.ends_with("/change.md"))
        .map(str::to_string)
        .collect())
    };
    let (head, known) = (files("HEAD")?, files(&onto)?);
    let mut added = Vec::new();
    let mut ids = Vec::new();
    for file in head.difference(&known) {
        let rel = file
            .strip_prefix("changes/")
            .and_then(|f| f.strip_suffix("/change.md"))
            .unwrap();
        let (id, archived) = match rel.strip_prefix("archive/") {
            Some(id) => (id, true),
            None => (rel, false),
        };
        let text = lgit(root, &["show", &format!("HEAD:{file}")])?;
        let status = if archived {
            "status: done"
        } else {
            "status: applied"
        };
        added.push(id.to_string());
        if text.lines().any(|l| l == status) {
            ids.push(id.to_string());
        }
    }
    ids.sort();
    let dir = TempDir::new().unwrap();
    let attributes = dir.path().join("attributes");
    let rules = if added.is_empty() {
        ""
    } else {
        "specs/** merge=clm-keep\n"
    };
    fs::write(&attributes, rules).unwrap();
    let config = [
        "-c".to_string(),
        format!("core.attributesFile={}", attributes.display()),
        "-c".to_string(),
        "merge.clm-keep.name=keep".to_string(),
        "-c".to_string(),
        "merge.clm-keep.driver=true".to_string(),
        "-c".to_string(),
        "rebase.backend=merge".to_string(),
        "rebase".to_string(),
        onto.clone(),
    ];
    let args: Vec<&str> = config.iter().map(String::as_str).collect();
    let editor = [("GIT_EDITOR", std::ffi::OsStr::new("true"))];
    let options = Options {
        env: &editor,
        ..Options::default()
    };
    if let Err(clm_git::Error::Failed { stderr, .. }) = run(root, &args, options) {
        let out = run(
            root,
            &["diff", "--name-only", "-z", "--diff-filter=U"],
            Options::default(),
        )?;
        let conflicts: Vec<String> = String::from_utf8_lossy(&out)
            .split('\0')
            .filter(|p| !p.is_empty())
            .map(|p| format!("conflict {p}"))
            .collect();
        if conflicts.is_empty() {
            return Err(Error::Git(format!("git rebase: {stderr}")));
        }
        return Ok(Report {
            lines: conflicts,
            ok: false,
        });
    }
    let mut lines = Vec::new();
    if !ids.is_empty() {
        let mut tree = match regenerated(root, &onto, &ids, true)? {
            Regenerated::Tree(tree) => tree,
            Regenerated::Stopped { merged, left } => {
                lines.extend(merged);
                lines.extend(left);
                return Ok(Report { lines, ok: false });
            }
        };
        lines.append(&mut tree.merged);
        mirror(root, tree.path())?;
        if !lgit(root, &["status", "--porcelain", "--", "specs"])?.is_empty() {
            lgit(root, &["add", "-A", "--", "specs"])?;
            lgit(
                root,
                &[
                    "commit",
                    "-q",
                    "-m",
                    &format!("Re-apply {}", ids.join(", ")),
                ],
            )?;
        }
        let (merged, left) = tree.other_deltas(root, &ids, &onto)?;
        lines.extend(merged);
        if !left.is_empty() {
            lines.extend(left);
            return Ok(Report { lines, ok: false });
        }
    }
    let check = clm_check::run(root);
    lines.extend(check.iter().map(clm_check::format));
    Ok(Report {
        lines,
        ok: check.is_empty(),
    })
}

/// Sets `root`'s tracked `specs/` to the regenerated tree's.
fn mirror(root: &Path, tree: &Path) -> Result<(), Error> {
    let mut fresh = BTreeSet::new();
    let mut stack = vec![PathBuf::from("specs")];
    while let Some(rel) = stack.pop() {
        let Ok(entries) = fs::read_dir(tree.join(&rel)) else {
            continue;
        };
        for entry in entries {
            let entry = entry.unwrap();
            let child = rel.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                stack.push(child);
            } else {
                fresh.insert(child);
            }
        }
    }
    for tracked in lgit(root, &["ls-files", "--", "specs"])?.lines() {
        if !fresh.contains(Path::new(tracked)) {
            fs::remove_file(root.join(tracked)).unwrap();
        }
    }
    for rel in &fresh {
        let to = root.join(rel);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(tree.join(rel), to).unwrap();
    }
    Ok(())
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

fn write(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn read(dir: &Path, rel: &str) -> String {
    fs::read_to_string(dir.join(rel)).unwrap()
}

fn commit(dir: &Path, msg: &str) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-qm", msg]);
}

fn requirement(slug: &str) -> String {
    format!("\n### Requirement: {slug}\nIt SHALL {slug}.\n\n#### Scenario: s\n- ok\n")
}

/// Node `a` with Contract `contract` and requirements `reqs`, at `status`.
fn node_a(status: &str, contract: &str, reqs: &[&str]) -> String {
    let mut s = format!(
        "---\ntitle: A\nstatus: {status}\nowners: []\n---\n\n## Contract\n{contract}\n\n\
         ## Design\nd\n\n## Requirements\n"
    );
    for r in reqs {
        s.push_str(&requirement(r));
    }
    s
}

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n";

struct Repo {
    _dirs: (TempDir, TempDir),
    origin: PathBuf,
    work: PathBuf,
}

/// A clone of a bare `origin` whose `main` has node `a` with requirement `p`, and HEAD on a
/// fresh branch `branch`.
fn repo(branch: &str) -> Repo {
    let (o, w) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let (origin, work) = (o.path().to_path_buf(), w.path().join("work"));
    git(&origin, &["init", "-q", "--bare", "-b", "main"]);
    git(
        w.path(),
        &[
            "clone",
            "-q",
            origin.to_str().unwrap(),
            work.to_str().unwrap(),
        ],
    );
    git(&work, &["config", "user.name", "t"]);
    git(&work, &["config", "user.email", "t@t"]);
    write(&work, "specs/SPEC.md", ROOT);
    write(&work, "specs/a/SPEC.md", &node_a("specd", "c", &["p"]));
    write(&work, "src/lib.rs", "one\ntwo\nthree\n");
    commit(&work, "base");
    git(&work, &["push", "-q", "origin", "HEAD:main"]);
    git(&work, &["checkout", "-q", "-b", branch]);
    Repo {
        _dirs: (o, w),
        origin,
        work,
    }
}

/// Commits `files` on `origin/main` from a second clone, as another PR merging would.
fn main_moves(r: &Repo, files: &[(&str, &str)]) {
    let other = r.work.with_file_name("other");
    if !other.exists() {
        git(
            r.work.parent().unwrap(),
            &["clone", "-q", r.origin.to_str().unwrap(), "other"],
        );
        git(&other, &["config", "user.name", "o"]);
        git(&other, &["config", "user.email", "o@o"]);
    }
    git(&other, &["pull", "-q", "origin", "main"]);
    for (rel, text) in files {
        write(&other, rel, text);
    }
    commit(&other, "main moves");
    git(&other, &["push", "-q", "origin", "HEAD:main"]);
}

fn origin_main(r: &Repo) -> String {
    git(&r.origin, &["rev-parse", "main"])
}

/// Runs `clm sync`, asserting the branch came out up to date and passing; its lines.
fn synced(r: &Repo) -> Vec<String> {
    let report = sync(&r.work).unwrap();
    assert!(report.ok, "{:?}", report.lines);
    report.lines
}

/// Runs `clm sync`, asserting something was left for the caller; its lines.
fn unfinished(r: &Repo) -> Vec<String> {
    let report = sync(&r.work).unwrap();
    assert!(!report.ok, "{:?}", report.lines);
    report.lines
}

/// A spec branch whose change `y` has `delta` for node `a`, applied with `clm apply` as
/// `propose` does.
fn applied_spec_branch(r: &Repo, delta: &str) {
    apply_change(r, "y", "", delta);
}

/// Adds change `id`, with `extra` frontmatter lines and `delta` for node `a`, applies it and
/// commits.
fn apply_change(r: &Repo, id: &str, extra: &str, delta: &str) {
    write(
        &r.work,
        &format!("changes/{id}/change.md"),
        &format!("---\nstatus: proposed\n{extra}---\n\n# {id}\n"),
    );
    write(&r.work, &format!("changes/{id}/deltas/a.md"), delta);
    clm_apply::apply(&r.work, id).unwrap();
    commit(&r.work, &format!("Apply {id}"));
}

fn modifies_contract(r: &Repo, text: &str) -> String {
    let base = clm_model::load(&r.work).0.nodes["a"]
        .hashes
        .contract
        .clone();
    format!(
        "---\ncomponent: a\nbase: {{ contract: \"{base}\" }}\n---\n## MODIFIED Contract\n{text}\n"
    )
}

fn adds_x() -> String {
    format!(
        "---\ncomponent: a\n---\n## ADDED Requirements\n{}",
        requirement("x")
    )
}

/// The hash `element` of node `a` has in the work clone's checkout, as `clm hashes` names it.
fn hash(r: &Repo, element: &str) -> String {
    let graph = clm_model::load(&r.work).0;
    clm_apply::hashes(&graph.nodes["a"])
        .into_iter()
        .find(|(e, _)| e == element)
        .unwrap()
        .1
}

/// The lines after the stale-delta violations, asserting there are some of each.
fn after_violations(lines: &[String]) -> &[String] {
    let first = lines
        .iter()
        .position(|l| l.starts_with("main changed") || l.starts_with("no text"))
        .unwrap_or_else(|| panic!("no report: {lines:?}"));
    assert!(
        lines[..first]
            .iter()
            .any(|l| l.contains("changes/y/deltas/a.md")),
        "the violations come first: {lines:?}"
    );
    &lines[first..]
}

/// The merge instruction for `change`'s delta of `a`, naming the hash to record.
fn merge_line(change: &str, hash: &str) -> String {
    format!(
        "merge the changes above into changes/{change}/deltas/a.md, then set its base to {hash}"
    )
}

/// Brings the work clone's branch up to `origin/main` with plain git.
fn pull(r: &Repo) {
    git(&r.work, &["pull", "-q", "--rebase", "origin", "main"]);
}

fn subject(r: &Repo) -> String {
    git(&r.work, &["log", "-1", "--format=%s"])
}

/// The hash `element` of node `a` has on `origin/main`, as `clm hashes a` gives it there.
fn main_hash(r: &Repo, element: &str) -> String {
    let other = r.work.with_file_name("other");
    git(&other, &["pull", "-q", "origin", "main"]);
    let graph = clm_model::load(&other).0;
    clm_apply::hashes(&graph.nodes["a"])
        .into_iter()
        .find(|(e, _)| e == element)
        .unwrap()
        .1
}

/// The subjects of the commits on the branch since `origin/main`, oldest first.
fn subjects_since_main(r: &Repo) -> Vec<String> {
    git(
        &r.work,
        &["log", "--reverse", "--format=%s", "origin/main..HEAD"],
    )
    .lines()
    .map(str::to_string)
    .collect()
}

/// Node `a` whose Design is `design`.
fn a_designed(design: &str) -> String {
    node_a("specd", "c", &["p"]).replace("## Design\nd", &format!("## Design\n{design}"))
}

/// A delta of `a` that rewrites its Design to `text`, with base `base`.
fn design_delta(base: &str, text: &str) -> String {
    format!("---\ncomponent: a\nbase: {{ design: \"{base}\" }}\n---\n## MODIFIED Design\n{text}\n")
}

/// Adds change `id`, already `applied`, with `delta` for node `a`, leaving `specs/` as it
/// is: a change folded onto the branch after an earlier one applied.
fn add_applied(r: &Repo, id: &str, delta: &str) {
    write(
        &r.work,
        &format!("changes/{id}/change.md"),
        &format!("---\nstatus: applied\n---\n\n# {id}\n"),
    );
    write(&r.work, &format!("changes/{id}/deltas/a.md"), delta);
    commit(&r.work, &format!("Add {id}"));
}

/// The lines from the first `<movers> changed` or `no text` line on.
fn from_report(lines: &[String]) -> &[String] {
    let first = lines
        .iter()
        .position(|l| l.contains(" changed ") || l.starts_with("no text"))
        .unwrap_or_else(|| panic!("no report: {lines:?}"));
    &lines[first..]
}

/// A spec branch `spec/x` adding changes `x` and `y`, both based on `origin/main`'s
/// three-line Design of `a`: `x` applied, rewriting the first line to `X1`, and `y`, added
/// as applied after it, rewriting the first line to `Y1`. Between them in ID order, `xw`
/// rewrites `a`'s Contract and leaves its Design alone. Returns the repo, `main`'s Design
/// hash and the Design's hash after `x` applies.
fn folded_x_and_y() -> (Repo, String, String) {
    let r = repo("spec/x");
    main_moves(&r, &[("specs/a/SPEC.md", &a_designed("one\ntwo\nthree"))]);
    pull(&r);
    let h1 = hash(&r, "design");
    apply_change(&r, "x", "", &design_delta(&h1, "X1\ntwo\nthree"));
    let h2 = hash(&r, "design");
    add_applied(&r, "xw", &modifies_contract(&r, "c by xw"));
    add_applied(&r, "y", &design_delta(&h1, "Y1\ntwo\nthree"));
    (r, h1, h2)
}

/// The three-paragraph Design of `a` the other-delta scenarios start from.
const THREE: &str = "one\n\ntwo\n\nthree";

/// A change `id` at `status` with `delta` for node `a`, as the files to write.
fn change_files(id: &str, status: &str, delta: &str) -> [(String, String); 2] {
    [
        (
            format!("changes/{id}/change.md"),
            format!("---\nstatus: {status}\n---\n\n# {id}\n"),
        ),
        (format!("changes/{id}/deltas/a.md"), delta.to_string()),
    ]
}

/// `repo("spec/x")` whose `origin/main` has `a`'s Design as `THREE` and a `proposed` change
/// `o` rewriting it to `theirs`, based on that Design; the branch then applies `x`, which
/// rewrites it to `mine`, and `main` gains an unrelated commit. Returns the repo, `main`'s
/// Design hash and `o`'s delta.
fn other_change_on_main(theirs: &str, mine: &str) -> (Repo, String, String) {
    other_change_at("proposed", theirs, mine)
}

/// `other_change_on_main` with `o` at `status`.
fn other_change_at(status: &str, theirs: &str, mine: &str) -> (Repo, String, String) {
    let r = repo("spec/x");
    main_moves(&r, &[("specs/a/SPEC.md", &a_designed(THREE))]);
    pull(&r);
    let h1 = hash(&r, "design");
    let o = design_delta(&h1, theirs);
    let files = change_files("o", status, &o);
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    main_moves(&r, &files);
    pull(&r);
    apply_change(&r, "x", "", &design_delta(&h1, mine));
    main_moves(&r, &[("src/other.rs", "other\n")]);
    (r, h1, o)
}

/// Sets change `id`'s status to `status` and moves it to `changes/archive/<id>/` in `dir`,
/// committing the result as `Archive <id>`.
fn archive_in(dir: &Path, id: &str, status: &str) {
    fs::create_dir_all(dir.join("changes/archive")).unwrap();
    git(
        dir,
        &[
            "mv",
            &format!("changes/{id}"),
            &format!("changes/archive/{id}"),
        ],
    );
    let path = format!("changes/archive/{id}/change.md");
    let md = read(dir, &path).replace("status: applied", &format!("status: {status}"));
    write(dir, &path, &md);
    commit(dir, &format!("Archive {id}"));
}

/// Installs `hook` in the work clone with the shell `body`, which may use `$lock` (the
/// worktree's `index.lock`) and `$seen` (a file outside the repository).
fn lock_hook(r: &Repo, hook: &str, body: &str) {
    let path = r.work.join(git(
        &r.work,
        &["rev-parse", "--git-path", &format!("hooks/{hook}")],
    ));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let lock = r
        .work
        .join(git(&r.work, &["rev-parse", "--git-path", "index.lock"]));
    let seen = r.work.with_file_name(format!("{hook}.seen"));
    write_exec(
        &path,
        &format!(
            "#!/bin/sh\nlock='{}'\nseen='{}'\n{body}\n",
            lock.display(),
            seen.display()
        ),
    );
}

/// `node` serving `architecture/foo`, so every rule traces to an entry.
fn serving(node: &str) -> String {
    node.replacen(
        "owners: []\n",
        "owners: []\nserves: [architecture/foo]\n",
        1,
    )
}

/// Node `a` with requirements `p` and `q`, where `p` has a `#### Serves` block of its own
/// and `q`'s scenario line is `q_line`.
fn a_with_p_serving(q_line: &str) -> String {
    serving(&node_a("specd", "c", &["p", "q"]))
        .replace(
            "It SHALL p.\n\n#### Scenario: s\n- ok\n",
            "It SHALL p.\n\n#### Scenario: s\n- ok\n\n#### Serves\n- architecture/foo\n",
        )
        .replace(
            "It SHALL q.\n\n#### Scenario: s\n- ok",
            &format!("It SHALL q.\n\n#### Scenario: s\n{q_line}"),
        )
}

/// A spec branch `spec/x` with `p` and `q` on `main`, whose change `x` modifies both, `p`
/// without a `#### Serves` block, each with its hash at the time as its base. The delta's `q`
/// has `q_text` as its rule and `q_line` as its scenario line. Returns the repo and the two
/// bases.
fn x_modifies_p_and_q(q_text: &str, q_line: &str) -> (Repo, String, String) {
    let r = repo("spec/x");
    let architecture = "# A\n\n## Constraints\n\n### Constraint: foo\nIt SHALL foo.\n\n#### Scenario: s\n- ok\n\n#### Untested\nNo.\n";
    main_moves(
        &r,
        &[
            ("ARCHITECTURE.md", architecture),
            (
                "specs/a/SPEC.md",
                &serving(&node_a("specd", "c", &["p", "q"])),
            ),
        ],
    );
    pull(&r);
    let (p, q) = (hash(&r, "requirement:p"), hash(&r, "requirement:q"));
    let delta = format!(
        "---\ncomponent: a\n---\n## MODIFIED Requirements\n\
         ### Requirement: p <!-- base: {p} -->\nIt SHALL p, mine.\n\n#### Scenario: s\n- ok\n\n\
         ### Requirement: q <!-- base: {q} -->\n{q_text}\n\n#### Scenario: s\n{q_line}\n"
    );
    apply_change(&r, "x", "", &delta);
    (r, p, q)
}

fn tree_text(tree: &clm_regenerate::Tree, rel: &str) -> String {
    std::fs::read_to_string(tree.path().join(rel)).unwrap()
}

#[test]
fn stale_delta_shows_what_main_changed_on_the_line_it_changed() {
    // req:sync.regenerate/stale-delta-diff@1b85fae3
    let r = repo("spec/y");
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "Do y.", &["p"]))],
    );
    pull(&r);
    let h1 = hash(&r, "contract");
    let delta = modifies_contract(&r, "Do y differently.");
    applied_spec_branch(&r, &delta);
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "Do y. Also z.", &["p"]))],
    );

    let left = unfinished(&r);
    assert!(!left.iter().any(|l| l.starts_with("merged")), "{left:?}");
    assert_eq!(
        after_violations(&left),
        [
            format!("main changed contract of a since {h1}"),
            "@@ -1 +1 @@".into(),
            "-Do y.".into(),
            "+Do y. Also z.".into(),
            merge_line("y", &main_hash(&r, "contract")),
        ]
    );
    let main_spec = git(&r.origin, &["show", "main:specs/a/SPEC.md"]);
    assert_eq!(read(&r.work, "specs/a/SPEC.md").trim(), main_spec);
}

#[test]
fn stale_requirement_shows_the_changed_scenario_line() {
    // req:sync.regenerate/stale-delta-diff@1b85fae3
    let r = repo("spec/y");
    let h1 = hash(&r, "requirement:p");
    let delta = format!(
        "---\ncomponent: a\n---\n## MODIFIED Requirements\n### Requirement: p <!-- base: {h1} -->\n\
         It SHALL p.\n\n#### Scenario: s\n- mine\n"
    );
    applied_spec_branch(&r, &delta);
    let theirs = node_a("specd", "c", &["p"]).replace("- ok", "- theirs");
    main_moves(&r, &[("specs/a/SPEC.md", &theirs)]);

    let left = unfinished(&r);
    let report = after_violations(&left);
    assert_eq!(
        report[0],
        format!("main changed requirement:p of a since {h1}")
    );
    assert!(report[1].starts_with("@@ "), "{report:?}");
    assert!(report.contains(&"-- ok".to_string()), "{report:?}");
    assert!(report.contains(&"+- theirs".to_string()), "{report:?}");
    assert_eq!(
        *report.last().unwrap(),
        merge_line("y", &main_hash(&r, "requirement:p"))
    );
}

#[test]
fn an_element_main_removed_diffs_to_nothing() {
    // req:sync.regenerate/stale-delta-diff@1b85fae3
    let r = repo("spec/y");
    let h1 = hash(&r, "requirement:p");
    let delta = format!(
        "---\ncomponent: a\n---\n## MODIFIED Requirements\n### Requirement: p <!-- base: {h1} -->\n\
         It SHALL p.\n\n#### Scenario: s\n- mine\n"
    );
    applied_spec_branch(&r, &delta);
    main_moves(&r, &[("specs/a/SPEC.md", &node_a("specd", "c", &[]))]);

    let left = unfinished(&r);
    let report = after_violations(&left);
    assert_eq!(
        report[0],
        format!("main changed requirement:p of a since {h1}")
    );
    let hunk = &report[2..report.len() - 1];
    assert!(!hunk.is_empty(), "{report:?}");
    assert!(hunk.iter().all(|l| l.starts_with('-')), "{report:?}");
    assert!(
        hunk.contains(&"-### Requirement: p".to_string()),
        "{report:?}"
    );
    assert_eq!(
        *report.last().unwrap(),
        "rework changes/y/deltas/a.md by hand: a no longer has requirement:p"
    );
    assert!(
        !report.iter().any(|l| l.starts_with("merge the changes")),
        "{report:?}"
    );
}

#[test]
fn a_base_no_commit_has_is_compared_by_hand() {
    // req:sync.regenerate/stale-delta-diff@1b85fae3
    let r = repo("spec/y");
    let design = hash(&r, "design");
    let delta = format!(
        "---\ncomponent: a\nbase: {{ design: \"{design}\" }}\n---\n## MODIFIED Design\nmine\n"
    );
    applied_spec_branch(&r, &delta);
    let typed = "0".repeat(design.len());
    write(
        &r.work,
        "changes/y/deltas/a.md",
        &delta.replace(&design, &typed),
    );
    commit(&r.work, "a base typed by hand");
    main_moves(&r, &[("src/other.rs", "other\n")]);

    let left = unfinished(&r);
    assert_eq!(
        after_violations(&left),
        [format!(
            "no text for design of a at {typed}; compare changes/y/deltas/a.md with \
             design of a after main by hand"
        )]
    );
}

#[test]
fn only_the_sections_a_delta_modifies_are_shown() {
    // req:sync.regenerate/stale-delta-diff@1b85fae3
    let r = repo("spec/y");
    let (contract, design) = (hash(&r, "contract"), hash(&r, "design"));
    let delta = format!(
        "---\ncomponent: a\nbase: {{ contract: \"{contract}\", design: \"{design}\" }}\n---\n\
         ## MODIFIED Contract\nmine\n"
    );
    applied_spec_branch(&r, &delta);
    let theirs = node_a("specd", "c2", &["p"]).replace("## Design\nd", "## Design\nd2");
    main_moves(&r, &[("specs/a/SPEC.md", &theirs)]);

    let left = unfinished(&r);
    let report = after_violations(&left);
    assert_eq!(
        report[0],
        format!("main changed contract of a since {contract}")
    );
    assert!(!report.iter().any(|l| l.contains("design")), "{report:?}");
}

#[test]
fn a_removed_requirement_main_changed_is_shown() {
    // req:sync.regenerate/stale-delta-diff@1b85fae3
    let r = repo("spec/y");
    let h1 = hash(&r, "requirement:p");
    let delta = format!(
        "---\ncomponent: a\n---\n## REMOVED Requirements\n### Requirement: p <!-- base: {h1} -->\n"
    );
    applied_spec_branch(&r, &delta);
    let theirs = node_a("specd", "c", &["p"]).replace("It SHALL p.", "It SHALL p, and more.");
    main_moves(&r, &[("specs/a/SPEC.md", &theirs)]);

    let left = unfinished(&r);
    let report = after_violations(&left);
    assert_eq!(
        report[0],
        format!("main changed requirement:p of a since {h1}")
    );
    assert!(
        report.contains(&"+It SHALL p, and more.".to_string()),
        "{report:?}"
    );
}

#[test]
fn a_stale_frontmatter_base_gets_no_report() {
    // req:sync.regenerate/stale-delta-diff@1b85fae3
    let r = repo("spec/y");
    let fm = hash(&r, "frontmatter");
    let delta = format!(
        "---\ncomponent: a\nbase: {{ frontmatter: \"{fm}\" }}\n---\n## MODIFIED Frontmatter\n\
         ```yaml\ntitle: Mine\n```\n"
    );
    applied_spec_branch(&r, &delta);
    let theirs = node_a("specd", "c", &["p"]).replace("title: A", "title: Theirs");
    main_moves(&r, &[("specs/a/SPEC.md", &theirs)]);

    let left = unfinished(&r);
    assert!(
        left.iter().any(|l| l.contains("apply/stale-base")),
        "{left:?}"
    );
    assert!(
        !left
            .iter()
            .any(|l| l.starts_with("main changed") || l.starts_with("no text")),
        "{left:?}"
    );
    // req:sync.regenerate/stale-delta-merge@83a398e5: nor is a frontmatter base merged.
    assert!(!left.iter().any(|l| l.starts_with("merged")), "{left:?}");
}

#[test]
fn edits_apart_are_merged_into_the_delta() {
    // req:sync.regenerate/stale-delta-merge@83a398e5
    let r = repo("spec/x");
    let three = "one\ntwo\nthree";
    main_moves(&r, &[("specs/a/SPEC.md", &node_a("specd", three, &["p"]))]);
    pull(&r);
    let delta = modifies_contract(&r, "ONE\ntwo\nthree");
    apply_change(&r, "x", "", &delta);
    main_moves(
        &r,
        &[(
            "specs/a/SPEC.md",
            &node_a("specd", "one\ntwo\nTHREE", &["p"]),
        )],
    );

    assert_eq!(
        synced(&r),
        ["merged contract of a into changes/x/deltas/a.md"]
    );
    assert_eq!(
        subjects_since_main(&r),
        ["Apply x", "Merge main into x", "Re-apply x"]
    );
    let merge = git(&r.work, &["show", "--name-only", "--format=", "HEAD~1"]);
    assert_eq!(merge, "changes/x/deltas/a.md");
    let merged = read(&r.work, "changes/x/deltas/a.md");
    let base = main_hash(&r, "contract");
    assert_eq!(
        merged,
        format!(
            "---\ncomponent: a\nbase: {{ contract: \"{base}\" }}\n---\n## MODIFIED Contract\n\
             ONE\ntwo\nTHREE\n"
        )
    );
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(spec.contains("## Contract\nONE\ntwo\nTHREE\n"), "{spec}");
    assert_eq!(git(&r.work, &["status", "--porcelain"]), "");
}

#[test]
fn a_block_map_base_is_set_in_place() {
    // req:sync.regenerate/stale-delta-merge@83a398e5
    let r = repo("spec/x");
    let three = "one\ntwo\nthree";
    main_moves(&r, &[("specs/a/SPEC.md", &node_a("specd", three, &["p"]))]);
    pull(&r);
    let (contract, design) = (hash(&r, "contract"), hash(&r, "design"));
    let delta = format!(
        "---\ncomponent: a\nbase:\n  design: \"{design}\"\n  contract: \"{contract}\"\n---\n\
         ## MODIFIED Contract\n\nONE\ntwo\nthree\n\n## MODIFIED Design\nd\n"
    );
    apply_change(&r, "x", "", &delta);
    main_moves(
        &r,
        &[(
            "specs/a/SPEC.md",
            &node_a("specd", "one\ntwo\nTHREE", &["p"]),
        )],
    );

    assert_eq!(
        synced(&r),
        ["merged contract of a into changes/x/deltas/a.md"]
    );
    let new = main_hash(&r, "contract");
    assert_eq!(
        read(&r.work, "changes/x/deltas/a.md"),
        format!(
            "---\ncomponent: a\nbase:\n  design: \"{design}\"\n  contract: \"{new}\"\n---\n\
             ## MODIFIED Contract\n\nONE\ntwo\nTHREE\n\n## MODIFIED Design\nd\n"
        )
    );
}

#[test]
fn overlapping_edits_are_left_for_the_caller() {
    // req:sync.regenerate/stale-delta-merge@83a398e5
    let r = repo("spec/x");
    let h1 = hash(&r, "contract");
    let delta = modifies_contract(&r, "mine");
    apply_change(&r, "x", "", &delta);
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "theirs", &["p"]))],
    );

    let left = unfinished(&r);
    assert!(!left.iter().any(|l| l.starts_with("merged")), "{left:?}");
    assert!(
        left.contains(&format!("main changed contract of a since {h1}")),
        "{left:?}"
    );
    assert_eq!(subjects_since_main(&r), ["Apply x"]);
    assert_eq!(read(&r.work, "changes/x/deltas/a.md"), delta);
}

#[test]
fn one_requirement_merged_one_left() {
    // req:sync.regenerate/stale-delta-merge@83a398e5
    let r = repo("spec/x");
    let architecture = "# A\n\n## Constraints\n\n### Constraint: foo\nIt SHALL foo.\n\n#### Scenario: s\n- ok\n\n#### Untested\nNo.\n";
    main_moves(
        &r,
        &[
            ("ARCHITECTURE.md", architecture),
            (
                "specs/a/SPEC.md",
                &serving(&node_a("specd", "c", &["p", "q"])),
            ),
        ],
    );
    pull(&r);
    let (p, q) = (hash(&r, "requirement:p"), hash(&r, "requirement:q"));
    let delta = format!(
        "---\ncomponent: a\n---\n## MODIFIED Requirements\n\
         ### Requirement: p <!-- base: {p} -->\nIt SHALL p, mine.\n\n#### Scenario: s\n- ok\n\n\
         ### Requirement: q <!-- base: {q} -->\nIt SHALL q.\n\n#### Scenario: s\n- mine\n"
    );
    apply_change(&r, "x", "", &delta);
    let theirs = node_a("specd", "c", &["p", "q"]).replace("- ok", "- theirs");
    main_moves(&r, &[("specs/a/SPEC.md", &theirs)]);

    let left = unfinished(&r);
    assert_eq!(
        left[0],
        "merged requirement:p of a into changes/x/deltas/a.md"
    );
    assert!(
        left.contains(&format!("main changed requirement:q of a since {q}")),
        "{left:?}"
    );
    assert!(
        !left.iter().any(|l| l.contains("requirement:p of a since")),
        "{left:?}"
    );
    assert_eq!(
        git(&r.work, &["log", "-1", "--format=%s"]),
        "Merge main into x"
    );
    let new_p = main_hash(&r, "requirement:p");
    assert_eq!(
        read(&r.work, "changes/x/deltas/a.md"),
        format!(
            "---\ncomponent: a\n---\n## MODIFIED Requirements\n\
             ### Requirement: p <!-- base: {new_p} -->\nIt SHALL p, mine.\n\n#### Scenario: s\n\
             - theirs\n\n\
             ### Requirement: q <!-- base: {q} -->\nIt SHALL q.\n\n#### Scenario: s\n- mine\n"
        )
    );
    let main_spec = git(&r.origin, &["show", "main:specs/a/SPEC.md"]);
    assert_eq!(read(&r.work, "specs/a/SPEC.md").trim(), main_spec);
}

#[test]
fn a_section_gone_from_main_is_not_merged() {
    // req:sync.regenerate/stale-delta-merge@83a398e5
    let r = repo("spec/x");
    let h1 = hash(&r, "requirement:p");
    let delta = format!(
        "---\ncomponent: a\n---\n## MODIFIED Requirements\n### Requirement: p <!-- base: {h1} -->\n\
         It SHALL p, mine.\n\n#### Scenario: s\n- ok\n"
    );
    apply_change(&r, "x", "", &delta);
    main_moves(&r, &[("specs/a/SPEC.md", &node_a("specd", "c", &[]))]);

    let left = unfinished(&r);
    assert!(!left.iter().any(|l| l.starts_with("merged")), "{left:?}");
    assert_eq!(read(&r.work, "changes/x/deltas/a.md"), delta);
    assert_eq!(subjects_since_main(&r), ["Apply x"]);
}

#[test]
fn a_removed_requirement_is_not_merged() {
    // req:sync.regenerate/stale-delta-merge@83a398e5
    let r = repo("spec/x");
    let h1 = hash(&r, "requirement:p");
    let delta = format!(
        "---\ncomponent: a\n---\n## REMOVED Requirements\n### Requirement: p <!-- base: {h1} -->\n"
    );
    apply_change(&r, "x", "", &delta);
    let theirs = node_a("specd", "c", &["p"]).replace("- ok", "- theirs");
    main_moves(&r, &[("specs/a/SPEC.md", &theirs)]);

    let left = unfinished(&r);
    assert!(!left.iter().any(|l| l.starts_with("merged")), "{left:?}");
    assert_eq!(read(&r.work, "changes/x/deltas/a.md"), delta);
    assert_eq!(subjects_since_main(&r), ["Apply x"]);
}

#[test]
fn an_earlier_change_on_the_branch_moved_it() {
    // req:sync.regenerate/stale-delta-diff@1b85fae3
    let (r, h1, h2) = folded_x_and_y();
    main_moves(&r, &[("src/other.rs", "other\n")]);

    let left = unfinished(&r);
    let at = left
        .iter()
        .position(|l| l.contains(" changed "))
        .unwrap_or_else(|| panic!("no report: {left:?}"));
    assert!(
        left[..at]
            .iter()
            .any(|l| l.contains("apply/stale-base") && l.contains("changes/y/deltas/a.md")),
        "{left:?}"
    );
    assert!(!left.iter().any(|l| l.starts_with("merged")), "{left:?}");
    assert!(!left.iter().any(|l| l.starts_with("main ")), "{left:?}");
    assert_eq!(
        from_report(&left),
        [
            format!("x changed design of a since {h1}"),
            "@@ -1,3 +1,3 @@".into(),
            "-one".into(),
            "+X1".into(),
            " two".into(),
            " three".into(),
            merge_line("y", &h2),
        ]
    );
}

#[test]
fn main_and_an_earlier_change_moved_it() {
    // req:sync.regenerate/stale-delta-diff@1b85fae3
    let (r, h1, _) = folded_x_and_y();
    main_moves(&r, &[("specs/a/SPEC.md", &a_designed("one\ntwo\nTHREE"))]);

    let left = unfinished(&r);
    assert_eq!(
        left[0], "merged design of a into changes/x/deltas/a.md",
        "{left:?}"
    );
    let report = from_report(&left);
    assert_eq!(
        report[0],
        format!("main and x changed design of a since {h1}")
    );
    for want in ["+X1", "+THREE"] {
        assert!(report.contains(&want.to_string()), "{want}: {report:?}");
    }
    assert!(
        subjects_since_main(&r).contains(&"Merge main into x".to_string()),
        "{:?}",
        subjects_since_main(&r)
    );
}

#[test]
fn a_base_from_an_earlier_change_is_merged() {
    // req:sync.regenerate/stale-delta-merge@83a398e5
    // Paragraphs, not adjacent lines: `git merge-file` takes edits to adjacent lines for a
    // conflict.
    let r = repo("spec/x");
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &a_designed("one\n\ntwo\n\nthree"))],
    );
    pull(&r);
    let h1 = hash(&r, "design");
    apply_change(&r, "x", "", &design_delta(&h1, "X1\n\ntwo\n\nthree"));
    let h2 = hash(&r, "design");
    add_applied(&r, "y", &design_delta(&h1, "one\n\nY2\n\nthree"));
    add_applied(&r, "z", &design_delta(&h2, "X1\n\ntwo\n\nZ3"));
    main_moves(&r, &[("src/other.rs", "other\n")]);

    assert_eq!(
        synced(&r),
        [
            "merged design of a into changes/y/deltas/a.md",
            "merged design of a into changes/z/deltas/a.md",
        ]
    );
    let subjects = subjects_since_main(&r);
    for want in ["Merge x into y", "Merge y into z"] {
        assert!(subjects.contains(&want.to_string()), "{want}: {subjects:?}");
    }
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(spec.contains("## Design\nX1\n\nY2\n\nZ3\n"), "{spec}");
}

#[test]
fn another_changes_delta_is_merged() {
    // req:sync.regenerate/other-delta-merge@c225106c
    let (r, _, _) = other_change_on_main("one\n\ntwo\n\nTHREE", "X1\n\ntwo\n\nthree");

    assert_eq!(
        synced(&r),
        ["merged design of a into changes/o/deltas/a.md"]
    );
    assert_eq!(subject(&r), "Merge x into o");
    assert_eq!(
        git(&r.work, &["show", "--name-only", "--format=", "HEAD"]),
        "changes/o/deltas/a.md"
    );
    let head = hash(&r, "design");
    assert_eq!(
        read(&r.work, "changes/o/deltas/a.md"),
        design_delta(&head, "X1\n\ntwo\n\nTHREE")
    );
    assert!(clm_check::run(&r.work).is_empty());
    assert_eq!(git(&r.work, &["status", "--porcelain"]), "");
}

#[test]
fn an_approved_changes_delta_is_merged() {
    // req:sync.regenerate/other-delta-merge@c225106c
    let (r, _, _) = other_change_at("approved", "one\n\ntwo\n\nTHREE", "X1\n\ntwo\n\nthree");

    assert_eq!(
        synced(&r),
        ["merged design of a into changes/o/deltas/a.md"]
    );
    assert_eq!(subject(&r), "Merge x into o");
}

#[test]
fn another_changes_overlapping_delta_is_left_for_the_caller() {
    // req:sync.regenerate/other-delta-merge@c225106c
    let (r, h1, o) = other_change_on_main("O1\n\ntwo\n\nthree", "X1\n\ntwo\n\nthree");

    let left = unfinished(&r);
    assert!(!left.iter().any(|l| l.starts_with("merged")), "{left:?}");
    assert!(
        !subjects_since_main(&r)
            .iter()
            .any(|s| s == "Merge x into o")
    );
    assert_eq!(read(&r.work, "changes/o/deltas/a.md"), o);
    let at = left
        .iter()
        .position(|l| l.contains(" changed "))
        .unwrap_or_else(|| panic!("no report: {left:?}"));
    assert!(
        left[..at]
            .iter()
            .any(|l| l.contains("apply/stale-base") && l.contains("changes/o/deltas/a.md")),
        "{left:?}"
    );
    let report = &left[at..];
    assert_eq!(report[0], format!("x changed design of a since {h1}"));
    assert!(report.contains(&"+X1".to_string()), "{report:?}");
    // The regenerate's own commits are kept.
    assert_eq!(git(&r.work, &["status", "--porcelain"]), "");
}

#[test]
fn a_proposed_change_the_branch_adds_is_merged() {
    // req:sync.regenerate/other-delta-merge@c225106c
    let r = repo("spec/x");
    main_moves(&r, &[("specs/a/SPEC.md", &a_designed(THREE))]);
    pull(&r);
    let h1 = hash(&r, "design");
    apply_change(&r, "x", "", &design_delta(&h1, "X1\n\ntwo\n\nthree"));
    for (path, text) in change_files("y", "proposed", &design_delta(&h1, "one\n\ntwo\n\nY3")) {
        write(&r.work, &path, &text);
    }
    commit(&r.work, "Add y");
    main_moves(&r, &[("src/other.rs", "other\n")]);

    assert_eq!(
        synced(&r),
        ["merged design of a into changes/y/deltas/a.md"]
    );
    assert_eq!(subject(&r), "Merge x into y");
}

#[test]
fn an_applied_change_on_main_is_left_alone() {
    // req:sync.regenerate/other-delta-merge@c225106c
    let r = repo("spec/x");
    main_moves(&r, &[("specs/a/SPEC.md", &a_designed(THREE))]);
    pull(&r);
    let h1 = hash(&r, "design");
    git(&r.work, &["checkout", "-q", "-b", "p"]);
    apply_change(&r, "p", "", &design_delta(&h1, "one\n\ntwo\n\nP3"));
    let p = read(&r.work, "changes/p/deltas/a.md");
    git(&r.work, &["push", "-q", "origin", "HEAD:main"]);
    git(&r.work, &["checkout", "-q", "spec/x"]);
    pull(&r);
    let h2 = hash(&r, "design");
    apply_change(&r, "x", "", &design_delta(&h2, "X1\n\ntwo\n\nP3"));
    main_moves(&r, &[("src/other.rs", "other\n")]);

    let lines = synced(&r);
    assert!(
        !lines.iter().any(|l| l.contains("changes/p/deltas/a.md")),
        "{lines:?}"
    );
    assert_eq!(read(&r.work, "changes/p/deltas/a.md"), p);
}

#[test]
fn a_branch_that_regenerates_nothing_merges_nothing() {
    // req:sync.regenerate/other-delta-merge@c225106c
    // `o`'s base is in `origin/main`'s history and its edit is apart from `main`'s, so only
    // the branch regenerating nothing keeps it from being merged.
    let r = repo("spec/x");
    main_moves(&r, &[("specs/a/SPEC.md", &a_designed(THREE))]);
    pull(&r);
    let h1 = hash(&r, "design");
    let o = design_delta(&h1, "O1\n\ntwo\n\nthree");
    let mut files: Vec<(String, String)> = change_files("o", "proposed", &o).into();
    files.push(("specs/a/SPEC.md".into(), a_designed("one\n\ntwo\n\nTHREE")));
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    main_moves(&r, &files);
    for (path, text) in change_files("x", "proposed", &modifies_contract(&r, "X")) {
        write(&r.work, &path, &text);
    }
    commit(&r.work, "Add x");

    let left = unfinished(&r);
    assert!(!left.iter().any(|l| l.starts_with("merged")), "{left:?}");
    assert_eq!(read(&r.work, "changes/o/deltas/a.md"), o);
    assert!(
        left.iter()
            .any(|l| l.contains("apply/stale-base") && l.contains("changes/o/deltas/a.md")),
        "{left:?}"
    );
}

#[test]
fn an_archived_change_is_merged() {
    // req:sync/reapply@e04f28bc req:sync.regenerate/stale-delta-merge@83a398e5
    let r = repo("spec/x");
    main_moves(
        &r,
        &[(
            "specs/a/SPEC.md",
            &node_a("specd", "one\ntwo\nthree", &["p"]),
        )],
    );
    pull(&r);
    let delta = modifies_contract(&r, "ONE\ntwo\nthree");
    apply_change(&r, "x", "", &delta);
    archive_in(&r.work, "x", "done");
    main_moves(
        &r,
        &[(
            "specs/a/SPEC.md",
            &node_a("specd", "one\ntwo\nTHREE", &["p"]),
        )],
    );

    assert_eq!(
        synced(&r),
        ["merged contract of a into changes/archive/x/deltas/a.md"]
    );
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(spec.contains("## Contract\nONE\ntwo\nTHREE\n"), "{spec}");
    assert_eq!(
        subjects_since_main(&r),
        ["Apply x", "Archive x", "Merge main into x", "Re-apply x"]
    );
}

#[test]
fn a_lock_lifted_before_the_retry_is_waited_out() {
    // req:sync.regenerate/retry-index-lock@0eb5526e
    let r = repo("spec/x");
    main_moves(&r, &[("src/other.rs", "other\n")]);
    // The first rebase finds the lock taken; the retried one finds it gone.
    lock_hook(
        &r,
        "pre-rebase",
        "if [ -e \"$seen\" ]; then rm -f \"$lock\"; else touch \"$seen\" \"$lock\"; fi",
    );

    assert_eq!(synced(&r), Vec::<String>::new());
    assert_eq!(git(&r.work, &["rev-parse", "HEAD"]), origin_main(&r));
    assert!(r.work.with_file_name("pre-rebase.seen").exists());
}

#[test]
fn a_lock_met_at_the_reapply_commit_is_waited_out() {
    // req:sync.regenerate/retry-index-lock@0eb5526e
    let r = repo("spec/x");
    apply_change(&r, "x", "", &adds_x());
    main_moves(
        &r,
        &[("specs/a/SPEC.md", &node_a("specd", "c", &["p", "y"]))],
    );
    // Held from the end of the rebase, released half a second later. No hook runs before
    // `git add`, so a timer releases it; the run taking that long shows it waited.
    lock_hook(
        &r,
        "post-rewrite",
        "touch \"$lock\"\n(sleep 0.5; rm -f \"$lock\") >/dev/null 2>&1 </dev/null &",
    );

    let started = std::time::Instant::now();
    assert_eq!(synced(&r), Vec::<String>::new());
    assert!(
        started.elapsed() >= std::time::Duration::from_millis(500),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(subject(&r), "Re-apply x");
    assert_eq!(git(&r.work, &["status", "--porcelain"]), "");
}

#[test]
fn a_conflict_is_not_retried() {
    // req:sync.regenerate/retry-index-lock@0eb5526e
    let r = repo("spec/x");
    write(&r.work, "src/lib.rs", "one\nmine\nthree\n");
    commit(&r.work, "mine");
    main_moves(&r, &[("src/lib.rs", "one\ntheirs\nthree\n")]);

    let started = std::time::Instant::now();
    assert_eq!(unfinished(&r), ["conflict src/lib.rs"]);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn a_lock_kept_fails_the_run_in_time() {
    // req:sync.regenerate/retry-index-lock@0eb5526e
    let r = repo("spec/x");
    main_moves(&r, &[("src/other.rs", "other\n")]);
    let lock = r
        .work
        .join(git(&r.work, &["rev-parse", "--git-path", "index.lock"]));
    write(&r.work, &lock.to_string_lossy(), "");

    let started = std::time::Instant::now();
    let err = sync(&r.work).unwrap_err();
    let took = started.elapsed();
    assert_eq!(err.exit_code(), 2, "{err}");
    assert!(err.to_string().contains("index.lock"), "{err}");
    assert!(took < std::time::Duration::from_secs(10), "{took:?}");
    // Retried: the waits add up to 3.75 seconds.
    assert!(took >= std::time::Duration::from_secs(3), "{took:?}");
}

#[test]
fn another_failure_is_not_retried() {
    // req:sync.regenerate/retry-index-lock@0eb5526e
    let r = repo("spec/x");
    let gone = r.work.with_file_name("gone");
    git(
        &r.work,
        &["remote", "set-url", "origin", gone.to_str().unwrap()],
    );

    let started = std::time::Instant::now();
    let err = sync(&r.work).unwrap_err();
    assert_eq!(err.exit_code(), 2, "{err}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn serves_edited_on_main_is_not_merged() {
    // req:sync.regenerate/stale-delta-merge@83a398e5
    let (r, p, _) = x_modifies_p_and_q("It SHALL q, mine.", "- ok");
    // `p`'s `#### Serves` block and a scenario line of `q` the delta didn't change.
    main_moves(&r, &[("specs/a/SPEC.md", &a_with_p_serving("- theirs"))]);

    let lines = synced(&r);
    let merged: Vec<&String> = lines.iter().filter(|l| l.starts_with("merged")).collect();
    assert_eq!(
        merged,
        ["merged requirement:q of a into changes/x/deltas/a.md"],
        "{lines:?}"
    );
    assert_eq!(
        git(&r.work, &["log", "-1", "--format=%s", "--grep=^Merge"]),
        "Merge main into x"
    );
    let delta = read(&r.work, "changes/x/deltas/a.md");
    assert!(
        delta.contains(&format!(
            "### Requirement: p <!-- base: {p} -->\nIt SHALL p, mine."
        )),
        "{delta}"
    );
    let spec = read(&r.work, "specs/a/SPEC.md");
    assert!(spec.contains("It SHALL p, mine."), "{spec}");
    assert!(spec.contains("#### Serves\n- architecture/foo"), "{spec}");
    assert!(spec.contains("- theirs"), "{spec}");
}

#[test]
fn serves_edited_on_main_is_not_reported_stale() {
    // req:sync.regenerate/stale-delta-diff@1b85fae3
    let (r, _, q) = x_modifies_p_and_q("It SHALL q.", "- mine");
    main_moves(&r, &[("specs/a/SPEC.md", &a_with_p_serving("- theirs"))]);

    let left = unfinished(&r);
    assert!(
        left.contains(&format!("main changed requirement:q of a since {q}")),
        "{left:?}"
    );
    assert!(
        left.iter()
            .any(|l| l.contains("apply/stale-base") && l.contains("`a/q`")),
        "{left:?}"
    );
    assert!(
        !left
            .iter()
            .any(|l| l.contains("requirement:p") || l.contains("`a/p`")),
        "{left:?}"
    );
}

#[test]
fn regenerated_builds_the_tree_without_touching_the_repository() {
    // req:sync.regenerate/regenerated-tree@e590dd2e
    let r = repo("spec/x");
    apply_change(&r, "x", "", &adds_x());
    let b = node_a("specd", "c", &["s"]).replace("title: A", "title: B");
    main_moves(&r, &[("specs/b/SPEC.md", &b)]);
    git(&r.work, &["fetch", "-q", "origin"]);
    let head = git(&r.work, &["rev-parse", "HEAD"]);
    let status = git(&r.work, &["status", "--porcelain"]);

    let onto = git(&r.work, &["rev-parse", "origin/main"]);
    let tree = match clm_regenerate::regenerated(&r.work, &onto, &["x".to_string()], false).unwrap()
    {
        clm_regenerate::Regenerated::Tree(tree) => tree,
        clm_regenerate::Regenerated::Stopped { left, .. } => panic!("{left:?}"),
    };
    assert!(tree_text(&tree, "specs/a/SPEC.md").contains("### Requirement: x"));
    assert!(tree_text(&tree, "specs/b/SPEC.md").contains("### Requirement: s"));
    assert_eq!(git(&r.work, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&r.work, &["status", "--porcelain"]), status);
}

#[test]
fn regenerated_merges_only_when_asked() {
    // req:sync.regenerate/regenerated-tree@e590dd2e
    let r = repo("spec/x");
    main_moves(
        &r,
        &[(
            "specs/a/SPEC.md",
            &node_a("specd", "one\ntwo\nthree", &["p"]),
        )],
    );
    pull(&r);
    let h1 = hash(&r, "contract");
    // Edits apart: `merge` would merge them.
    let delta = modifies_contract(&r, "ONE\ntwo\nthree");
    apply_change(&r, "x", "", &delta);
    main_moves(
        &r,
        &[(
            "specs/a/SPEC.md",
            &node_a("specd", "one\ntwo\nTHREE", &["p"]),
        )],
    );
    git(&r.work, &["fetch", "-q", "origin"]);
    let head = git(&r.work, &["rev-parse", "HEAD"]);
    let status = git(&r.work, &["status", "--porcelain"]);
    let onto = git(&r.work, &["rev-parse", "origin/main"]);
    let ids = ["x".to_string()];

    match clm_regenerate::regenerated(&r.work, &onto, &ids, false).unwrap() {
        clm_regenerate::Regenerated::Stopped { merged, left } => {
            assert!(merged.is_empty(), "{merged:?}");
            let stale = left.iter().position(|l| l.contains("apply/stale-base"));
            let report = left
                .iter()
                .position(|l| *l == format!("main changed contract of a since {h1}"));
            assert!(stale.is_some() && stale < report, "{left:?}");
        }
        clm_regenerate::Regenerated::Tree(_) => panic!("merged without being asked"),
    }
    assert_eq!(git(&r.work, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&r.work, &["status", "--porcelain"]), status);

    match clm_regenerate::regenerated(&r.work, &onto, &ids, true).unwrap() {
        clm_regenerate::Regenerated::Tree(tree) => {
            assert_eq!(
                tree.merged,
                ["merged contract of a into changes/x/deltas/a.md"]
            );
            let spec = tree_text(&tree, "specs/a/SPEC.md");
            assert!(spec.contains("ONE\ntwo\nTHREE"), "{spec}");
        }
        clm_regenerate::Regenerated::Stopped { left, .. } => panic!("{left:?}"),
    }
    assert_eq!(
        git(&r.work, &["log", "-1", "--format=%s"]),
        "Merge main into x"
    );
}

#[test]
fn regenerated_reads_a_clm_root_in_a_subdirectory() {
    // req:sync.regenerate/regenerated-tree@e590dd2e
    let r = repo("spec/x");
    let sub = r.work.join("sub");
    main_moves(
        &r,
        &[
            ("sub/specs/SPEC.md", ROOT),
            ("sub/specs/a/SPEC.md", &node_a("specd", "c", &["p"])),
        ],
    );
    git(&r.work, &["fetch", "-q", "origin"]);
    git(&r.work, &["merge", "-q", "--ff-only", "origin/main"]);
    write(
        &sub,
        "changes/x/change.md",
        "---\nstatus: proposed\n---\n\n# x\n",
    );
    write(&sub, "changes/x/deltas/a.md", &adds_x());
    clm_apply::apply(&sub, "x").unwrap();
    commit(&r.work, "Apply x");
    main_moves(
        &r,
        &[("sub/specs/a/SPEC.md", &node_a("specd", "c", &["p", "w"]))],
    );
    git(&r.work, &["fetch", "-q", "origin"]);

    let onto = git(&r.work, &["rev-parse", "origin/main"]);
    match clm_regenerate::regenerated(&sub, &onto, &["x".to_string()], false).unwrap() {
        clm_regenerate::Regenerated::Tree(tree) => {
            let spec = tree_text(&tree, "specs/a/SPEC.md");
            assert!(spec.contains("### Requirement: x"), "{spec}");
            assert!(spec.contains("### Requirement: w"), "{spec}");
        }
        clm_regenerate::Regenerated::Stopped { left, .. } => panic!("{left:?}"),
    }
}

#[test]
fn the_tree_holds_the_human_layers_and_decisions() {
    // req:sync.regenerate/regenerated-tree@e590dd2e
    let r = repo("spec/x");
    let serving_e = |slug: &str| {
        format!(
            "{}\n#### Serves\n- architecture/e\n",
            requirement(slug).trim_end_matches('\n')
        )
    };
    let arch = "## Constraints\n\n### Constraint: e\nIt SHALL e.\n\n#### Scenario: s\n- ok\n\n#### Untested\nwhy\n";
    write(&r.work, "ARCHITECTURE.md", arch);
    write(
        &r.work,
        "CAPABILITIES.md",
        "## Capabilities\n\n### Capability: c\nIt works.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n",
    );
    write(
        &r.work,
        "docs/decisions/d.md",
        "---\ntitle: \"D\"\ndate: 2026-11-02\nstatus: accepted\n---\n\nWe chose d.\n",
    );
    commit(&r.work, "human layers");
    let delta = |slug: &str| {
        format!(
            "---\ncomponent: a\n---\n## ADDED Requirements\n{}",
            serving_e(slug)
        )
    };
    write(
        &r.work,
        "changes/y/change.md",
        "---\nstatus: proposed\n---\n\n# y\n",
    );
    write(&r.work, "changes/y/deltas/a.md", &delta("z"));
    commit(&r.work, "propose y");
    apply_change(&r, "x", "", &delta("w"));
    git(&r.work, &["fetch", "-q", "origin"]);

    let onto = git(&r.work, &["rev-parse", "origin/main"]);
    let tree = match clm_regenerate::regenerated(&r.work, &onto, &["x".to_string()], false).unwrap()
    {
        clm_regenerate::Regenerated::Tree(tree) => tree,
        clm_regenerate::Regenerated::Stopped { left, .. } => panic!("{left:?}"),
    };
    assert_eq!(tree_text(&tree, "ARCHITECTURE.md"), arch);
    assert!(tree_text(&tree, "CAPABILITIES.md").contains("Capability: c"));
    assert!(tree_text(&tree, "docs/decisions/d.md").contains("We chose d."));
    let (graph, _) = clm_model::load(tree.path());
    assert!(graph.requirement("architecture/e").is_some());
    let change = &graph.changes["y"];
    let violations = clm_apply::validate(&graph, change);
    assert!(
        violations.iter().all(|v| v.rule != "apply/resulting-graph"),
        "{violations:?}"
    );
}

/// `regenerated(.., false)` on `origin/main` for `ids`, in the order given.
fn regenerate_ids(r: &Repo, ids: &[&str]) -> Regenerated {
    git(&r.work, &["fetch", "-q", "origin"]);
    let onto = git(&r.work, &["rev-parse", "origin/main"]);
    let ids: Vec<String> = ids.iter().map(|s| s.to_string()).collect();
    regenerated(&r.work, &onto, &ids, false).unwrap()
}

/// A delta of `a` that rewrites its Contract to `text`, with base `base`.
fn contract_delta(base: &str, text: &str) -> String {
    format!(
        "---\ncomponent: a\nbase: {{ contract: \"{base}\" }}\n---\n## MODIFIED Contract\n{text}\n"
    )
}

#[test]
fn a_change_built_on_another_sorts_first() {
    // req:sync.regenerate/regenerated-tree@e590dd2e
    let r = repo("spec/x");
    main_moves(&r, &[("specs/a/SPEC.md", &a_designed("one\ntwo\nthree"))]);
    pull(&r);
    let h1 = hash(&r, "design");
    apply_change(&r, "x", "", &design_delta(&h1, "X1\ntwo\nthree"));
    let h2 = hash(&r, "design");
    add_applied(&r, "w", &design_delta(&h2, "X1\nW2\nthree"));

    match regenerate_ids(&r, &["w", "x"]) {
        Regenerated::Tree(tree) => {
            let spec = tree_text(&tree, "specs/a/SPEC.md");
            assert!(spec.contains("## Design\nX1\nW2\nthree\n"), "{spec}");
        }
        Regenerated::Stopped { left, .. } => panic!("{left:?}"),
    }
}

#[test]
fn a_change_that_adds_what_another_modifies_goes_after_it() {
    // req:sync.regenerate/regenerated-tree@e590dd2e
    let r = repo("spec/x");
    apply_change(&r, "x", "", &adds_x());
    let base = hash(&r, "requirement:x");
    let delta = format!(
        "---\ncomponent: a\n---\n## MODIFIED Requirements\n\
         ### Requirement: x <!-- base: {base} -->\nIt SHALL x, reworked.\n\n#### Scenario: s\n- ok\n"
    );
    add_applied(&r, "w", &delta);

    match regenerate_ids(&r, &["w", "x"]) {
        Regenerated::Tree(tree) => {
            let spec = tree_text(&tree, "specs/a/SPEC.md");
            assert!(spec.contains("It SHALL x, reworked."), "{spec}");
        }
        Regenerated::Stopped { left, .. } => panic!("{left:?}"),
    }
}

#[test]
fn a_change_stale_against_main_goes_before_the_one_built_on_it() {
    // req:sync.regenerate/regenerated-tree@e590dd2e
    let r = repo("spec/x");
    main_moves(
        &r,
        &[(
            "specs/a/SPEC.md",
            &node_a("specd", "one\ntwo\nthree", &["p"]),
        )],
    );
    pull(&r);
    let h0 = hash(&r, "contract");
    apply_change(&r, "x", "", &contract_delta(&h0, "X1\ntwo\nthree"));
    let h2 = hash(&r, "contract");
    add_applied(&r, "w", &contract_delta(&h2, "X1\nW2\nthree"));
    main_moves(
        &r,
        &[(
            "specs/a/SPEC.md",
            &node_a("specd", "M1\ntwo\nthree", &["p"]),
        )],
    );

    match regenerate_ids(&r, &["w", "x"]) {
        Regenerated::Stopped { left, .. } => {
            assert!(
                left.iter()
                    .any(|l| l.contains("apply/stale-base") && l.contains("changes/x/deltas/a.md")),
                "{left:?}"
            );
            assert!(
                left.contains(&format!("main changed contract of a since {h0}")),
                "{left:?}"
            );
            assert!(!left.iter().any(|l| l.contains("changes/w/")), "{left:?}");
        }
        Regenerated::Tree(_) => panic!("x is stale against main"),
    }
}

#[test]
fn independent_changes_keep_id_order() {
    // req:sync.regenerate/regenerated-tree@e590dd2e
    let (r, h1, _) = folded_x_and_y();

    match regenerate_ids(&r, &["y", "x"]) {
        Regenerated::Stopped { left, .. } => {
            assert!(
                left.iter()
                    .any(|l| l.contains("apply/stale-base") && l.contains("changes/y/deltas/a.md")),
                "{left:?}"
            );
            assert!(
                left.contains(&format!("x changed design of a since {h1}")),
                "{left:?}"
            );
        }
        Regenerated::Tree(_) => panic!("y is stale after x"),
    }
}

#[test]
fn when_every_change_has_an_unknown_base_the_first_in_id_order_goes() {
    // req:sync.regenerate/regenerated-tree@e590dd2e
    let r = repo("spec/x");
    add_applied(&r, "w", &design_delta(&"0".repeat(64), "W1"));
    add_applied(&r, "x", &design_delta(&"1".repeat(64), "X1"));

    match regenerate_ids(&r, &["x", "w"]) {
        Regenerated::Stopped { left, .. } => {
            assert!(
                left.iter()
                    .any(|l| l.contains("apply/stale-base") && l.contains("changes/w/deltas/a.md")),
                "{left:?}"
            );
            assert!(!left.iter().any(|l| l.contains("changes/x/")), "{left:?}");
        }
        Regenerated::Tree(_) => panic!("both bases are unknown"),
    }
}
