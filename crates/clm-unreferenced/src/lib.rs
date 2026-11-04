//! `clm unreferenced`: the public items of a workspace's libraries and packages that nothing
//! outside them uses. It reads the tracked manifests, sources and embedding files from the
//! working tree and runs no `cargo`, `npm`, `node` or `python`, so the answer needs no build.

mod python;
mod rust;
mod typescript;

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use tree_sitter::Node;

/// A public item nothing outside its library or package uses, before the allow-list.
pub(crate) struct Finding {
    pub(crate) path: String,
    pub(crate) line: usize,
    /// The crate or package it belongs to, as an `--allow` entry names it.
    pub(crate) library: String,
    /// `<library>::<name>`.
    pub(crate) name: String,
    /// Whether it is a TypeScript or Python package's, which words its line differently.
    pub(crate) package: bool,
}

/// A name a library declares that carries others: a use of `name` from outside the library is
/// a use of each of `words`.
pub(crate) struct Carrier {
    pub(crate) name: String,
    pub(crate) words: Vec<String>,
}

/// A tracked file read from the working tree, with invalid UTF-8 replaced.
pub(crate) struct Source {
    pub(crate) path: PathBuf,
    pub(crate) text: String,
}

/// The tracked files `scan` reads, by what they are.
#[derive(Default)]
struct Tracked {
    cargo: Vec<Source>,
    package_json: Vec<Source>,
    pyproject: Vec<Source>,
    rust: Vec<Source>,
    typescript: Vec<Source>,
    python: Vec<Source>,
    /// `.vue`, `.svelte`, `.astro`, `.html` and `.mdx`: mentions for TypeScript.
    web: Vec<Source>,
    /// `.ipynb`: mentions for Python.
    notebooks: Vec<Source>,
}

impl Tracked {
    /// The list a tracked file at `path` goes in, if `scan` reads it.
    fn list(&mut self, path: &Path) -> Option<&mut Vec<Source>> {
        let name = path.file_name()?.to_str()?;
        match name {
            "Cargo.toml" => return Some(&mut self.cargo),
            "package.json" => return Some(&mut self.package_json),
            "pyproject.toml" => return Some(&mut self.pyproject),
            _ => {}
        }
        Some(match path.extension()?.to_str()? {
            "rs" => &mut self.rust,
            e if typescript::EXTENSIONS.contains(&e) => &mut self.typescript,
            "py" | "pyi" => &mut self.python,
            "vue" | "svelte" | "astro" | "html" | "mdx" => &mut self.web,
            "ipynb" => &mut self.notebooks,
            _ => return None,
        })
    }
}

/// The lines `clm unreferenced` prints, in order: one per public item of a library or package
/// that nothing outside it uses, sorted by path, line and text; then one per `allow` entry that
/// skips no finding, then one per `entry` attribute that marks no item. An error is one line
/// naming the file or command.
pub fn scan(root: &Path, allow: &[String], entry: &[String]) -> Result<Vec<String>, String> {
    // git's error carries its stderr, which may run to several lines.
    let paths = clm_git::tracked_files(root).map_err(|e| one_line(&e.to_string()))?;
    let mut tracked = Tracked::default();
    for path in paths {
        let Some(list) = tracked.list(&path) else {
            continue;
        };
        let bytes = match std::fs::read(root.join(&path)) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        let text = String::from_utf8_lossy(&bytes).into_owned();
        list.push(Source { path, text });
    }

    let (mut findings, marks) = rust::findings(&tracked.cargo, &tracked.rust, entry)?;
    findings.extend(typescript::findings(
        &tracked.package_json,
        &tracked.typescript,
        &tracked.web,
    )?);
    findings.extend(python::findings(
        &tracked.pyproject,
        &tracked.python,
        &tracked.notebooks,
    )?);

    let mut skipped = vec![false; allow.len()];
    findings.retain(|f| {
        let mut keep = true;
        for (entry, skips) in allow.iter().zip(&mut skipped) {
            if *entry == f.library || *entry == f.name {
                *skips = true;
                keep = false;
            }
        }
        keep
    });
    let mut lines: Vec<(String, usize, String)> = findings
        .into_iter()
        .map(|f| {
            let tail = if f.package {
                "is exported but nothing outside its package uses it"
            } else {
                "is public but nothing outside its library uses it"
            };
            let text = format!("{}:{}: {} {tail}", f.path, f.line, f.name);
            (f.path, f.line, text)
        })
        .collect();
    lines.sort();
    let stale = allow
        .iter()
        .zip(skipped)
        .filter(|(_, skips)| !skips)
        .map(|(entry, _)| format!("clm.toml: allow {entry} skips no finding"));
    let unmarked = entry
        .iter()
        .zip(marks)
        .filter(|(_, marks)| !marks)
        .map(|(name, _)| format!("clm.toml: entry {name} marks no item"));
    Ok(lines
        .into_iter()
        .map(|(_, _, text)| text)
        .chain(stale)
        .chain(unmarked)
        .collect())
}

/// Who mentions each word: the library or package a file belongs to, or `None` for a file
/// outside every one.
#[derive(Default)]
pub(crate) struct Mentions<'t> {
    by: HashMap<&'t str, HashSet<Option<usize>>>,
}

impl<'t> Mentions<'t> {
    pub(crate) fn add(&mut self, owner: Option<usize>, words: impl IntoIterator<Item = &'t str>) {
        for word in words {
            self.by.entry(word).or_default().insert(owner);
        }
    }

    /// Whether a file outside `library` mentions `word`.
    pub(crate) fn outside(&self, word: &str, library: usize) -> bool {
        self.by
            .get(word)
            .is_some_and(|owners| owners.iter().any(|o| *o != Some(library)))
    }

    /// The words `carriers`, those of `library`, make used from outside it: each carrier whose
    /// name is mentioned outside, is in `used` already, or is carried by one that is, carries
    /// its words, to any depth.
    pub(crate) fn carried<'c>(
        &self,
        library: usize,
        carriers: &[&'c Carrier],
        mut used: HashSet<&'c str>,
    ) -> HashSet<&'c str> {
        let mut rest = carriers.to_vec();
        loop {
            let (live, dead): (Vec<_>, Vec<_>) = rest
                .into_iter()
                .partition(|c| self.outside(&c.name, library) || used.contains(c.name.as_str()));
            if live.is_empty() {
                return used;
            }
            used.extend(live.iter().flat_map(|c| &c.words).map(String::as_str));
            rest = dead;
        }
    }
}

/// The whole words of `range`, a part of `text` that `node` spans, outside the nodes under it
/// that `plain` says are plain comments.
pub(crate) fn words_outside<'s>(
    text: &'s str,
    node: Node,
    range: std::ops::Range<usize>,
    plain: impl Fn(Node, &str) -> bool,
) -> HashSet<&'s str> {
    let mut comments = Vec::new();
    let mut cursor = node.walk();
    'walk: loop {
        let node = cursor.node();
        if plain(node, &text[node.byte_range()]) {
            comments.push(node.byte_range());
        } else if cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                break 'walk;
            }
        }
    }
    let mut words = HashSet::new();
    let mut start = range.start;
    for comment in comments {
        if comment.start >= start && comment.end <= range.end {
            words.extend(ascii_words(&text[start..comment.start]));
            start = comment.end;
        }
    }
    words.extend(ascii_words(&text[start..range.end]));
    words
}

/// The runs of ASCII letters, digits and `_` in `text`.
pub(crate) fn ascii_words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
}

/// Which of `roots` holds `path` deepest, by index.
pub(crate) fn deepest<'r>(roots: impl Iterator<Item = &'r Path>, path: &Path) -> Option<usize> {
    roots
        .enumerate()
        .filter(|(_, root)| path.starts_with(root))
        .max_by_key(|(_, root)| root.components().count())
        .map(|(index, _)| index)
}

/// `path` with its `.` and `..` components folded, or `None` when `..` climbs above its start.
pub(crate) fn normalise(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            other => out.push(other),
        }
    }
    Some(out)
}

/// The directory a manifest at `path` roots.
pub(crate) fn parent(path: &Path) -> PathBuf {
    path.parent().unwrap_or(Path::new("")).to_path_buf()
}

/// `text` with its runs of whitespace, newlines included, as single spaces.
pub(crate) fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
