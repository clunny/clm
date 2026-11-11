//! Triage (see specs/orchestrator/triage/SPEC.md): `clm triage` groups the backlog's
//! untaken items into units of design work, and `clm draft <slug>...` claims a group and
//! writes the prompt for an unattended proposing session to a file (`backlog-triage`,
//! `build-prompt-file`), holding one of the clone's build places until
//! `clm draft --finish <change>` frees it (`build-places`). `clm draft --fold <id>...`
//! writes the prompt for one session that folds the drafts needing review into one spec branch
//! (`yes-no-review`). The queue, the claims, the places and what holds an item
//! are `orchestrator`'s; this crate owns the grouping rule, the draft budget, the change ID
//! and the prompts' text.

mod draft;
mod finish;
mod fold;
mod intake;

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use clm_git::git;
use clm_model::{
    BacklogItem, BacklogKind, ChangeStatus, Graph, Priority, parse_change_frontmatter,
};
pub use clm_orchestrator::Error;
use clm_orchestrator::{Holding, REMOTE, taken_items};

pub use draft::{Clock, Draft, DraftError, Drafted, draft};
pub use finish::finish;
pub use fold::{Fold, fold};
pub use intake::intake;

/// The most items one draft takes; a larger group is cut into groups of at most this.
pub const BUDGET: usize = 6;

/// Where a draft or fold session writes its pull request body (`own-pr-body`).
const PR_BODY: &str = ".clm/pr-body.md";

/// How a draft or fold session reads `clm gate` beyond its first line (`yes-no-review`).
const GATE: &str = "A change that needs review also gets `gate/approval`, and the gate \
    exits 1, until the human approves it: that is expected, not something to fix, and a \
    burden is never removed to make the gate pass. Any other first line (`code` or `mixed`) \
    means a file outside the spec paths is on the branch: take it off and run the gate again.";

/// What `clm draft` prints in place of its prompt: a few lines that hand the prompt at
/// `file` to the session, which is the prompt for `what` (`build-prompt-file`).
fn launcher(file: &Path, what: &str) -> String {
    format!(
        "Read `{}` with the Read tool, all of it (in parts when it is too large to read at \
         once), before anything else, and follow it as your instructions.\n\
         It is the prompt for {what}, and its first step is `git branch --show-current`.",
        file.display()
    )
}

/// What `clm triage` prints: its lines, and the warnings for stderr.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Triage {
    pub lines: Vec<String>,
    pub warnings: Vec<String>,
}

/// One line before sorting: its form's rank, priority, releases and the text after the
/// form word.
struct Line {
    form: &'static str,
    priority: Priority,
    releases: usize,
    rest: String,
}

/// The forms in the order `clm triage` sorts them.
const FORMS: [&str; 8] = [
    "group",
    "designed",
    "open",
    "main",
    "human",
    "idea",
    "deferred",
    "workaround",
];

impl Line {
    fn rank(&self) -> usize {
        FORMS
            .iter()
            .position(|f| *f == self.form)
            .unwrap_or(FORMS.len())
    }
}

/// `clm triage`: the changes holding backlog items, the groups of untaken `bug` items cut
/// to [`BUDGET`], the untaken `question`, `human-task`, `idea`, `deferred` and `workaround` items,
/// sorted most urgent first.
pub fn triage(root: &Path, expiry_hours: u64) -> Result<Triage, Error> {
    let taken = taken_items(root, expiry_hours)?;
    let graph = main_graph(root)?;
    let backlog = &graph.backlog;
    let mut lines = Vec::new();
    let mut held: BTreeSet<&str> = BTreeSet::new();
    for c in &taken.changes {
        let items: Vec<&BacklogItem> = c.items.iter().filter_map(|s| backlog.get(s)).collect();
        held.extend(c.items.iter().map(String::as_str));
        if items.is_empty() {
            continue;
        }
        let form = match c.state {
            Holding::Main => "main",
            Holding::Proposed if designed(root, &c.id) => "designed",
            Holding::Proposed => "open",
        };
        lines.push(line(form, &c.id, &c.nodes, &items, true));
    }
    held.extend(taken.claims.iter().map(|c| c.item.as_str()));
    let mut groups: BTreeMap<BTreeSet<String>, Vec<&BacklogItem>> = BTreeMap::new();
    for item in backlog.values().filter(|i| !held.contains(i.slug.as_str())) {
        let nodes: BTreeSet<String> = item.frontmatter.nodes.iter().cloned().collect();
        // `matches!`, never an exhaustive `match`: a kind the model adds later gets no line.
        let kind = &item.frontmatter.kind;
        if matches!(kind, BacklogKind::Bug) {
            groups.entry(nodes).or_default().push(item);
        } else if matches!(kind, BacklogKind::Question | BacklogKind::Human) {
            lines.push(line("human", &item.slug, &nodes, &[item], false));
        } else if matches!(kind, BacklogKind::Idea) {
            lines.push(line("idea", &item.slug, &nodes, &[item], false));
        } else if matches!(kind, BacklogKind::Deferred) {
            lines.push(line("deferred", &item.slug, &nodes, &[item], false));
        } else if matches!(kind, BacklogKind::Friction) {
            lines.push(line("workaround", &item.slug, &nodes, &[item], false));
        }
    }
    for (nodes, mut items) in groups {
        items.sort_by(|a, b| {
            (a.frontmatter.priority, &a.slug).cmp(&(b.frontmatter.priority, &b.slug))
        });
        let key = format!(
            "bug:{}",
            nodes.iter().cloned().collect::<Vec<_>>().join("+")
        );
        for cut in items.chunks(BUDGET) {
            lines.push(line("group", &key, &nodes, cut, true));
        }
    }
    lines.sort_by(|a, b| {
        (a.rank(), a.priority, Reverse(a.releases), &a.rest).cmp(&(
            b.rank(),
            b.priority,
            Reverse(b.releases),
            &b.rest,
        ))
    });
    Ok(Triage {
        lines: lines
            .into_iter()
            .map(|l| format!("{} {}", l.form, l.rest))
            .collect(),
        warnings: taken.warnings,
    })
}

/// `<form> <name> [<nodes>] <P> <n>`, then the items' slugs when `slugs` is set.
fn line(
    form: &'static str,
    name: &str,
    nodes: &BTreeSet<String>,
    items: &[&BacklogItem],
    slugs: bool,
) -> Line {
    let priority = items
        .iter()
        .map(|i| i.frontmatter.priority)
        .min()
        .unwrap_or_default();
    let releases = items
        .iter()
        .flat_map(|i| &i.frontmatter.blocks)
        .collect::<BTreeSet<_>>()
        .len();
    let nodes = nodes.iter().cloned().collect::<Vec<_>>().join(",");
    let mut rest = format!("{name} [{nodes}] {priority} {releases}");
    if slugs {
        for item in items {
            rest.push(' ');
            rest.push_str(&item.slug);
        }
    }
    Line {
        form,
        priority,
        releases,
        rest,
    }
}

/// Whether the proposed change `id` is designed: its `change.md`, read from
/// `origin/spec/<id>` else `origin/autosync/spec/<id>`, has `status: applied`, or, when that
/// ref holds no live `change.md`, its `changes/archive/<id>/change.md` has `status: done`
/// (`designed-archived`). A live file is judged alone; one that can't be read or doesn't
/// parse is still being designed.
fn designed(root: &Path, id: &str) -> bool {
    ["spec", "autosync/spec"]
        .iter()
        .find_map(|prefix| {
            [
                (format!("changes/{id}/change.md"), ChangeStatus::Applied),
                (
                    format!("changes/archive/{id}/change.md"),
                    ChangeStatus::Done,
                ),
            ]
            .into_iter()
            .find_map(|(path, want)| {
                let rev = format!("refs/remotes/{REMOTE}/{prefix}/{id}:{path}");
                git(root, &["show", &rev])
                    .ok()
                    .map(|text| parse_change_frontmatter(&text).is_some_and(|fm| fm.status == want))
            })
        })
        .unwrap_or(false)
}

/// Fetches `origin` as `clm draft`, `--finish` and `--fold` do, trying again over a ref
/// race with a build in another worktree (`draft-fetch-retries-race`).
pub(crate) fn fetch_origin(root: &Path) -> Result<(), Error> {
    clm_git::fetch_retrying(
        root,
        &["fetch", "--prune", "--quiet", clm_orchestrator::REMOTE],
    )
    .map_err(|e| Error::Git(e.to_string()))
}

/// The graph of `origin/main`'s tree as of the last fetch, as `clm next` reads it.
fn main_graph(root: &Path) -> Result<Graph, Error> {
    let tmp = clm_git::extract_tree(root, &format!("{REMOTE}/main"))?;
    let (graph, violations) = clm_model::load(tmp.path());
    if violations.is_empty() {
        Ok(graph)
    } else {
        Err(Error::Spec(violations))
    }
}
