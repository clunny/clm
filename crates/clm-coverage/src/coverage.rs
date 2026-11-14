//! Requirement coverage: `req:<id>@<version>` markers in non-Markdown files (see
//! specs/check/coverage/SPEC.md and `versioned-markers`).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
#[cfg(feature = "checkout")]
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use clm_model::{Graph, Layer, Requirement};

/// A marker at `path:line` (1-based): a requirement ID for `req:<id>`, a change ID for
/// `clm:remove-by <change-id>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marker {
    pub id: String,
    /// The requirement version a `req:` marker cites, `@` and 8 hex digits; `None` for a
    /// bare marker, which cites no version and never makes a requirement current.
    pub version: Option<String>,
    pub path: PathBuf,
    pub line: usize,
}

impl fmt::Display for Marker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.path.display(), self.line)
    }
}

/// What the markers say about a requirement (`versioned-markers`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Markers cite it, every one at its current version.
    Current,
    /// A marker cites it at another version, or without one, even beside current ones.
    Behind,
    /// Nothing cites it.
    Unbuilt,
    /// It no longer exists, but a change outside the archive removed it, so its markers
    /// mark code the node's next build removes (`removed-requirements-build-out`).
    Obsolete,
}

/// Every `State` with the word `clm coverage` and `clm pending` print for it
/// (`check.coverage/state-names`).
pub const STATE_NAMES: [(State, &str); 4] = [
    (State::Current, "current"),
    (State::Behind, "behind"),
    (State::Unbuilt, "unbuilt"),
    (State::Obsolete, "obsolete"),
];

impl State {
    /// Not current: the node's code hasn't caught up with its spec.
    pub fn is_pending(self) -> bool {
        self != State::Current
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            State::Current => "current",
            State::Behind => "behind",
            State::Unbuilt => "unbuilt",
            State::Obsolete => "obsolete",
        })
    }
}

/// How an entry that no marker proves stands (`human-layer-proof`): an architecture entry,
/// which the rules its `#### Enforced by` names prove, or a capability holding
/// `#### Untested`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Proof {
    /// It holds `#### Enforced by`, and every rule it names is current.
    Enforced,
    /// It holds `#### Enforced by`; the rules it names that aren't current, in block order.
    Pending(Vec<String>),
    /// An architecture entry holding only `#### Untested`, or a capability holding it.
    Untested,
    /// An architecture entry holding neither block, which `check/unenforced` fails.
    Unenforced,
}

impl fmt::Display for Proof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Proof::Enforced => f.write_str("current"),
            Proof::Pending(rules) => write!(f, "pending {}", rules.join(",")),
            Proof::Untested => f.write_str("untested"),
            Proof::Unenforced => f.write_str("unenforced"),
        }
    }
}

/// One `clm coverage` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coverage {
    pub id: String,
    /// What the markers say; for an entry with a `proof`, `Current` when it is `Enforced`
    /// and `Unbuilt` otherwise, since no marker proves it.
    pub state: State,
    /// The requirement's current version; `None` when it's obsolete.
    pub version: Option<String>,
    /// The markers citing it; none for an entry with a `proof`.
    pub markers: Vec<Marker>,
    /// For an architecture entry, or a capability holding `#### Untested`, how it stands;
    /// such an entry is never pending (`human-layer-proof`). `None` for everything else.
    pub proof: Option<Proof>,
}

impl Coverage {
    /// Whether it is pending: not current, and not an entry that stands by a `proof`.
    pub fn is_pending(&self) -> bool {
        self.proof.is_none() && self.state.is_pending()
    }
}

impl Coverage {
    /// The markers the line lists: all of them, but for a `Behind` requirement only those
    /// citing another version, or none (`stale-markers-keep-behind`).
    pub fn listed(&self) -> impl Iterator<Item = &Marker> {
        let behind = self.state == State::Behind;
        self.markers
            .iter()
            .filter(move |m| !behind || m.version != self.version)
    }
}

impl fmt::Display for Coverage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(proof) = &self.proof {
            return write!(f, "{} {proof}", self.id);
        }
        write!(f, "{} {}", self.id, self.state)?;
        for m in self.listed() {
            write!(f, " {m}")?;
        }
        Ok(())
    }
}

/// Each requirement ID that a change outside the archive lists under `REMOVED Requirements`
/// and no node holds, and each `capability/<slug>` it lists under `removes_capabilities`
/// that `CAPABILITIES.md` doesn't hold (`removed-capabilities`), with the IDs of those changes, sorted: what makes a marker obsolete
/// rather than dangling (`removed-requirements-build-out`).
pub fn removed(graph: &Graph) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for c in graph.changes.values().filter(|c| !c.archived) {
        for d in &c.deltas {
            for r in &d.removed {
                let id = format!("{}/{}", d.node, r.slug);
                if graph.requirement(&id).is_none() {
                    out.entry(id).or_default().push(c.id.clone());
                }
            }
        }
    }
    // A capability a change removes by hand: listed, and no longer in `CAPABILITIES.md`
    // (`removed-capabilities`).
    for c in graph.changes.values().filter(|c| !c.archived) {
        for slug in &c.frontmatter.removes_capabilities {
            let id = format!("capability/{slug}");
            if graph.requirement(&id).is_none() {
                out.entry(id).or_default().push(c.id.clone());
            }
        }
    }
    for changes in out.values_mut() {
        changes.sort();
        changes.dedup();
    }
    out
}

/// Every requirement in `graph`, the human layers' entries included, sorted by ID, as
/// `current` (every marker at its version), `behind` (any marker not) or `unbuilt`, with the markers citing it, or, for an entry that stands
/// by a `Proof`, as that; then, sorted by ID, each requirement a live change removed that a
/// marker still cites, as `obsolete`.
pub fn report(graph: &Graph, markers: &[Marker]) -> Vec<Coverage> {
    let mut by_id: BTreeMap<&str, Vec<Marker>> = graph
        .requirements()
        .map(|r| (r.id.as_str(), Vec::new()))
        .collect();
    let removed = removed(graph);
    let mut obsolete: BTreeMap<&str, Vec<Marker>> = BTreeMap::new();
    for m in markers {
        if let Some(list) = by_id.get_mut(m.id.as_str()) {
            list.push(m.clone());
        } else if removed.contains_key(&m.id) {
            obsolete.entry(m.id.as_str()).or_default().push(m.clone());
        }
    }
    let versions: BTreeMap<&str, &str> = graph
        .requirements()
        .map(|r| (r.id.as_str(), r.version()))
        .collect();
    let existing = by_id.into_iter().map(|(id, markers)| {
        let version = versions.get(id).copied();
        // One marker at another version keeps the requirement behind, even beside current
        // ones (`stale-markers-keep-behind`).
        let stale = |m: &Marker| m.version.as_deref() != version;
        let state = if markers.is_empty() {
            State::Unbuilt
        } else if markers.iter().any(stale) {
            State::Behind
        } else {
            State::Current
        };
        Coverage {
            id: id.into(),
            state,
            version: version.map(str::to_string),
            markers,
            proof: None,
        }
    });
    let mut out: Vec<Coverage> = existing.collect();
    // An entry's proof reads its rules' states, so it is settled once they are. Only a
    // node's rule counts: an entry named under `#### Enforced by` is a model violation,
    // and enforces nothing whatever cites it.
    let current: BTreeSet<String> = out
        .iter()
        .filter(|c| c.state == State::Current && Layer::of(&c.id).is_none())
        .map(|c| c.id.clone())
        .collect();
    for c in &mut out {
        if let Some(proof) = graph.entry(&c.id).and_then(|e| proof(e, &current)) {
            c.state = if proof == Proof::Enforced {
                State::Current
            } else {
                State::Unbuilt
            };
            c.markers.clear();
            c.proof = Some(proof);
        }
    }
    out.extend(obsolete.into_iter().map(|(id, markers)| Coverage {
        id: id.into(),
        state: State::Obsolete,
        version: None,
        markers,
        proof: None,
    }));
    out
}

/// Whether a human-layer entry is cited by tests: a capability not holding `#### Untested`.
/// Every other entry stands by a `Proof` and is never pending (`human-layer-proof`).
pub(crate) fn cited_entry(entry: &Requirement) -> bool {
    Layer::of(&entry.id) == Some(Layer::Capability) && !entry.untested
}

/// How an entry stands when no marker proves it: an architecture entry by the rules its
/// `#### Enforced by` names, else `untested` or `unenforced`; a capability holding
/// `#### Untested` as `untested`. `None` for a cited entry.
fn proof(entry: &Requirement, current: &BTreeSet<String>) -> Option<Proof> {
    if cited_entry(entry) {
        return None;
    }
    match Layer::of(&entry.id)? {
        Layer::Capability => Some(Proof::Untested),
        Layer::Architecture if entry.enforced_by.is_empty() => Some(if entry.untested {
            Proof::Untested
        } else {
            Proof::Unenforced
        }),
        Layer::Architecture => {
            let pending: Vec<String> = entry
                .enforced_by
                .iter()
                .filter(|r| !current.contains(*r))
                .cloned()
                .collect();
            Some(if pending.is_empty() {
                Proof::Enforced
            } else {
                Proof::Pending(pending)
            })
        }
    }
}

/// The markers in a repo's files.
pub struct Scan {
    /// `req:<id>` markers.
    pub req: Vec<Marker>,
    /// `clm:remove-by <change-id>` markers; each `id` is the change's.
    pub remove_by: Vec<Marker>,
}

/// Markers in every non-Markdown file git would track (all files outside hidden dirs and
/// `target/` when `root` isn't a git work tree).
#[cfg(feature = "checkout")]
pub fn scan(root: &Path) -> Scan {
    let mut files = git_files(root).unwrap_or_else(|| {
        let mut files = Vec::new();
        walk(root, root, &mut files);
        files
    });
    files.sort();

    let mut scan = Scan {
        req: Vec::new(),
        remove_by: Vec::new(),
    };
    for rel in files {
        let Ok(text) = fs::read_to_string(root.join(&rel)) else {
            continue; // unreadable or not UTF-8: not a test source
        };
        scan_text(&rel, &text, &mut scan);
    }
    scan
}

/// The `req:<id>[@<version>]` markers in one file, `path` repo-relative: what `scan` finds
/// in it. None for Markdown, where markers are examples rather than claims.
pub fn markers_in(path: &Path, text: &str) -> Vec<Marker> {
    let mut scan = Scan {
        req: Vec::new(),
        remove_by: Vec::new(),
    };
    scan_text(path, text, &mut scan);
    scan.req
}

/// A `req:` marker's ID, version and the byte range of its whole text in the text searched
/// (`req:` through the slug, and through the version when it carries one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerSpan {
    pub id: String,
    pub version: Option<String>,
    pub range: Range<usize>,
}

/// The `req:` markers in `text`, as `markers_in` finds them (same order, ID and version),
/// each with its byte range, so a caller that rewrites markers in place rewrites exactly the
/// markers the coverage counts (`marker-spans`). It reads no path, so the caller decides
/// which files to read.
pub fn marker_spans(text: &str) -> Vec<MarkerSpan> {
    let mut out = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        out.extend(parse_markers(line).into_iter().map(|mut s| {
            s.range = s.range.start + offset..s.range.end + offset;
            s
        }));
        offset += line.len();
    }
    out
}

fn scan_text(path: &Path, text: &str, scan: &mut Scan) {
    if path.extension().is_some_and(|e| e == "md") {
        return;
    }
    for (i, line) in text.lines().enumerate() {
        let marker = |id, version| Marker {
            id,
            version,
            path: path.to_path_buf(),
            line: i + 1,
        };
        scan.req.extend(
            parse_markers(line)
                .into_iter()
                .map(|s| marker(s.id, s.version)),
        );
        scan.remove_by.extend(
            parse_removal_markers(line)
                .into_iter()
                .map(|id| marker(id, None)),
        );
    }
}

#[cfg(feature = "checkout")]
fn git_files(root: &Path) -> Option<Vec<PathBuf>> {
    let args = [
        "ls-files",
        "-z",
        "--cached",
        "--others",
        "--exclude-standard",
    ];
    let out = clm_git::run(root, "git", &args, clm_git::Options::default()).ok()?;
    Some(
        out.split(|b| *b == 0)
            .filter(|s| !s.is_empty())
            .map(|s| PathBuf::from(String::from_utf8_lossy(s).into_owned()))
            .filter(|p| root.join(p).is_file())
            .collect(),
    )
}

#[cfg(feature = "checkout")]
fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !name.starts_with('.') && name != "target" {
                walk(root, &path, out);
            }
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_path_buf());
        }
    }
}

fn is_id(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'
}

/// Whether the text before byte `at` of `line` ends with a word character.
fn follows_word(line: &str, at: usize) -> bool {
    line[..at]
        .chars()
        .next_back()
        .is_some_and(|c| c.is_alphanumeric() || c == '_')
}

/// Each `req:<node-id>/<slug>[@<version>]` marker in `line`, with its byte range, where
/// `req:` doesn't follow a word character. The version is `@` followed by exactly 8 lowercase
/// hex digits, as `clm hashes` prints them; anything else after the slug leaves the marker bare.
fn parse_markers(line: &str) -> Vec<MarkerSpan> {
    let mut ids = Vec::new();
    for (at, _) in line.match_indices("req:") {
        if follows_word(line, at) {
            continue;
        }
        let rest = &line[at + 4..];
        let node_len = rest
            .find(|c: char| !(is_id(c) || c == '.'))
            .unwrap_or(rest.len());
        let node = &rest[..node_len];
        let Some(after) = rest[node_len..].strip_prefix('/') else {
            continue;
        };
        let slug_len = after.find(|c: char| !is_id(c)).unwrap_or(after.len());
        let slug = &after[..slug_len];
        let valid_node = !node.is_empty() && node.split('.').all(|p| !p.is_empty());
        if valid_node && !slug.is_empty() {
            let version = after[slug_len..].strip_prefix('@').and_then(|v| {
                let hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
                let digits = v.find(|c: char| !hex(c)).unwrap_or(v.len());
                (digits == 8).then(|| v[..8].to_string())
            });
            // `req:`, the node, `/`, the slug, and `@` plus 8 digits when versioned.
            let end = at + 4 + node_len + 1 + slug_len + version.as_ref().map_or(0, |_| 9);
            ids.push(MarkerSpan {
                id: format!("{node}/{slug}"),
                version,
                range: at..end,
            });
        }
    }
    ids
}

/// Change IDs of `clm:remove-by <change-id>` markers in `line`: the regex
/// `clm:remove-by (\d{8}-[a-z0-9-]+)`, where `clm:` doesn't follow a word character.
fn parse_removal_markers(line: &str) -> Vec<String> {
    const PREFIX: &str = "clm:remove-by ";
    let mut ids = Vec::new();
    for (at, _) in line.match_indices(PREFIX) {
        if follows_word(line, at) {
            continue;
        }
        let rest = &line[at + PREFIX.len()..];
        let id = &rest[..rest.find(|c: char| !is_id(c)).unwrap_or(rest.len())];
        let (date, slug) = id.split_at(id.len().min(8));
        let valid = date.len() == 8
            && date.bytes().all(|b| b.is_ascii_digit())
            && slug.len() > 1
            && slug.starts_with('-');
        if valid {
            ids.push(id.to_string());
        }
    }
    ids
}
