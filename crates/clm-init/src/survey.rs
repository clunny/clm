//! `clm survey`: a compact, stable map of a repository's tracked files and the import edges
//! between its directories.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Parser, Query, QueryCursor};

/// Top-level directories that hold CLM's own files, not the project's code.
const SKIPPED: [&str; 4] = ["specs/", "changes/", "backlog/", ".github/"];

/// File names that declare a package or a workspace.
const MANIFESTS: [&str; 12] = [
    "Cargo.toml",
    "package.json",
    "pyproject.toml",
    "setup.py",
    "setup.cfg",
    "requirements.txt",
    "go.mod",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "Gemfile",
    "composer.json",
];

/// Directories whose Markdown files are ADRs.
const ADR_DIRS: [&str; 3] = ["docs/adr", "docs/decisions", "adr"];

/// A surveyed file.
pub(crate) struct File {
    pub lang: &'static str,
    pub lines: usize,
    pub text: String,
}

/// The surveyed files and the import edges between them.
pub(crate) struct Map {
    pub files: BTreeMap<String, File>,
    /// `(from file, to file)` to the number of imports.
    pub edges: BTreeMap<(String, String), usize>,
}

/// Files and lines per language.
type Counts<'a> = BTreeMap<&'a str, (usize, usize)>;

/// The map of the repository at `root`, as text.
pub fn survey(root: &Path) -> Result<String, String> {
    let map = map(root)?;
    let mut out = String::new();

    out.push_str("# directories\n");
    let mut dirs: BTreeMap<Vec<&str>, (&str, Counts)> = BTreeMap::new();
    for (path, file) in &map.files {
        let dir = dir_of(path);
        let entry = &mut dirs.entry(key(dir)).or_insert((dir, BTreeMap::new())).1;
        let counts = entry.entry(file.lang).or_default();
        counts.0 += 1;
        counts.1 += file.lines;
    }
    for (dir, langs) in dirs.values() {
        let counts: Vec<String> = langs
            .iter()
            .map(|(lang, (files, lines))| {
                format!(
                    "{lang} {files} file{}, {lines} line{}",
                    plural(*files),
                    plural(*lines)
                )
            })
            .collect();
        out.push_str(&format!("{dir}  {}\n", counts.join("; ")));
    }

    out.push_str("\n# manifests\n");
    let mut manifests: Vec<&String> = map
        .files
        .keys()
        .filter(|p| MANIFESTS.contains(&file_name(p)))
        .collect();
    manifests.sort_by_key(|p| key(p));
    for path in manifests {
        match members(path, &map.files[path].text) {
            Some(members) => out.push_str(&format!("{path}  workspace: {members}\n")),
            None => out.push_str(&format!("{path}\n")),
        }
    }

    out.push_str("\n# tests\n");
    let tests: BTreeMap<Vec<&str>, &str> = map
        .files
        .iter()
        .filter(|(p, f)| is_test(p, f))
        .map(|(p, _)| (key(dir_of(p)), dir_of(p)))
        .collect();
    for dir in tests.values() {
        out.push_str(&format!("{dir}\n"));
    }

    out.push_str("\n# docs\n");
    let mut docs: BTreeMap<Vec<&str>, String> = BTreeMap::new();
    let mut adrs: BTreeMap<&str, usize> = BTreeMap::new();
    for path in map.files.keys().filter(|p| p.ends_with(".md")) {
        let dir = dir_of(path);
        if ADR_DIRS
            .iter()
            .any(|a| dir == *a || dir.ends_with(&format!("/{a}")))
        {
            *adrs.entry(dir).or_default() += 1;
        } else {
            docs.insert(key(path), path.clone());
        }
    }
    for (dir, n) in adrs {
        docs.insert(key(dir), format!("{dir}  {n} ADR{}", plural(n)));
    }
    for line in docs.values() {
        out.push_str(&format!("{line}\n"));
    }

    out.push_str("\n# edges\n");
    for ((from, to), n) in dir_edges(&map) {
        out.push_str(&format!("{from} -> {to}  {n}\n"));
    }
    Ok(out)
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// Edges between directories, without a directory's edges to itself.
pub(crate) fn dir_edges(map: &Map) -> BTreeMap<(&str, &str), usize> {
    let mut edges = BTreeMap::new();
    for ((from, to), n) in &map.edges {
        let (from, to) = (dir_of(from), dir_of(to));
        if from != to {
            *edges.entry((from, to)).or_default() += n;
        }
    }
    edges
}

/// Reads the tracked files under `root` and the edges between them.
pub(crate) fn map(root: &Path) -> Result<Map, String> {
    let mut files = BTreeMap::new();
    for path in tracked(root)? {
        if SKIPPED.iter().any(|s| path.starts_with(s)) {
            continue;
        }
        // A tracked file deleted from the working tree has nothing to count.
        let Ok(bytes) = std::fs::read(root.join(&path)) else {
            continue;
        };
        if bytes.iter().take(8000).any(|b| *b == 0) {
            continue;
        }
        let lines = bytecount(&bytes) + usize::from(bytes.last().is_some_and(|b| *b != b'\n'));
        let text = String::from_utf8_lossy(&bytes).into_owned();
        files.insert(
            path.clone(),
            File {
                lang: lang(&path),
                lines,
                text,
            },
        );
    }
    let edges = edges(&files);
    Ok(Map { files, edges })
}

/// Each import edge `clm survey` finds between two tracked files of the repository at
/// `root`, as `(importing file, imported file)`, each pair once, sorted: the edges whose
/// directories `clm survey` prints and `clm survey --components` checks, for `check`'s
/// `undeclared-imports`. A `root` outside a git work tree has none; a file that doesn't
/// parse adds none.
pub fn import_edges(root: &Path) -> Result<Vec<(String, String)>, String> {
    match clm_git::git(root, &["rev-parse", "--is-inside-work-tree"]) {
        Ok(inside) if inside == "true" => {}
        Ok(_) | Err(clm_git::Error::Failed { .. }) => return Ok(Vec::new()),
        Err(e) => return Err(e.to_string()),
    }
    Ok(map(root)?.edges.into_keys().collect())
}

/// The files the index holds under `root`, each once, as lossy strings: a name that isn't
/// UTF-8 can't be read back from its lossy form, so reading it fails and it isn't counted.
pub(crate) fn tracked(root: &Path) -> Result<Vec<String>, String> {
    Ok(lossy(
        clm_git::tracked_files(root).map_err(|e| e.to_string())?,
    ))
}

/// The *code files* `check/unowned-files` reads: the tracked files under `root`, binary files
/// and `.github/` included, other than the spec paths.
pub(crate) fn code_files(root: &Path) -> Result<Vec<String>, String> {
    Ok(code_only(tracked(root)?))
}

/// `files` without the spec paths.
pub(crate) fn code_only(mut files: Vec<String>) -> Vec<String> {
    const SPEC_DIRS: [&str; 4] = ["specs/", "changes/", "backlog/", "docs/"];
    const SPEC_FILES: [&str; 4] = [
        "ARCHITECTURE.md",
        "CAPABILITIES.md",
        "CLAUDE.md",
        "AGENTS.md",
    ];
    files.retain(|f| {
        !SPEC_DIRS.iter().any(|d| f.starts_with(d)) && !SPEC_FILES.contains(&f.as_str())
    });
    files
}

pub(crate) fn lossy(paths: Vec<std::path::PathBuf>) -> Vec<String> {
    paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

fn bytecount(bytes: &[u8]) -> usize {
    bytes.iter().filter(|b| **b == b'\n').count()
}

/// The language of a file, by its extension or name.
fn lang(path: &str) -> &'static str {
    let name = file_name(path);
    let ext = name.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    match ext {
        "rs" => "rust",
        "ts" | "tsx" | "mts" | "cts" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "c" | "h" => "c",
        "cc" | "cpp" | "cxx" | "hpp" | "hh" => "c++",
        "cs" => "c#",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "sh" | "bash" => "shell",
        "md" | "markdown" => "markdown",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        "html" | "htm" => "html",
        "css" | "scss" | "sass" => "css",
        "sql" => "sql",
        "txt" => "text",
        _ => match name {
            "Dockerfile" => "docker",
            "Makefile" => "make",
            _ => "other",
        },
    }
}

fn file_name(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, n)| n)
}

/// The directory holding `path`, `.` at the top.
pub(crate) fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map_or(".", |(d, _)| d)
}

/// A sort key that lists a directory's contents before its next sibling.
fn key(path: &str) -> Vec<&str> {
    if path == "." {
        Vec::new()
    } else {
        path.split('/').collect()
    }
}

/// A test file by convention: under a tests directory, named as a test, or Rust with
/// `#[cfg(test)]`.
fn is_test(path: &str, file: &File) -> bool {
    let name = file_name(path);
    dir_of(path)
        .split('/')
        .any(|c| matches!(c, "tests" | "test" | "__tests__"))
        || name.contains("_test.")
        || name.contains(".test.")
        || name.contains(".spec.")
        || (name.starts_with("test_") && name.ends_with(".py"))
        || (file.lang == "rust" && file.text.contains("#[cfg(test)]"))
}

/// A manifest's workspace members, comma-separated, when it declares a workspace.
fn members(path: &str, text: &str) -> Option<String> {
    let list: Vec<String> = match file_name(path) {
        "Cargo.toml" => {
            let manifest: toml::Table = text.parse().ok()?;
            manifest
                .get("workspace")?
                .get("members")?
                .as_array()?
                .iter()
                .filter_map(|m| m.as_str().map(str::to_owned))
                .collect()
        }
        "package.json" => {
            let manifest: serde_json::Value = serde_json::from_str(text).ok()?;
            let ws = &manifest["workspaces"];
            ws.as_array()
                .or(ws["packages"].as_array())?
                .iter()
                .filter_map(|m| m.as_str().map(str::to_owned))
                .collect()
        }
        _ => return None,
    };
    (!list.is_empty()).then(|| list.join(", "))
}

/// Joins a `/`-separated relative path onto `dir`, resolving `.` and `..`; none when it
/// leaves the repository.
fn join(dir: &str, rel: &str) -> Option<String> {
    let mut parts: Vec<&str> = key(dir);
    for part in rel.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            p => parts.push(p),
        }
    }
    Some(if parts.is_empty() {
        ".".into()
    } else {
        parts.join("/")
    })
}

/// `name` in `dir`, `.` being the top.
fn child(dir: &str, name: &str) -> String {
    if dir == "." {
        name.into()
    } else {
        format!("{dir}/{name}")
    }
}

/// The import edges between `files`, by language.
fn edges(files: &BTreeMap<String, File>) -> BTreeMap<(String, String), usize> {
    let mut edges = BTreeMap::new();
    let rust = Extractor::new(tree_sitter_rust::LANGUAGE.into(), RUST_QUERY);
    let ts = Extractor::new(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(), TS_QUERY);
    let tsx = Extractor::new(tree_sitter_typescript::LANGUAGE_TSX.into(), TS_QUERY);
    let python = Extractor::new(tree_sitter_python::LANGUAGE.into(), PYTHON_QUERY);
    let packages = packages(files);
    let python_roots = python_roots(files);
    for (path, file) in files {
        let targets: Vec<String> = if file_name(path) == "Cargo.toml" {
            cargo_deps(path, &file.text, files)
        } else if path.ends_with(".rs") {
            rust.captures(&file.text)
                .into_iter()
                .flat_map(|(cap, text)| rust_targets(path, &cap, &text, files))
                .collect()
        } else if path.ends_with(".tsx") {
            ts_targets(path, &tsx, &file.text, files, &packages)
        } else if file.lang == "typescript" {
            ts_targets(path, &ts, &file.text, files, &packages)
        } else if path.ends_with(".py") {
            python
                .captures(&file.text)
                .into_iter()
                .flat_map(|(_, text)| python_targets(path, &text, &python_roots, files))
                .collect()
        } else {
            Vec::new()
        };
        for target in targets {
            if target != *path {
                *edges.entry((path.clone(), target)).or_default() += 1;
            }
        }
    }
    edges
}

const RUST_QUERY: &str = include_str!("../queries/rust.scm");
const TS_QUERY: &str = include_str!("../queries/typescript.scm");
const PYTHON_QUERY: &str = include_str!("../queries/python.scm");

/// A parser and a query for one language.
struct Extractor {
    language: Language,
    query: Query,
}

impl Extractor {
    fn new(language: Language, query: &str) -> Self {
        // The queries are compiled in and the tests run each one, so a bad one can't ship.
        let query = Query::new(&language, query).expect("query compiles");
        Extractor { language, query }
    }

    /// Each capture's name and text, in source order.
    fn captures(&self, text: &str) -> Vec<(String, String)> {
        let mut parser = Parser::new();
        if parser.set_language(&self.language).is_err() {
            return Vec::new();
        }
        let Some(tree) = parser.parse(text, None) else {
            return Vec::new();
        };
        let mut cursor = QueryCursor::new();
        let mut found = Vec::new();
        let mut captures = cursor.captures(&self.query, tree.root_node(), text.as_bytes());
        while let Some((m, i)) = captures.next() {
            let cap = m.captures()[*i];
            let name = self.query.capture_names()[cap.index as usize];
            if let Ok(t) = cap.node.utf8_text(text.as_bytes()) {
                found.push((name.to_owned(), t.to_owned()));
            }
        }
        found
    }
}

/// The first of `candidates` that is a surveyed file.
fn first(files: &BTreeMap<String, File>, candidates: &[String]) -> Option<String> {
    candidates.iter().find(|c| files.contains_key(*c)).cloned()
}

/// The manifests of a `Cargo.toml`'s path dependencies, direct or through its workspace.
fn cargo_deps(path: &str, text: &str, files: &BTreeMap<String, File>) -> Vec<String> {
    let Ok(manifest) = text.parse::<toml::Table>() else {
        return Vec::new();
    };
    let dir = dir_of(path);
    let mut targets = Vec::new();
    // The manifest's own dependency tables, then each `[target.<cfg>]`'s.
    let platforms = manifest.get("target").and_then(|t| t.as_table());
    let scopes = std::iter::once(&manifest).chain(
        platforms
            .into_iter()
            .flat_map(|t| t.values())
            .filter_map(|v| v.as_table()),
    );
    for scope in scopes {
        for table in ["dependencies", "dev-dependencies", "build-dependencies"] {
            let Some(deps) = scope.get(table).and_then(|d| d.as_table()) else {
                continue;
            };
            targets.extend(deps.iter().filter_map(|(name, dep)| {
                let target = if let Some(rel) = dep.get("path").and_then(|p| p.as_str()) {
                    join(dir, rel)
                } else if dep.get("workspace").and_then(|w| w.as_bool()) == Some(true) {
                    workspace_path(dir, name, files)
                } else {
                    None
                };
                target.and_then(|t| first(files, &[child(&t, "Cargo.toml")]))
            }));
        }
    }
    targets
}

/// The directory a workspace dependency's `path` names, from the nearest manifest above
/// `dir` that declares a workspace.
fn workspace_path(dir: &str, name: &str, files: &BTreeMap<String, File>) -> Option<String> {
    let mut at = Some(dir);
    while let Some(d) = at {
        if let Some(text) = files.get(&child(d, "Cargo.toml")).map(|f| &f.text)
            && let Ok(manifest) = text.parse::<toml::Table>()
            && let Some(ws) = manifest.get("workspace")
        {
            let rel = ws.get("dependencies")?.get(name)?.get("path")?.as_str()?;
            return join(d, rel);
        }
        at = (d != ".").then(|| dir_of(d));
    }
    None
}

/// The files a Rust `mod x;` or `use crate::...` resolves to.
fn rust_targets(
    path: &str,
    capture: &str,
    text: &str,
    files: &BTreeMap<String, File>,
) -> Vec<String> {
    let dir = dir_of(path);
    if capture == "mod" {
        let stem = file_name(path).trim_end_matches(".rs");
        // A crate root or `mod.rs` declares modules beside it; any other file, in a
        // directory named after it.
        let root = matches!(stem, "lib" | "main" | "mod" | "build")
            || matches!(file_name(dir), "tests" | "examples" | "benches" | "bin");
        let base = if root {
            dir.to_owned()
        } else {
            child(dir, stem)
        };
        return first(
            files,
            &[
                child(&base, &format!("{text}.rs")),
                child(&base, &format!("{text}/mod.rs")),
            ],
        )
        .into_iter()
        .collect();
    }
    let Some(rest) = text.strip_prefix("crate::") else {
        return Vec::new();
    };
    let Some(src) = crate_src(path, files) else {
        return Vec::new();
    };
    let paths: Vec<&str> = match rest.strip_prefix('{').and_then(|r| r.strip_suffix('}')) {
        Some(list) => list.split(',').map(str::trim).collect(),
        None => vec![rest],
    };
    paths
        .into_iter()
        .filter_map(|p| {
            let segments: Vec<&str> = p
                .split("::")
                .map_while(|s| {
                    let s = s.split_whitespace().next().unwrap_or("");
                    (!s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_'))
                        .then_some(s)
                })
                .collect();
            (1..=segments.len()).rev().find_map(|n| {
                let module = segments[..n].join("/");
                first(
                    files,
                    &[
                        child(&src, &format!("{module}.rs")),
                        child(&src, &format!("{module}/mod.rs")),
                    ],
                )
            })
        })
        .collect()
}

/// The `src` directory of the crate whose library or binary `path` belongs to.
fn crate_src(path: &str, files: &BTreeMap<String, File>) -> Option<String> {
    let mut at = dir_of(path);
    loop {
        if file_name(at) == "src" && files.contains_key(&child(dir_of(at), "Cargo.toml")) {
            return Some(at.to_owned());
        }
        if at == "." {
            return None;
        }
        at = dir_of(at);
    }
}

/// Package names declared by `package.json` files, to the file.
fn packages(files: &BTreeMap<String, File>) -> BTreeMap<String, String> {
    files
        .iter()
        .filter(|(p, _)| file_name(p) == "package.json")
        .filter_map(|(p, f)| {
            let manifest: serde_json::Value = serde_json::from_str(&f.text).ok()?;
            Some((manifest["name"].as_str()?.to_owned(), p.clone()))
        })
        .collect()
}

/// The files a TypeScript file's relative and workspace-package imports resolve to.
fn ts_targets(
    path: &str,
    extractor: &Extractor,
    text: &str,
    files: &BTreeMap<String, File>,
    packages: &BTreeMap<String, String>,
) -> Vec<String> {
    extractor
        .captures(text)
        .into_iter()
        .filter_map(|(_, spec)| {
            if spec.starts_with("./") || spec.starts_with("../") {
                let base = join(dir_of(path), &spec)?;
                let stem = [".js", ".jsx", ".mjs", ".cjs"]
                    .iter()
                    .find_map(|e| base.strip_suffix(e))
                    .unwrap_or(&base);
                first(
                    files,
                    &[
                        base.clone(),
                        format!("{stem}.ts"),
                        format!("{stem}.tsx"),
                        format!("{base}.js"),
                        format!("{base}/index.ts"),
                        format!("{base}/index.tsx"),
                    ],
                )
            } else {
                packages
                    .iter()
                    .find(|(name, _)| spec == **name || spec.starts_with(&format!("{name}/")))
                    .map(|(_, manifest)| manifest.clone())
            }
        })
        .collect()
}

/// Directories absolute Python imports resolve from: the top, each Python project's
/// directory (holding `pyproject.toml`, `setup.py` or `setup.cfg`), and their `src/`.
fn python_roots(files: &BTreeMap<String, File>) -> BTreeSet<String> {
    let mut roots = BTreeSet::from([".".to_owned(), "src".to_owned()]);
    for path in files.keys() {
        if matches!(file_name(path), "pyproject.toml" | "setup.py" | "setup.cfg") {
            roots.insert(dir_of(path).to_owned());
            roots.insert(child(dir_of(path), "src"));
        }
    }
    roots
}

/// The files a Python `import` or `from ... import` statement in `path` resolves to,
/// trying absolute imports from the roots holding `path` first, innermost first.
fn python_targets(
    path: &str,
    statement: &str,
    roots: &BTreeSet<String>,
    files: &BTreeMap<String, File>,
) -> Vec<String> {
    let holds = |root: &str| root == "." || path.starts_with(&format!("{root}/"));
    let mut absolute: Vec<&str> = roots.iter().map(String::as_str).collect();
    absolute.sort_by_key(|r| (!holds(r), std::cmp::Reverse(key(r).len())));
    let statement = statement.split_whitespace().collect::<Vec<_>>().join(" ");
    // The module `segments` names below `base`: `a/b.py` or `a/b/__init__.py`.
    let exact = |base: &str, segments: &[&str]| -> Option<String> {
        let dir = join(base, &segments.join("/"))?;
        let mut candidates = vec![child(&dir, "__init__.py")];
        if !segments.is_empty() {
            candidates.insert(0, format!("{dir}.py"));
        }
        first(files, &candidates)
    };
    // The longest prefix of `module` that resolves, down to `min` segments.
    let longest = |base: &str, module: &str, min: usize| -> Option<String> {
        let segments: Vec<&str> = module.split('.').filter(|s| !s.is_empty()).collect();
        (min..=segments.len())
            .rev()
            .find_map(|n| exact(base, &segments[..n]))
    };
    let mut targets = BTreeSet::new();
    if let Some(list) = statement.strip_prefix("import ") {
        for module in list.split(',') {
            let module = module.split(" as ").next().unwrap_or("").trim();
            targets.extend(absolute.iter().find_map(|r| longest(r, module, 1)));
        }
    } else if let Some(rest) = statement.strip_prefix("from ") {
        let Some((module, names)) = rest.split_once(" import ") else {
            return Vec::new();
        };
        let dots = module.chars().take_while(|c| *c == '.').count();
        let bases: Vec<String> = if dots == 0 {
            absolute.iter().map(|r| r.to_string()).collect()
        } else {
            (1..dots)
                .try_fold(dir_of(path).to_owned(), |d, _| {
                    (d != ".").then(|| dir_of(&d).to_owned())
                })
                .into_iter()
                .collect()
        };
        let module = &module[dots..];
        let names = names.trim_matches(|c| c == '(' || c == ')' || c == ' ');
        // An absolute module must resolve past the top level, so `from os import path`
        // doesn't find a stray `__init__.py` at the root.
        let min = usize::from(dots == 0);
        for name in names.split(',') {
            let name = name.split(" as ").next().unwrap_or("").trim();
            // A name may be a submodule; otherwise it's something in the module.
            let mut segments: Vec<&str> = module.split('.').filter(|s| !s.is_empty()).collect();
            segments.push(name);
            targets.extend(bases.iter().find_map(|base| {
                (name != "*" && !name.is_empty())
                    .then(|| exact(base, &segments))
                    .flatten()
                    .or_else(|| longest(base, module, min))
            }));
        }
    }
    targets.into_iter().collect()
}
