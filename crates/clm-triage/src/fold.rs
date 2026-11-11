//! `clm draft --fold <id>...` (`yes-no-review`): writes the prompt for one
//! session that folds the changes needing review that a triage run's drafts left into one spec branch
//! with one pull request, so the human approves them once, and prints the launcher that names it.
//! It takes no place, claims nothing, pushes nothing, creates no branch or worktree and
//! starts no session.

use std::fs;
use std::path::{Path, PathBuf};

use clm_git::git;
use clm_model::{ChangeStatus, parse_change_frontmatter, split_frontmatter};
use clm_orchestrator::REMOTE;

use crate::draft::prompts_dir;
use crate::{Error, GATE, PR_BODY, fetch_origin};

/// Where `clm draft --fold` writes its prompts, one file per first folded change.
const PROMPTS: &str = ".clm/prompts/fold";

/// What `clm draft --fold` reports: each change it left with the reason, and the launcher
/// for the prompt it wrote, `None` when nothing was left to fold.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Fold {
    pub left: Vec<(String, String)>,
    pub launcher: Option<String>,
}

/// One change to fold: its ID, the items it resolves and whether it is a rename change,
/// which `clm rename` leaves archived on its own branch (`fold-rename-alone`).
struct Folded {
    id: String,
    resolves: Vec<String>,
    rename: bool,
}

impl Folded {
    /// Where the change's `change.md` is on its branch.
    fn path(&self) -> String {
        if self.rename {
            format!("changes/archive/{}/change.md", self.id)
        } else {
            format!("changes/{}/change.md", self.id)
        }
    }
}

/// Fetches `origin`, reads each change `ids` names from `origin/spec/<id>`, in the order
/// given, leaving one that isn't there or isn't `applied`, and writes the fold prompt for
/// the rest to `<root>/.clm/prompts/fold/<first folded id>.md`.
pub fn fold(root: &Path, ids: &[String]) -> Result<Fold, Error> {
    fetch_origin(root)?;
    let mut left = Vec::new();
    let mut folded: Vec<Folded> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for id in ids.iter().filter(|id| seen.insert(id.as_str())) {
        match read_change(root, id) {
            Ok((resolves, rename)) => folded.push(Folded {
                id: id.clone(),
                resolves,
                rename,
            }),
            Err(why) => left.push((id.clone(), why)),
        }
    }
    if folded.len() > 1 {
        // A rename change folds alone: the gate classifies its branch by replaying it
        // (`fold-rename-alone`, `rename-by-replay`).
        let (renames, rest): (Vec<Folded>, Vec<Folded>) =
            folded.into_iter().partition(|f| f.rename);
        for f in renames {
            let why = format!(
                "a rename change folds alone (rename-by-replay): run clm draft --fold {}",
                f.id
            );
            left.push((f.id, why));
        }
        folded = rest;
    }
    let Some(first) = folded.first() else {
        return Ok(Fold {
            left,
            launcher: None,
        });
    };
    let dir = prompts_dir(root, PROMPTS)?;
    let file: PathBuf = dir.join(format!("{}.md", first.id));
    let text = if let [one] = folded.as_slice() {
        prompt_one(one)
    } else {
        prompt_many(&folded, &batch(root, &first.id))
    };
    fs::write(&file, text).map_err(|e| Error::Refused(format!("{}: {e}", file.display())))?;
    let ids: Vec<String> = folded.iter().map(|f| format!("`{}`", f.id)).collect();
    let launcher = crate::launcher(&file, &format!("folding {}", ids.join(", ")));
    Ok(Fold {
        left,
        launcher: Some(launcher),
    })
}

/// The items change `id` on `origin/spec/<id>` resolves, and whether it is a rename change,
/// or why it can't be folded.
fn read_change(root: &Path, id: &str) -> Result<(Vec<String>, bool), String> {
    if id.is_empty() || id.contains('/') || id.starts_with('-') {
        return Err(format!("`{id}` isn't a change ID"));
    }
    let branch = format!("refs/remotes/{REMOTE}/spec/{id}");
    if git(root, &["rev-parse", "--verify", "-q", &branch]).is_err() {
        return Err(format!("`{REMOTE}` has no `spec/{id}`"));
    }
    let path = format!("changes/{id}/change.md");
    let Ok(text) = git(root, &["show", &format!("{branch}:{path}")]) else {
        let archived = format!("changes/archive/{id}/change.md");
        return git(root, &["show", &format!("{branch}:{archived}")])
            .ok()
            .and_then(|text| rename_change(&text))
            .map(|resolves| (resolves, true))
            .ok_or_else(|| format!("`{REMOTE}/spec/{id}` has no `{path}`"));
    };
    let fm = parse_change_frontmatter(&text)
        .ok_or_else(|| format!("the frontmatter of `{path}` doesn't parse"))?;
    if fm.status != ChangeStatus::Applied {
        return Err(format!(
            "its status is `{}`, not `applied`, and `clm rebase` re-applies only applied \
             changes",
            format!("{:?}", fm.status).to_lowercase()
        ));
    }
    Ok((fm.resolves, false))
}

/// The items an archived `change.md` resolves when it is a rename change: `status: done`
/// and a `renames` list of at least one entry, whose shape is left to `clm rename`.
fn rename_change(text: &str) -> Option<Vec<String>> {
    let text = text.replace("\r\n", "\n");
    let (fm, _) = split_frontmatter(&text)?;
    let fm: serde_yaml::Value = serde_yaml::from_str(fm).ok()?;
    if fm.get("status")?.as_str()? != "done" || fm.get("renames")?.as_sequence()?.is_empty() {
        return None;
    }
    match fm.get("resolves") {
        Some(list) => list
            .as_sequence()?
            .iter()
            .map(|s| s.as_str().map(str::to_string))
            .collect(),
        None => Some(Vec::new()),
    }
}

/// The fold's branch name: `<first>-fold`, with `-2`, `-3` and so on added while `origin`
/// has that `spec/` or `autosync/spec/` branch or the clone has that local `spec/` branch.
fn batch(root: &Path, first: &str) -> String {
    let exists = |rf: String| git(root, &["rev-parse", "--verify", "-q", &rf]).is_ok();
    let base = format!("{first}-fold");
    (1..)
        .map(|n| {
            if n == 1 {
                base.clone()
            } else {
                format!("{base}-{n}")
            }
        })
        .find(|name| {
            !exists(format!("refs/remotes/{REMOTE}/spec/{name}"))
                && !exists(format!("refs/remotes/{REMOTE}/autosync/spec/{name}"))
                && !exists(format!("refs/heads/spec/{name}"))
        })
        .expect("some suffix is free")
}

/// The list of the folded changes, each with its branch and the items it resolves.
fn changes(folded: &[Folded]) -> String {
    folded
        .iter()
        .map(|f| {
            let resolves: Vec<String> = f.resolves.iter().map(|s| format!("`{s}`")).collect();
            format!(
                "- `{id}`, on `origin/spec/{id}`, resolving [{}]\n",
                resolves.join(", "),
                id = f.id
            )
        })
        .collect()
}

/// What every fold prompt says about running unattended.
const UNATTENDED: &str = "You are running unattended: no one will answer questions in this \
    conversation. When a choice comes up, decide it by this repository's rules (`CLAUDE.md`, \
    `ARCHITECTURE.md`, `docs/decisions/`) and keep going. Never approve or review a change \
    yourself, and never push `main`: the triage session lands this branch after the human's \
    approval. Claude Code may ask the human to confirm a tool call: wait for that. When \
    Claude Code refuses a write, don't write it another way, such as through Bash: stop and \
    say why. A subagent you launch names its model: set the Agent tool's `model` to \
    `sonnet` (`model: \"sonnet\"`), the model builds run on (`model-per-launch`); never \
    launch a subagent without naming its model. Keep the scratch files you write while \
    working under this worktree's `.clm/`, which git ignores, and never in a scratchpad or \
    temporary directory your system prompt names or any other directory shared with other \
    sessions: the drafts and builds launched beside you are given the same directory and \
    would overwrite one another's files.";

/// Read `clm rebase` by its last line, as everywhere in these prompts.
const REBASE: &str = "run `clm rebase` and resolve what it reports (conflicted files, stale \
    deltas, check failures), committing each fix, until its last line is `rebase ok`";

/// The fold prompt for one change: nothing to merge, only its pull request.
fn prompt_one(f: &Folded) -> String {
    let id = &f.id;
    format!(
        "You are opening the pull request for one CLM spec change that needs review, unattended, \
         which a triage run's draft left on its branch, so the human can approve it \
         (`yes-no-review`). There is nothing to merge.\n\n\
         ## Change\n\n{changes}\n\
         ## Steps\n\n\
         First check `git branch --show-current`. Then read the change from its branch: \
         `git show origin/spec/{id}:{path}`, and its diff with \
         `git diff origin/main...origin/spec/{id}`.\n\n\
         {UNATTENDED}\n\n\
         Before the pull request, report any CLM defects you met with `clm defect` \
         (`defect-reports`). Write the pull request body with `Write` to `{PR_BODY}` (outside git), \
         from the change: `## Intent`, `## Decisions`, `## Open questions` and \
         `## CLM defects`, saying that the change needs review and that the human's approval \
         lets the triage session land it. Open the pull request with \
         `clm pr open spec/{id} --title <title> --body-file {PR_BODY}`; to correct the \
         body, rewrite that file and run `clm pr body {PR_BODY}`. Then stop: the triage \
         session asks the human for the approval and lands the branch.\n\n\
         End your final message with one line: `proposed {id} review <url>`, with the pull request's \
         URL, or `stopped: <reason>`.\n",
        changes = changes(std::slice::from_ref(f)),
        path = f.path(),
    )
}

/// The fold prompt for several changes, onto the branch `spec/<batch>`.
fn prompt_many(folded: &[Folded], batch: &str) -> String {
    let each: String = folded
        .iter()
        .map(|f| {
            let id = &f.id;
            let diff = format!(".clm/fold-{id}.diff");
            format!(
                "- `{id}`: `git diff origin/main...origin/spec/{id} --output={diff} -- . \
                 ':(exclude)specs'`, then `git apply --3way {diff}`, then commit it as \
                 `Fold {id}`.\n"
            )
        })
        .collect();
    let branches: Vec<String> = folded.iter().map(|f| format!("`spec/{}`", f.id)).collect();
    format!(
        "You are folding {n} CLM spec changes that need review, unattended, which a triage \
         run's drafts left on their branches, into one spec branch, `spec/{batch}`, with one \
         pull request, so the human approves them at once (`yes-no-review`). Each change keeps \
         its ID, its `change.md` and its deltas: one branch carries several changes.\n\n\
         ## Changes\n\n{changes}\n\
         ## Steps\n\n\
         First check `git branch --show-current`, then run \
         `git switch -c spec/{batch} origin/main` and push it with \
         `clm branch push`.\n\n\
         {UNATTENDED} When a change can't be folded without a decision only the human can \
         make, such as two changes that contradict each other, leave it out before anything \
         of it is brought over, and put `left <id>: <reason>` for it in your final message, \
         before its closing line; its branch stays as its draft left it, for \
         `/clm:propose`.\n\n\
         Bring each change onto the branch as one commit, in this order, keeping the branch \
         linear: `clm rebase` rebases without keeping merge commits, so a merge's conflict \
         resolutions would be lost at the next rebase. Write each change's diff outside \
         `specs/` to a file under `.clm/`, which git ignores here (create it first with \
         `mkdir -p .clm`), and apply it:\n\n{each}\n\
         A conflict (`backlog/`, `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`) \
         keeps both \
         changes' edits, merging the text where they overlap. Never bring `specs/` over: \
         `clm rebase` regenerates it from `origin/main` and every change on the branch.\n\n\
         Then {REBASE}. It applies the changes in ID order, so a stale delta it reports is a \
         later change modifying an element an earlier change on the branch also modifies. \
         Merge the earlier change's edit, which rebase prints, into the later change's delta, \
         and set that delta's base from `clm hashes <component>` on `specs/` as it stands with \
         the earlier changes applied: restore `specs/` from `origin/main` \
         (`git restore --source origin/main --staged --worktree specs/`), set each earlier \
         change to `status: proposed` and run `clm apply <id>` on it, in ID order, read the \
         hash, then restore `specs/` from `origin/main` again and set every folded change \
         back to `status: applied` before committing, so rebase regenerates `specs/` from the \
         committed deltas.\n\n\
         Then run `clm check`, then `clm gate --base origin/main`, whose first line is \
         `spec review`, since the folded changes need review. {GATE} A `human-task` item the \
         fold adds is a burden: declare one for it in the `human_burdens` of the change it \
         concerns. Then the builder check (`blocked-by-backlog`) on the elements where \
         two changes met: launch a fresh subagent, its `model` `sonnet` as above, that \
         reads them as their builder, without this conversation, and lists its questions. \
         In the same message, launch the `architecture-auditor` subagent beside it, with the \
         Agent tool's `model` set to the alias of your own model, in the words \
         `your own model`, since the audit is a design review (`model-per-launch`), giving it \
         only the change IDs and the path of your checkout and asking it about the same \
         elements where two changes met. Ask the builder check's subagent, for each requirement on those elements, which requirements not yet current \
         it relies on, of any component, among the changes' own and those `clm pending` lists, \
         answering `none` for a requirement that relies on none \
         (`builder-check-asks-order`). Settle in the changes what the \
         repository answers; give a requirement that waits on another component's pending \
         requirement a `#### Needs` block (`requirement-needs`), and a requirement that \
         relies on a pending requirement of its own component that has a `#### Needs` block that \
         block's needs too, since a need may not name its own component; for work a requirement \
         relies on from another component that no requirement of that component states, add a \
         requirement to that component, or modify one, in the change's delta for it, since a \
         Contract or Design edit builds nothing, and name it in the relying requirement's \
         `#### Needs` (`dependency-work-is-a-requirement`); turn facts and steps only a \
         human can supply into `question` or `human-task` backlog items whose `blocks` name the \
         requirements that wait (`requirement-blocks`); and list the rest under \
         `## Open questions`. Ask the builder check's subagent, too, which capability tests see something different at the product's boundary because of each \
         requirement on those elements: such work \
         is neither a requirement of `capability`, which holds none of its own, nor a \
         `#### Needs`, nor a backlog item, since the requirement's own build updates those tests \
         (`capability-tests-follow-rules`), so name them in that requirement's text; a \
         requirement under which a `CAPABILITIES.md` entry as written no longer holds edits \
         that entry in the change instead, a hand edit. Fix in the \
         changes each `architecture-auditor` finding you agree with, and write each one you leave under \
         `## Open questions` in the `change.md` of the folded change whose element it \
         concerns, with why, since the pull request body carries each change's open questions \
         to the human approving it. The audit advises and never gates \
         (`spec-architecture-audit`). When the builder check or the audit changed anything, \
         run `clm check`, commit, and \
         {REBASE}, then run `clm gate --base origin/main` again.\n\n\
         Never push to, rebase or delete a folded change's branch ({branches}).\n\n\
         After each commit, {REBASE}, then push with `clm branch push`. Read \
         `clm rebase`'s outcome from its last line, never from its exit code: \
         `rebase incomplete` means something is left for you, and `rebase failed` means git \
         itself failed, so run it once more and, if it fails again, stop and say why.\n\n\
         Before the pull request, report any CLM defects you met with `clm defect` \
         (`defect-reports`). Write the pull request body with `Write` to `{PR_BODY}` (outside git): \
         `## Intent`, a line naming the folded changes followed by each change's own intent \
         under its ID; `## Decisions` and `## Open questions`, each change's under its ID, with the elements where two changes met and how you merged \
         them; `## CLM defects`, the `kind: bug` items the folded branches add and your own \
         reports; and that the branch needs review, the human approving every folded change \
         at once. Name the drafts' branches ({branches}) and say that their changes reach \
         `main` with this one. Open the pull request with \
         `clm pr open spec/{batch} --title <title> --body-file {PR_BODY}`; to correct the \
         body, rewrite that file and run `clm pr body {PR_BODY}`. Then stop: the triage \
         session asks the human for the one approval and lands the branch.\n\n\
         End your final message with one line: `proposed {batch} review <url>`, with the \
         pull request's URL, or `stopped: <reason>`. Any `left <id>: <reason>` lines come before \
         it.\n",
        n = folded.len(),
        changes = changes(folded),
        branches = branches.join(", "),
    )
}
