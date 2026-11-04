//! What only git knows, and the report for a working tree (`effort/history`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use clm_model::{BacklogFrontmatter, BacklogKind, Graph};

use crate::{
    Constraint, Error, History, Inputs, ItemLife, Report, architecture, flag_text, report,
};

/// The patch log over `backlog/`, its shape pinned as `clm-health`'s churn pins it, oldest
/// commit first, each commit's time on a line of its own after `\x01`, and each patch
/// holding its whole file as context, so every version's text can be read from it.
const LOG: [&str; 24] = [
    "-c",
    "diff.noprefix=false",
    "-c",
    "diff.srcPrefix=a/",
    "-c",
    "diff.dstPrefix=b/",
    "-c",
    "log.showRoot=true",
    "-c",
    "diff.mnemonicPrefix=false",
    "-c",
    "core.quotePath=false",
    "log",
    "-p",
    "--unified=1000000",
    "--reverse",
    "--format=%x01%ct",
    "--no-color",
    "--no-renames",
    "--no-ext-diff",
    "--no-textconv",
    "--",
    "backlog/*.md",
    ":!backlog/*/**",
];

/// Whether `HEAD` names a commit; a git failure other than an unborn `HEAD` is an error.
fn has_head(root: &Path) -> Result<bool, Error> {
    match clm_git::git(root, &["rev-parse", "--verify", "--quiet", "HEAD^{commit}"]) {
        Ok(_) => Ok(true),
        Err(clm_git::Error::Failed { status, .. }) if status.code() == Some(1) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// The slug of a top-level `backlog/<slug>.md`.
fn item_slug(path: &str) -> Option<&str> {
    let slug = path.strip_prefix("backlog/")?.strip_suffix(".md")?;
    (!slug.contains('/')).then_some(slug)
}

/// One file's patch in the log: its paths before and after, and its text after.
#[derive(Default)]
struct Patch {
    before: Option<String>,
    after: Option<String>,
    text: String,
    /// Past the header, in the hunks.
    hunks: bool,
}

/// Applies a finished patch to the items' lives: a file added starts a life (of a
/// `question`, `human` or `friction` item), a file changed adds the `nodes` and `blocks` it
/// now holds to its open life, and a file deleted ends it.
fn apply(lives: &mut Vec<ItemLife>, patch: Patch, at: i64) {
    let side = |path: &Option<String>, prefix: &str| {
        let path = path.as_deref()?.strip_prefix(prefix)?;
        item_slug(path).map(str::to_string)
    };
    let (before, after) = (side(&patch.before, "a/"), side(&patch.after, "b/"));
    let frontmatter = || {
        let (fm, _) = clm_model::split_frontmatter(&patch.text)?;
        serde_yaml::from_str::<BacklogFrontmatter>(fm).ok()
    };
    let open = |lives: &[ItemLife], slug: &str| {
        lives
            .iter()
            .rposition(|l| l.slug == slug && l.deleted.is_none())
    };
    match (before, after) {
        (None, Some(slug)) => {
            let Some(fm) = frontmatter() else { return };
            if matches!(
                fm.kind,
                BacklogKind::Question | BacklogKind::Human | BacklogKind::Friction
            ) {
                lives.push(ItemLife {
                    slug,
                    kind: fm.kind,
                    nodes: fm.nodes,
                    blocks: fm.blocks,
                    added: at,
                    deleted: None,
                });
            }
        }
        (Some(slug), None) => {
            if let Some(i) = open(lives, &slug) {
                lives[i].deleted = Some(at);
            }
        }
        (Some(slug), Some(_)) => {
            let (Some(i), Some(fm)) = (open(lives, &slug), frontmatter()) else {
                return;
            };
            let life = &mut lives[i];
            for (have, more) in [(&mut life.nodes, fm.nodes), (&mut life.blocks, fm.blocks)] {
                for id in more {
                    if !have.contains(&id) {
                        have.push(id);
                    }
                }
            }
        }
        (None, None) => {}
    }
}

/// Each `question`, `human` and `friction` item's lives, from one patch log. A life's
/// `nodes` and `blocks` are every one its file held while it lived.
fn items(root: &Path) -> Result<Vec<ItemLife>, Error> {
    let log = clm_git::run(root, "git", &LOG, clm_git::Options::default())?;
    let log = String::from_utf8_lossy(&log);
    let mut lives: Vec<ItemLife> = Vec::new();
    let mut at = 0;
    let mut patch: Option<Patch> = None;
    for line in log.lines() {
        if let Some(time) = line.strip_prefix('\x01') {
            if let Some(done) = patch.take() {
                apply(&mut lives, done, at);
            }
            at = time.trim().parse().unwrap_or(at);
            continue;
        }
        if line.starts_with("diff --git ") {
            if let Some(done) = patch.replace(Patch::default()) {
                apply(&mut lives, done, at);
            }
            continue;
        }
        let Some(p) = patch.as_mut() else { continue };
        if p.hunks {
            // A content line: its first character says which side it is on.
            if let Some(kept) = line.strip_prefix([' ', '+']) {
                p.text.push_str(kept);
                p.text.push('\n');
            }
            continue;
        }
        // Git ends the header of a path holding a space with a tab.
        let path = |rest: &str| {
            let rest = rest.strip_suffix('\t').unwrap_or(rest);
            (rest != "/dev/null").then(|| rest.to_string())
        };
        if let Some(rest) = line.strip_prefix("--- ") {
            p.before = path(rest);
        } else if let Some(rest) = line.strip_prefix("+++ ") {
            p.after = path(rest);
        } else if line.starts_with("@@") {
            p.hunks = true;
        }
    }
    if let Some(done) = patch {
        apply(&mut lives, done, at);
    }
    Ok(lives)
}

/// The commits reachable from `HEAD` that touched each node's `SPEC.md` or `paths`.
fn commits(root: &Path, graph: &Graph) -> Result<BTreeMap<String, BTreeSet<String>>, Error> {
    let mut out = BTreeMap::new();
    for node in graph.nodes.values() {
        let spec = node.path.to_string_lossy().replace('\\', "/");
        let specs = std::iter::once(format!(":(literal){spec}"));
        let paths = node.frontmatter.paths.iter().map(|p| format!(":(glob){p}"));
        let pathspecs: Vec<String> = specs.chain(paths).collect();
        let mut args = vec!["log", "--format=%H", "--"];
        args.extend(pathspecs.iter().map(String::as_str));
        let log = clm_git::git(root, &args)?;
        out.insert(node.id.clone(), log.lines().map(str::to_string).collect());
    }
    Ok(out)
}

fn history_of(root: &Path, graph: &Graph) -> Result<History, Error> {
    let (commits, items) = if has_head(root)? {
        (commits(root, graph)?, items(root)?)
    } else {
        (BTreeMap::new(), Vec::new())
    };
    // The budgets don't change the counts.
    let budgets = clm_check::Budgets {
        spec_tokens: 0,
        code_tokens: 0,
    };
    let sizes = clm_check::size(root, graph, &clm_check::scan(root).req, budgets)
        .map_err(Error::Io)?
        .into_iter()
        .map(|s| (s.node, s.spec.tokens + s.code.tokens))
        .collect();
    Ok(History {
        commits,
        items,
        sizes,
        now: clm_git::now() as i64,
    })
}

/// The graph of the CLM repository at `root`, or a refusal.
pub(crate) fn graph(root: &Path) -> Result<Graph, Error> {
    if !root.join("specs/SPEC.md").is_file() {
        return Err(Error::Refused(format!(
            "not a CLM repository: {} has no specs/SPEC.md",
            root.display()
        )));
    }
    let (graph, violations) = clm_model::load(root);
    match violations.first() {
        Some(v) => Err(Error::Refused(format!(
            "the specs don't load: {}: {}",
            v.path.display(),
            v.message
        ))),
        None => Ok(graph),
    }
}

/// What only git knows about the CLM repository at `root`.
pub fn history(root: &Path) -> Result<History, Error> {
    history_of(root, &graph(root)?)
}

/// The report for the working tree at `root`: its graph, its ledgers' fold, its history and
/// its friction items' text.
pub fn capabilities(root: &Path) -> Result<Report, Error> {
    let graph = graph(root)?;
    let fold = clm_ledger::fold(root)?;
    let history = history_of(root, &graph)?;
    let flag_text = flag_text(&graph, &|path| {
        std::fs::read_to_string(root.join(path)).ok()
    });
    Ok(report(&Inputs {
        graph: &graph,
        fold: &fold,
        history: Some(&history),
        flag_text: &flag_text,
    }))
}

/// The architecture review for the working tree at `root`: its graph, its ledgers' fold and
/// its friction items' text, with the requirements current in it as `clm coverage` gives them.
pub fn architecture_review(root: &Path) -> Result<Vec<Constraint>, Error> {
    let graph = graph(root)?;
    let fold = clm_ledger::fold(root)?;
    let flag_text = flag_text(&graph, &|path| {
        std::fs::read_to_string(root.join(path)).ok()
    });
    let current: BTreeSet<String> = clm_check::report(&graph, &clm_check::scan(root).req)
        .into_iter()
        .filter(|c| c.state == clm_check::State::Current)
        .map(|c| c.id)
        .collect();
    Ok(architecture(
        &Inputs {
            graph: &graph,
            fold: &fold,
            history: None,
            flag_text: &flag_text,
        },
        Some(&current),
    ))
}
