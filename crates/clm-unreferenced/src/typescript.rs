//! TypeScript and JavaScript packages: a tracked `package.json` with a string `name`, whose
//! public items are the names its entry modules export, re-exports followed within it.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use tree_sitter::{Language, Node, Parser};

use crate::{Carrier, Finding, Mentions, Source, ascii_words, deepest, normalise, one_line};

/// The extensions of TypeScript and JavaScript sources, in the order entry modules are tried.
pub(crate) const EXTENSIONS: [&str; 8] = ["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];

/// The extensions of a built file, `.d.*` first, each tried with the source extensions.
const BUILT: [&str; 7] = [".d.ts", ".d.mts", ".d.cts", ".js", ".mjs", ".cjs", ".jsx"];

/// The `package.json` fields an entry module is read from, in order.
const ENTRY_FIELDS: [&str; 6] = ["exports", "main", "module", "browser", "types", "typings"];

/// A package: a tracked `package.json` whose `name` is a string, rooted at its directory.
struct Package {
    root: PathBuf,
    name: String,
    /// The string values of its entry fields, in order.
    entries: Vec<String>,
}

/// Where a name is declared: the file, the 1-based line and the name it is declared as.
#[derive(Clone)]
struct Declared<'s> {
    path: &'s Path,
    line: usize,
    word: &'s str,
}

/// What one module exports, before re-exports are followed.
enum Export<'s> {
    /// `export <declaration>` or `export { local as name }`.
    Local { local: &'s str, name: &'s str },
    /// `export { imported as name } from '<spec>'`.
    From {
        spec: &'s str,
        imported: &'s str,
        name: &'s str,
    },
    /// `export * from '<spec>'`.
    Star { spec: &'s str },
}

/// What one source file declares, exports, imports, carries and mentions.
#[derive(Default)]
struct Module<'s> {
    words: HashSet<&'s str>,
    /// Each top-level declared name, at its first declaration's line.
    declared: HashMap<&'s str, usize>,
    /// Each name an `import { imported as local } from '<spec>'` binds: `local` to
    /// `(spec, imported)`.
    imports: HashMap<&'s str, (&'s str, &'s str)>,
    exports: Vec<Export<'s>>,
    carriers: Vec<Carrier>,
}

/// The exported names of the TypeScript packages `manifests` declare that nothing outside
/// their package mentions: in `sources` outside plain comments, or anywhere in `web`.
pub(crate) fn findings(
    manifests: &[Source],
    sources: &[Source],
    web: &[Source],
) -> Result<Vec<Finding>, String> {
    let mut packages = Vec::new();
    for manifest in manifests {
        packages.extend(package(&manifest.path, &manifest.text)?);
    }
    let mut parsers = (
        parser(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        parser(tree_sitter_typescript::LANGUAGE_TSX.into()),
    );
    let modules: HashMap<&Path, Module> = sources
        .iter()
        .map(|s| (s.path.as_path(), parse(&mut parsers, s)))
        .collect();
    // The package each package file belongs to: a source under a package root, by the deepest
    // root, other than a test file or one under `node_modules`.
    let owner: HashMap<&Path, usize> = sources
        .iter()
        .filter(|s| !s.path.components().any(|c| c.as_os_str() == "node_modules"))
        .filter_map(|s| {
            let index = deepest(packages.iter().map(|p| p.root.as_path()), &s.path)?;
            let below = s.path.strip_prefix(&packages[index].root).ok()?;
            (!test_file(below)).then_some((s.path.as_path(), index))
        })
        .collect();

    let mut mentions = Mentions::default();
    for (path, module) in &modules {
        mentions.add(owner.get(path).copied(), module.words.iter().copied());
    }
    for file in web {
        mentions.add(None, ascii_words(&file.text));
    }

    let graph = Graph {
        modules: &modules,
        owner: &owner,
        cache: RefCell::default(),
    };
    let mut findings = Vec::new();
    for (index, package) in packages.iter().enumerate() {
        let mut carriers: Vec<&Carrier> = modules
            .iter()
            .filter(|(path, _)| owner.get(*path) == Some(&index))
            .flat_map(|(_, m)| &m.carriers)
            .collect();
        let mut items: Vec<(&str, Declared)> = Vec::new();
        for entry in graph.entries(package, index) {
            for (name, declared) in graph.exports(entry, &mut HashSet::new()) {
                let seen = items
                    .iter()
                    .any(|(n, d)| *n == name && d.path == declared.path && d.line == declared.line);
                if !seen {
                    items.push((name, declared));
                }
            }
        }
        // A name exported under another carries the name it is declared as.
        let renames: Vec<Carrier> = items
            .iter()
            .filter(|(name, d)| *name != d.word)
            .map(|(name, d)| Carrier {
                name: name.to_string(),
                words: vec![d.word.to_string()],
            })
            .collect();
        carriers.extend(&renames);
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

/// The package a `package.json` declares, if its `name` is a string.
fn package(path: &Path, text: &str) -> Result<Option<Package>, String> {
    let manifest: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| format!("{}: {}", path.display(), one_line(&e.to_string())))?;
    let Some(name) = manifest.get("name").and_then(|n| n.as_str()) else {
        return Ok(None);
    };
    let mut entries = Vec::new();
    for field in ENTRY_FIELDS {
        match manifest.get(field) {
            // `browser` may map files to replacements, which names no entry.
            Some(serde_json::Value::String(s)) => entries.push(s.clone()),
            Some(value) if field != "browser" => strings(value, &mut entries),
            _ => {}
        }
    }
    Ok(Some(Package {
        root: crate::parent(path),
        name: name.to_string(),
        entries,
    }))
}

/// Every string in `value`, at any depth.
fn strings(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::String(s) => out.push(s.clone()),
        serde_json::Value::Array(values) => values.iter().for_each(|v| strings(v, out)),
        serde_json::Value::Object(map) => map.values().for_each(|v| strings(v, out)),
        _ => {}
    }
}

/// Whether `below`, a path below its package's root, is a test file.
fn test_file(below: &Path) -> bool {
    let name = below.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let dotted = EXTENSIONS.iter().any(|ext| {
        name.ends_with(&format!(".test.{ext}")) || name.ends_with(&format!(".spec.{ext}"))
    });
    let in_test_dir = below.parent().is_some_and(|dir| {
        dir.iter()
            .any(|c| ["__tests__", "test", "tests"].contains(&c.to_str().unwrap_or("")))
    });
    dotted || in_test_dir
}

/// `path` with `suffix` appended to its last component.
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = OsString::from(path);
    s.push(suffix);
    PathBuf::from(s)
}

/// What a path names as a module: itself, then, with `appended`, with each source extension
/// appended and as a directory holding `index`.
fn as_module(path: PathBuf, appended: bool, out: &mut Vec<PathBuf>) {
    out.push(path.clone());
    if appended {
        out.extend(
            EXTENSIONS
                .iter()
                .map(|ext| with_suffix(&path, &format!(".{ext}"))),
        );
        out.extend(
            EXTENSIONS
                .iter()
                .map(|ext| path.join(format!("index.{ext}"))),
        );
    }
}

/// The paths, relative to the package root, an entry field's value is tried as, in order.
fn entry_candidates(value: &str) -> Vec<PathBuf> {
    if value.contains('*') {
        return Vec::new();
    }
    let value = value.strip_prefix("./").unwrap_or(value);
    let mut base = vec![value.to_string()];
    if let Some(stem) = BUILT.iter().find_map(|ext| value.strip_suffix(ext)) {
        base.extend(["ts", "tsx", "mts", "cts"].map(|ext| format!("{stem}.{ext}")));
    }
    let src: Vec<String> = base
        .iter()
        .map(|b| match b.split_once('/') {
            Some((_, rest)) => format!("src/{rest}"),
            None => format!("src/{b}"),
        })
        .collect();
    let mut out = Vec::new();
    // An entry value names a file; only one without extension may name a module by its stem.
    for candidate in base.into_iter().chain(src).map(PathBuf::from) {
        let bare = candidate.extension().is_none();
        as_module(candidate, bare, &mut out);
    }
    out
}

/// The modules and which package each package file belongs to.
struct Graph<'g, 's> {
    modules: &'g HashMap<&'s Path, Module<'s>>,
    owner: &'g HashMap<&'s Path, usize>,
    /// Each module's exports, once read.
    cache: RefCell<Exports<'s>>,
}

/// The exports of each module read so far.
type Exports<'s> = HashMap<&'s Path, Vec<(&'s str, Declared<'s>)>>;

impl<'g, 's> Graph<'g, 's> {
    /// The package file of package `index` at `path`, if there is one.
    fn file(&self, path: &Path, index: usize) -> Option<&'s Path> {
        let (&file, _) = self.owner.get_key_value(normalise(path)?.as_path())?;
        (self.owner[file] == index).then_some(file)
    }

    /// The entry modules of `package`, the `index`th: what its entry fields resolve to, else
    /// the first of `src/index.<ext>` and `index.<ext>`.
    fn entries(&self, package: &Package, index: usize) -> Vec<&'s Path> {
        let mut out: Vec<&Path> = Vec::new();
        for value in &package.entries {
            let found = entry_candidates(value)
                .into_iter()
                .find_map(|c| self.file(&package.root.join(c), index));
            if let Some(found) = found.filter(|f| !out.contains(f)) {
                out.push(found);
            }
        }
        if out.is_empty() {
            let fallback = ["src/index", "index"].into_iter().flat_map(|stem| {
                EXTENSIONS
                    .iter()
                    .map(move |ext| package.root.join(format!("{stem}.{ext}")))
            });
            out.extend(fallback.into_iter().find_map(|c| self.file(&c, index)));
        }
        out
    }

    /// The package file a relative specifier in `from` names: as given, with a source
    /// extension appended, as a directory holding `index`, or with a JavaScript extension
    /// replaced by its TypeScript one.
    fn resolve(&self, from: &Path, spec: &str) -> Option<&'s Path> {
        if !spec.starts_with('.') {
            return None;
        }
        let index = *self.owner.get(from)?;
        let target = from.parent().unwrap_or(Path::new("")).join(spec);
        let mut candidates = Vec::new();
        as_module(target, true, &mut candidates);
        for (js, ts) in [
            (".js", ".ts"),
            (".mjs", ".mts"),
            (".cjs", ".cts"),
            (".jsx", ".tsx"),
        ] {
            if let Some(stem) = spec.strip_suffix(js) {
                let stem = from.parent().unwrap_or(Path::new("")).join(stem);
                candidates.push(with_suffix(&stem, ts));
            }
        }
        candidates.into_iter().find_map(|c| self.file(&c, index))
    }

    /// The names the package file at `path` exports, each with its declaration, re-exports
    /// followed within the package; `visiting` holds the modules being read, so a cycle ends.
    fn exports(
        &self,
        path: &'s Path,
        visiting: &mut HashSet<&'s Path>,
    ) -> Vec<(&'s str, Declared<'s>)> {
        let mut out: Vec<(&str, Declared)> = Vec::new();
        let Some(module) = self.modules.get(path) else {
            return out;
        };
        if let Some(exports) = self.cache.borrow().get(path) {
            return exports.clone();
        }
        if !visiting.insert(path) {
            return out;
        }
        // A module's own exports shadow what `export *` brings, so they are read first.
        let (stars, own): (Vec<&Export>, Vec<&Export>) = module
            .exports
            .iter()
            .partition(|e| matches!(e, Export::Star { .. }));
        for export in own.into_iter().chain(stars) {
            let found: Vec<(&str, Declared)> = match *export {
                Export::Local { local, name } => {
                    let declared = match module.declared.get(local) {
                        Some(&line) => Some(Declared {
                            path,
                            line,
                            word: local,
                        }),
                        None => module.imports.get(local).and_then(|&(spec, imported)| {
                            self.follow(path, spec, imported, visiting)
                        }),
                    };
                    declared.map(|d| (name, d)).into_iter().collect()
                }
                Export::From {
                    spec,
                    imported,
                    name,
                } => self
                    .follow(path, spec, imported, visiting)
                    .map(|d| (name, d))
                    .into_iter()
                    .collect(),
                Export::Star { spec } => match self.resolve(path, spec) {
                    Some(target) => self.exports(target, visiting),
                    None => Vec::new(),
                },
            };
            for (name, declared) in found {
                if !out.iter().any(|(n, _)| *n == name) {
                    out.push((name, declared));
                }
            }
        }
        visiting.remove(path);
        // Read inside a cycle, a module's exports may lack what the cycle brings, which can
        // only miss dead code.
        self.cache.borrow_mut().insert(path, out.clone());
        out
    }

    /// The declaration of `imported` in the module `spec` names from `from`; never a default.
    fn follow(
        &self,
        from: &Path,
        spec: &str,
        imported: &str,
        visiting: &mut HashSet<&'s Path>,
    ) -> Option<Declared<'s>> {
        if imported == "default" {
            return None;
        }
        let target = self.resolve(from, spec)?;
        self.exports(target, visiting)
            .into_iter()
            .find(|(name, _)| *name == imported)
            .map(|(_, d)| d)
    }
}

/// A parser for `language`, `None` if the grammar can't be loaded, in which case every file
/// reads as declaring and mentioning nothing.
fn parser(language: Language) -> Option<Parser> {
    let mut parser = Parser::new();
    parser.set_language(&language).ok().map(|()| parser)
}

/// Reads one source with the `tsx` grammar for `.tsx` and `.jsx`, else `typescript`; a file
/// with syntax errors is used as far as it parses.
fn parse<'s>(parsers: &mut (Option<Parser>, Option<Parser>), source: &'s Source) -> Module<'s> {
    let jsx = source
        .path
        .extension()
        .is_some_and(|e| e == "tsx" || e == "jsx");
    let parser = if jsx { &mut parsers.1 } else { &mut parsers.0 };
    let text = source.text.as_str();
    let Some(tree) = parser.as_mut().and_then(|p| p.parse(text, None)) else {
        return Module {
            words: ascii_words(text).collect(),
            ..Module::default()
        };
    };
    let reader = Reader { text };
    let mut module = Module {
        words: reader.words(tree.root_node()),
        ..Module::default()
    };
    let root = tree.root_node();
    let mut cursor = root.walk();
    for statement in root.named_children(&mut cursor) {
        reader.statement(statement, &mut module);
    }
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

    /// The whole words of `node` outside plain comments: `/** */` counts.
    fn words(&self, node: Node) -> HashSet<&'s str> {
        crate::words_outside(self.text, node, node.byte_range(), is_plain_comment)
    }

    /// Adds `node`'s words, if there is one, to `out`.
    fn add_words(&self, node: Option<Node>, out: &mut Vec<String>) {
        if let Some(node) = node {
            out.extend(self.words(node).into_iter().map(String::from));
        }
    }

    /// One top-level statement.
    fn statement(&self, node: Node, module: &mut Module<'s>) {
        match node.kind() {
            "export_statement" => self.export(node, module),
            "import_statement" => self.import(node, module),
            _ => {
                self.declaration(node, module);
            }
        }
    }

    /// `import { imported as local } from '<spec>'`, and a default import as `default`.
    fn import(&self, node: Node, module: &mut Module<'s>) {
        let Some(spec) = node.child_by_field_name("source").map(|s| self.string(s)) else {
            return;
        };
        let mut cursor = node.walk();
        let Some(clause) = node
            .named_children(&mut cursor)
            .find(|c| c.kind() == "import_clause")
        else {
            return;
        };
        let mut cursor = clause.walk();
        for part in clause.named_children(&mut cursor) {
            match part.kind() {
                "identifier" => {
                    module.imports.insert(self.text(part), (spec, "default"));
                }
                "named_imports" => {
                    let mut inner = part.walk();
                    for specifier in part.named_children(&mut inner) {
                        let Some(name) = specifier.child_by_field_name("name") else {
                            continue;
                        };
                        let local = specifier.child_by_field_name("alias").unwrap_or(name);
                        module
                            .imports
                            .insert(self.text(local), (spec, self.string(name)));
                    }
                }
                _ => {}
            }
        }
    }

    /// An `export` statement.
    fn export(&self, node: Node, module: &mut Module<'s>) {
        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();
        let default = children.iter().any(|c| c.kind() == "default");
        if let Some(declaration) = node.child_by_field_name("declaration") {
            let names = self.declaration(declaration, module);
            if !default {
                module.exports.extend(
                    names
                        .into_iter()
                        .map(|name| Export::Local { local: name, name }),
                );
            }
            return;
        }
        if default {
            return;
        }
        let spec = node.child_by_field_name("source").map(|s| self.string(s));
        for child in &children {
            match (child.kind(), spec) {
                ("*", Some(spec)) => module.exports.push(Export::Star { spec }),
                ("export_clause", _) => {
                    let mut inner = child.walk();
                    for specifier in child.named_children(&mut inner) {
                        let Some(local) = specifier.child_by_field_name("name") else {
                            continue;
                        };
                        let local = self.string(local);
                        let name = specifier
                            .child_by_field_name("alias")
                            .map_or(local, |a| self.string(a));
                        if local == "default" || name == "default" {
                            continue;
                        }
                        module.exports.push(match spec {
                            Some(spec) => Export::From {
                                spec,
                                imported: local,
                                name,
                            },
                            None => Export::Local { local, name },
                        });
                    }
                }
                _ => {}
            }
        }
    }

    /// The text of a string literal without its quotes, or of an identifier.
    fn string(&self, node: Node) -> &'s str {
        let text = self.text(node);
        if node.kind() == "string" {
            text.get(1..text.len().saturating_sub(1)).unwrap_or("")
        } else {
            text
        }
    }

    /// A declaration at the top level: records the names it declares and what each carries,
    /// and returns the names.
    fn declaration(&self, node: Node, module: &mut Module<'s>) -> Vec<&'s str> {
        let mut names = Vec::new();
        match node.kind() {
            "ambient_declaration" => {
                let mut cursor = node.walk();
                let inner: Vec<Node> = node.named_children(&mut cursor).collect();
                for child in inner {
                    names.extend(self.declaration(child, module));
                }
                return names;
            }
            "lexical_declaration" | "variable_declaration" => {
                let mut cursor = node.walk();
                let declarators: Vec<Node> = node
                    .named_children(&mut cursor)
                    .filter(|c| c.kind() == "variable_declarator")
                    .collect();
                for declarator in declarators {
                    let Some(pattern) = declarator.child_by_field_name("name") else {
                        continue;
                    };
                    let mut bound = Vec::new();
                    self.pattern(pattern, &mut bound);
                    let mut words = Vec::new();
                    self.add_words(declarator.child_by_field_name("type"), &mut words);
                    if let Some(value) = declarator.child_by_field_name("value")
                        && matches!(
                            value.kind(),
                            "arrow_function" | "function_expression" | "function"
                        )
                    {
                        self.signature(value, &mut words);
                    }
                    for name in bound {
                        self.declare(name, &declarator, words.clone(), module);
                        names.push(name);
                    }
                }
                return names;
            }
            _ => {}
        }
        let Some(name) = node.child_by_field_name("name") else {
            return names;
        };
        if !matches!(name.kind(), "identifier" | "type_identifier") {
            return names;
        }
        let mut words = Vec::new();
        match node.kind() {
            "function_declaration" | "generator_function_declaration" | "function_signature" => {
                self.signature(node, &mut words)
            }
            "class_declaration" | "abstract_class_declaration" | "class" => {
                self.class(node, &mut words)
            }
            "interface_declaration" | "type_alias_declaration" => {
                let rest = name.end_byte()..node.end_byte();
                words.extend(
                    crate::words_outside(self.text, node, rest, is_plain_comment)
                        .into_iter()
                        .map(String::from),
                );
            }
            "enum_declaration" | "internal_module" | "module" => {}
            _ => return names,
        }
        let name = self.text(name);
        self.declare(name, &node, words, module);
        names.push(name);
        names
    }

    /// Records `name` as declared at `node`, carrying `words`.
    fn declare(&self, name: &'s str, node: &Node, words: Vec<String>, module: &mut Module<'s>) {
        module
            .declared
            .entry(name)
            .or_insert(node.start_position().row + 1);
        if !words.is_empty() {
            module.carriers.push(Carrier {
                name: name.to_string(),
                words,
            });
        }
    }

    /// The names a binding pattern binds.
    fn pattern(&self, node: Node, out: &mut Vec<&'s str>) {
        match node.kind() {
            "identifier" | "shorthand_property_identifier_pattern" => out.push(self.text(node)),
            "pair_pattern" => {
                if let Some(value) = node.child_by_field_name("value") {
                    self.pattern(value, out);
                }
            }
            "assignment_pattern" | "object_assignment_pattern" => {
                if let Some(left) = node.child_by_field_name("left") {
                    self.pattern(left, out);
                }
            }
            _ => {
                let mut cursor = node.walk();
                let children: Vec<Node> = node.named_children(&mut cursor).collect();
                for child in children {
                    self.pattern(child, out);
                }
            }
        }
    }

    /// The words of a function's type parameters, parameters' types and return type.
    fn signature(&self, node: Node, out: &mut Vec<String>) {
        self.add_words(node.child_by_field_name("type_parameters"), out);
        self.add_words(node.child_by_field_name("return_type"), out);
        if let Some(parameters) = node.child_by_field_name("parameters") {
            let mut cursor = parameters.walk();
            // A constructor's `private x: T` is a private member.
            for parameter in parameters.named_children(&mut cursor) {
                if !self.private(parameter) {
                    self.add_words(parameter.child_by_field_name("type"), out);
                }
            }
        }
    }

    /// The words of a class's type parameters, heritage and the signatures of its members not
    /// marked `private` or named with `#`.
    fn class(&self, node: Node, out: &mut Vec<String>) {
        self.add_words(node.child_by_field_name("type_parameters"), out);
        let mut cursor = node.walk();
        let heritage = node
            .named_children(&mut cursor)
            .find(|c| c.kind() == "class_heritage");
        self.add_words(heritage, out);
        let Some(body) = node.child_by_field_name("body") else {
            return;
        };
        let mut cursor = body.walk();
        for member in body.named_children(&mut cursor) {
            let hashed = member
                .child_by_field_name("name")
                .is_some_and(|n| n.kind() == "private_property_identifier");
            if self.private(member) || hashed {
                continue;
            }
            match member.kind() {
                "public_field_definition" | "property_signature" => {
                    self.add_words(member.child_by_field_name("type"), out)
                }
                "method_definition" | "method_signature" | "abstract_method_signature" => {
                    self.signature(member, out)
                }
                "index_signature" => self.add_words(Some(member), out),
                _ => {}
            }
        }
    }

    /// Whether a member or constructor parameter is marked `private`.
    fn private(&self, node: Node) -> bool {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .any(|c| c.kind() == "accessibility_modifier" && self.text(c) == "private")
    }
}

/// A plain comment, as the TypeScript grammars parse it: any but `/** */`.
fn is_plain_comment(node: Node, text: &str) -> bool {
    node.kind().ends_with("comment") && !text.starts_with("/**")
}
