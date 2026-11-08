//! Merge gate: decides whether `HEAD` may merge into a base ref (see specs/gate/SPEC.md).

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

pub use clm_model::Violation;

mod policy;

pub use clm_fingerprint::{
    Entry, added_changes, adds_change, fingerprint, fingerprint_of, parse_ls_tree,
};
pub use policy::{approval_changed, needs_review_of, settings_unchanged, upgrade_stamp_only};
use policy::{change_needs_review, path_needs_review};

const SPEC_DIRS: [&str; 4] = ["specs/", "changes/", "backlog/", "docs/"];
const SPECS: &str = "specs/";
const CHANGES: &str = "changes/";
const ARCHIVE: &str = "changes/archive/";
const SPEC_FILES: [&str; 3] = ["ARCHITECTURE.md", "CAPABILITIES.md", "CLAUDE.md"];
/// The repository's settings, whose top-level `approval` key is the approval policy.
const POLICY: &str = "clm.toml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Spec,
    Code,
    Mixed,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Kind::Spec => "spec",
            Kind::Code => "code",
            Kind::Mixed => "mixed",
        })
    }
}

#[derive(Debug)]
pub struct Outcome {
    pub kind: Kind,
    /// Whether the branch needs a human's review (`needs-review`); false unless `kind` is
    /// `Spec`.
    pub review: bool,
    pub violations: Vec<Violation>,
}

pub fn is_spec_path(path: &str) -> bool {
    SPEC_DIRS.iter().any(|d| path.starts_with(d)) || SPEC_FILES.contains(&path)
}

/// Evaluates the gate for the revision `head` against `base` in the repo at `repo`, as
/// `clm gate --base <base>` does with `head` checked out (`gate/evaluate-at`): `head` is read
/// only as a revision, never through the working tree or the index. `approved` says the
/// ledger holds an approval of this diff that the policy accepts, as `clm land` decides.
/// `Err` means git itself failed (not a gate violation).
pub fn evaluate_at(repo: &Path, base: &str, head: &str, approved: bool) -> Result<Outcome, String> {
    let here = PathBuf::from(format!("{base}..{head}"));
    let mut violations = Vec::new();

    // Every read sees the one commit `head` names now, however the name moves meanwhile.
    let head = &clm_git::git(
        repo,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{head}^{{commit}}"),
        ],
    )
    .map_err(|e| e.to_string())?;
    let range = format!("{base}...{head}");
    let merge_base = clm_git::git(repo, &["merge-base", base, head]).map_err(|e| e.to_string())?;
    let at = At {
        repo,
        merge_base: &merge_base,
        head,
    };
    violations.extend(up_to_date(&at, base, &here)?);
    let changed = changed_files(repo, &range)?;
    let entries: Vec<(char, &str)> = changed.iter().map(|(s, p)| (*s, p.as_str())).collect();
    let with_status = |s: char| -> BTreeSet<&str> {
        entries
            .iter()
            .filter(|(st, _)| *st == s)
            .map(|(_, p)| *p)
            .collect()
    };
    let (added, deleted) = (with_status('A'), with_status('D'));
    // `changes/<id>/<f>` -> `changes/archive/<id>/<f>` with at most `status:` lines changed.
    let is_archive_move = |from: &str, to: &str| -> Result<bool, String> {
        Ok(deleted.contains(from)
            && added.contains(to)
            && status_only(
                repo,
                &[&format!("{merge_base}:{from}"), &format!("{head}:{to}")],
                true,
            )?)
    };
    let resolved = if deleted.iter().any(|f| f.starts_with("backlog/")) {
        let mut slugs = resolved_slugs(repo, &merge_base)?;
        slugs.extend(fixed_bugs(&at, &entries, &deleted)?);
        slugs
    } else {
        BTreeSet::new()
    };
    let resolved_delete = |status: char, file: &str| {
        status == 'D'
            && file
                .strip_prefix("backlog/")
                .and_then(|f| f.strip_suffix(".md"))
                .is_some_and(|slug| resolved.contains(slug))
    };

    let policy = match entries.iter().find(|(_, f)| *f == POLICY) {
        Some(&(status, _)) => Some(policy_change(&at, status)?),
        None => None,
    };
    let (mut spec, mut code) = (Vec::new(), Vec::new());
    let mut stamp_only = None;
    // IDs of changes archived in their own spec PR (`archive-in-spec-pr`): their
    // `change.md` is added under `changes/archive/<id>/` other than by an archive move.
    let mut new_archived = BTreeSet::new();
    for &(status, file) in &entries {
        if let Some(rest) = file.strip_prefix(ARCHIVE) {
            if status != 'A' {
                violations.push(Violation {
                    path: PathBuf::from(file),
                    rule: "gate/archive-immutable",
                    message: "archived changes are history and can't be modified or deleted".into(),
                });
            } else if !is_archive_move(&format!("{CHANGES}{rest}"), file)? {
                if let Some(id) = rest
                    .strip_suffix("/change.md")
                    .filter(|id| !id.contains('/'))
                {
                    new_archived.insert(id);
                }
                spec.push((status, file));
            }
        } else if let Some(rest) = file.strip_prefix(CHANGES).filter(|_| status == 'D')
            && is_archive_move(file, &format!("{ARCHIVE}{rest}"))?
        {
            // the deleting half of an archive move
        } else if file.starts_with("backlog/") && status == 'A' {
            // new backlog items may come from any PR (`fold-small-defer-large`)
        } else if status == 'M' && is_backlog_item(file) && blocks_only_added(&at, file)? {
            // a build recorded that a requirement waits on an existing item (`requirement-blocks`)
        } else if resolved_delete(status, file) {
            // listed in `resolves` by a change that a spec PR merged
        } else if file == POLICY && policy == Some(PolicyChange::Approval) {
            // whose approval counts changes only with an approval (`approval-setting-is-spec`)
            spec.push((status, file));
        } else if file == POLICY && policy == Some(PolicyChange::Stamp) {
            // left out of the classification (`upgrade-stamp-left-out`)
            stamp_only = Some((status, file));
        } else if !is_spec_path(file) {
            code.push(file);
        } else if path_needs_review(file)
            || !(status == 'M' && status_only(repo, &[&range, "--", file], false)?)
        {
            // A file that needs review never counts as a status-only edit.
            spec.push((status, file));
        }
    }

    for &(_, file) in entries.iter().filter(|(s, _)| matches!(s, 'A' | 'M')) {
        if let Some((id, "change.md")) = live_change(file) {
            violations.extend(unnamed_resolves(&at, file, file, id)?);
        }
    }
    // A new archived change is tested as a live one; its list at the merge base is that of
    // the live `changes/<id>/change.md` it may have been moved from.
    for id in &new_archived {
        let file = format!("{ARCHIVE}{id}/change.md");
        let before = format!("{CHANGES}{id}/change.md");
        violations.extend(unnamed_resolves(&at, &before, &file, id)?);
    }

    // A branch `rename/replay` accepts is a spec branch, its rewritten code files included
    // (`classify`): asked only of one that adds exactly one change that sets `renames`.
    let replayed = match rename_change(&at, &entries, &new_archived)? {
        Some(text) => clm_rename::replays(repo, &merge_base, head)
            .map_err(|e| e.to_string())?
            .then_some(text),
        None => None,
    };
    if replayed.is_some() {
        spec.extend(code.drain(..).map(|f| ('M', f)));
    }

    // Alone, the upgrade notes are a spec branch that needs no review.
    let setting_changed = policy == Some(PolicyChange::Approval);
    if let Some(entry) = stamp_only
        && spec.is_empty()
        && code.is_empty()
    {
        spec.push(entry);
    }

    let kind = match (spec.is_empty(), code.is_empty()) {
        (false, false) => Kind::Mixed,
        (false, true) => Kind::Spec,
        _ => Kind::Code,
    };

    let mut review = false;
    if kind == Kind::Spec {
        // Every path the diff touches, and every change one of whose files it adds or
        // modifies, a status-only edit included, as the Worker passes them.
        let changes = change_frontmatters(&at, &entries, &new_archived)?;
        let paths: Vec<&str> = entries.iter().map(|&(_, f)| f).collect();
        review = match &replayed {
            // Its replayed edits to the human layers change no intent (`needs-review`).
            Some(text) => {
                records_term_rename(text)
                    || changes
                        .iter()
                        .any(|fm| fm.as_ref().is_none_or(change_needs_review))
            }
            None => needs_review_of(&paths, setting_changed, &changes),
        };
        violations.extend(undeclared_burdens(&at, &entries, &changes));
    }

    match kind {
        Kind::Mixed => {
            let spec: Vec<&str> = spec.iter().map(|(_, f)| *f).collect();
            violations.push(Violation {
                path: here,
                rule: "gate/classify",
                message: format!(
                    "PR mixes spec changes ({}) with code changes ({}); split it into a spec PR and a code PR",
                    spec.join(", "),
                    code.join(", ")
                ),
            })
        }
        Kind::Spec if review && !approved => violations.push(Violation {
            path: here,
            rule: "gate/approval",
            message: "this branch needs a human's review: ask them to run `clm approve <branch>`"
                .to_string(),
        }),
        _ => {}
    }

    Ok(Outcome {
        kind,
        review,
        violations,
    })
}

/// The files `head` changes against `base` from their merge base, as `classify` reads them,
/// that aren't spec paths, sorted; none when `rename/replay` accepts `head`
/// (`gate/files-outside-spec-paths`). `head` is read only as a revision, as `evaluate_at` does.
pub fn files_outside_spec_paths(
    repo: &Path,
    base: &str,
    head: &str,
) -> Result<Vec<String>, String> {
    let head = clm_git::git(
        repo,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{head}^{{commit}}"),
        ],
    )
    .map_err(|e| e.to_string())?;
    let merge_base = clm_git::git(repo, &["merge-base", base, &head]).map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    for (status, path) in changed_files(repo, &format!("{base}...{head}"))? {
        if is_spec_path(&path) {
            continue;
        }
        if path == POLICY {
            let at = At {
                repo,
                merge_base: &merge_base,
                head: &head,
            };
            if policy_change(&at, status)? != PolicyChange::Other {
                continue;
            }
        }
        files.push(path);
    }
    files.sort();
    if !files.is_empty()
        && clm_rename::replays(repo, &merge_base, &head).map_err(|e| e.to_string())?
    {
        files.clear();
    }
    Ok(files)
}

/// The `change.md` text at `HEAD` of the one change the branch adds, when it is exactly one
/// and its frontmatter sets a non-empty `renames`: the only branch `rename/replay` can
/// accept, so no other pays for the replay.
fn rename_change(
    at: &At,
    entries: &[(char, &str)],
    new_archived: &BTreeSet<&str>,
) -> Result<Option<String>, String> {
    let mut added: Vec<String> = entries
        .iter()
        .filter(|&&(status, file)| {
            status == 'A' && live_change(file).is_some_and(|(_, rest)| rest == "change.md")
        })
        .map(|&(_, file)| file.to_string())
        .collect();
    added.extend(
        new_archived
            .iter()
            .map(|id| format!("{ARCHIVE}{id}/change.md")),
    );
    let [path] = added.as_slice() else {
        return Ok(None);
    };
    let Ok(text) = show(at.repo, &format!("{}:{path}", at.head)) else {
        return Ok(None);
    };
    Ok(renames_of(&text)
        .is_some_and(|r| !r.is_empty())
        .then_some(text))
}

/// The entries of the `renames` list in a `change.md`'s frontmatter, when it parses.
fn renames_of(text: &str) -> Option<Vec<serde_yaml::Value>> {
    let text = text.replace("\r\n", "\n");
    let (fm, _) = clm_model::split_frontmatter(&text)?;
    let fm: serde_yaml::Value = serde_yaml::from_str(fm).ok()?;
    Some(fm.get("renames")?.as_sequence()?.clone())
}

/// Whether a `change.md` records a `term` rename, which only a human can judge to keep a
/// rule's meaning (`needs-review`).
fn records_term_rename(text: &str) -> bool {
    renames_of(text).is_some_and(|r| r.iter().any(|entry| entry.get("term").is_some()))
}

/// The frontmatter at `HEAD` of the `change.md` of each change, live or new archived, one
/// of whose files is among the `entries` the branch adds or modifies; `None` for one that
/// is missing, isn't UTF-8 or whose frontmatter doesn't parse (`needs-review`).
fn change_frontmatters(
    at: &At,
    entries: &[(char, &str)],
    new_archived: &BTreeSet<&str>,
) -> Result<Vec<Option<clm_model::ChangeFrontmatter>>, String> {
    let dirs: BTreeSet<&str> = entries
        .iter()
        .filter(|(status, _)| matches!(status, 'A' | 'M'))
        .filter_map(|(_, file)| added_change(file, new_archived).map(|(dir, _, _)| dir))
        .collect();
    let mut out = Vec::new();
    for dir in dirs {
        let path = format!("{dir}change.md");
        // Listed in the tree, it is read, and a file git can't read fails the gate.
        let listed = clm_git::git(at.repo, &["ls-tree", "--name-only", at.head, "--", &path])
            .map_err(|e| e.to_string())?;
        let fm = if !listed.is_empty() {
            String::from_utf8(show_bytes(at.repo, &format!("{}:{path}", at.head))?)
                .ok()
                .and_then(|t| clm_model::parse_change_frontmatter(&t))
        } else {
            None
        };
        out.push(fm);
    }
    Ok(out)
}

/// `gate/undeclared-burden`: each `kind: human-task` backlog item (or its old word `human`)
/// the branch adds, when none of `changes` declares `human_burdens`.
fn undeclared_burdens(
    at: &At,
    entries: &[(char, &str)],
    changes: &[Option<clm_model::ChangeFrontmatter>],
) -> Vec<Violation> {
    if changes.iter().flatten().any(change_needs_review) {
        return Vec::new();
    }
    entries
        .iter()
        .filter(|&&(status, file)| {
            status == 'A'
                && is_backlog_item(file)
                && backlog_kind(at.repo, at.head, file) == Some(clm_model::BacklogKind::Human)
        })
        .map(|(_, file)| Violation {
            path: PathBuf::from(file),
            rule: "gate/undeclared-burden",
            message: "a `kind: human-task` item is a step only the human can take: declare it in a `change.md`'s `human_burdens`".into(),
        })
        .collect()
}

/// The repository and the two revisions the gate compares: the merge base and the head,
/// which is read only as a revision (`gate/evaluate-at`).
#[derive(Clone, Copy)]
struct At<'a> {
    repo: &'a Path,
    merge_base: &'a str,
    head: &'a str,
}

/// `(status, path)` for each file `git diff --name-status --no-renames <range>` lists.
fn changed_files(repo: &Path, range: &str) -> Result<Vec<(char, String)>, String> {
    // `-z` gives `<status>\0<path>\0` per file, with paths unquoted.
    let changed = clm_git::git(
        repo,
        &["diff", "--name-status", "--no-renames", "-z", range],
    )
    .map_err(|e| e.to_string())?;
    let fields: Vec<&str> = changed.split('\0').collect();
    Ok(fields
        .as_chunks::<2>()
        .0
        .iter()
        .filter_map(|[st, path]| Some((st.chars().next()?, path.to_string())))
        .collect())
}

/// `(dir, id, rest)` for a file at `<dir><rest>` of a change the PR adds: a live
/// `changes/<id>/`, or a new archived `changes/archive/<id>/` whose `id` is in
/// `new_archived` (`archive-in-spec-pr`).
fn added_change<'a>(
    file: &'a str,
    new_archived: &BTreeSet<&str>,
) -> Option<(&'a str, &'a str, &'a str)> {
    let (id, rest) = live_change(file).or_else(|| {
        file.strip_prefix(ARCHIVE)?
            .split_once('/')
            .filter(|(id, _)| new_archived.contains(id))
    })?;
    Some((&file[..file.len() - rest.len()], id, rest))
}

/// The value of `key` in the frontmatter of the backlog file at `<rev>:<file>`, read alone
/// so that a key the model rejects can't hide it. `None` when the file isn't UTF-8, its
/// frontmatter doesn't parse, or `key` is missing.
fn backlog_key(repo: &Path, rev: &str, file: &str, key: &str) -> Option<serde_yaml::Value> {
    let text = show(repo, &format!("{rev}:{file}"))
        .ok()?
        .replace("\r\n", "\n");
    let (fm, _) = clm_model::split_frontmatter(&text)?;
    let fm: serde_yaml::Value = serde_yaml::from_str(fm).ok()?;
    fm.get(key).cloned()
}

/// The backlog item's `kind` at `<rev>:<file>`, read as `backlog_key` reads keys.
fn backlog_kind(repo: &Path, rev: &str, file: &str) -> Option<clm_model::BacklogKind> {
    serde_yaml::from_value(backlog_key(repo, rev, file, "kind")?).ok()
}

/// The components a backlog item names at `<rev>:<file>`, from its `components` or, when it
/// sets that key instead, its `nodes` (`item-components`). Empty when it sets neither;
/// `None` when it sets both, which the model rejects, or its value isn't a list of strings.
fn backlog_components(repo: &Path, rev: &str, file: &str) -> Option<Vec<String>> {
    let components = backlog_key(repo, rev, file, "components");
    let nodes = backlog_key(repo, rev, file, "nodes");
    match (components, nodes) {
        (Some(_), Some(_)) => None,
        (Some(serde_yaml::Value::Null), None) | (None, Some(serde_yaml::Value::Null)) => {
            Some(Vec::new())
        }
        (Some(list), None) | (None, Some(list)) => serde_yaml::from_value(list).ok(),
        (None, None) => Some(Vec::new()),
    }
}

/// Loads `specs/` as committed at `rev`, with the `change.md` of each live change there, so
/// the node frontmatter keys they declare are accepted (`declared-keys-loaded`). Each file
/// holds what a checkout of `rev` writes (`base-specs-as-checkout`); load violations are
/// ignored (`clm check` reports them). A `rev` without `specs/` loads empty.
fn load_specs_at(repo: &Path, rev: &str) -> Result<clm_model::Graph, String> {
    let mut paths = Vec::new();
    // Only git's own refusal means "no `specs/`"; a git that can't run fails the gate.
    match clm_git::git(repo, &["cat-file", "-e", &format!("{rev}:specs")]) {
        Ok(_) => paths.push("specs".to_string()),
        Err(clm_git::Error::Failed { .. }) => {}
        Err(e) => return Err(e.to_string()),
    }
    // `extract_paths` takes its paths literally, so only files that exist are named: a path
    // that isn't UTF-8 can't be named exactly, and is left out rather than failing the gate.
    let listed = clm_git::run(
        repo,
        "git",
        &["ls-tree", "-r", "-z", "--name-only", rev, "--", CHANGES],
        Default::default(),
    )
    .map_err(|e| e.to_string())?;
    paths.extend(
        listed
            .split(|&b| b == 0)
            .filter_map(|f| std::str::from_utf8(f).ok())
            .filter(|f| live_change(f).is_some_and(|(_, rest)| rest == "change.md"))
            .map(str::to_string),
    );
    let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
    let tmp = clm_git::extract_paths(repo, rev, &paths).map_err(|e| e.to_string())?;
    Ok(clm_model::load(tmp.path()).0)
}

/// The file at `<rev>:<path>` as committed: untrimmed, and an error unless it is UTF-8.
fn show(repo: &Path, object: &str) -> Result<String, String> {
    String::from_utf8(show_bytes(repo, object)?).map_err(|e| format!("git show {object}: {e}"))
}

/// The bytes of the file at `<rev>:<path>` as committed.
fn show_bytes(repo: &Path, object: &str) -> Result<Vec<u8>, String> {
    clm_git::run(repo, "git", &["show", object], Default::default()).map_err(|e| e.to_string())
}

/// `(id, rest)` for a file at `changes/<id>/<rest>` outside the archive.
fn live_change(file: &str) -> Option<(&str, &str)> {
    file.strip_prefix(CHANGES)?
        .split_once('/')
        .filter(|(id, _)| *id != "archive")
}

/// Backlog slugs a branch may delete (`gate/backlog-additions`): those listed in `resolves`
/// by every change outside the archive at `merge_base`. Read at `merge_base`, never at
/// `HEAD`, so a branch can't list a slug and delete its item in one go. This covers the
/// changes the branch archives too: a clean archive move keeps `resolves`, and any other
/// `change.md` added to the archive is itself a spec change.
fn resolved_slugs(repo: &Path, merge_base: &str) -> Result<BTreeSet<String>, String> {
    let listed = clm_git::git(
        repo,
        &["ls-tree", "-r", "--name-only", merge_base, "--", CHANGES],
    )
    .map_err(|e| e.to_string())?;
    let mut resolved = BTreeSet::new();
    for file in listed
        .lines()
        .filter(|f| live_change(f).is_some_and(|(_, rest)| rest == "change.md"))
    {
        let text = show(repo, &format!("{merge_base}:{file}"))?;
        if let Some(fm) = clm_model::parse_change_frontmatter(&text) {
            resolved.extend(fm.resolves);
        }
    }
    Ok(resolved)
}

/// Backlog slugs a code fix may delete (`code-fix-resolves-bug`, and deferred work by
/// `deferred-work-resolves-in-code`): each named by a
/// `Resolves: backlog/<slug>` trailer between `merge_base` and `HEAD` whose item the branch
/// deletes, is of `kind: bug` or `kind: deferred` at `merge_base`, and names a node there
/// that a code file the branch adds, modifies or deletes belongs to: one of the node's `paths` matches it, as a
/// `:(glob)` pathspec, or its text at `merge_base` or `HEAD` holds `req:<node>/`
/// (`fixes-count-where-cited`). Only `kind` and `nodes` are read, through `backlog_key`;
/// an item whose frontmatter doesn't parse isn't exempt.
fn fixed_bugs(
    at: &At,
    entries: &[(char, &str)],
    deleted: &BTreeSet<&str>,
) -> Result<BTreeSet<String>, String> {
    let At {
        repo,
        merge_base,
        head,
    } = *at;
    let trailers = clm_git::git(
        repo,
        &[
            "log",
            "--format=%(trailers:key=Resolves,valueonly)",
            &format!("{merge_base}..{head}"),
        ],
    )
    .map_err(|e| e.to_string())?;
    let named: BTreeSet<&str> = trailers
        .lines()
        .filter_map(|v| v.trim().strip_prefix("backlog/"))
        .filter(|slug| {
            let file = format!("backlog/{slug}.md");
            is_backlog_item(&file) && deleted.contains(file.as_str())
        })
        .collect();
    let mut fixed = BTreeSet::new();
    if named.is_empty() {
        return Ok(fixed);
    }
    let graph = load_specs_at(repo, merge_base)?;
    // The text of each code file the branch changes, at the merge base and at `HEAD`, read
    // once and only when a node's `paths` don't settle it.
    let mut texts: Option<Vec<Vec<u8>>> = None;
    for slug in named {
        let file = format!("backlog/{slug}.md");
        if !matches!(
            backlog_kind(repo, merge_base, &file),
            Some(clm_model::BacklogKind::Bug | clm_model::BacklogKind::Deferred)
        ) {
            continue;
        }
        let Some(nodes) = backlog_components(repo, merge_base, &file) else {
            continue;
        };
        let nodes: Vec<&clm_model::Node> =
            nodes.iter().filter_map(|id| graph.nodes.get(id)).collect();
        if nodes.is_empty() {
            continue;
        }
        if paths_match(at, &nodes)? {
            fixed.insert(slug.to_string());
            continue;
        }
        let texts = match &mut texts {
            Some(texts) => texts,
            None => texts.insert(code_texts(at, entries)),
        };
        let cited = nodes.iter().any(|n| {
            let marker = format!("req:{}/", n.id);
            texts
                .iter()
                .any(|t| t.windows(marker.len()).any(|w| w == marker.as_bytes()))
        });
        if cited {
            fixed.insert(slug.to_string());
        }
    }
    Ok(fixed)
}

/// Whether one of `nodes`' `paths`, as a `:(glob)` pathspec, matches a code file the
/// branch adds, modifies or deletes.
fn paths_match(at: &At, nodes: &[&clm_model::Node]) -> Result<bool, String> {
    let At {
        repo,
        merge_base,
        head,
    } = *at;
    let globs: Vec<String> = nodes
        .iter()
        .flat_map(|n| &n.frontmatter.paths)
        .map(|p| format!(":(glob){p}"))
        .collect();
    if globs.is_empty() {
        return Ok(false);
    }
    let mut args = vec![
        "diff",
        "--name-only",
        "--no-renames",
        "-z",
        merge_base,
        head,
        "--",
    ];
    args.extend(globs.iter().map(String::as_str));
    let matched = clm_git::git(repo, &args).map_err(|e| e.to_string())?;
    Ok(matched
        .split('\0')
        .any(|f| !f.is_empty() && !is_spec_path(f)))
}

/// The bytes of each code file in `entries` at `merge_base` and at `HEAD`, where it exists.
/// Read with one `git cat-file --batch`. A side that can't be read (missing, or a path
/// `--batch` can't name) holds no marker, so it is left out rather than failing.
fn code_texts(at: &At, entries: &[(char, &str)]) -> Vec<Vec<u8>> {
    let At {
        repo,
        merge_base,
        head,
    } = *at;
    let mut objects = String::new();
    for &(status, file) in entries.iter().filter(|(_, f)| !is_spec_path(f)) {
        if file.contains('\n') {
            continue;
        }
        for (exists, rev) in [(status != 'A', merge_base), (status != 'D', head)] {
            if exists {
                objects.push_str(&format!("{rev}:{file}\n"));
            }
        }
    }
    if objects.is_empty() {
        return Vec::new();
    }
    let options = clm_git::Options {
        stdin: Some(objects.as_bytes()),
        ..Default::default()
    };
    let Ok(out) = clm_git::run(repo, "git", &["cat-file", "--batch"], options) else {
        return Vec::new();
    };
    // `<id> <type> <size>\n<contents>\n` per object, or `<object> missing\n`.
    let mut texts = Vec::new();
    let mut rest = out.as_slice();
    while let Some(eol) = rest.iter().position(|&b| b == b'\n') {
        let header = String::from_utf8_lossy(&rest[..eol]).into_owned();
        rest = &rest[eol + 1..];
        let mut fields = header.split(' ');
        let size = match (fields.next(), fields.next(), fields.next(), fields.next()) {
            (Some(_), Some(_), Some(size), None) => size.parse::<usize>().ok(),
            _ => None,
        };
        // A `missing` or `ambiguous` line has no contents.
        let Some(size) = size.filter(|&s| s <= rest.len()) else {
            continue;
        };
        texts.push(rest[..size].to_vec());
        rest = rest.get(size + 1..).unwrap_or_default();
    }
    texts
}

/// `gate/resolves-named-items`: each `resolves` entry this branch adds to the `change.md`
/// at `file` (of change `id`), compared with the one at `before` at the merge base, must
/// name a backlog item at `merge_base` or at `HEAD`. Entries already listed at `merge_base`
/// aren't tested, so a task may delete an item its change resolves
/// (`task-deletes-fixed-items`). Parsed lists are compared, not diff lines, so rewrapping a
/// list reports only what it adds. An unparseable `change.md` is the model's to report.
fn unnamed_resolves(at: &At, before: &str, file: &str, id: &str) -> Result<Vec<Violation>, String> {
    let At {
        repo,
        merge_base,
        head,
    } = *at;
    let resolves_at = |rev: &str, path: &str| {
        show(repo, &format!("{rev}:{path}"))
            .ok()
            .and_then(|text| clm_model::parse_change_frontmatter(&text))
            .map(|fm| fm.resolves)
    };
    let Some(ours) = resolves_at(head, file) else {
        return Ok(Vec::new());
    };
    let theirs: BTreeSet<String> = resolves_at(merge_base, before)
        .unwrap_or_default()
        .into_iter()
        .collect();
    let added: BTreeSet<String> = ours.into_iter().filter(|s| !theirs.contains(s)).collect();
    if added.is_empty() {
        return Ok(Vec::new());
    }
    // Top-level `backlog/<slug>.md` at either rev; nested files aren't items.
    let mut items = BTreeSet::new();
    for rev in [merge_base, head] {
        // `-z` leaves names unquoted whatever `core.quotePath` says.
        let listed = clm_git::git(
            repo,
            &["ls-tree", "-z", "--name-only", rev, "--", "backlog/"],
        )
        .map_err(|e| e.to_string())?;
        items.extend(listed.split('\0').map(str::to_string));
    }
    let mut violations = Vec::new();
    for slug in added {
        if !items.contains(&format!("backlog/{slug}.md")) {
            violations.push(Violation {
                path: PathBuf::from(file),
                rule: "gate/resolves-named-items",
                message: format!(
                    "change `{id}` resolves `{slug}`, which isn't in backlog/ at the merge base or at HEAD"
                ),
            });
        }
    }
    Ok(violations)
}

/// How a branch, whose diff lists `clm.toml`, changes it.
#[derive(PartialEq)]
enum PolicyChange {
    /// Its top-level `approval` changes, or a side can't be read (`gate/approval-setting`).
    Approval,
    /// Only the upgrade notes change (`gate/upgrade-stamp-left-out`).
    Stamp,
    /// Anything else.
    Other,
}

/// How the branch, whose diff lists `clm.toml` with `status`, changes it, decided by
/// `approval_changed` and `upgrade_stamp_only` from the file's bytes at the merge base and
/// at the head.
fn policy_change(at: &At, status: char) -> Result<PolicyChange, String> {
    let side = |exists: bool, rev: &str| -> Result<Option<Vec<u8>>, String> {
        exists
            .then(|| show_bytes(at.repo, &format!("{rev}:{POLICY}")))
            .transpose()
    };
    let before = side(status != 'A', at.merge_base)?;
    let after = side(status != 'D', at.head)?;
    let (before, after) = (before.as_deref(), after.as_deref());
    Ok(if approval_changed(before, after) {
        PolicyChange::Approval
    } else if upgrade_stamp_only(before, after) {
        PolicyChange::Stamp
    } else {
        PolicyChange::Other
    })
}

/// A top-level `backlog/<slug>.md`.
fn is_backlog_item(file: &str) -> bool {
    file.strip_prefix("backlog/")
        .and_then(|f| f.strip_suffix(".md"))
        .is_some_and(|slug| !slug.is_empty() && !slug.contains('/'))
}

/// Whether the only effect of this branch's change to the backlog item at `file` is to add
/// entries to its `blocks` (`gate/backlog-additions`, `requirement-blocks`): the body and
/// every other frontmatter key are the same at `merge_base` and `HEAD`, and the set of
/// `HEAD`'s `blocks` is a strict superset of the merge base's. Frontmatters are compared
/// parsed, so reformatting a list doesn't count as an edit; an unparseable one is an
/// ordinary modification, which the model reports.
fn blocks_only_added(at: &At, file: &str) -> Result<bool, String> {
    let At {
        repo,
        merge_base,
        head,
    } = *at;
    let parse = |rev: &str| -> Option<(serde_yaml::Mapping, BTreeSet<String>, String)> {
        let text = show(repo, &format!("{rev}:{file}")).ok()?;
        let text = text.replace("\r\n", "\n");
        let (fm, body) = clm_model::split_frontmatter(&text)?;
        let mut fm: serde_yaml::Mapping = serde_yaml::from_str(fm).ok()?;
        let blocks = match fm.remove("blocks") {
            None => BTreeSet::new(),
            Some(serde_yaml::Value::Sequence(items)) => items
                .into_iter()
                .map(|v| v.as_str().map(str::to_string))
                .collect::<Option<BTreeSet<_>>>()?,
            Some(_) => return None,
        };
        Some((fm, blocks, body.to_string()))
    };
    let (Some(before), Some(after)) = (parse(merge_base), parse(head)) else {
        return Ok(false);
    };
    let (before_fm, before_blocks, before_body) = before;
    let (after_fm, after_blocks, after_body) = after;
    Ok(before_fm == after_fm
        && before_body == after_body
        && before_blocks.is_subset(&after_blocks)
        && before_blocks != after_blocks)
}

/// True if every line `git diff -U0 <diff_args>` changes is a `status:` line. An empty diff
/// counts only when `allow_identical`.
fn status_only(repo: &Path, diff_args: &[&str], allow_identical: bool) -> Result<bool, String> {
    let mut args = vec!["diff", "-U0"];
    args.extend_from_slice(diff_args);
    let diff = clm_git::git(repo, &args).map_err(|e| e.to_string())?;
    // Only lines inside hunks; the `--- a/` / `+++ b/` headers come before the first `@@`.
    let mut changed = diff
        .lines()
        .skip_while(|l| !l.starts_with("@@"))
        .filter(|l| l.starts_with('+') || l.starts_with('-'))
        .peekable();
    Ok((allow_identical || changed.peek().is_some())
        && changed.all(|l| l[1..].starts_with("status: ")))
}

/// `gate/up-to-date`: what CI's merge of `base` into `HEAD` can't settle (`merge-tested-tree`):
/// a textual conflict, or a file under `specs/` changed on both sides, which is regenerated
/// rather than merged (`regenerate-dont-merge`). Merely being behind `base` is fine.
/// `here` is the path a conflict is reported at.
fn up_to_date(at: &At, base: &str, here: &Path) -> Result<Vec<Violation>, String> {
    let At {
        repo,
        merge_base,
        head,
    } = *at;
    let mut violations = Vec::new();
    match clm_git::git(repo, &["merge-tree", "--write-tree", base, head]) {
        Ok(_) => {}
        Err(clm_git::Error::Failed { status, .. }) if status.code() == Some(1) => {
            violations.push(Violation {
                path: here.to_path_buf(),
                rule: "gate/up-to-date",
                message: format!("conflicts with `{base}`; run `clm rebase`"),
            })
        }
        Err(e) => return Err(e.to_string()),
    }
    let specs_changed = |rev: &str| -> Result<BTreeSet<String>, String> {
        let names = clm_git::git(
            repo,
            &[
                "diff",
                "--name-only",
                "--no-renames",
                "-z",
                merge_base,
                rev,
                "--",
                SPECS,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(names
            .split('\0')
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect())
    };
    let ours = specs_changed(head)?;
    for path in specs_changed(base)?.intersection(&ours) {
        violations.push(Violation {
            path: PathBuf::from(path),
            rule: "gate/up-to-date",
            message: format!(
                "`{path}` changed on both this branch and `{base}`; run `clm rebase` to regenerate it"
            ),
        });
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(repo: &Path, files: &[(&str, &str)]) {
        for (rel, text) in files {
            let path = repo.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        let id = ["-c", "user.name=t", "-c", "user.email=t@t"];
        clm_git::git(repo, &[&id[..], &["add", "-A"]].concat()).unwrap();
        clm_git::git(repo, &[&id[..], &["commit", "-qm", "c"]].concat()).unwrap();
    }

    #[test]
    fn resolves_are_read_at_the_merge_base() {
        // req:gate/backlog-additions@85a57cc9
        let dir = tempfile::TempDir::new().unwrap();
        let d = dir.path();
        clm_git::git(d, &["init", "-q", "-b", "main"]).unwrap();
        let change = |slugs: &str| format!("---\nstatus: applied\nresolves: [{slugs}]\n---\n");
        commit(
            d,
            &[
                ("changes/x/change.md", &change("")),
                ("changes/z/change.md", &change("z")),
                ("changes/archive/old/change.md", &change("y")),
            ],
        );
        let merge_base = clm_git::git(d, &["rev-parse", "HEAD"]).unwrap();
        commit(d, &[("changes/x/change.md", &change("x"))]);
        let slugs = resolved_slugs(d, &merge_base).unwrap();
        assert_eq!(slugs, BTreeSet::from(["z".to_string()]));
    }
}
