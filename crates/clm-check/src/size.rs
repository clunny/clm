//! Each node's size in tokens against its budget (`node-size`, see specs/check/SPEC.md).
//!
//! `size_of` counts from a listing of file lengths and builds for wasm (`size-of-listing`);
//! `size`, behind the `detectors` feature, lists and probes the working tree and calls it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use clm_model::Graph;

use crate::Marker;

/// The budgets of a node whose `size_budget` doesn't set them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budgets {
    pub spec_tokens: u64,
    pub code_tokens: u64,
}

/// A count of tokens and the budget it is held to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Count {
    pub tokens: u64,
    pub budget: u64,
}

impl Count {
    /// Over its budget, the default or a `size_budget` the node set.
    pub fn over(&self) -> bool {
        self.tokens > self.budget
    }
}

/// One node's spec and code tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Size {
    pub node: String,
    pub spec: Count,
    pub code: Count,
}

impl Size {
    /// Either count over its budget (`oversized-is-a-state`).
    pub fn oversized(&self) -> bool {
        self.spec.over() || self.code.over()
    }
}

/// `<node>\tspec <tokens>/<budget>\tcode <tokens>/<budget>[\toversized]`
impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (s, c) = (self.spec, self.code);
        write!(
            f,
            "{}\tspec {}/{}\tcode {}/{}",
            self.node, s.tokens, s.budget, c.tokens, c.budget
        )?;
        if self.oversized() {
            write!(f, "\toversized")?;
        }
        Ok(())
    }
}

/// A file's length in bytes, and its inline test part when it has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Length {
    pub bytes: u64,
    pub tests: Option<TestPart>,
}

/// A Rust file's *test part*: from its first line whose text, trimmed, is exactly
/// `#[cfg(test)]` to the end of the file (`inline-tests-count-where-cited`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TestPart {
    /// The 1-based line it starts on: a marker on this line or later is in it.
    pub line: usize,
    pub bytes: u64,
}

/// A file of one part.
impl From<u64> for Length {
    fn from(bytes: u64) -> Self {
        Length { bytes, tests: None }
    }
}

impl Length {
    /// The length of the file at `path` holding `text`, split in two when it is a `.rs` file
    /// with a `#[cfg(test)]` line.
    pub fn of(path: &Path, text: &[u8]) -> Length {
        let bytes = text.len() as u64;
        let tests = path
            .extension()
            .is_some_and(|e| e == "rs")
            .then(|| {
                let mut start = 0;
                for (i, line) in text.split(|&b| b == b'\n').enumerate() {
                    if line.trim_ascii() == b"#[cfg(test)]" {
                        return Some(TestPart {
                            line: i + 1,
                            bytes: bytes - start as u64,
                        });
                    }
                    start += line.len() + 1;
                }
                None
            })
            .flatten();
        Length { bytes, tests }
    }
}

/// Every node's size, sorted by ID, from `files`: each tracked file that may count, binary
/// and linguist-excluded files already left out, with its length. A node's spec tokens are
/// its `SPEC.md`'s length in `files` (0 when missing). A part of a file holding `markers`
/// counts toward each node they name (the part of a marker's ID before `/`); one holding
/// none counts toward the node that owns its file (`check.coverage/owner`, `tests-count-where-cited`).
pub fn size_of(
    graph: &Graph,
    files: &BTreeMap<PathBuf, Length>,
    markers: &[Marker],
    defaults: Budgets,
) -> Vec<Size> {
    // The nodes cited in each file's source part (false) and test part (true).
    let mut cited: BTreeMap<(&Path, bool), BTreeSet<&str>> = BTreeMap::new();
    for m in markers {
        let in_tests = files
            .get(&m.path)
            .and_then(|l| l.tests)
            .is_some_and(|t| m.line >= t.line);
        let node = m.id.split('/').next().unwrap_or_default();
        cited
            .entry((m.path.as_path(), in_tests))
            .or_default()
            .insert(node);
    }
    let owners: BTreeMap<&Path, Option<&str>> = files
        .keys()
        .map(|f| (f.as_path(), crate::owner(graph, f)))
        .collect();
    graph
        .nodes
        .values()
        .map(|node| {
            let owns = |file: &Path| owners.get(file) == Some(&Some(node.id.as_str()));
            let counts = |file: &Path, in_tests: bool| match cited.get(&(file, in_tests)) {
                Some(nodes) => nodes.contains(node.id.as_str()),
                None => owns(file),
            };
            let code = files
                .iter()
                .map(|(file, len)| {
                    let tests = len.tests.map_or(0, |t| t.bytes);
                    let source = if counts(file, false) {
                        len.bytes - tests
                    } else {
                        0
                    };
                    let test = if tests > 0 && counts(file, true) {
                        tests
                    } else {
                        0
                    };
                    source + test
                })
                .sum();
            let spec = files.get(&node.path).map_or(0, |l| l.bytes);
            let budget = &node.frontmatter.size_budget;
            Size {
                node: node.id.clone(),
                spec: Count {
                    tokens: tokens(spec),
                    budget: budget.spec_tokens.unwrap_or(defaults.spec_tokens),
                },
                code: Count {
                    tokens: tokens(code),
                    budget: budget.code_tokens.unwrap_or(defaults.code_tokens),
                },
            }
        })
        .collect()
}

/// Four bytes to a token, rounded up.
fn tokens(bytes: u64) -> u64 {
    bytes.div_ceil(4)
}

#[cfg(feature = "detectors")]
pub use tree::size;

#[cfg(feature = "detectors")]
mod tree {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs::File;
    use std::io::{self, Read};
    use std::path::{Path, PathBuf};

    use clm_model::Graph;

    use super::{Budgets, Length, Size, size_of};
    use crate::Marker;

    /// Bytes git treats as the start of a file when deciding it is binary.
    const BINARY_PROBE: u64 = 8000;

    /// Every node's size, sorted by ID, for the working tree at `root`: `size_of` of the
    /// tracked files some node may count, and of each node's `SPEC.md` as it is on disk.
    pub fn size(
        root: &Path,
        graph: &Graph,
        markers: &[Marker],
        defaults: Budgets,
    ) -> io::Result<Vec<Size>> {
        let cited: BTreeSet<&Path> = markers
            .iter()
            .filter(|m| {
                let node = m.id.split('/').next().unwrap_or_default();
                graph.nodes.contains_key(node)
            })
            .map(|m| m.path.as_path())
            .collect();
        let mut owned = BTreeSet::new();
        for node in graph.nodes.values() {
            for globs in [&node.frontmatter.paths, &node.frontmatter.tests] {
                owned.extend(clm_git::tracked_matching(root, globs).map_err(io::Error::other)?);
            }
        }
        // Only the files some node may count are probed, not the whole repository.
        let mut files = countable(root, |f| cited.contains(f) || owned.contains(f))?;
        for node in graph.nodes.values() {
            let len = std::fs::metadata(root.join(&node.path))?.len();
            files.insert(node.path.clone(), len.into());
        }
        Ok(size_of(graph, &files, markers, defaults))
    }

    /// Each tracked file `wanted` accepts that counts, and its length: every one but binary,
    /// generated and vendored ones, and any that isn't a regular file in the working tree
    /// (missing, a symlink or a submodule).
    fn countable(
        root: &Path,
        wanted: impl Fn(&Path) -> bool,
    ) -> io::Result<BTreeMap<PathBuf, Length>> {
        let files: Vec<PathBuf> = clm_git::tracked_files(root)
            .map_err(io::Error::other)?
            .into_iter()
            .filter(|f| wanted(f))
            .collect();
        let excluded = marked(root, &files)?;
        let mut out = BTreeMap::new();
        for rel in files.into_iter().filter(|f| !excluded.contains(f)) {
            let path = root.join(&rel);
            let len = match std::fs::symlink_metadata(&path) {
                Ok(m) if m.is_file() => m.len(),
                Ok(_) => continue,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e),
            };
            let rust = rel.extension().is_some_and(|e| e == "rs");
            // A Rust file is read whole, to find its test part.
            let mut head = Vec::new();
            let mut file = File::open(&path)?;
            if rust {
                file.read_to_end(&mut head)?;
            } else {
                file.take(BINARY_PROBE).read_to_end(&mut head)?;
            }
            let probe = &head[..head.len().min(BINARY_PROBE as usize)];
            if probe.contains(&0) {
                continue;
            }
            let length = if rust {
                Length::of(&rel, &head)
            } else {
                len.into()
            };
            out.insert(rel, length);
        }
        Ok(out)
    }

    /// A path as git reads it, byte for byte where the platform allows.
    #[cfg(unix)]
    fn path_bytes(path: &Path) -> Vec<u8> {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes().to_vec()
    }

    #[cfg(not(unix))]
    fn path_bytes(path: &Path) -> Vec<u8> {
        path.to_string_lossy().replace('\\', "/").into_bytes()
    }

    /// The files whose `linguist-generated` or `linguist-vendored` attribute is set or `true`.
    fn marked(root: &Path, files: &[PathBuf]) -> io::Result<BTreeSet<PathBuf>> {
        if files.is_empty() {
            return Ok(BTreeSet::new());
        }
        let mut stdin = Vec::new();
        for f in files {
            stdin.extend(path_bytes(f));
            stdin.push(0);
        }
        let args = [
            "check-attr",
            "-z",
            "--stdin",
            "linguist-generated",
            "linguist-vendored",
        ];
        let options = clm_git::Options {
            stdin: Some(&stdin),
            ..Default::default()
        };
        let out = clm_git::run(root, "git", &args, options).map_err(io::Error::other)?;
        // Records of `<path> NUL <attribute> NUL <value> NUL`.
        let fields: Vec<&[u8]> = out.split(|&b| b == 0).collect();
        let excluded: BTreeSet<&[u8]> = fields
            .as_chunks::<3>()
            .0
            .iter()
            .filter(|[_, _, value]| *value == b"set" || *value == b"true")
            .map(|[path, _, _]| *path)
            .collect();
        Ok(files
            .iter()
            .filter(|f| excluded.contains(path_bytes(f).as_slice()))
            .cloned()
            .collect())
    }
}
