//! `clm draft <slug>...`: claims backlog items and writes the prompt for a session that
//! designs one change from them unattended to a file, returning the launcher that names it
//! (`backlog-triage`, `build-prompt-file`). It makes no worktree, creates no branch and
//! starts no session: the `triage` skill gives the launcher to a background subagent in its
//! own worktree (`builds-in-subagents`).

use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use clm_git::{Options, git};
use clm_model::{BacklogItem, Priority, split_frontmatter};
use clm_orchestrator::{
    PlaceKind, Placed, REMOTE, claim, free_place, places, rename_place, take_place,
};

use crate::{Error, GATE, PR_BODY, fetch_origin, main_graph};

const WORKDIR: &str = ".clm";
/// Where `clm draft` writes its prompts, one file per change ID.
const PROMPTS: &str = ".clm/prompts/draft";

/// The clock and time zone `clm draft` dates its change by: `now` in Unix seconds, and
/// `tz` as `TZ` gives it (`None` leaves the process's own).
#[derive(Debug, Clone)]
pub struct Clock {
    pub now: u64,
    pub tz: Option<OsString>,
}

/// What `clm draft` reports: each refused item with the claim's reason, and the change it
/// drafted, `None` when no item was claimed.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Draft {
    pub left: Vec<(String, String)>,
    pub drafted: Option<Drafted>,
}

/// The change ID `clm draft` chose, which names its place, the file it wrote the session's
/// prompt to, and the launcher that names it.
#[derive(Debug, PartialEq, Eq)]
pub struct Drafted {
    pub id: String,
    pub file: PathBuf,
    pub launcher: String,
}

/// Why `clm draft` drafted nothing.
#[derive(Debug)]
pub enum DraftError {
    /// The clone has no free build place (exit 3); the message names the cap.
    NoPlace(String),
    /// The draft was refused, or git failed, as the [`Error`] says.
    Failed(Error),
}

impl From<Error> for DraftError {
    fn from(e: Error) -> Self {
        DraftError::Failed(e)
    }
}

/// Takes a place (`build-places`), claims each of `slugs` in order with the backlog claim
/// and, when any was claimed, chooses the change ID, names the place by it, keeps `.clm/`
/// out of git and returns the ID with the prompt for the claimed items. A draft that claims
/// nothing, or fails, frees the place it took. `answers` is the file of what the human
/// said in the triage session (`answers-reach-drafts`), read before fetching, taking a
/// place or claiming, so a file that can't be read claims nothing; a relative path is read against the current
/// directory.
pub fn draft(
    root: &Path,
    slugs: &[String],
    answers: Option<&Path>,
    setup: Option<&str>,
    max_builds: u64,
    expiry_hours: u64,
    clock: &Clock,
) -> Result<Draft, DraftError> {
    let answers = match answers {
        Some(file) => fs::read_to_string(file)
            .map_err(|e| Error::Refused(format!("{}: {e}", file.display())))?
            .trim()
            .to_string(),
        None => String::new(),
    };
    fetch_origin(root)?;
    // Before the place and the claims, so a checkout that can't hold the prompt holds
    // nothing.
    let prompts = prompts_dir(root, PROMPTS)?;
    // Unique to this invocation, since a process ID is never reused while it runs.
    let first = slugs.first().map_or("", String::as_str);
    let provisional = format!("draft-{}-{first}", std::process::id());
    let placed = take_place(
        root,
        &provisional,
        PlaceKind::Draft,
        &[],
        max_builds,
        expiry_hours,
    )?;
    if let Placed::Full { held, max_builds } = placed {
        return Err(DraftError::NoPlace(format!(
            "no free build place: {held} of max_builds = {max_builds} in use; wait for a \
             draft or build to land, or finish one whose session has returned"
        )));
    }
    let mut place = provisional;
    let drafted = claimed(
        root,
        slugs,
        (&answers, setup),
        (max_builds, expiry_hours),
        clock,
        &prompts,
        &mut place,
    );
    if !matches!(&drafted, Ok(d) if d.drafted.is_some()) {
        // Best effort: a place left behind stops counting once its time is up.
        let _ = free_place(root, &place);
    }
    Ok(drafted?)
}

/// `clm draft --resume <change>`: takes the place `change`, reusing it while it counts, and
/// writes the prompt for a session that carries on the draft pushed as `spec/<change>` or
/// `autosync/spec/<change>`, the newer when both exist (`draft-resume`). A draft that left
/// no branch frees the place it took and refuses; nothing is claimed, since the draft's
/// items stay held until their claims lapse.
pub fn resume(
    root: &Path,
    change: &str,
    setup: Option<&str>,
    max_builds: u64,
    expiry_hours: u64,
) -> Result<Drafted, DraftError> {
    if change.is_empty() || change.contains('/') {
        return Err(Error::Refused(format!("`{change}` isn't a change ID")).into());
    }
    fetch_origin(root)?;
    let prompts = prompts_dir(root, PROMPTS)?;
    let kept = places(root, expiry_hours)?
        .into_iter()
        .find(|p| p.name == change && p.counts)
        .and_then(|p| p.file.ok())
        .map(|f| f.branches);
    let mut branches = kept.unwrap_or_default();
    let own = format!("spec/{change}");
    if !branches.contains(&own) {
        branches.insert(0, own);
    }
    let placed = take_place(
        root,
        change,
        PlaceKind::Draft,
        &branches,
        max_builds,
        expiry_hours,
    )?;
    if let Placed::Full { held, max_builds } = placed {
        return Err(DraftError::NoPlace(format!(
            "no free build place: {held} of max_builds = {max_builds} in use; wait for a \
             draft or build to land, or finish one whose session has returned"
        )));
    }
    let written = resume_prompt(root, change, setup, &prompts);
    // Only a place this took: a live draft's own place stays.
    if written.is_err() && placed == Placed::New {
        let _ = free_place(root, change);
    }
    Ok(written?)
}

/// Writes the resume prompt for `change` into `prompts`, from the newer of its two spec
/// branches on `origin` (`spec/` on a tie), or refuses when neither exists.
fn resume_prompt(
    root: &Path,
    change: &str,
    setup: Option<&str>,
    prompts: &Path,
) -> Result<Drafted, Error> {
    let tip = |prefix: &str| {
        let rf = format!("refs/remotes/{REMOTE}/{prefix}/{change}");
        git(root, &["log", "-1", "--format=%ct", &rf])
            .ok()
            .and_then(|t| t.trim().parse::<u64>().ok())
    };
    let remote = match (tip("spec"), tip("autosync/spec")) {
        (Some(a), Some(b)) if b > a => format!("autosync/spec/{change}"),
        (Some(_), _) => format!("spec/{change}"),
        (None, Some(_)) => format!("autosync/spec/{change}"),
        (None, None) => {
            return Err(Error::Refused(format!(
                "the draft `{change}` left nothing to resume: no `spec/{change}` or \
                 `autosync/spec/{change}` on `{REMOTE}`; its items stay held until their \
                 claims lapse"
            )));
        }
    };
    let file = prompts.join(format!("{change}.md"));
    fs::write(&file, prompt_for(&[], "", setup, change, Some(&remote)))
        .map_err(|e| Error::Refused(format!("{}: {e}", file.display())))?;
    let launcher = crate::launcher(&file, &format!("resuming the draft `{change}`"));
    Ok(Drafted {
        id: change.to_string(),
        file,
        launcher,
    })
}

/// What `draft` does once it holds the place `place`, which it renames to the change ID,
/// writing the prompt, with the trimmed `answers`, into `prompts`. `limits` are
/// `max_builds` and the expiry in hours.
fn claimed(
    root: &Path,
    slugs: &[String],
    (answers, setup): (&str, Option<&str>),
    (max_builds, expiry_hours): (u64, u64),
    clock: &Clock,
    prompts: &Path,
    place: &mut String,
) -> Result<Draft, Error> {
    let mut claimed: Vec<&str> = Vec::new();
    let mut left = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for slug in slugs.iter().filter(|s| seen.insert(s.as_str())) {
        match claim(root, &format!("backlog/{slug}"), expiry_hours) {
            Ok(_) => claimed.push(slug),
            Err(Error::Refused(why)) => left.push((slug.clone(), why)),
            Err(e) => return Err(e),
        }
    }
    if claimed.is_empty() {
        return Ok(Draft {
            left,
            drafted: None,
        });
    }
    let graph = main_graph(root)?;
    let mut items = Vec::new();
    for slug in claimed {
        // A later claim's fetch may find the item deleted from `origin/main` meanwhile.
        let Some(item) = graph.backlog.get(slug) else {
            left.push((
                slug.to_string(),
                format!("`backlog/{slug}.md` is no longer in `{REMOTE}/main`"),
            ));
            continue;
        };
        let path = item.path.to_string_lossy();
        let text = git(root, &["show", &format!("{REMOTE}/main:{path}")])?.replace("\r\n", "\n");
        let body = split_frontmatter(&text).map_or("", |(_, body)| body).trim();
        items.push((item, body.to_string()));
    }
    if items.is_empty() {
        return Ok(Draft {
            left,
            drafted: None,
        });
    }
    // Named by the items the change takes, so not by one found gone.
    let slugs: Vec<&str> = items.iter().map(|(i, _)| i.slug.as_str()).collect();
    let id = name_place(root, &slugs, max_builds, expiry_hours, clock, place)?;
    let file = prompts.join(format!("{id}.md"));
    // A write that fails leaves the claims to lapse, and `draft` frees the place.
    fs::write(&file, prompt(&items, answers, setup, &id))
        .map_err(|e| Error::Refused(format!("{}: {e}", file.display())))?;
    let launcher = crate::launcher(&file, &format!("drafting `{id}`"));
    Ok(Draft {
        left,
        drafted: Some(Drafted { id, file, launcher }),
    })
}

/// Chooses the change ID for the draft of the claimed items `claimed`, named by the first,
/// renames the place `place` to it, and records the draft's branches in it. A name taken
/// meanwhile, by a place that appeared since, moves on to the next suffix.
fn name_place(
    root: &Path,
    claimed: &[&str],
    max_builds: u64,
    expiry_hours: u64,
    clock: &Clock,
    place: &mut String,
) -> Result<String, Error> {
    let first = claimed.first().copied().unwrap_or_default();
    let words: Vec<&str> = first.split('-').take(4).collect();
    let base = format!("{}-{}", local_date(root, clock)?, words.join("-"));
    let mut n = 1;
    let id = loop {
        let id = if n == 1 {
            base.clone()
        } else {
            format!("{base}-{n}")
        };
        n += 1;
        if held(root, &id, expiry_hours)? {
            continue;
        }
        match rename_place(root, place, &id) {
            Ok(()) => break id,
            // A place of that name appeared since it was chosen.
            Err(Error::Refused(_)) if held(root, &id, expiry_hours)? => continue,
            Err(e) => return Err(e),
        }
    };
    *place = id.clone();
    let branches: Vec<String> = std::iter::once(format!("spec/{id}"))
        .chain(claimed.iter().map(|s| format!("propose/{s}")))
        .collect();
    // The renamed place counts, so this rewrites it with its branches.
    match take_place(
        root,
        &id,
        PlaceKind::Draft,
        &branches,
        max_builds,
        expiry_hours,
    )? {
        Placed::Reused => Ok(id),
        // Another command freed it since the rename: the draft would hold nothing.
        _ => Err(Error::Refused(format!(
            "the place `{id}` changed under `clm draft`; run it again"
        ))),
    }
}

/// Whether change ID `id` is held: by a change on `origin/main`, by a branch on `origin`
/// that a draft of it would use, by a local `spec/<id>` the session couldn't create again,
/// or by a place of the clone.
fn held(root: &Path, id: &str, expiry_hours: u64) -> Result<bool, Error> {
    let exists = |rf: String| git(root, &["rev-parse", "--verify", "-q", &rf]).is_ok();
    let on_origin = ["spec", "autosync/spec", "propose"]
        .iter()
        .any(|prefix| exists(format!("refs/remotes/{REMOTE}/{prefix}/{id}")));
    Ok(crate::finish::landed(root, id)
        || on_origin
        || exists(format!("refs/heads/spec/{id}"))
        || places(root, expiry_hours)?.iter().any(|p| p.name == id))
}

/// The prompt for the claimed `items`, each with its body, for the change `id`, with the
/// human's `answers` when they hold anything (`answers-reach-drafts`).
fn prompt(
    items: &[(&BacklogItem, String)],
    answers: &str,
    setup: Option<&str>,
    id: &str,
) -> String {
    prompt_for(items, answers, setup, id, None)
}

/// The prompt for the change `id`: for the claimed `items` when `resume` is `None`, and
/// otherwise for a session that carries on the draft pushed as the remote branch `resume`
/// (`draft-resume`), under the same steps.
fn prompt_for(
    items: &[(&BacklogItem, String)],
    answers: &str,
    setup: Option<&str>,
    id: &str,
    resume: Option<&str>,
) -> String {
    let list = |v: &[String]| format!("[{}]", v.join(", "));
    let slugs: Vec<String> = items.iter().map(|(i, _)| i.slug.clone()).collect();
    let highest = items
        .iter()
        .map(|(i, _)| i.frontmatter.priority)
        .min()
        .unwrap_or_default();
    let priority = if highest == Priority::P5 {
        "no `priority`, since every item is `P5`".to_string()
    } else {
        format!("`priority: {highest}`, the highest among them")
    };
    let rebase = "run `clm rebase` and resolve what it reports (conflicted files, stale deltas, \
                check failures), committing each fix, until its last line is `rebase ok`";
    let mut p = String::from(if resume.is_some() {
        "You are continuing the design of one CLM spec change, unattended, whose draft's \
         session ended before it returned: `clm draft --resume` wrote this prompt for you. \
         The draft's commits are on its spec branch on `origin`, and the backlog items it \
         claimed are the ones its `change.md` names in `resolves`.\n"
    } else {
        "You are designing one CLM spec change, unattended, from the backlog items below, \
         which `clm draft` claimed for you as `propose/<slug>` on `origin`.\n\n## Items\n"
    });
    for (item, body) in items {
        let fm = &item.frontmatter;
        let requirement = fm
            .requirement
            .as_ref()
            .map(|r| format!(" (requirement `{}@{}`)", r.id, r.version))
            .unwrap_or_default();
        p.push_str(&format!(
            "\n### `backlog/{slug}.md`\n\n- `{slug}`: kind `{kind}`{requirement}, priority \
             `{prio}`, components `{components}`, blocks `{blocks}`\n\n{body}\n",
            slug = item.slug,
            kind = fm.kind,
            prio = fm.priority,
            components = list(&fm.nodes),
            blocks = list(&fm.blocks),
        ));
    }
    if !answers.is_empty() {
        p.push_str(&format!("\n## The human's answers\n\n{answers}\n"));
    }
    let answered = if answers.is_empty() {
        String::new()
    } else {
        "The answers above are the human's, given in the triage session: design from those \
         that bear on your items, and use only those. An answer settles what it answers, \
         outranking the text of the item it answers but never `ARCHITECTURE.md`, a decision \
         or a requirement: an item whose \
         answer would break one of those is left, with `left <slug>: <reason>` naming the \
         rule. An item the answers make designable is designed, not left. Every answer you \
         used is quoted in `change.md` under `## The human's answers`, so the answer is kept \
         in the change and reaches the reviewer and the archive. A `question` or `human-task` \
         item the answers settle stays in `resolves`, and the fact it waited on goes into \
         the specs the change edits, as a requirement or a decision, never only in the \
         quote. An item the human's answers drop stays in `resolves`: delete its \
         `backlog/<slug>.md` on `spec/<id>` and say in `## Intent` that the human dropped it \
         and why, taking the backlog-only route below. An item the answers only edit, \
         leaving it open and still waiting on the human (such as a `human-task` item whose \
         done steps the human has trimmed out), is left out of `resolves` (take it out of the list you first wrote), since a change \
         deletes the items it resolves when it is archived: edit its `backlog/<slug>.md` \
         on `spec/<id>` to what the answers leave of it, say in `## Intent` what you \
         changed and that the item stays open, and take the backlog-only route \
         (`draft-backlog-only-change`) when nothing else is left to design. A `question` \
         item the answers settle while leaving only code to change, a requirement or \
         scenario as written already stating what that code must do \
         (`draft-code-fix-written-rule`) and no requirement the item blocks waiting to be \
         built for it, is the exception to the settled items that stay in `resolves`: take \
         it out of `resolves` (the list you first wrote) and have it turned into a bug. On \
         `spec/<id>` set its `kind` to `bug`, remove its `blocks`, set its `components` to \
         the components whose `paths` hold the fix and rewrite its body to the code work \
         left, naming the requirement the fix restores and quoting the answer, keeping its \
         slug and its other fields, such as `priority` and `issues`; say in `## Intent` that \
         the answer turns it into a bug, and take the backlog-only route when nothing else \
         is left to design. Once `clm land spec/<id>` has landed the branch, fix that bug \
         in this draft by component, as you fix any code-only bug, with \
         `clm claim <component> --fix backlog/<slug>` and a `Resolves: backlog/<slug>` \
         trailer, leaving it with `left <slug>: <reason>` when the claim is refused, and end \
         with the spec change's `landed <sha>`. When the branch needs review it stops the \
         draft as any does, and the bug is left to the next triage.\n\n"
            .to_string()
    };
    // Only when the project declares a setup: otherwise the prompt names no `clm setup`.
    let setup_step = if setup.is_some() {
        "Once you are on the branch you fix code on (the `impl/<component>` branch \
         `clm claim <component> --fix` switched you to, or a branch of your own for test \
         plumbing) and before you run any test there, run `clm setup` as one plain command: it \
         runs the project's `[build] setup`, so that its tests run against your own \
         worktree's code and dependencies, never the main checkout's \
         (`tests-run-in-their-own-tree`). Run it again after a `clm rebase` or an edit of \
         your own that changes the project's dependency declarations. When `clm setup` fails, \
         or Claude Code refuses or denies it, leave the item you were fixing with \
         `left <slug>: setup failed`, and report it with `clm defect` when it was refused \
         or denied, since no test result is then to be trusted. "
    } else {
        ""
    };
    let start = match resume {
        None => format!(
            "Your change ID, `<id>` below, is `{id}`: `clm draft` chose it and holds this \
             clone's place by it. First check `git branch --show-current`, then run \
             `git switch -c spec/{id} origin/main`. Write `changes/{id}/change.md` with \
             `status: proposed`, `resolves` naming every item above ({resolves}), and \
             {priority}. Commit it and push it with `clm branch push`: the spec work then \
             holds the items past their claims' expiry (`propose-claims`). Never rename \
             `<id>` after that.",
            resolves = slugs.join(", "),
        ),
        Some(remote) => format!(
            "Your change ID, `<id>` below, is `{id}`: `clm draft` holds this clone's place by \
             it. First check `git branch --show-current`, then run \
             `git switch -c spec/{id} --no-track {REMOTE}/{remote}`. Read \
             `changes/{id}/change.md`, with the human's answers it quotes, and \
             `git log {REMOTE}/main..HEAD`, then each backlog item its `resolves` names in \
             `backlog/<slug>.md` on `{REMOTE}/main`, and carry the draft on from where its \
             commits leave off: the steps below say what a draft does, so do the ones its \
             commits and files don't already show done, and repeat none that is. Never \
             rename `<id>`."
        ),
    };
    p.push_str(&format!(
        "\n## Steps\n\n\
         {start}\n\n\
         {answered}Before the job, check each item against `origin/main`: an item filed while another \
         change was fixing the same thing outlives that change, and drafting it costs a whole \
         session that ends by deleting it. Take the date the item was filed, \
         `git log -1 --diff-filter=A --format=%as origin/main -- backlog/<slug>.md` \
         (`YYYY-MM-DD`; a change ID's date is `YYYYMMDD`). Read the `## Intent` of each change \
         under `changes/` and `changes/archive/` whose ID is dated no earlier than a week \
         before that and that has a delta for one of the item's components or no delta at all \
         (every change so dated when the item names no components). List the code fixes landed \
         since a week before that, with `<week-before>` that date less seven days, \
         `git log --since=<week-before> --grep='Resolves: backlog/' --format='%h %s' \
         origin/main`, \
         and check whether the decisions, requirements and commands the item names already \
         exist (`docs/decisions/`, `clm hashes <component>`). An item that one of those changes or \
         fixes already fixes stays in `resolves`: delete its `backlog/<slug>.md` on \
         `spec/<id>`, name in `## Intent` what fixed it, and run no `clm context` and design \
         nothing for it. When the \
         change has no delta to write (each item already fixed, accepted, fixed on a code \
         branch, or left) and the branch edits `backlog/` (an item deleted as already fixed \
         or accepted, a bug's `components` corrected, or a CLM defect reported), there is nothing to build, so skip the design and the builder check \
         below: write `## Intent`, with one line saying what the change does to the backlog; \
         run `clm check` until it passes and `clm apply <id>`; commit; \
         {rebase}; run `clm gate --base origin/main`; and go to the pull request and the land as for a \
         change that needs no review below (or stop as for one that does, should the \
         gate's first line be `spec review`).\n\n\
         You are running unattended: no one will answer questions in this conversation. When \
         a choice comes up, decide it by this repository's rules (`CLAUDE.md`, \
         `ARCHITECTURE.md`, `docs/decisions/`) and keep going. Never approve or review a \
         change yourself, and never push `main`: only `clm land` does. When an item can't be \
         designed without a fact only the human knows or a step only they can take, leave it \
         out of `resolves` and put `left <slug>: <reason>` for it in your final message, \
         before its closing line; its claim lapses. Claude Code may ask the human to confirm \
         a tool call: wait for that. When Claude Code refuses a write, don't write it another \
         way, such as through Bash: report the item as left. Every subagent you launch, on \
         any path, names its model through the Agent tool's `model`. Keep the scratch files \
         you write while working (context excerpts, parts of a delta, notes) under this \
         worktree's `.clm/`, which git ignores, and never in a scratchpad or temporary \
         directory your system prompt names or any other directory shared with other \
         sessions: the drafts and builds launched beside you are given the same directory \
         and would overwrite one another's files.\n\n\
         Read each item, and `clm context <component>` for each of its components. Judge each item by \
         the principles as they stand, not as it was filed: `redesign-before-workaround`, \
         `yagni` and `reuse-first`, which `ARCHITECTURE.md` at the head of every context \
         states. For a bug the first question is whether the design is wrong, and the answer \
         is a redesign, an acceptance or a small workaround for a genuine edge case. An \
         accepted item stays in `resolves`: delete its `backlog/<slug>.md` on `spec/<id>` \
         and say in `## Intent` why the behaviour is accepted, a complete answer, not a \
         deferral. List in `change.md` the edge cases and workarounds the change already \
         knows it needs, so a long list shows before anything is built. For a `bug`, decide \
         first whether its fix changes anything under `specs/`, `changes/`, `docs/`, \
         `ARCHITECTURE.md`, `CAPABILITIES.md` or `CLAUDE.md`. A bug is code-only only when a \
         requirement or scenario as written already states the behaviour its fix restores, \
         and the fix's test cites it, as `req:<component>/<slug>@<version>` with the version from \
         `clm hashes <component>`. A bug whose behaviour no requirement states changes `specs/`: it \
         goes into the change, as a requirement or scenario the change adds, stays in \
         `resolves`, and its code is fixed when the component is built \
         (`code-fix-needs-a-written-rule`). An `ARCHITECTURE.md` entry, carried or the \
         project's own, is not such a requirement, since a test never cites an entry \
         (`human-layer-proof`): a bug whose behaviour only an architecture entry states is a \
         bug no requirement states, unless its work is test plumbing. A `bug` or `deferred` \
         item whose work is test plumbing only is never a requirement of any component: \
         test plumbing is harness-only work, changing only the plumbing of the capability \
         suite and nothing a capability test observes, or the plumbing of a component's \
         own test suite (its runner settings, the ports and servers its tests start, the \
         paths they write), changing nothing any of its tests asserts. When the Design that \
         holds that plumbing, if any, stays true as written, \
         fix it in this draft on a branch of your own from `origin/main`, named outside \
         `impl/`, `spec/`, `propose/` and `autosync/`, with no claim and any test it adds \
         citing no requirement: delete the item in a commit whose \
         trailer block holds `Resolves: backlog/<slug>`, land it with `clm land <branch>` \
         once `clm gate --base origin/main` prints `code`, delete the pushed branch with \
         `clm branch delete <branch>`, and take the item out of `resolves` \
         (`small-change-no-claim`). When that Design must change, the change edits the \
         Design, keeps the item in `resolves` and adds a `kind: deferred` item for the code. \
         The bugs whose fix is code-only and lies in one \
         of the bug's `components` are fixed by component (`code-fix-resolves-bug`): run \
         `clm claim <component> --fix backlog/<slug>` for the component's first such bug, then, on that \
         `impl/<component>` branch, fix each of the component's code-only bugs test first and delete its \
         `backlog/<slug>.md` in a commit whose trailer block holds \
         `Resolves: backlog/<slug>`. {setup_step}Run the repository's checks as `CLAUDE.md` asks, confirm \
         `clm gate --base origin/main` prints `code`, write the pull request body to `{PR_BODY}`, \
         open the pull request with `clm pr open impl/<component> --title <title> --body-file {PR_BODY}` \
         and land the branch with `clm land impl/<component>`, then `git switch spec/<id>` to go \
         on. Those items leave `resolves`. When the fix claim is refused, each of that component's \
         code-only bugs is left, with `left <slug>: <reason>` naming the claim's reason, since \
         a spec change has nothing to say about them. A code-only bug whose fix lies in no \
         component the item names is filed on the wrong component, since `clm claim --fix` takes only \
         a component the item on `origin/main` names: set the item's `components` on `spec/<id>` to \
         the components whose `paths` hold the fix, leave it out of `resolves` with \
         `left <slug>: components corrected to <components>`, and fix nothing; the next `clm triage` \
         groups it by its corrected components, and that draft fixes it. A claimed `workaround` \
         item is a build's flag against a requirement (`workaround-flags`): the change \
         resolves it by rewording or re-scoping that requirement, or adding the \
         `#### Needs` or Design it lacks, deleting the flag, or by re-recording its \
         `requirement` at the new version when the workaround still applies to the new \
         text. A claimed `deferred` \
         item's condition was judged met by the human, so the session designs the work it \
         names, except that when the work is code-only, changing nothing under `specs/`, \
         `changes/`, `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md` or `CLAUDE.md` and leaving \
         every requirement and scenario as written true, and lies in one of the item's \
         `components`, it is done by component as a code-only bug is, with \
         `clm claim <component> --fix backlog/<slug>` and a commit whose trailer block holds \
         `Resolves: backlog/<slug>` (`deferred-work-resolves-in-code`): the item leaves \
         `resolves`, and is left with `left <slug>: <reason>` naming the claim's reason when \
         the claim is refused. Otherwise the session designs the work into the change. When, after this (an \
         acceptance, a fix by component or a components correction), no item is left to design, the \
         change has no delta to write: take the route above, and design nothing.\n\n\
         Design the rest as one change on `spec/<id>`. Write the design into `change.md`: \
         `## Intent`, the problem, `## Approaches`, the outcome and the constraints. In its \
         frontmatter, list under `human_burdens` what the change spends of the human's money or \
         time or can't undo, one short line each; a `kind: human-task` backlog item it adds is \
         one, and a spec branch that adds one with no burden is blocked \
         (`gate/undeclared-burden`). Once `CAPABILITIES.md` or `ARCHITECTURE.md` holds an entry, \
         give every component you add a `serves` list naming the entries its rules serve, and a \
         rule whose purpose differs from its component's a `#### Serves` block, since `clm check` \
         fails a rule that serves nothing (`check/untraced`); before then, write neither. When a \
         requirement the change adds or modifies introduces a term `clm/glossary-covers-terms` \
         covers (a top-level `clm` command, a requirement state, a component status, a change \
         status, a backlog kind or a key of `change.md`'s frontmatter), add the term's entry to \
         `docs/guide/glossary.md` on `spec/<id>`, since the code branch that builds the term \
         can't carry a spec path and its land fails without the entry. A change whose deltas \
         add, modify and remove no requirement makes no component pending, so no build does \
         code work its outcome or its Design edit relies on (`component-reconciliation`); \
         before you apply the change, record each piece of such work: work that a requirement \
         would have to state is added to the change as that requirement, so the change has \
         something to build (`dependency-work-is-a-requirement`); work that changes nothing a \
         requirement states, such as removing code nothing calls, goes into a `kind: deferred` \
         backlog item whose `components` name the components whose `paths` hold the code and \
         whose body names that code, which a code branch later resolves with the trailer \
         `Resolves: backlog/<slug>` (`deferred-work-resolves-in-code`). Write one `deltas/<component>.md` per component it changes, with bases from \
         `clm hashes <component>`, record decisions as `docs/decisions/<name>.md`, and make the \
         backlog edits it needs. Run `clm check` until it passes, then `clm apply <id>`, \
         commit, and {rebase}. Run `clm gate --base origin/main`: its first line says whether \
         the change needs review (`spec review`) or lands on checks (`spec`). {GATE} A \
         `check/priority-caps` report is an item left for the human, never a reason to \
         demote a priority.\n\n\
         Then the builder check (`blocked-by-backlog`): launch a fresh subagent that reads the \
         change as its builder, without this conversation, and lists its questions, with the \
         Agent tool's `model` set to `sonnet` (`model: \"sonnet\"`), the model builds run on \
         (`model-per-launch`); never launch a subagent without naming its model. In the same \
         message, launch the `architecture-auditor` subagent beside it, with the Agent tool's \
         `model` set to the alias of your own model, in the words `your own model`, since the \
         audit is a design review (`model-per-launch`), giving it only the change ID and the \
         path of your checkout. Ask the builder check's subagent, for \
         each requirement the change adds or modifies, which requirements not yet current it \
         relies on, of any component, among the change's own and those `clm pending` lists, \
         answering `none` for a requirement that relies on none \
         (`builder-check-asks-order`). Ask it too whether the change's outcome promises code \
         work that no requirement it adds or modifies makes pending, and record each such piece \
         of work as the design step says. Settle in \
         the change what the repository answers; give a requirement that waits on another \
         component's pending requirement a `#### Needs` block (`requirement-needs`), and a \
         requirement that relies on a pending requirement of its own component that has a \
         `#### Needs` block that block's needs too, since a need may not name its own component; for work a \
         requirement relies on from another component that no requirement of that component states, \
         add a requirement to that component, or modify one, in the change's delta for it, since \
         a Contract or Design edit builds nothing, and name it in the relying requirement's \
         `#### Needs` (`dependency-work-is-a-requirement`); turn facts and steps only a human \
         can supply into `question` or `human-task` backlog items whose `blocks` name the \
         requirements that wait (`requirement-blocks`), listing under `choices` of a `question` \
         item that a short answer settles the two to five answers that settle it, each a short line of under 40 characters, distinct, on one line, \
         so the human can answer it with a tap in the web app \
         (`web-answers-as-ledger-records`), and listing none for a question that needs \
         explaining; and list the rest under \
         `## Open questions`. Ask the builder check's subagent, too, which capability tests see something different at the product's boundary because of each \
         requirement the change adds or modifies: such work is neither a requirement of `capability`, which holds none of its \
         own, nor a `#### Needs`, nor a backlog item, since the requirement's own build updates \
         those tests (`capability-tests-follow-rules`), so name them in that requirement's text; \
         a requirement under which a `CAPABILITIES.md` entry as written no longer holds edits \
         that entry in the change instead, a hand edit. A requirement that removes or renames part of a component's Contract \
         that another component's code uses needs the requirements that move each such component off \
         it (`contract-removal-needs`). Write a rename as two requirements of the provider in \
         the same change (`rename-adds-then-removes`): an adding one that offers the new name \
         beside the old, needs nothing and leaves every user's code building as it stands (a \
         second name, never a widened type a user matches exhaustively), and a removing one, \
         stated as an added or modified requirement, that needs each user's move, each move \
         needing the adding one, so the needs form no cycle; the adding one's build marks the old \
         name `clm:remove-by` the change, which stays live until the removal is built \
         (`expand-contract`). The builder check searches each user's code for exhaustive uses \
         (a `match`, a `switch` with no default, a map keyed by the type) the adding one must \
         not break. No build edits another component's code or tests to follow a change \
         (`path-ownership`), so a requirement that widens a type another component matches \
         exhaustively (an added enum variant) needs that component's requirement that first \
         handles a value it doesn't know with a default arm marked `clm:remove-by` the change; \
         that component's requirement that handles the new value, removing the arm, needs the \
         widening, and a requirement that adds a field another component builds by struct \
         literal needs that component's requirement that builds it so the field leaves it \
         compiling (`coupled-changes-sequence`). The builder check searches each user's code \
         for exhaustive uses of a type the change widens too. When the removal can't carry that `#### Needs` (the \
         removing component is required and the component that must move is optional, or the removed \
         text is a whole requirement under `REMOVED Requirements`), put the contract step in \
         a second change on the same branch, left `proposed`, and add a `kind: deferred` \
         backlog item for it whose `components` are the contract change's components and whose body \
         names the contract change and each requirement it waits on with the version it must \
         be current at; the contract change's `resolves` never names the item, so `clm triage` \
         prints it as a `deferred` line until the human judges it due \
         (`contract-step-deferred`). A claimed `deferred` item for a contract step is \
         designed by applying the change it names with `clm apply`, rebasing its stale \
         deltas on `clm hashes <component>` first (a no-op when its bases are current), your own \
         change listing the item in `resolves`. A `human-task` \
         item it adds is a burden: declare one for it in \
         `human_burdens`. Fix in the change each `architecture-auditor` finding you agree \
         with, and write each one you leave under `## Open questions` in `change.md`, with \
         why, since the pull request body carries the open questions to the human. The audit \
         advises and never gates (`spec-architecture-audit`). When the builder check or the audit changed the change \
         (any edit to \
         `changes/<id>/` or `backlog/`), run `clm check`, commit, and {rebase}, which \
         re-applies the edited deltas, since the change is `applied` by then and \
         `clm apply` refuses it. Then run `clm gate --base origin/main` again, whose first \
         line is the one the pull request and the land below go by, and push as below.\n\n\
         After each commit, {rebase}, then push with `clm branch push`. Read \
         `clm rebase`'s outcome from its last line, never from its exit code: `rebase incomplete` \
         means something is left for you, and `rebase failed` means git itself failed, so run \
         it once more and, if it fails again, stop and say why.\n\n\
         Before the pull request, or before stopping on a change that needs review, report any CLM \
         defects you met with `clm defect` (`defect-reports`). Then go by the first line of \
         your last `clm gate` run. When the branch changes a file outside `specs/`, `changes/` \
         and `backlog/` (a hand edit, such as one to `ARCHITECTURE.md`, `CAPABILITIES.md`, \
         `CLAUDE.md`, `README.md` or `docs/`), which a code test may read and which neither \
         `clm check` nor the gate tests, run `clm land --check-only` after that gate run, \
         before the pull request or the stop (`draft-checks-hand-edits`). If a test fails \
         there and it reads a file the branch changes, it is the change's: name each such \
         test, its component and the hand edit that breaks it under `## Open questions` in \
         `change.md`, commit, {rebase} and push as after any commit, and don't land; then, \
         still going by that gate run's first line, stop as the `spec review` case below says, \
         or, when it is `spec`, open no pull request and end with \
         `stopped: land check fails: <tests>`. A failing test that reads no file the branch \
         changes isn't the change's and doesn't stop it, and when the check passes you go on \
         as below. When it is `spec review`, open no pull request and stop after \
         your last push, with the branch rebased: the triage session folds every change \
         needing review that its drafts left into one branch with one pull request, for the human to \
         approve at once. When it is `spec`, write the pull request body with `Write` to \
         `{PR_BODY}` (outside git): `## Intent`, `## Decisions`, `## Open questions` and \
         `## CLM defects`. Open the pull request with \
         `clm pr open spec/<id> --title <title> --body-file {PR_BODY}`; to correct the \
         body, rewrite that file and run `clm pr body {PR_BODY}`. Then run \
         `clm land spec/<id>` and go by what it prints: a line `landed <sha> ...` ends the \
         draft (unless an item the answers turned into a bug still waits for its fix); otherwise go by its last line. `gate/up-to-date` or a conflict with `main` \
         means {rebase}, push with `clm branch push` and land again; a failing \
         check means fix it, commit, push and land again.\n\n\
         Before deciding there is no spec change, report the CLM defects you met with \
         `clm defect` on `spec/<id>` (`defect-reports`): `clm defect` writes to the current \
         branch, and once the branch is deleted a report has nowhere to go. A report is a \
         backlog edit, so the branch then has no delta to write and takes the route above, \
         and lands. Only when every item was fixed on a code branch or left and `spec/<id>` \
         changes nothing but its `change.md` is there no spec change: run \
         `git switch --detach origin/main`, delete `spec/<id>` locally, and delete the \
         branches on `origin`, each with its own `clm branch delete <ref>`: \
         `clm branch delete spec/<id>`, then \
         `clm branch delete autosync/spec/<id>`. A ref the clone never fetched fails only \
         its own call, which is fine, and leaves the other to be deleted. So no empty change holds the items.\n\n\
         End your final message with one line: `landed <sha>`, with the spec change's land \
         (a change that only changes the backlog included, even when its items are on `left` \
         lines), or the last fix branch's when there was no spec change; \
         `proposed <id> review` for a change that needs review, left on its branch for the \
         fold; or `stopped: <reason>`, which a draft that landed nothing ends with, such as \
         one whose items were all left with nothing on its branch. Any \
         `left <slug>: <reason>` lines come before it. Wait for a command you run in the \
         background, such as `clm land spec/<id>` or the checks of the code-fix path, \
         through that command's own completion notice, never through a loop, watch or \
         monitor that matches processes by name (such as `pgrep -f`): such a match also \
         finds other drafts' and builds' processes and outlives the command it waits for. \
         Running a command in the background is a setting of the tool call, not a change to \
         the command. Every command you run yourself, such as the checks of \
         the code-fix path, other than `clm land` in any form, which you run as written, \
         ends within a time limit (`bounded-test-runs`): run it in the foreground, where the \
         tool call's own timeout ends it, or, when you run it in the background, as \
         `timeout <limit> <command>`, with a limit well past the time the run should take, \
         whatever the command is, a test or check run or any other, and for each command of \
         a chain you start in the background. No command you run reads standard input that \
         nothing feeds it, such as `cat` with no file to read or a `cat >> <file>` with no \
         input given, since nothing ever types into a command you run: write a file with \
         your editing tools instead; a command given its input by a heredoc or a pipe, such \
         as `clm defect` or `clm workaround` with the report on stdin, is fine, since its \
         input ends. The reason: a command that never ends by itself, such as a test that \
         hangs on a polling wait with no deadline of its own or a command waiting on \
         standard input, never ends a run started in the background, so its completion notice never comes, \
         and nothing wakes a session that waits for it, nor the triage session, which counts \
         a waiting draft as still running, so the draft sits idle with no one told. A test or check run \
         your limit ends counts as a failing run, whose hanging test you find as for any \
         failure: fix the test when it is in the files of the component you are fixing, \
         and otherwise record it as a backlog item and name that item in your pull request \
         body. Wait for a `clm land` you started until its completion notice \
         arrives, however long that takes, since lands queue behind other drafts' and \
         builds' lands and can take many minutes. Never write your final message while one \
         is still running, since the triage session reads the final line as the draft's end \
         and then frees its place and removes its worktree. When your turn ends while a \
         command you started in the background still runs, because waiting for its \
         completion notice is all that is left, the message ending that turn is not your \
         final message, since the triage session may receive it as the draft's result: it \
         carries no closing line, neither `landed <sha>`, `proposed <id> review` nor \
         `stopped: <reason>`, and ends with the line `waiting: <command>`, naming the \
         command you wait for; your final message comes in a later turn, once the command \
         has ended. You hand back one report, your final message \
         (`one-report-per-run`): a tool that delivers your report to the triage session and \
         accepts one report per run, such as Claude Code's `SubagentHandback`, is never \
         called for the message ending such a turn, which you end as plain text, and is \
         called once, with the final message, since a report sent while waiting uses up \
         the run's one report and the final message would then never reach the triage \
         session. Before you write your final message, landed, proposed or stopped, \
         stop every background task you started that is still running (background shells, \
         monitors, watches) and wait for every subagent you launched, the builder check \
         among them, to return, so that no task of a finished draft wakes the triage \
         session later.\n",
        start = start,
    ));
    p
}

/// The clock's date in its time zone, as `YYYYMMDD`. Git knows the zone's offset at a
/// given time: `git var GIT_COMMITTER_IDENT` with `GIT_COMMITTER_DATE` set to `@<now>`
/// prints the offset `TZ` gives for that moment, so no time zone database is needed here.
fn local_date(root: &Path, clock: &Clock) -> Result<String, Error> {
    let date = OsString::from(format!("@{}", clock.now));
    let ident = OsStr::new("clm");
    let mut env: Vec<(&str, &OsStr)> = vec![
        ("GIT_COMMITTER_DATE", &date),
        // Whatever identity the clone has, or none.
        ("GIT_COMMITTER_NAME", ident),
        ("GIT_COMMITTER_EMAIL", ident),
    ];
    if let Some(tz) = &clock.tz {
        env.push(("TZ", tz));
    }
    let out = clm_git::run(
        root,
        "git",
        &["var", "GIT_COMMITTER_IDENT"],
        Options {
            stdin: None,
            env: &env,
        },
    )?;
    let out = String::from_utf8_lossy(&out);
    let zone = out.trim().rsplit(' ').next().unwrap_or_default();
    let offset = offset(zone).ok_or_else(|| {
        Error::Git(format!(
            "git var GIT_COMMITTER_IDENT: no zone offset in `{}`",
            out.trim()
        ))
    })?;
    let (y, m, d) = civil((clock.now as i64 + offset).div_euclid(86_400));
    Ok(format!("{y:04}{m:02}{d:02}"))
}

/// The seconds east of UTC that a zone offset like `+0900` or `-0430` names.
fn offset(zone: &str) -> Option<i64> {
    let sign = match zone.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let (h, m) = (
        zone.get(1..3)?.parse::<i64>().ok()?,
        zone.get(3..5)?.parse::<i64>().ok()?,
    );
    (zone.len() == 5).then_some(sign * (h * 3600 + m * 60))
}

/// The proleptic Gregorian date of `days` since 1970-01-01 (Howard Hinnant's
/// `civil_from_days`).
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// Keeps `.clm/` out of git and creates the prompts directory `dir` under `root`, returning
/// its absolute path, or refuses naming the path when it can't.
pub(crate) fn prompts_dir(root: &Path, dir: &str) -> Result<PathBuf, Error> {
    exclude_workdir(root)?;
    let path = root.join(dir);
    fs::create_dir_all(&path)
        .and_then(|()| path.canonicalize())
        .map_err(|e| Error::Refused(format!("{}: {e}", path.display())))
}

/// Keeps `.clm/` out of git through `info/exclude`, which every worktree shares, as
/// `clm run` does.
fn exclude_workdir(root: &Path) -> Result<(), Error> {
    let common = git(root, &["rev-parse", "--git-common-dir"])?;
    let info = root.join(common).join("info");
    let exclude = info.join("exclude");
    let line = format!("{WORKDIR}/");
    let current = fs::read_to_string(&exclude).unwrap_or_default();
    if current.lines().any(|l| l == line) {
        return Ok(());
    }
    let io = |e: std::io::Error| Error::Refused(format!("{}: {e}", exclude.display()));
    fs::create_dir_all(&info).map_err(io)?;
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&exclude)
        .map_err(io)?;
    let sep = if current.is_empty() || current.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    writeln!(f, "{sep}{line}").map_err(io)
}

#[cfg(test)]
mod tests {
    use super::{civil, offset};

    #[test]
    fn zone_offsets() {
        assert_eq!(offset("+0000"), Some(0));
        assert_eq!(offset("+0900"), Some(9 * 3600));
        assert_eq!(offset("-0430"), Some(-(4 * 3600 + 30 * 60)));
        assert_eq!(offset("0900"), None);
        assert_eq!(offset("+09"), None);
        assert_eq!(offset("+09000"), None);
    }

    #[test]
    fn civil_dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(-1), (1969, 12, 31));
        assert_eq!(civil(11_016), (2000, 2, 29));
        assert_eq!(civil(20_752), (2026, 10, 26));
    }
}
