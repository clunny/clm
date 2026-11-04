//! Python packages: a directory with a tracked `__init__.py` whose parent has none, whose
//! public items are the names that `__init__.py` exposes, imports followed within it.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use tree_sitter::{Node, Parser};

use crate::{Carrier, Finding, Mentions, Source, ascii_words, one_line, words_outside};

/// The `pyproject.toml` tables whose string values name entry points.
const SCRIPT_TABLES: [&[&str]; 3] = [
    &["project", "scripts"],
    &["project", "gui-scripts"],
    &["tool", "poetry", "scripts"],
];

/// The `pyproject.toml` tables of groups, each group a table of entry points.
const GROUP_TABLES: [&[&str]; 2] = [&["project", "entry-points"], &["tool", "poetry", "plugins"]];

/// The compound statements and clauses whose bodies are still the module's top level.
const COMPOUND: [&str; 14] = [
    "if_statement",
    "elif_clause",
    "else_clause",
    "try_statement",
    "except_clause",
    "except_group_clause",
    "finally_clause",
    "with_statement",
    "for_statement",
    "while_statement",
    "match_statement",
    "case_clause",
    "block",
    "decorated_definition",
];

/// A package: its directory, named by the directory.
struct Package {
    root: PathBuf,
    name: String,
}

/// A module an import names: how many leading dots, and its dotted segments.
#[derive(Clone, Copy)]
struct Spec<'s> {
    dots: usize,
    text: &'s str,
}

/// What a top-level statement binds, in order.
enum Binding<'s> {
    /// A `def`, `class` or assignment, at its 1-based line.
    Defined { name: &'s str, line: usize },
    /// `from <spec> import imported as name`.
    From {
        name: &'s str,
        spec: Spec<'s>,
        imported: &'s str,
    },
    /// `from <spec> import *`.
    Star { spec: Spec<'s> },
    /// `import m` or `import m as name`: a module.
    Module { name: &'s str },
}

/// What one source file binds, carries and mentions.
#[derive(Default)]
struct Module<'s> {
    words: HashSet<&'s str>,
    /// The strings of `__all__`, when the module assigns it.
    all: Option<Vec<&'s str>>,
    bindings: Vec<Binding<'s>>,
    carriers: Vec<Carrier>,
}

/// Where a name is bound.
struct Declared<'s> {
    path: &'s Path,
    line: usize,
    /// The name it is defined as.
    word: &'s str,
}

/// The names the Python packages in `sources` expose that nothing outside their package
/// mentions: in `sources` outside `#` comments, anywhere in `notebooks`, or in the entry points
/// of `pyprojects`.
pub(crate) fn findings(
    pyprojects: &[Source],
    sources: &[Source],
    notebooks: &[Source],
) -> Result<Vec<Finding>, String> {
    let mut scripts = Vec::new();
    for pyproject in pyprojects {
        let table: toml::Table = pyproject.text.parse().map_err(|e: toml::de::Error| {
            format!("{}: {}", pyproject.path.display(), one_line(e.message()))
        })?;
        entry_points(&table, &mut scripts);
    }

    let inits: HashSet<&Path> = sources
        .iter()
        .filter(|s| s.path.file_name().is_some_and(|n| n == "__init__.py"))
        .map(|s| s.path.parent().unwrap_or(Path::new("")))
        .collect();
    let mut packages: Vec<Package> = inits
        .iter()
        .filter(|dir| !dir.parent().is_some_and(|p| inits.contains(p)))
        .filter_map(|dir| {
            let name = dir.file_name()?.to_str()?;
            (!["test", "tests"].contains(&name)).then(|| Package {
                root: dir.to_path_buf(),
                name: name.to_string(),
            })
        })
        .collect();
    // Sorted, the first root holding a file is the outermost: a directory with `__init__.py`
    // below a package is part of it.
    packages.sort_by(|a, b| a.root.cmp(&b.root));
    let owner: HashMap<&Path, usize> = sources
        .iter()
        .filter_map(|s| {
            let index = packages.iter().position(|p| s.path.starts_with(&p.root))?;
            let below = s.path.strip_prefix(&packages[index].root).ok()?;
            (!test_file(below)).then_some((s.path.as_path(), index))
        })
        .collect();

    let mut parser = Parser::new();
    let mut parser = parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .ok()
        .map(|()| parser);
    let modules: HashMap<&Path, Module> = sources
        .iter()
        .map(|s| (s.path.as_path(), parse(&mut parser, &s.text)))
        .collect();

    let mut mentions = Mentions::default();
    for (path, module) in &modules {
        mentions.add(owner.get(path).copied(), module.words.iter().copied());
    }
    for notebook in notebooks {
        mentions.add(None, ascii_words(&notebook.text));
    }
    mentions.add(None, scripts.iter().flat_map(|s| ascii_words(s)));

    let graph = Graph {
        modules: &modules,
        owner: &owner,
        packages: &packages,
        apis: RefCell::default(),
    };
    let mut findings = Vec::new();
    for (index, package) in packages.iter().enumerate() {
        let Some(entry) = graph.file(&package.root.join("__init__.py")) else {
            continue;
        };
        let mut items = Vec::new();
        for name in graph.api(entry, &mut HashSet::new()) {
            if items.iter().any(|(n, _)| *n == name) {
                continue;
            }
            if let Some(declared) = graph.binding(entry, name, &mut HashSet::new()) {
                items.push((name, declared));
            }
        }
        // A name imported under another carries the name it is defined as.
        let renames: Vec<Carrier> = items
            .iter()
            .filter(|(name, d)| *name != d.word)
            .map(|(name, d)| Carrier {
                name: name.to_string(),
                words: vec![d.word.to_string()],
            })
            .collect();
        let carriers: Vec<&Carrier> = modules
            .iter()
            .filter(|(path, _)| owner.get(*path) == Some(&index))
            .flat_map(|(_, m)| &m.carriers)
            .chain(&renames)
            .collect();
        let carried = mentions.carried(index, &carriers, HashSet::new());
        for (name, declared) in items {
            if !(mentions.outside(name, index) || carried.contains(name)) {
                findings.push(Finding {
                    path: declared.path.to_string_lossy().into_owned(),
                    line: declared.line,
                    library: package.name.clone(),
                    name: format!("{}::{name}", package.name),
                    package: true,
                });
            }
        }
    }
    Ok(findings)
}

/// The strings of `pyproject.toml`'s entry-point tables, at any depth.
fn entry_points(table: &toml::Table, out: &mut Vec<String>) {
    let at = |path: &[&str]| {
        path.iter()
            .try_fold(table, |t, key| t.get(*key)?.as_table())
    };
    let mut tables: Vec<&toml::Table> = SCRIPT_TABLES.iter().filter_map(|p| at(p)).collect();
    for groups in GROUP_TABLES.iter().filter_map(|p| at(p)) {
        tables.extend(groups.values().filter_map(toml::Value::as_table));
    }
    for t in tables {
        t.values().for_each(|v| strings(v, out));
    }
}

/// Every string in `value`, at any depth.
fn strings(value: &toml::Value, out: &mut Vec<String>) {
    match value {
        toml::Value::String(s) => out.push(s.clone()),
        toml::Value::Array(values) => values.iter().for_each(|v| strings(v, out)),
        toml::Value::Table(t) => t.values().for_each(|v| strings(v, out)),
        _ => {}
    }
}

/// Whether `below`, a path below its package's directory, is a test file.
fn test_file(below: &Path) -> bool {
    let name = below.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let named = name == "conftest.py"
        || (name.starts_with("test_") && name.ends_with(".py"))
        || name.ends_with("_test.py");
    let in_test_dir = below
        .parent()
        .is_some_and(|dir| dir.iter().any(|c| c == "tests" || c == "test"));
    named || in_test_dir
}

/// The modules and which package each package file belongs to.
struct Graph<'g, 's> {
    modules: &'g HashMap<&'s Path, Module<'s>>,
    owner: &'g HashMap<&'s Path, usize>,
    packages: &'g [Package],
    /// Each module's API, once read.
    apis: RefCell<HashMap<&'s Path, Vec<&'s str>>>,
}

impl<'g, 's> Graph<'g, 's> {
    /// The package file at `path`, if there is one.
    fn file(&self, path: &Path) -> Option<&'s Path> {
        self.owner.get_key_value(path).map(|(&file, _)| file)
    }

    /// The directory a module `spec` imported in `from` names, within `from`'s package.
    fn dir(&self, from: &Path, spec: Spec) -> Option<PathBuf> {
        let package = &self.packages[*self.owner.get(from)?];
        let mut segments = spec.text.split('.').filter(|s| !s.is_empty());
        let mut dir = if spec.dots == 0 {
            if segments.next()? != package.name {
                return None;
            }
            package.root.clone()
        } else {
            let mut dir = from.parent()?.to_path_buf();
            for _ in 1..spec.dots {
                dir = dir.parent()?.to_path_buf();
            }
            dir
        };
        dir.extend(segments);
        // A relative import that climbs out of the package names nothing of it.
        dir.starts_with(&package.root).then_some(dir)
    }

    /// The module file `spec` names from `from`: `m.py` or `m/__init__.py`.
    fn resolve(&self, from: &Path, spec: Spec) -> Option<&'s Path> {
        let dir = self.dir(from, spec)?;
        self.file(&dir.with_extension("py"))
            .filter(|_| !spec.text.trim_matches('.').is_empty())
            .or_else(|| self.file(&dir.join("__init__.py")))
    }

    /// Whether `name` imported from `spec` in `from` is a module of the package.
    fn submodule(&self, from: &Path, spec: Spec, name: &str) -> bool {
        self.dir(from, spec).is_some_and(|dir| {
            let path = dir.join(name);
            self.file(&path.with_extension("py")).is_some()
                || self.file(&path.join("__init__.py")).is_some()
        })
    }

    /// The names the module at `path` exposes: `__all__` when it assigns one, else every
    /// top-level name it binds without a leading `_`.
    fn api(&self, path: &'s Path, visiting: &mut HashSet<&'s Path>) -> Vec<&'s str> {
        let Some(module) = self.modules.get(path) else {
            return Vec::new();
        };
        if let Some(all) = &module.all {
            return all.clone();
        }
        if let Some(api) = self.apis.borrow().get(path) {
            return api.clone();
        }
        if !visiting.insert(path) {
            return Vec::new();
        }
        let mut out = Vec::new();
        for binding in &module.bindings {
            match *binding {
                Binding::Defined { name, .. }
                | Binding::From { name, .. }
                | Binding::Module { name } => {
                    if !name.starts_with('_') {
                        out.push(name);
                    }
                }
                // The target's API by its own rules, its `__all__` included.
                Binding::Star { spec } => {
                    if let Some(target) = self.resolve(path, spec) {
                        out.extend(self.api(target, visiting));
                    }
                }
            }
        }
        visiting.remove(path);
        // Read inside a cycle, an API may lack what the cycle brings, which can only miss
        // dead code.
        self.apis.borrow_mut().insert(path, out.clone());
        out
    }

    /// Where `name`, as the module at `path` binds it first, is defined, following imports
    /// within the package; `None` for a module, or a name from outside the package.
    fn binding(
        &self,
        path: &'s Path,
        name: &str,
        visiting: &mut HashSet<&'s Path>,
    ) -> Option<Declared<'s>> {
        let module = self.modules.get(path)?;
        if !visiting.insert(path) {
            return None;
        }
        let mut found = None;
        for binding in &module.bindings {
            match *binding {
                Binding::Defined { name: n, line } if n == name => {
                    found = Some(Declared {
                        path,
                        line,
                        word: n,
                    });
                    break;
                }
                Binding::From {
                    name: n,
                    spec,
                    imported,
                } if n == name => {
                    if !self.submodule(path, spec, imported)
                        && let Some(target) = self.resolve(path, spec)
                    {
                        found = self.binding(target, imported, visiting);
                    }
                    break;
                }
                Binding::Module { name: n } if n == name => break,
                Binding::Star { spec } => {
                    let Some(target) = self.resolve(path, spec) else {
                        continue;
                    };
                    // A cycle back to a module being read binds nothing here.
                    if self.api(target, &mut HashSet::new()).contains(&name)
                        && let Some(declared) = self.binding(target, name, visiting)
                    {
                        found = Some(declared);
                        break;
                    }
                }
                _ => {}
            }
        }
        visiting.remove(path);
        found
    }
}

/// Reads one source; a file with syntax errors is used as far as it parses.
fn parse<'s>(parser: &mut Option<Parser>, text: &'s str) -> Module<'s> {
    let Some(tree) = parser.as_mut().and_then(|p| p.parse(text, None)) else {
        return Module {
            words: ascii_words(text).collect(),
            ..Module::default()
        };
    };
    let reader = Reader { text };
    let root = tree.root_node();
    let mut module = Module {
        words: reader.words(root),
        ..Module::default()
    };
    reader.statements(root, &mut module);
    module
}

/// Reads parts of one file's syntax tree.
struct Reader<'s> {
    text: &'s str,
}

impl<'s> Reader<'s> {
    fn text(&self, node: Node) -> &'s str {
        &self.text[node.byte_range()]
    }

    /// The whole words of `node` outside `#` comments.
    fn words(&self, node: Node) -> HashSet<&'s str> {
        words_outside(self.text, node, node.byte_range(), |n, _| {
            n.kind() == "comment"
        })
    }

    /// The top-level statements under `node`, compound statements' bodies included.
    fn statements(&self, node: Node, module: &mut Module<'s>) {
        let mut cursor = node.walk();
        let children: Vec<Node> = node.named_children(&mut cursor).collect();
        for child in children {
            match child.kind() {
                "import_statement" => self.import(child, module),
                "import_from_statement" => self.import_from(child, module),
                "expression_statement" => {
                    let mut inner = child.walk();
                    let parts: Vec<Node> = child.named_children(&mut inner).collect();
                    for part in parts {
                        self.assignment(part, child, module);
                    }
                }
                "function_definition" | "class_definition" => self.definition(child, module),
                kind if COMPOUND.contains(&kind) => self.statements(child, module),
                _ => {}
            }
        }
    }

    /// `import a.b` binds `a`, and `import a.b as c` binds `c`, each to a module.
    fn import(&self, node: Node, module: &mut Module<'s>) {
        let mut cursor = node.walk();
        for name in node.children_by_field_name("name", &mut cursor) {
            let bound = match name.kind() {
                "aliased_import" => name.child_by_field_name("alias").map(|a| self.text(a)),
                _ => self.text(name).split('.').next(),
            };
            if let Some(name) = bound {
                module.bindings.push(Binding::Module { name });
            }
        }
    }

    /// `from <spec> import x as y, z` and `from <spec> import *`.
    fn import_from(&self, node: Node, module: &mut Module<'s>) {
        let Some(from) = node.child_by_field_name("module_name") else {
            return;
        };
        let text = self.text(from);
        let spec = Spec {
            dots: text.chars().take_while(|c| *c == '.').count(),
            text: text.trim_start_matches('.').trim(),
        };
        let mut cursor = node.walk();
        if node
            .named_children(&mut cursor)
            .any(|c| c.kind() == "wildcard_import")
        {
            module.bindings.push(Binding::Star { spec });
            return;
        }
        let mut cursor = node.walk();
        for name in node.children_by_field_name("name", &mut cursor) {
            let (imported, bound) = match name.kind() {
                "aliased_import" => (
                    name.child_by_field_name("name").map(|n| self.text(n)),
                    name.child_by_field_name("alias").map(|a| self.text(a)),
                ),
                _ => (Some(self.text(name)), Some(self.text(name))),
            };
            if let (Some(imported), Some(name)) = (imported, bound) {
                module.bindings.push(Binding::From {
                    name,
                    spec,
                    imported,
                });
            }
        }
    }

    /// An assignment, annotated or augmented, in the statement `statement`: the names it binds,
    /// `__all__`'s strings, and an annotation's words.
    fn assignment(&self, node: Node, statement: Node, module: &mut Module<'s>) {
        let augmented = match node.kind() {
            "assignment" => false,
            "augmented_assignment" => true,
            _ => return,
        };
        let Some(left) = node.child_by_field_name("left") else {
            return;
        };
        let right = node.child_by_field_name("right");
        if self.text(left) == "__all__" {
            let plus = node
                .child_by_field_name("operator")
                .is_some_and(|o| self.text(o) == "+=");
            if augmented && !plus {
                return;
            }
            // An `__all__` built otherwise is read as none, so the names the module binds count.
            if let Some(right) = right.filter(|r| matches!(r.kind(), "list" | "tuple")) {
                let all = module.all.get_or_insert_with(Vec::new);
                let mut cursor = right.walk();
                for item in right.named_children(&mut cursor) {
                    let mut inner = item.walk();
                    let content = item
                        .named_children(&mut inner)
                        .find(|c| c.kind() == "string_content");
                    if let Some(content) = content.filter(|_| item.kind() == "string") {
                        all.push(self.text(content));
                    }
                }
            }
            return;
        }
        if augmented {
            return;
        }
        let mut names = Vec::new();
        self.targets(left, &mut names);
        let line = statement.start_position().row + 1;
        if let Some(annotation) = node.child_by_field_name("type") {
            let words: Vec<String> = self
                .words(annotation)
                .into_iter()
                .map(String::from)
                .collect();
            for name in &names {
                module.carriers.push(Carrier {
                    name: name.to_string(),
                    words: words.clone(),
                });
            }
        }
        for name in names {
            module.bindings.push(Binding::Defined { name, line });
        }
        // A chain, `a = b = 1`, nests the rest as the right side.
        if let Some(right) = right.filter(|r| r.kind() == "assignment") {
            self.assignment(right, statement, module);
        }
    }

    /// The names an assignment's target binds: identifiers, in tuples and lists at any depth.
    fn targets(&self, node: Node, out: &mut Vec<&'s str>) {
        match node.kind() {
            "identifier" => out.push(self.text(node)),
            "pattern_list" | "tuple_pattern" | "list_pattern" | "list_splat_pattern" => {
                let mut cursor = node.walk();
                let children: Vec<Node> = node.named_children(&mut cursor).collect();
                for child in children {
                    self.targets(child, out);
                }
            }
            _ => {}
        }
    }

    /// A `def` or `class`: binds its name and carries its annotations.
    fn definition(&self, node: Node, module: &mut Module<'s>) {
        let Some(name) = node.child_by_field_name("name").map(|n| self.text(n)) else {
            return;
        };
        let mut words = Vec::new();
        if node.kind() == "function_definition" {
            if let Some(parameters) = node.child_by_field_name("parameters") {
                self.annotations(parameters, &mut words);
            }
            if let Some(returns) = node.child_by_field_name("return_type") {
                words.extend(self.words(returns).into_iter().map(String::from));
            }
        } else {
            if let Some(bases) = node.child_by_field_name("superclasses") {
                words.extend(self.words(bases).into_iter().map(String::from));
            }
            if let Some(body) = node.child_by_field_name("body") {
                self.annotations(body, &mut words);
            }
        }
        module.bindings.push(Binding::Defined {
            name,
            line: node.start_position().row + 1,
        });
        if !words.is_empty() {
            module.carriers.push(Carrier {
                name: name.to_string(),
                words,
            });
        }
    }

    /// The words of every annotation (`type` node) under `node`.
    fn annotations(&self, node: Node, out: &mut Vec<String>) {
        if node.kind() == "type" {
            out.extend(self.words(node).into_iter().map(String::from));
            return;
        }
        let mut cursor = node.walk();
        let children: Vec<Node> = node.named_children(&mut cursor).collect();
        for child in children {
            self.annotations(child, out);
        }
    }
}
