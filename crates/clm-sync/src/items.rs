//! The backlog items `clm sync` files on its own: the open web proposals
//! (`file-proposal-items`) and the reconsider items of the branch's node
//! (`file-reconsider-item`).

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use clm_ledger::Body;
use clm_model::Graph;

use crate::{Error, git, write};

/// `file-reconsider-item` (`health-advises`): each requirement of `node` whose reconsider
/// score, from the graph and backlog items in `scored` (`HEAD`'s tree), reaches the
/// threshold, and whose `backlog/reconsider-<node>-<slug>.md` doesn't exist in `root`,
/// tracked or not, filed there with the text `check.health` gives and committed alone, in
/// requirement-ID order. Returns the `filed` lines.
pub(crate) fn file_reconsider(
    root: &Path,
    scored: &Path,
    node: &str,
    settings: &clm_health::Settings,
) -> Result<Vec<String>, Error> {
    let (graph, _) = clm_model::load(scored);
    let mut due = clm_health::reconsider(&graph, &graph.backlog, settings, node);
    due.sort_by(|a, b| a.id.cmp(&b.id));
    let mut lines = Vec::new();
    for r in due {
        let name = format!("reconsider-{}", r.id.replacen('/', "-", 1));
        let item = format!("backlog/{name}.md");
        if root.join(&item).exists() {
            continue;
        }
        commit_alone(root, &item, &r.text, &format!("File {name}"))?;
        lines.push(format!("filed {item}"));
    }
    Ok(lines)
}

/// Writes `text` to `item` under `root` and commits it alone as `message`. Should the commit
/// fail, the item is removed again: left behind, it would read as filed on the next run and
/// never be committed.
pub(crate) fn commit_alone(
    root: &Path,
    item: &str,
    text: &str,
    message: &str,
) -> Result<(), Error> {
    write(&root.join(item), text)?;
    let committed = git(root, &["add", "--", item])
        .and_then(|_| git(root, &["commit", "-q", "-m", message, "--", item]));
    if let Err(e) = committed {
        let _ = git(root, &["reset", "-q", "--", item]);
        let _ = fs::remove_file(root.join(item));
        return Err(e.into());
    }
    Ok(())
}

/// Whether `slug` is lowercase kebab case, `[a-z0-9]+(-[a-z0-9]+)*`.
fn kebab(slug: &str) -> bool {
    !slug.is_empty()
        && slug.split('-').all(|part| {
            !part.is_empty() && part.bytes().all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9'))
        })
}

/// `file-proposal-items` (`web-proposals-as-ledger-records`): each open proposal of the
/// ledger fold, in slug order, whose slug no item of `graph` has (nor an untracked file in
/// `root`) and no change of `graph`, archived or not, names in `resolves`, written as
/// `backlog/<slug>.md` from the record alone and committed as `File <slug>`. Returns the
/// `filed` lines and the warnings: each record skipped for its slug, each ledger line the
/// fold skipped, or why the ledger couldn't be read, which files nothing, since filing never
/// changes the outcome.
pub(crate) fn file_proposals(
    root: &Path,
    graph: &Graph,
) -> Result<(Vec<String>, Vec<String>), Error> {
    let fold = match clm_ledger::fold(root) {
        Ok(fold) => fold,
        Err(e) => {
            let why = format!("no proposals filed: the ledger can't be read: {e}");
            return Ok((Vec::new(), vec![why]));
        }
    };
    let mut warnings: Vec<String> = fold
        .warnings
        .iter()
        .map(|w| format!("ledger: {w}"))
        .collect();
    let resolved: BTreeSet<&String> = graph
        .changes
        .values()
        .flat_map(|c| &c.frontmatter.resolves)
        .collect();
    let mut open: Vec<_> = fold
        .proposals
        .values()
        .filter_map(|r| match &r.body {
            Body::Proposed {
                slug,
                title,
                body,
                nodes,
            } => Some((slug, title, body, nodes, r)),
            _ => None,
        })
        .collect();
    open.sort_by(|a, b| (a.0, &a.4.at, &a.4.id).cmp(&(b.0, &b.4.at, &b.4.id)));
    let mut lines = Vec::new();
    for (slug, title, body, nodes, record) in open {
        if !kebab(slug) {
            warnings.push(format!(
                "skipped proposal {} by {}: its slug {slug:?} isn't lowercase kebab case",
                record.id, record.writer
            ));
            continue;
        }
        let item = format!("backlog/{slug}.md");
        if graph.backlog.contains_key(slug) || resolved.contains(slug) || root.join(&item).exists()
        {
            continue;
        }
        let title = serde_json::to_string(title).expect("a string");
        let components: Vec<&str> = nodes
            .iter()
            .filter(|n| graph.nodes.contains_key(n.as_str()))
            .map(String::as_str)
            .collect();
        let body = body.trim_end_matches(['\n', '\r']);
        let mut text = format!(
            "---\ntitle: {title}\nkind: idea\ncomponents: [{}]\n---\n\n",
            components.join(", ")
        );
        if !body.is_empty() {
            text.push_str(body);
            text.push_str("\n\n");
        }
        text.push_str(&format!(
            "Proposed in the web app by {} at {} (record {}).\n",
            record.writer, record.at, record.id
        ));
        commit_alone(root, &item, &text, &format!("File {slug}"))?;
        lines.push(format!("filed {item}"));
    }
    Ok((lines, warnings))
}
