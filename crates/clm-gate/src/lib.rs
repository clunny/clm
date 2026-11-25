//! Merge gate: decides whether `HEAD` may merge into a base ref (see specs/gate/SPEC.md).

use std::cell::OnceCell;
use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

pub use clm_model::Violation;

#[cfg(not(target_arch = "wasm32"))]
mod command;
mod cuts;
mod light;
mod policy;
mod reading;

pub use cuts::{
    Cut, ListLine, Side, TestList, cuts, cuts_or_unloaded, list_lines, places_a_test, sentences,
    stating_sentences, widened_exemptions,
};

#[cfg(not(target_arch = "wasm32"))]
pub use command::{Args, command};

pub use clm_fingerprint::{
    Entry, added_changes, adds_change, fingerprint, fingerprint_of, parse_ls_tree,
};
pub use policy::{approval_changed, needs_review_of, settings_unchanged, upgrade_stamp_only};
use policy::{change_needs_review, path_needs_review};
use reading::{AddedExemption, GitSide};
pub use reading::{FileReview, FileSide, review_from_files};

const SPEC_DIRS: [&str; 4] = ["specs/", "changes/", "backlog/", "docs/"];
const SPECS: &str = "specs/";
const CHANGES: &str = "changes/";
const ARCHIVE: &str = "changes/archive/";
const SPEC_FILES: [&str; 4] = [
    "ARCHITECTURE.md",
    "CAPABILITIES.md",
    "CLAUDE.md",
    "AGENTS.md",
];
/// The repository's settings, whose top-level `approval` key is the approval policy.
const POLICY: &str = "clm.toml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Spec,
    Code,
    /// Scenarios added to one component's requirements with the code that proves them
    /// (`light`).
    Light,
    Mixed,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Kind::Spec => "spec",
            Kind::Code => "code",
            Kind::Light => "light",
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
    /// Whether a `light` or `mixed` branch would need review were it a `spec` branch, by
    /// `needs-review`'s rule under the version applied; `review` for any other kind
    /// (`evaluate-at`).
    pub spec_part_review: bool,
    pub violations: Vec<Violation>,
    /// What a `spec` branch cuts (`cuts`), each printing as its `gate/cut` line; empty for
    /// any other kind.
    pub cuts: Vec<Cut>,
    /// The component whose work the branch is: a `light` branch's, and a `code` branch's
    /// that one component accounts for (`own-files`, `light`); `None` otherwise.
    pub component: Option<String>,
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
    evaluate_by(repo, base, head, approved, RULES)
}

/// The version of the rules by which `evaluate_at` decides a branch's kind and whether it
/// needs review (`rules-version`): 1 before `cuts` made a branch need review, 2 before a
/// changed outcome did, 3 before a sentence holding a placement phrase and not the word
/// `test` counted as stating, 4 before paths were read relative to the CLM root, 5 before a
/// placement phrase matched across a line break, 6 before the light route, 7 before a
/// shortened merge did, and 8 adds the shortened merge.
pub const RULES: u32 = 8;

/// `evaluate_at` as the gate decided under the rules of version `rules` (`rules-version`):
/// the reading of paths and the review, and with it `gate/approval`, depend on the version.
/// `Err` for a version this gate doesn't know, as for git failing.
pub fn evaluate_by(
    repo: &Path,
    base: &str,
    head: &str,
    approved: bool,
    rules: u32,
) -> Result<Outcome, String> {
    if !(1..=RULES).contains(&rules) {
        return Err(format!(
            "the gate knows no rules version {rules}: it knows 1 to RULES ({RULES})"
        ));
    }
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
    let root = Root::of(repo, rules)?;
    let base_specs = OnceCell::new();
    let at = At {
        repo,
        merge_base: &merge_base,
        head,
        root: &root,
        base_specs: &base_specs,
    };
    violations.extend(up_to_date(&at, base, &here)?);
    let changed = changed_files(repo, &root, &range)?;
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
                &[&at.object(&merge_base, from), &at.object(head, to)],
                true,
            )?)
    };
    let resolved = if deleted.iter().any(|f| f.starts_with("backlog/")) {
        let mut slugs = resolved_slugs(&at)?;
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
    let mut stamp_only = false;
    // IDs of changes archived in their own spec PR (`archive-in-spec-pr`): their
    // `change.md` is added under `changes/archive/<id>/` other than by an archive move.
    let mut new_archived = BTreeSet::new();
    // IDs of changes whose `change.md` an archive move takes into the archive.
    let mut moved = BTreeSet::new();
    for &(status, file) in &entries {
        if let Some(rest) = file.strip_prefix(ARCHIVE) {
            if status != 'A' {
                violations.push(Violation {
                    path: PathBuf::from(file),
                    rule: "gate/archive-immutable",
                    message: "archived changes are history and can't be modified or deleted".into(),
                });
            } else if is_archive_move(&format!("{CHANGES}{rest}"), file)? {
                if let Some(id) = rest
                    .strip_suffix("/change.md")
                    .filter(|id| !id.contains('/'))
                {
                    moved.insert(id);
                }
            } else {
                if archived_at_base(&at, rest)? {
                    violations.push(Violation {
                        path: PathBuf::from(file),
                        rule: "gate/archive-immutable",
                        message: "archived changes are history and can't be added to".into(),
                    });
                }
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
            stamp_only = true;
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

    let setting_changed = policy == Some(PolicyChange::Approval);
    // The kind is read from the files (`classify`): no code file is a spec branch, an empty
    // diff and one whose every file is left out included.
    let spec_change = !spec.is_empty();
    let mut kind = match (spec_change, code.is_empty()) {
        (_, true) => Kind::Spec,
        (false, false) => Kind::Code,
        (true, false) => Kind::Mixed,
    };
    let part = SpecPart {
        at: &at,
        entries: &entries,
        moved: &moved,
        replayed: replayed.as_deref(),
        setting_changed,
        rules,
    };

    let (mut review, mut spec_part_review, mut cut) = (false, false, Vec::new());
    let mut component = None;
    let mut why_not_light = None;
    match kind {
        Kind::Spec if spec_change => {
            let read = part.read(&mut violations)?;
            (review, cut) = (read.review, read.cuts);
            spec_part_review = review;
            violations.extend(undeclared_burdens(&at, &entries, &read.changes));
            violations.extend(exemptions_name_their_items(&at, &read.added)?);
        }
        // A `spec` branch with no spec change needs nothing (`needs-review`); its upgrade notes
        // still carry the burdens it files (`undeclared-burden`).
        Kind::Spec => {
            if stamp_only {
                let head = GitSide { at: &at, rev: head };
                let changes = reading::change_frontmatters(&head, &entries, &moved)?;
                violations.extend(undeclared_burdens(&at, &entries, &changes));
            }
        }
        Kind::Code => {
            let files = code_entries(&entries, &code);
            let (owned, found) = own_files(&at, &files)?;
            component = found;
            if component.is_none() {
                violations.extend(owned.into_iter().map(|f| Violation {
                    path: PathBuf::from(f.path),
                    rule: "gate/own-files",
                    message: format!(
                        "owned by {}; a code branch changes one component's files, the wiring and a capability test that component serves",
                        f.owner
                    ),
                }));
            }
        }
        Kind::Light | Kind::Mixed => {
            let mut scratch = Vec::new();
            let read = part.read(&mut scratch)?;
            spec_part_review = read.review;
            if rules >= 7 {
                let files = code_entries(&entries, &code);
                match light::judge(&at, &spec, &files, &new_archived, &read)? {
                    Ok(c) => {
                        kind = Kind::Light;
                        component = Some(c);
                    }
                    Err(why) => why_not_light = Some(why),
                }
            }
        }
    }

    match kind {
        Kind::Mixed => {
            let spec: Vec<&str> = spec.iter().map(|(_, f)| *f).collect();
            violations.push(Violation {
                path: here,
                rule: "gate/classify",
                message: format!(
                    "PR mixes spec changes ({}) with code changes ({}); split it into a spec PR and a code PR; a build edits no spec path: it leaves a requirement that needs a spec edit unbuilt and adds a `question` backlog item whose `blocks` names it{}",
                    spec.join(", "),
                    code.join(", "),
                    why_not_light
                        .map(|why| format!(
                            "; spec and code land together only on a light branch (gate/light): {why}"
                        ))
                        .unwrap_or_default()
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
        spec_part_review,
        violations,
        cuts: cut,
        component,
    })
}

/// What `evaluate_by` reads a branch's spec part by: the diff, the changes it adds, whether
/// `rename/replay` accepts it and how `clm.toml` changes, under rules version `rules`.
struct SpecPart<'a> {
    at: &'a At<'a>,
    entries: &'a [(char, &'a str)],
    /// The changes whose `change.md` an archive move takes into the archive.
    moved: &'a BTreeSet<&'a str>,
    replayed: Option<&'a str>,
    setting_changed: bool,
    rules: u32,
}

/// A branch's spec part, read as a `spec` branch's (`needs-review`, `cuts`).
pub(crate) struct ReadPart {
    /// Whether it needs review were the branch a `spec` branch.
    pub(crate) review: bool,
    pub(crate) cuts: Vec<Cut>,
    changes: Vec<Option<clm_model::ChangeFrontmatter>>,
    added: Vec<AddedExemption>,
    /// The spec graphs at the merge base and the head with their load violations, when the
    /// cuts were read from them.
    pub(crate) graphs: Option<Graphs>,
}

/// The spec graphs at the merge base and at the head, loaded as `cuts` loads them, each with
/// the violations its loading gave.
pub(crate) struct Graphs {
    pub(crate) base: (clm_model::Graph, Vec<Violation>),
    pub(crate) head: (clm_model::Graph, Vec<Violation>),
}

impl SpecPart<'_> {
    /// Reads the spec part, adding a `gate/deleted-tests` violation to `violations` for each
    /// list line that names no test (`cuts`).
    fn read(&self, violations: &mut Vec<Violation>) -> Result<ReadPart, String> {
        let SpecPart {
            at,
            entries,
            moved,
            replayed,
            setting_changed,
            rules,
        } = *self;
        // One reading of the branch's files (`review-from-files`): every path the diff
        // touches, and every change one of whose files it adds or modifies, a status-only
        // edit included, as the Worker passes them.
        let base = GitSide {
            at,
            rev: at.merge_base,
        };
        let head = GitSide { at, rev: at.head };
        let read = reading::read_branch(&base, &head, entries, moved, rules, true)?;
        violations.extend(read.deleted_tests.iter().cloned());
        let (changes, cut) = (&read.changes, &read.cuts);
        let paths: Vec<&str> = entries.iter().map(|&(_, f)| f).collect();
        // Under versions 1 to 7 a shortened merge makes no branch need review. Under versions
        // 2 to 5 a shortened requirement is counted as that version counted its sentences,
        // `shortened_then`, rather than by today's cut; version 1 counts none.
        let counted_before_v8 = |c: &Cut| !matches!(c, Cut::Merged { .. });
        let counted_before_v6 =
            |c: &Cut| counted_before_v8(c) && !matches!(c, Cut::Shortened { .. });
        // Read from the two graphs, or from the text when they don't load, under every
        // version, so the versions differ only by the cuts.
        let widened = read.widened.unwrap_or(!read.added.is_empty());
        let shortened_then = read.shortened_then;
        let by_paths = || needs_review_of(&paths, setting_changed, changes);
        let review = match replayed {
            // Its replayed edits to the human layers change no intent (`needs-review`).
            Some(text) => {
                records_term_rename(text)
                    || changes
                        .iter()
                        .any(|fm| fm.as_ref().is_none_or(change_needs_review))
            }
            // Version 1, before a cut needed review.
            None if rules == 1 => by_paths() || widened,
            // Version 2, before a changed outcome needed review.
            None if rules == 2 => {
                by_paths()
                    || widened
                    || shortened_then
                    || cut
                        .iter()
                        .any(|c| counted_before_v6(c) && !matches!(c, Cut::Outcome { .. }))
            }
            // Versions 3 to 5, before a placement phrase matched across a line; version 3
            // also before a sentence holding a phrase and no word `test` stated.
            None if rules < 6 => {
                by_paths() || widened || shortened_then || cut.iter().any(counted_before_v6)
            }
            // Versions 6 and 7, before a shortened merge needed review.
            None if rules < 8 => by_paths() || widened || cut.iter().any(counted_before_v8),
            // Relaxing a rule, or widening a relaxation, and a cut are always seen by a
            // human: the review `review_from_files` gives.
            None => read.review(by_paths()),
        };
        Ok(ReadPart {
            review,
            cuts: read.cuts,
            changes: read.changes,
            added: read.added,
            graphs: read.graphs,
        })
    }
}

/// The code files among `entries`, with their status, in the diff's order.
fn code_entries<'a>(entries: &[(char, &'a str)], code: &[&str]) -> Vec<(char, &'a str)> {
    entries
        .iter()
        .filter(|(_, f)| code.contains(f))
        .copied()
        .collect()
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
    let root = Root::of(repo, RULES)?;
    let mut files = Vec::new();
    for (status, path) in changed_files(repo, &root, &format!("{base}...{head}"))? {
        if is_spec_path(&path) {
            continue;
        }
        if path == POLICY {
            let base_specs = OnceCell::new();
            let at = At {
                repo,
                merge_base: &merge_base,
                head: &head,
                root: &root,
                base_specs: &base_specs,
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
    let Ok(text) = show(at.repo, &at.object(at.head, path)) else {
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
                && backlog_kind(at, at.head, file) == Some(clm_model::BacklogKind::Human)
        })
        .map(|(_, file)| Violation {
            path: PathBuf::from(file),
            rule: "gate/undeclared-burden",
            message: "a `kind: human-task` item is a step only the human can take: declare it in a `change.md`'s `human_burdens`".into(),
        })
        .collect()
}

/// `gate/exemption-names-its-item`: each added or widened exemption whose `until` names no
/// `backlog/<slug>.md` at `head`.
fn exemptions_name_their_items(
    at: &At,
    added: &[AddedExemption],
) -> Result<Vec<Violation>, String> {
    let mut out = Vec::new();
    for AddedExemption { file, rule, until } in added {
        let item = until
            .strip_prefix("backlog/")
            .filter(|slug| !slug.is_empty() && !slug.contains('/'))
            .map(|slug| format!("backlog/{slug}.md"));
        let exists = match item {
            Some(path) => !clm_git::git(at.repo, &["ls-tree", "--name-only", at.head, "--", &path])
                .map_err(|e| e.to_string())?
                .is_empty(),
            None => false,
        };
        if !exists {
            out.push(Violation {
                path: PathBuf::from(file),
                rule: "gate/exemption-names-its-item",
                message: format!(
                    "exemption from {rule} until {until} names no backlog item; add it"
                ),
            });
        }
    }
    Ok(out)
}

/// How the gate names the CLM root's files to git (`paths-under-the-root`): `prefix` is the
/// root's place in its repository, as `git rev-parse --show-prefix` gives it, empty at the
/// top, and `dot` says a file at a revision is read as `<rev>:./<path>`. Rules versions 1 to
/// 4 read paths as `git diff` prints them and `<rev>:<path>`: no prefix, no dot.
struct Root {
    prefix: String,
    dot: bool,
}

impl Root {
    /// The reading of rules version `rules` for the CLM root at `repo`.
    fn of(repo: &Path, rules: u32) -> Result<Root, String> {
        if rules < 5 {
            return Ok(Root {
                prefix: String::new(),
                dot: false,
            });
        }
        let prefix =
            clm_git::git(repo, &["rev-parse", "--show-prefix"]).map_err(|e| e.to_string())?;
        Ok(Root { prefix, dot: true })
    }

    /// The object naming the file at `path`, relative to the root, at `rev`.
    fn object(&self, rev: &str, path: &str) -> String {
        if self.dot && !self.prefix.is_empty() {
            format!("{rev}:./{path}")
        } else {
            format!("{rev}:{path}")
        }
    }

    /// A path as `git diff` prints it, relative to the repository's top, named relative to
    /// the root: under it, the prefix taken off; outside it, the shortest path from the root,
    /// climbing out with `../`.
    fn name_from_top(&self, path: &str) -> String {
        if let Some(rest) = path.strip_prefix(self.prefix.as_str()) {
            return rest.to_string();
        }
        let root: Vec<&str> = self.prefix.split('/').filter(|c| !c.is_empty()).collect();
        let parts: Vec<&str> = path.split('/').collect();
        let shared = root.iter().zip(&parts).take_while(|(a, b)| a == b).count();
        let mut out = "../".repeat(root.len() - shared);
        out.push_str(&parts[shared..].join("/"));
        out
    }
}

/// Whether a path, relative to the CLM root, names a file outside it.
fn outside_root(path: &str) -> bool {
    path.starts_with("../")
}

/// The repository and the two revisions the gate compares: the merge base and the head,
/// which is read only as a revision (`gate/evaluate-at`), and how it names the root's files.
#[derive(Clone, Copy)]
struct At<'a> {
    repo: &'a Path,
    merge_base: &'a str,
    head: &'a str,
    root: &'a Root,
    /// The specs at the merge base, loaded once when first read (`base_specs`).
    base_specs: &'a OnceCell<clm_model::Graph>,
}

impl At<'_> {
    /// The specs at the merge base, as `GitSide::load` loads them without the human layers,
    /// load violations ignored (`clm check` reports them), loaded once.
    fn base_specs(&self) -> Result<&clm_model::Graph, String> {
        if let Some(graph) = self.base_specs.get() {
            return Ok(graph);
        }
        let side = GitSide {
            at: self,
            rev: self.merge_base,
        };
        let graph = side.load(false)?.0;
        Ok(self.base_specs.get_or_init(|| graph))
    }

    /// The object naming the root's file at `path` at `rev`.
    fn object(&self, rev: &str, path: &str) -> String {
        self.root.object(rev, path)
    }
}

/// Whether the archived change holding `rest` (`<id>/<file>` under `changes/archive/`)
/// held a file at the merge base, so a file added to it adds to history (`archive-immutable`).
fn archived_at_base(at: &At, rest: &str) -> Result<bool, String> {
    let Some((id, _)) = rest.split_once('/') else {
        return Ok(false);
    };
    let dir = format!("{ARCHIVE}{id}/");
    let listed = clm_git::git(
        at.repo,
        &["ls-tree", "--name-only", at.merge_base, "--", &dir],
    )
    .map_err(|e| e.to_string())?;
    Ok(!listed.is_empty())
}

/// `(status, path)` for each file `git diff --name-status --no-renames <range>` lists, each
/// path named as `root` reads it.
fn changed_files(repo: &Path, root: &Root, range: &str) -> Result<Vec<(char, String)>, String> {
    // `-z` gives `<status>\0<path>\0` per file, with paths unquoted.
    let changed = clm_git::git(
        repo,
        &[
            "diff",
            "--name-status",
            "--no-renames",
            "--no-relative",
            "-z",
            range,
        ],
    )
    .map_err(|e| e.to_string())?;
    let fields: Vec<&str> = changed.split('\0').collect();
    Ok(fields
        .as_chunks::<2>()
        .0
        .iter()
        .filter_map(|[st, path]| Some((st.chars().next()?, root.name_from_top(path))))
        .collect())
}

/// The value of `key` in the frontmatter of the backlog file at `<rev>:<file>`, read alone
/// so that a key the model rejects can't hide it. `None` when the file isn't UTF-8, its
/// frontmatter doesn't parse, or `key` is missing.
fn backlog_key(at: &At, rev: &str, file: &str, key: &str) -> Option<serde_yaml::Value> {
    let text = show(at.repo, &at.object(rev, file))
        .ok()?
        .replace("\r\n", "\n");
    let (fm, _) = clm_model::split_frontmatter(&text)?;
    let fm: serde_yaml::Value = serde_yaml::from_str(fm).ok()?;
    fm.get(key).cloned()
}

/// The backlog item's `kind` at `<rev>:<file>`, read as `backlog_key` reads keys.
fn backlog_kind(at: &At, rev: &str, file: &str) -> Option<clm_model::BacklogKind> {
    serde_yaml::from_value(backlog_key(at, rev, file, "kind")?).ok()
}

/// The components a backlog item names at `<rev>:<file>`, from its `components` or, when it
/// sets that key instead, its `nodes` (`item-components`). Empty when it sets neither;
/// `None` when it sets both, which the model rejects, or its value isn't a list of strings.
fn backlog_components(at: &At, rev: &str, file: &str) -> Option<Vec<String>> {
    let components = backlog_key(at, rev, file, "components");
    let nodes = backlog_key(at, rev, file, "nodes");
    match (components, nodes) {
        (Some(_), Some(_)) => None,
        (Some(serde_yaml::Value::Null), None) | (None, Some(serde_yaml::Value::Null)) => {
            Some(Vec::new())
        }
        (Some(list), None) | (None, Some(list)) => serde_yaml::from_value(list).ok(),
        (None, None) => Some(Vec::new()),
    }
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
fn resolved_slugs(at: &At) -> Result<BTreeSet<String>, String> {
    let (repo, merge_base) = (at.repo, at.merge_base);
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
        let text = show(repo, &at.object(merge_base, file))?;
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
        ..
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
    let graph = at.base_specs()?;
    // The text of each code file the branch changes, at the merge base and at `HEAD`, read
    // once and only when a node's `paths` don't settle it.
    let mut texts: Option<Vec<Vec<u8>>> = None;
    for slug in named {
        let file = format!("backlog/{slug}.md");
        if !matches!(
            backlog_kind(at, merge_base, &file),
            Some(clm_model::BacklogKind::Bug | clm_model::BacklogKind::Deferred)
        ) {
            continue;
        }
        let Some(nodes) = backlog_components(at, merge_base, &file) else {
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
        root,
        ..
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
        "--no-relative",
        "-z",
        merge_base,
        head,
        "--",
    ];
    args.extend(globs.iter().map(String::as_str));
    let matched = clm_git::git(repo, &args).map_err(|e| e.to_string())?;
    Ok(matched.split('\0').any(|f| {
        let f = root.name_from_top(f);
        !f.is_empty() && !outside_root(&f) && !is_spec_path(&f)
    }))
}

/// The bytes of each code file in `entries` at `merge_base` and at `HEAD`, where it exists.
/// Read with one `git cat-file --batch`. A side that can't be read (missing, or a path
/// `--batch` can't name) holds no marker, so it is left out rather than failing.
fn code_texts(at: &At, entries: &[(char, &str)]) -> Vec<Vec<u8>> {
    let At {
        repo,
        merge_base,
        head,
        ..
    } = *at;
    let mut objects = String::new();
    for &(status, file) in entries
        .iter()
        .filter(|(_, f)| !is_spec_path(f) && !outside_root(f))
    {
        if file.contains('\n') {
            continue;
        }
        for (exists, rev) in [(status != 'A', merge_base), (status != 'D', head)] {
            if exists {
                objects.push_str(&at.object(rev, file));
                objects.push('\n');
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
        ..
    } = *at;
    let resolves_at = |rev: &str, path: &str| {
        show(repo, &at.object(rev, path))
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
            .then(|| show_bytes(at.repo, &at.object(rev, POLICY)))
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
        ..
    } = *at;
    let parse = |rev: &str| -> Option<(serde_yaml::Mapping, BTreeSet<String>, String)> {
        let text = show(repo, &at.object(rev, file)).ok()?;
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

/// `gate/up-to-date`: what `clm land`'s merge of `base` into `HEAD` can't settle
/// (`land-locally`): a textual conflict, or a file under `specs/` changed on both sides,
/// which is regenerated rather than merged (`regenerate-dont-merge`), one line per file in
/// path order. Merely being behind `base` is fine. `here` is the path a conflict is reported
/// at.
fn up_to_date(at: &At, base: &str, here: &Path) -> Result<Vec<Violation>, String> {
    let At {
        repo,
        merge_base,
        head,
        root,
        ..
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
                "--no-relative",
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
            .map(|p| root.name_from_top(p))
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

/// A code file a build isn't to change, with the component that owns it (`own-files`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignFile {
    pub path: String,
    pub owner: String,
}

/// The code files `head` changes against the merge base with `base` that aren't
/// `component`'s build's to change, sorted by path (`own-files`). Nothing when the root at
/// the merge base sets no `wiring` or its specs hold no `component`. `head` is read only as
/// a revision (`evaluate-at`).
pub fn foreign_files(
    repo: &Path,
    base: &str,
    head: &str,
    component: &str,
) -> Result<Vec<ForeignFile>, String> {
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
    let root = Root::of(repo, RULES)?;
    let base_specs = OnceCell::new();
    let at = At {
        repo,
        merge_base: &merge_base,
        head: &head,
        root: &root,
        base_specs: &base_specs,
    };
    let graph = at.base_specs()?;
    let Some(wiring) = graph.wiring() else {
        return Ok(Vec::new());
    };
    let changed = changed_files(repo, &root, &format!("{merge_base}...{head}"))?;
    let files: Vec<(char, &str)> = changed
        .iter()
        .filter(|(_, f)| !is_spec_path(f))
        .map(|(s, f)| (*s, f.as_str()))
        .collect();
    Ok(foreign_in(&at, graph, wiring, component, &files))
}

/// Of `files`, the code files a branch changes with their status, those that aren't
/// `component`'s build's to change by the specs `graph` at the merge base and the `wiring`
/// globs, sorted by path (`own-files`): a file outside the CLM root with an empty owner, and
/// each file another component owns that no `wiring` glob matches, unless the branch only
/// modifies it and its text at the merge base cites a capability `component` serves. Nothing
/// when `graph` holds no `component`.
pub(crate) fn foreign_in(
    at: &At,
    graph: &clm_model::Graph,
    wiring: &[String],
    component: &str,
    files: &[(char, &str)],
) -> Vec<ForeignFile> {
    let Some(node) = graph.nodes.get(component) else {
        return Vec::new();
    };
    let served: BTreeSet<&String> = node
        .requirements
        .iter()
        .flat_map(|r| graph.effective_serves(&r.id))
        .filter(|s| s.starts_with("capability/"))
        .collect();
    let mut found = Vec::new();
    for &(status, file) in files {
        // No component's, whatever a component's `paths` match (`paths-under-the-root`).
        if outside_root(file) {
            found.push(ForeignFile {
                path: file.to_string(),
                owner: String::new(),
            });
            continue;
        }
        let Some(owner) = clm_coverage::owner(graph, Path::new(file)) else {
            continue;
        };
        if owner == component || wiring.iter().any(|g| clm_coverage::matches(g, file)) {
            continue;
        }
        // A build may drop a detector's allowance its work made stale (`own-files`).
        if file == POLICY && status == 'M' {
            let side = |rev: &str| show(at.repo, &at.object(rev, file)).ok();
            if let (Some(before), Some(after)) = (side(at.merge_base), side(at.head))
                && allowances_dropped(&before, &after)
            {
                continue;
            }
        }
        if status == 'M' {
            let before = show_bytes(at.repo, &at.object(at.merge_base, file)).unwrap_or_default();
            let before = String::from_utf8_lossy(&before);
            if clm_coverage::marker_spans(&before)
                .iter()
                .any(|m| served.contains(&m.id))
            {
                continue;
            }
        }
        found.push(ForeignFile {
            path: file.to_string(),
            owner: owner.to_string(),
        });
    }
    // `git diff` lists paths from the repository's top, so a file named from the root with
    // `../` may come out of order.
    found.sort_by(|a, b| a.path.cmp(&b.path));
    found
}

/// Whether `after`, the root's `clm.toml` at the head, only drops detectors' allowances
/// from `before`, its text at the merge base (`own-files`): its lines, each with its line
/// ending, are `before`'s in order with one or more taken out, and both read as TOML, the
/// head's equal to the merge base's once one or more `"--allow"` elements, each with the
/// element after it, are taken out of the `command` arrays of its `[[detectors]]` tables.
fn allowances_dropped(before: &str, after: &str) -> bool {
    let lines = |t: &str| {
        t.split_inclusive('\n')
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    let (was, now) = (lines(before), lines(after));
    let mut kept = was.iter();
    if now.len() >= was.len() || !now.iter().all(|l| kept.any(|w| w == l)) {
        return false;
    }
    let (Ok(mut was), Ok(mut now)) = (before.parse::<toml::Table>(), after.parse::<toml::Table>())
    else {
        return false;
    };
    let (Some(toml::Value::Array(was_detectors)), Some(toml::Value::Array(now_detectors))) =
        (was.remove("detectors"), now.remove("detectors"))
    else {
        return false;
    };
    if was != now || was_detectors.len() != now_detectors.len() {
        return false;
    }
    let mut dropped = false;
    for (was, now) in was_detectors.iter().zip(&now_detectors) {
        let (Some(was), Some(now)) = (was.as_table(), now.as_table()) else {
            return false;
        };
        let (mut was, mut now) = (was.clone(), now.clone());
        let (Some(toml::Value::Array(was_command)), Some(toml::Value::Array(now_command))) =
            (was.remove("command"), now.remove("command"))
        else {
            if was == now {
                continue;
            }
            return false;
        };
        if was != now || !pairs_dropped(&was_command, &now_command) {
            return false;
        }
        dropped |= was_command.len() != now_command.len();
    }
    dropped
}

/// Whether `now` is `was` with zero or more `"--allow"` elements, each with the element after
/// it, taken out, any such choice counting.
fn pairs_dropped(was: &[toml::Value], now: &[toml::Value]) -> bool {
    match (was, now) {
        ([], []) => true,
        ([], _) => false,
        ([first, rest @ ..], _) => {
            (now.first() == Some(first) && pairs_dropped(rest, &now[1..]))
                || (first.as_str() == Some("--allow")
                    && !rest.is_empty()
                    && pairs_dropped(&rest[1..], now))
        }
    }
}

/// A `code` branch's component and the files a component owns outside the `wiring`, read
/// from its code `files` over the specs at the merge base (`own-files`): the component is
/// the first owner by ID of a file no `wiring` glob matches for which `foreign_in`, files
/// outside the CLM root left out, returns nothing. Nothing when the root at the merge base
/// sets no `wiring`, as when its `SPEC.md` doesn't load.
fn own_files(
    at: &At,
    files: &[(char, &str)],
) -> Result<(Vec<ForeignFile>, Option<String>), String> {
    let graph = at.base_specs()?;
    let Some(wiring) = graph.wiring() else {
        return Ok((Vec::new(), None));
    };
    let mut owned: Vec<ForeignFile> = files
        .iter()
        .filter(|(_, f)| !outside_root(f) && !wiring.iter().any(|g| clm_coverage::matches(g, f)))
        .filter_map(|(_, f)| {
            Some(ForeignFile {
                path: f.to_string(),
                owner: clm_coverage::owner(graph, Path::new(f))?.to_string(),
            })
        })
        .collect();
    owned.sort_by(|a, b| a.path.cmp(&b.path));
    let candidates: BTreeSet<&str> = owned.iter().map(|f| f.owner.as_str()).collect();
    let component = candidates
        .into_iter()
        .find(|c| {
            foreign_in(at, graph, wiring, c, files)
                .iter()
                .all(|f| f.owner.is_empty())
        })
        .map(str::to_string);
    Ok((owned, component))
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
        // req:gate/backlog-additions@1653493f
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
        let root = Root::of(d, RULES).unwrap();
        let base_specs = OnceCell::new();
        let at = At {
            repo: d,
            merge_base: &merge_base,
            head: "HEAD",
            root: &root,
            base_specs: &base_specs,
        };
        let slugs = resolved_slugs(&at).unwrap();
        assert_eq!(slugs, BTreeSet::from(["z".to_string()]));
    }

    #[test]
    fn a_path_is_named_from_the_root() {
        // req:gate/paths-under-the-root@6ac586a5
        let root = Root {
            prefix: "a/b/".into(),
            dot: true,
        };
        assert_eq!(root.name_from_top("a/b/specs/x.md"), "specs/x.md");
        assert_eq!(root.name_from_top("a/x"), "../x");
        assert_eq!(root.name_from_top("b"), "../../b");
        assert_eq!(root.name_from_top("c/d/e"), "../../c/d/e");
        // a sibling whose name begins as the root's does is outside it
        assert_eq!(root.name_from_top("a/bc/x"), "../bc/x");
        assert_eq!(root.object("r", "../x"), "r:./../x");
        let top = Root {
            prefix: String::new(),
            dot: true,
        };
        assert_eq!(top.name_from_top("specs/x.md"), "specs/x.md");
    }
}
