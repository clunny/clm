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
    let mut path: Option<String> = None;
    // A file's header runs from its `diff ` line to its first `@@` hunk header; past
    // that, a `+++ ` line is an added line whose text starts `++ `. No hunk line starts
    // `diff ` or `@@`, since each starts ` `, `+`, `-` or `\`.
    let mut in_hunk = false;
    let log = String::from_utf8_lossy(&log);
    for line in log.lines() {
        if line.starts_with("diff ") {
            in_hunk = false;
            path = None;
            continue;
        }
        if line.starts_with("@@") {
            in_hunk = true;
            continue;
        }
        if !in_hunk {
            if let Some(name) = line.strip_prefix("+++ ") {
                path = header_path(name);
            }
            continue;
        }
        let (Some(text), Some(file)) = (line.strip_prefix('+'), &path) else {
            continue;
        };
        if Path::new(file).extension().is_some_and(|e| e == "md") {
            continue;
        }
        for m in clm_coverage::marker_spans(text) {
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

/// The file a `+++ ` header names, from the text after `+++ `: `b/<path>`, with the tab git
/// ends a path holding a space with taken off, or `"b/<path>"` as git quotes it, unquoted.
/// `/dev/null`, a deleted file, names none.
fn header_path(name: &str) -> Option<String> {
    let name = match name.strip_prefix('"') {
        Some(quoted) => unquote(quoted)?,
        None => name.strip_suffix('\t').unwrap_or(name).to_string(),
    };
    name.strip_prefix("b/").map(str::to_string)
}

/// The text of a path git quoted, from after its opening `"`: its C-style escapes and octal
/// bytes read back, up to the closing `"`. `None` when it isn't closed or an escape is unknown.
fn unquote(quoted: &str) -> Option<String> {
    let mut bytes = Vec::new();
    let mut rest = quoted.bytes();
    loop {
        match rest.next()? {
            b'"' => return Some(String::from_utf8_lossy(&bytes).into_owned()),
            b'\\' => {
                let byte = match rest.next()? {
                    b'a' => 0x07,
                    b'b' => 0x08,
                    b't' => b'\t',
                    b'n' => b'\n',
                    b'v' => 0x0b,
                    b'f' => 0x0c,
                    b'r' => b'\r',
                    b'"' => b'"',
                    b'\\' => b'\\',
                    d @ b'0'..=b'3' => {
                        let mut value = u32::from(d - b'0');
                        for _ in 0..2 {
                            match rest.next()? {
                                o @ b'0'..=b'7' => value = value * 8 + u32::from(o - b'0'),
                                _ => return None,
                            }
                        }
                        value as u8
                    }
                    _ => return None,
                };
                bytes.push(byte);
            }
            b => bytes.push(b),
        }
    }
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
