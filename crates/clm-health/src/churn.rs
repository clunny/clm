//! `churn`: the requirements the committed history cites at three or more versions,
//! counted from the markers each commit added (`churn-signal`,
//! `churn-from-marker-history`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use clm_git::Failure;
use clm_model::Graph;

use crate::{Section, finding};

/// A requirement ID the history cited, with its distinct versions, sorted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Churn {
    /// `<node>/<slug>`
    pub id: String,
    /// The 8-hex-digit versions, without their `@`.
    pub versions: Vec<String>,
}

/// The patch log, its shape pinned against the machine's git configuration: the
/// Design's settings, and also the prefixes git 2.45 lets a user set and `log.showRoot`,
/// so the root commit's patch is shown.
const LOG: [&str; 19] = [
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
    "--format=",
    "--no-color",
    "--no-renames",
    "--no-ext-diff",
    "--no-textconv",
];

/// Every requirement ID the commits reachable from `HEAD` cite with a version, with its
/// versions, sorted by ID; `None` when `HEAD` names no commit.
pub fn churn(root: &Path) -> Result<Option<Vec<Churn>>, Failure> {
    if !crate::has_history(root)? {
        return Ok(None);
    }
    let log = clm_git::run(root, "git", &LOG, clm_git::Options::default())?;
    let mut cited: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut path: Option<&Path> = None;
    let mut after_minus = false;
    let log = String::from_utf8_lossy(&log);
    for line in log.lines() {
        let header = after_minus && line.starts_with("+++ ");
        after_minus = line.starts_with("--- ");
        if header {
            // `+++ /dev/null` names no file, nor does a path git still quotes. Git ends
            // the header of a path holding a space with a tab.
            path = line
                .strip_prefix("+++ b/")
                .map(|p| Path::new(p.strip_suffix('\t').unwrap_or(p)));
            continue;
        }
        let (Some(text), Some(file)) = (line.strip_prefix('+'), path) else {
            continue;
        };
        for m in clm_check::markers_in(file, text) {
            if let Some(version) = m.version {
                let version = version.trim_start_matches('@').to_string();
                cited.entry(m.id).or_default().insert(version);
            }
        }
    }
    Ok(Some(
        cited
            .into_iter()
            .map(|(id, versions)| Churn {
                id,
                versions: versions.into_iter().collect(),
            })
            .collect(),
    ))
}

/// Each requirement the graph holds that the history cites at three or more versions.
pub(crate) fn section(root: &Path, graph: &Graph) -> Result<Section, Failure> {
    let Some(churn) = churn(root)? else {
        return Ok(Section::skipped("churn", "no history".into()));
    };
    let versions: BTreeMap<&str, usize> = churn
        .iter()
        .map(|c| (c.id.as_str(), c.versions.len()))
        .collect();
    let lines = crate::requirements::requirements(graph)
        .filter_map(|(n, r)| {
            let count = *versions.get(r.id.as_str())?;
            (count >= 3).then(|| {
                finding(
                    n.path.display(),
                    "churn",
                    format!("{} cited at {count} versions", r.id),
                )
            })
        })
        .collect();
    Ok(Section::of("churn", "requirements", lines))
}
