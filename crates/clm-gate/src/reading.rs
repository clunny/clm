//! One reading of a spec branch (`review-from-files`): the changes' frontmatters, the text
//! exemptions, the two graphs with their load violations, the cuts, the withheld lists and
//! the `gate/deleted-tests` violations, read once over two sides. `evaluate_by` reads the
//! sides from git (`GitSide`); a caller that fetched the files itself hands them over as
//! `FileSide`s to `review_from_files`, so the two give one answer.

use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;
use std::path::{Path, PathBuf};

use clm_model::{ChangeFrontmatter, Files, Graph, Violation};

use crate::cuts::{self, Cut, ListLine, Side, TestList};
use crate::{ARCHIVE, At, CHANGES, Graphs, SPECS, live_change, needs_review_of, show_bytes};

/// One side of a branch's diff, its merge base or its head, as a caller holds it: its
/// revision, the paths of the files it holds, and a reader of their bytes, which gives none
/// for a file whose bytes can't be had.
pub struct FileSide<'a> {
    pub revision: &'a str,
    pub paths: &'a BTreeSet<String>,
    pub read: &'a dyn Fn(&str) -> Option<Vec<u8>>,
}

/// What `review_from_files` reads of a spec branch: whether it needs review, what it cuts,
/// and each `deleted-tests.txt` whose bytes can't be had, with the revision of the first
/// side, merge base then head, that withholds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReview {
    pub review: bool,
    pub cuts: Vec<Cut>,
    pub withheld: Vec<(String, String)>,
}

/// A spec branch's review and cuts read from the files of its two sides alone, as
/// `evaluate_at` decides them under `RULES` (`review-from-files`). `paths` are every path the
/// diff changes with its status as `git diff --no-renames --name-status` gives it, `moved`
/// the IDs of the changes whose `change.md`'s move into the archive is an archive move, and
/// `approval_changed` whether `approval-setting` counts `clm.toml` as changed. Runs no
/// process.
pub fn review_from_files(
    paths: &[(char, &str)],
    moved: &[&str],
    approval_changed: bool,
    base: &FileSide,
    head: &FileSide,
) -> FileReview {
    let moved: BTreeSet<&str> = moved.iter().copied().collect();
    let Ok(read) = read_branch(base, head, paths, &moved, crate::RULES, false);
    let all: Vec<&str> = paths.iter().map(|&(_, p)| p).collect();
    let review = read.review(needs_review_of(&all, approval_changed, &read.changes));
    FileReview {
        review,
        cuts: read.cuts,
        withheld: read.withheld,
    }
}

/// A side of a branch's diff the reading reads its files from.
pub(crate) trait Reading {
    /// What reading the side fails with: git failing, for the gate's own side.
    type Error;
    /// The side's revision, as the cuts print it.
    fn revision(&self) -> &str;
    /// Whether the side holds a file at `path`.
    fn holds(&self, path: &str) -> Result<bool, Self::Error>;
    /// The bytes of the file the side holds at `path`; none when they can't be had.
    fn bytes(&self, path: &str) -> Result<Option<Vec<u8>>, Self::Error>;
    /// The side's spec graph and the violations its loading gave, loaded from its files
    /// under `specs/`, its `ARCHITECTURE.md` and `CAPABILITIES.md` and its live
    /// `changes/<id>/change.md` files, and no other (`cuts`).
    fn graph(&self) -> Result<(Graph, Vec<Violation>), Self::Error>;
}

impl Reading for FileSide<'_> {
    type Error = Infallible;

    fn revision(&self) -> &str {
        self.revision
    }

    fn holds(&self, path: &str) -> Result<bool, Infallible> {
        Ok(self.paths.contains(path))
    }

    fn bytes(&self, path: &str) -> Result<Option<Vec<u8>>, Infallible> {
        Ok(self
            .paths
            .contains(path)
            .then(|| (self.read)(path))
            .flatten())
    }

    /// A file the model reads whose bytes can't be had is a violation of this side's
    /// loading; any other such file is left out.
    fn graph(&self) -> Result<(Graph, Vec<Violation>), Infallible> {
        let mut files = Files::new();
        let mut unreadable = Vec::new();
        for path in self.paths.iter().filter(|p| graph_file(p)) {
            match (self.read)(path) {
                Some(bytes) => {
                    files.insert(PathBuf::from(path), bytes);
                }
                None if model_reads(path) => unreadable.push(Violation {
                    path: PathBuf::from(path),
                    rule: "gate/review-from-files",
                    message: "its bytes can't be had, so this side's specs can't be loaded".into(),
                }),
                None => {}
            }
        }
        let (graph, mut violations) = clm_model::load_files(&files);
        violations.extend(unreadable);
        Ok((graph, violations))
    }
}

/// A file the graph the cuts are read over is loaded from.
fn graph_file(path: &str) -> bool {
    path.starts_with(SPECS)
        || path == "ARCHITECTURE.md"
        || path == "CAPABILITIES.md"
        || live_change(path).is_some_and(|(_, rest)| rest == "change.md")
}

/// A file of the graph that the model reads: a `SPEC.md` or an asset under `specs/`, either
/// human layer, or a live `change.md`; any other file under `specs/` it leaves alone.
fn model_reads(path: &str) -> bool {
    let Some(rest) = path.strip_prefix(SPECS) else {
        return graph_file(path);
    };
    let parts: Vec<&str> = rest.split('/').collect();
    let (last, dirs) = parts.split_last().unwrap_or((&"", &[]));
    dirs.contains(&"assets") || *last == "SPEC.md"
}

/// The gate's own side of a branch: the revision `rev` read through git.
pub(crate) struct GitSide<'a> {
    pub(crate) at: &'a At<'a>,
    pub(crate) rev: &'a str,
}

impl GitSide<'_> {
    /// Loads `specs/` as committed at the side's revision, with the `change.md` of each live
    /// change there, so the component frontmatter keys they declare are accepted
    /// (`declared-keys-loaded`), and, when `layers`, with `ARCHITECTURE.md` and
    /// `CAPABILITIES.md` where they exist, so the rules' `serves` resolve (`cuts`). Each file
    /// holds what a checkout of the revision writes (`base-specs-as-checkout`); a revision
    /// without `specs/` loads empty. The files are the root's: `extract_paths` takes paths
    /// from the repository's top, so each is named with the root's prefix and the graph loaded
    /// from the prefix's directory of the extraction.
    pub(crate) fn load(&self, layers: bool) -> Result<(Graph, Vec<Violation>), String> {
        let GitSide { at, rev } = *self;
        let repo = at.repo;
        let mut paths = Vec::new();
        let tops: &[&str] = if layers {
            &["specs", "ARCHITECTURE.md", "CAPABILITIES.md"]
        } else {
            &["specs"]
        };
        for top in tops {
            if self.holds(top)? {
                paths.push(top.to_string());
            }
        }
        // `extract_paths` takes its paths literally, so only files that exist are named: a
        // path that isn't UTF-8 can't be named exactly, and is left out rather than failing
        // the gate.
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
        let prefix = &at.root.prefix;
        let paths: Vec<String> = paths.iter().map(|p| format!("{prefix}{p}")).collect();
        let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
        let tmp = clm_git::extract_paths(repo, rev, &paths).map_err(|e| e.to_string())?;
        Ok(clm_model::load(&tmp.path().join(prefix)))
    }
}

impl Reading for GitSide<'_> {
    type Error = String;

    fn revision(&self) -> &str {
        self.rev
    }

    /// Only git's own refusal means no such file; a git that can't run fails the gate.
    fn holds(&self, path: &str) -> Result<bool, String> {
        match clm_git::git(
            self.at.repo,
            &["cat-file", "-e", &self.at.object(self.rev, path)],
        ) {
            Ok(_) => Ok(true),
            Err(clm_git::Error::Failed { .. }) => Ok(false),
            Err(e) => Err(e.to_string()),
        }
    }

    /// A file git can't read fails the gate: its bytes are always had.
    fn bytes(&self, path: &str) -> Result<Option<Vec<u8>>, String> {
        show_bytes(self.at.repo, &self.at.object(self.rev, path)).map(Some)
    }

    fn graph(&self) -> Result<(Graph, Vec<Violation>), String> {
        self.load(true)
    }
}

/// What the one reading gives of a spec branch.
pub(crate) struct Read {
    /// The head's `change.md` frontmatter of each change a file of which the branch adds or
    /// modifies (`needs-review`).
    pub(crate) changes: Vec<Option<ChangeFrontmatter>>,
    /// The exemptions the branch adds or widens, read from the text.
    pub(crate) added: Vec<AddedExemption>,
    pub(crate) cuts: Vec<Cut>,
    pub(crate) withheld: Vec<(String, String)>,
    /// Whether the graphs widen an exemption, `None` when a side doesn't load.
    pub(crate) widened: Option<bool>,
    /// Whether a requirement states fewer sentences as rules version below 6 counted them.
    pub(crate) shortened_then: bool,
    pub(crate) graphs: Option<Graphs>,
    /// A `gate/deleted-tests` violation for each line a list gains that is no list line or
    /// names no test with a marker in its file at the head.
    pub(crate) deleted_tests: Vec<Violation>,
}

impl Read {
    /// Whether the branch needs review under `RULES`, `by_paths` being what `needs_review_of`
    /// gives: an exemption added or widened, read from the graphs or, when a side doesn't
    /// load, from the text, a cut, or a list whose text can't be had.
    pub(crate) fn review(&self, by_paths: bool) -> bool {
        by_paths
            || self.widened.unwrap_or(!self.added.is_empty())
            || !self.cuts.is_empty()
            || !self.withheld.is_empty()
    }
}

/// A change's `deleted-tests.txt`, live or archived: `changes/[archive/]<id>/deleted-tests.txt`.
pub(crate) fn test_list(file: &str) -> Option<&str> {
    let rest = file.strip_prefix(CHANGES)?;
    let rest = rest.strip_prefix("archive/").unwrap_or(rest);
    let (id, name) = rest.split_once('/')?;
    (name == "deleted-tests.txt" && id != "archive").then_some(id)
}

/// Reads a spec branch from its two sides: `paths` with their statuses, `moved` the archive
/// moves of changes, `rules` the version whose `shortened_then` is read, and `tests` whether
/// the `gate/deleted-tests` violations are read, which only the gate reports.
pub(crate) fn read_branch<S: Reading>(
    base: &S,
    head: &S,
    paths: &[(char, &str)],
    moved: &BTreeSet<&str>,
    rules: u32,
    tests: bool,
) -> Result<Read, S::Error> {
    let changes = change_frontmatters(head, paths, moved)?;
    let added = added_exemptions(base, head, paths)?;
    let lists: Vec<&str> = paths
        .iter()
        .filter(|(status, file)| matches!(status, 'A' | 'M') && test_list(file).is_some())
        .map(|&(_, file)| file)
        .collect();
    if lists.is_empty() && !paths.iter().any(|(_, f)| f.starts_with(SPECS)) {
        return Ok(Read {
            changes,
            added,
            cuts: Vec::new(),
            withheld: Vec::new(),
            widened: Some(false),
            shortened_then: false,
            graphs: None,
            deleted_tests: Vec::new(),
        });
    }
    let mut texts = Vec::new();
    let mut withheld = Vec::new();
    for path in lists {
        let head_bytes = if head.holds(path)? {
            head.bytes(path)?
        } else {
            Some(Vec::new())
        };
        // An archived list with no text of its own at the merge base takes the live one's.
        let live = test_list(path)
            .filter(|_| path.starts_with(ARCHIVE))
            .map(|id| format!("{CHANGES}{id}/deleted-tests.txt"));
        let base_path = if base.holds(path)? {
            Some(path.to_string())
        } else {
            match live {
                Some(live) if base.holds(&live)? => Some(live),
                _ => None,
            }
        };
        let base_bytes = match &base_path {
            Some(p) => Some(base.bytes(p)?),
            None => None,
        };
        let lossy = |b: Vec<u8>| String::from_utf8_lossy(&b).into_owned();
        match (base_bytes, head_bytes) {
            (Some(None), _) => withheld.push((path.to_string(), base.revision().to_string())),
            (_, None) => withheld.push((path.to_string(), head.revision().to_string())),
            (base_bytes, Some(head_bytes)) => {
                texts.push((path, lossy(head_bytes), base_bytes.flatten().map(lossy)))
            }
        }
    }
    let lists: Vec<TestList> = texts
        .iter()
        .map(|(path, head, base)| TestList {
            path,
            head,
            base: base.as_deref(),
        })
        .collect();
    let deleted_tests = if tests {
        deleted_tests(head, &lists)?
    } else {
        Vec::new()
    };

    let (base_graph, base_violations) = base.graph()?;
    let (head_graph, head_violations) = head.graph()?;
    let cuts = cuts::cuts_or_unloaded(
        &Side {
            graph: &base_graph,
            violations: &base_violations,
            revision: base.revision(),
        },
        &Side {
            graph: &head_graph,
            violations: &head_violations,
            revision: head.revision(),
        },
        &lists,
    );
    let loaded = !cuts.iter().any(|c| matches!(c, Cut::Unloaded { .. }));
    let widened = loaded.then(|| !cuts::widened_exemptions(&base_graph, &head_graph).is_empty());
    let shortened_then =
        loaded && rules < 6 && cuts::shortened_under(&base_graph, &head_graph, rules);
    Ok(Read {
        changes,
        added,
        cuts,
        withheld,
        widened,
        shortened_then,
        graphs: Some(Graphs {
            base: (base_graph, base_violations),
            head: (head_graph, head_violations),
        }),
        deleted_tests,
    })
}

/// `gate/deleted-tests`: each line `lists` gain that is no list line, or that names no test
/// with a marker in its file at `head`, each file read once by the name the line gives.
fn deleted_tests<S: Reading>(head: &S, lists: &[TestList]) -> Result<Vec<Violation>, S::Error> {
    let mut files: BTreeMap<String, String> = BTreeMap::new();
    let mut violations = Vec::new();
    for list in lists {
        for line in cuts::added_lines(list) {
            let message = match line {
                Err(line) => format!(
                    "`{line}` is no list line: write `DELETE <file>::<name>`, a `#` comment or a blank line"
                ),
                Ok(ListLine { file, name }) => {
                    let text = match files.get(&file) {
                        Some(text) => text,
                        None => {
                            let text = if head.holds(&file)? {
                                head.bytes(&file)?
                                    .map(|b| String::from_utf8_lossy(&b).into_owned())
                                    .unwrap_or_default()
                            } else {
                                String::new()
                            };
                            files.entry(file.clone()).or_insert(text)
                        }
                    };
                    if cuts::names_a_test(Path::new(&file), text, &name) {
                        continue;
                    }
                    format!("`DELETE {file}::{name}` names no test with a marker in `{file}`")
                }
            };
            violations.push(Violation {
                path: PathBuf::from(list.path),
                rule: "gate/deleted-tests",
                message,
            });
        }
    }
    Ok(violations)
}

/// The frontmatter at `head` of the `change.md` of each change one of whose files is among
/// the `paths` the branch adds or modifies: a live change, or an archived one whose
/// `change.md` the branch adds other than by an archive move (`moved`); `None` for one that
/// is missing, can't be read, isn't UTF-8 or whose frontmatter doesn't parse
/// (`needs-review`).
pub(crate) fn change_frontmatters<S: Reading>(
    head: &S,
    paths: &[(char, &str)],
    moved: &BTreeSet<&str>,
) -> Result<Vec<Option<ChangeFrontmatter>>, S::Error> {
    let new_archived: BTreeSet<&str> = paths
        .iter()
        .filter(|(status, _)| *status == 'A')
        .filter_map(|(_, file)| {
            file.strip_prefix(ARCHIVE)?
                .strip_suffix("/change.md")
                .filter(|id| !id.contains('/') && !moved.contains(id))
        })
        .collect();
    let dirs: BTreeSet<&str> = paths
        .iter()
        .filter(|(status, _)| matches!(status, 'A' | 'M'))
        .filter_map(|&(_, file)| {
            let rest = match live_change(file) {
                Some((_, rest)) => rest,
                None => {
                    file.strip_prefix(ARCHIVE)?
                        .split_once('/')
                        .filter(|(id, _)| new_archived.contains(id))?
                        .1
                }
            };
            Some(&file[..file.len() - rest.len()])
        })
        .collect();
    let mut out = Vec::new();
    for dir in dirs {
        let path = format!("{dir}change.md");
        let fm = if head.holds(&path)? {
            head.bytes(&path)?
                .and_then(|b| String::from_utf8(b).ok())
                .and_then(|t| clm_model::parse_change_frontmatter(&t))
        } else {
            None
        };
        out.push(fm);
    }
    Ok(out)
}

/// One entry of a component's `exemptions` as written: its rule, files and `until`.
type RawExemption = (String, Vec<String>, String);

/// The `exemptions` a `SPEC.md`'s frontmatter holds, in order; an entry of the wrong shape is
/// left out, since loading rejects it (`model/exemptions`).
fn exemptions_in(text: &str) -> Vec<RawExemption> {
    let text = text.replace("\r\n", "\n");
    let Some((yaml, _)) = clm_model::split_frontmatter(&text) else {
        return Vec::new();
    };
    let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(yaml) else {
        return Vec::new();
    };
    let Some(list) = value.get("exemptions").and_then(|v| v.as_sequence()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|entry| {
            let rule = entry.get("rule")?.as_str()?.to_string();
            let until = entry.get("until")?.as_str()?.to_string();
            let files = entry
                .get("files")?
                .as_sequence()?
                .iter()
                .map(|f| f.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>()?;
            Some((rule, files, until))
        })
        .collect()
}

/// An exemption a component's `SPEC.md` holds at `head` that its file at the merge base
/// doesn't cover: added or widened, as `needs-review` counts them (`temporary-exemptions`).
pub(crate) struct AddedExemption {
    pub(crate) file: String,
    pub(crate) rule: String,
    pub(crate) until: String,
}

/// Every added or widened exemption, read from the text: one with no exemption of the same
/// `rule` and `until` at the merge base whose `files` hold all of its own, every exemption of
/// a new component counting.
fn added_exemptions<S: Reading>(
    base: &S,
    head: &S,
    paths: &[(char, &str)],
) -> Result<Vec<AddedExemption>, S::Error> {
    let text = |side: &S, file: &str| -> Result<String, S::Error> {
        Ok(String::from_utf8_lossy(&side.bytes(file)?.unwrap_or_default()).into_owned())
    };
    let mut out = Vec::new();
    for &(status, file) in paths {
        let is_component = file
            .strip_prefix(SPECS)
            .is_some_and(|f| f.ends_with("SPEC.md"))
            && file != "specs/SPEC.md";
        if !is_component || !matches!(status, 'A' | 'M') {
            continue;
        }
        let now = exemptions_in(&text(head, file)?);
        if now.is_empty() {
            continue;
        }
        let before = if status == 'A' {
            Vec::new()
        } else {
            exemptions_in(&text(base, file)?)
        };
        for (rule, files, until) in now {
            let covered = before.iter().any(|(r, fs, u)| {
                *r == rule && *u == until && files.iter().all(|f| fs.contains(f))
            });
            if !covered {
                out.push(AddedExemption {
                    file: file.to_string(),
                    rule,
                    until,
                });
            }
        }
    }
    Ok(out)
}
