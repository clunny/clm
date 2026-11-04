//! Rust libraries: what each file declares and mentions, read from tree-sitter's syntax tree
//! with a cursor, and the library items nothing outside their library uses.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use tree_sitter::{Node, Parser, Tree};

use crate::{Carrier, Finding, Mentions, Source, ascii_words, deepest, one_line, words_outside};

/// The attributes that mark an entry point called from outside Rust, beyond those given with
/// `--entry`.
const ENTRY_POINTS: [&str; 6] = [
    "no_mangle",
    "export_name",
    "wasm_bindgen",
    "proc_macro",
    "proc_macro_derive",
    "proc_macro_attribute",
];

/// A package: a tracked `Cargo.toml` with a `[package]` table.
struct Package {
    root: PathBuf,
    /// `[lib] name`, else the package name with `-` replaced by `_`.
    krate: String,
}

/// The public items of the libraries `manifests` declare that no tracked `.rs` file outside
/// their library mentions, other than entry points (items carrying an attribute in
/// `ENTRY_POINTS` or `entry`), and for each `entry` attribute whether it marks an item.
pub(crate) fn findings(
    manifests: &[Source],
    sources: &[Source],
    entry: &[String],
) -> Result<(Vec<Finding>, Vec<bool>), String> {
    let mut packages = Vec::new();
    for manifest in manifests {
        packages.extend(package(&manifest.path, &manifest.text)?);
    }
    let package_of = |path: &Path| deepest(packages.iter().map(|p| p.root.as_path()), path);

    let mut parser = parser();
    let files: HashMap<&Path, File> = sources
        .iter()
        .map(|s| (s.path.as_path(), File::parse(&mut parser, &s.text)))
        .collect();

    // Each library's module tree, from its `src/lib.rs`: the library a file belongs to, and
    // what each library file declares.
    let mut owner: HashMap<&Path, usize> = HashMap::new();
    let mut libraries: Vec<(&Path, usize, Declarations)> = Vec::new();
    // Whether each library file was reached only as test code so far: a file a module
    // declares both as test code and not is read as code that isn't a test.
    let mut test_only: HashMap<&Path, bool> = HashMap::new();
    for (index, package) in packages.iter().enumerate() {
        let lib = package.root.join("src/lib.rs");
        let Some((&lib, _)) = files.get_key_value(lib.as_path()) else {
            continue;
        };
        let mut queue = vec![(lib, false)];
        while let Some((path, test)) = queue.pop() {
            match test_only.get(path) {
                Some(true) if !test => libraries.retain(|(p, _, _)| *p != path),
                Some(_) => continue,
                None => {}
            }
            test_only.insert(path, test);
            owner.insert(path, index);
            let declarations = files[path].declarations(&module_dir(path, path == lib), test);
            for module in &declarations.modules {
                for file in [module.dir.with_extension("rs"), module.dir.join("mod.rs")] {
                    if let Some((&file, _)) = files.get_key_value(file.as_path())
                        && package_of(file) == Some(index)
                    {
                        queue.push((file, test || module.test));
                    }
                }
            }
            libraries.push((path, index, declarations));
        }
    }

    let mut mentions = Mentions::default();
    for (path, file) in &files {
        mentions.add(owner.get(path).copied(), file.words());
    }

    // Entry points: items carrying an attribute in `ENTRY_POINTS` or `entry`.
    let mut marks = vec![false; entry.len()];
    let mut is_entry = |item: &Declaration| {
        let mut found = false;
        for attribute in &item.attributes {
            found |= ENTRY_POINTS.contains(&attribute.as_str());
            for (name, marks) in entry.iter().zip(&mut marks) {
                if name == attribute {
                    *marks = true;
                    found = true;
                }
            }
        }
        found
    };
    let entries: Vec<Vec<bool>> = libraries
        .iter()
        .map(|(_, _, declarations)| declarations.items.iter().map(&mut is_entry).collect())
        .collect();

    // The words each library's carriers make used from outside it, followed to any depth: an
    // entry point's signature counts as an item's used from outside does.
    let mut carriers_of = vec![Vec::new(); packages.len()];
    let mut seeds = vec![HashSet::new(); packages.len()];
    for ((_, package, declarations), entries) in libraries.iter().zip(&entries) {
        carriers_of[*package].extend(&declarations.carriers);
        for (item, entry) in declarations.items.iter().zip(entries) {
            match entry {
                true => seeds[*package].extend(item.signature.words.iter().map(String::as_str)),
                false => carriers_of[*package].push(&item.signature),
            }
        }
    }
    let carried: Vec<HashSet<&str>> = carriers_of
        .iter()
        .zip(seeds)
        .enumerate()
        .map(|(index, (carriers, seed))| mentions.carried(index, carriers, seed))
        .collect();

    let mut findings = Vec::new();
    for ((path, package, declarations), entries) in libraries.iter().zip(&entries) {
        for (item, entry) in declarations.items.iter().zip(entries) {
            if *entry {
                continue;
            }
            let used = mentions.outside(&item.word, *package)
                || carried[*package].contains(item.word.as_str());
            if !used {
                let krate = &packages[*package].krate;
                findings.push(Finding {
                    path: path.to_string_lossy().into_owned(),
                    line: item.line,
                    library: krate.clone(),
                    name: format!("{krate}::{}", item.name),
                    package: false,
                });
            }
        }
    }
    Ok((findings, marks))
}

/// The package a manifest declares, if it has a `[package]` table.
fn package(path: &Path, text: &str) -> Result<Option<Package>, String> {
    let invalid = |why: &str| format!("{}: {why}", path.display());
    let manifest: toml::Table = text
        .parse()
        .map_err(|e: toml::de::Error| invalid(&one_line(e.message())))?;
    let Some(table) = manifest.get("package").and_then(toml::Value::as_table) else {
        return Ok(None);
    };
    let Some(name) = table.get("name").and_then(toml::Value::as_str) else {
        return Err(invalid("[package] has no name"));
    };
    let lib_name = manifest
        .get("lib")
        .and_then(|lib| lib.get("name"))
        .and_then(toml::Value::as_str);
    Ok(Some(Package {
        krate: lib_name.map_or_else(|| name.replace('-', "_"), str::to_string),
        root: crate::parent(path),
    }))
}

/// The directory the out-of-line child modules of the file at `path` sit in: its own
/// directory for the crate root (`lib`) and for `mod.rs`, `x/` beside any other file `x.rs`.
fn module_dir(path: &Path, lib: bool) -> PathBuf {
    let parent = path.parent().unwrap_or(Path::new(""));
    match path.file_stem().and_then(|s| s.to_str()) {
        Some("mod") | None => parent.to_path_buf(),
        _ if lib => parent.to_path_buf(),
        Some(stem) => parent.join(stem),
    }
}

/// The item kinds listed when declared directly in a module.
const MODULE_ITEMS: [&str; 8] = [
    "function_item",
    "struct_item",
    "enum_item",
    "union_item",
    "trait_item",
    "type_item",
    "const_item",
    "static_item",
];

/// The item kinds listed when declared in an inherent `impl` block.
const IMPL_ITEMS: [&str; 2] = ["function_item", "const_item"];

/// A parsed file: its text and syntax tree.
struct File<'s> {
    text: &'s str,
    tree: Option<Tree>,
}

/// A parser for Rust, `None` if the grammar can't be loaded, in which case every file reads as
/// declaring and mentioning nothing.
fn parser() -> Option<Parser> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .ok()
        .map(|()| parser)
}

impl<'s> File<'s> {
    /// Parses `text`; a file with syntax errors is kept as far as it parses.
    fn parse(parser: &mut Option<Parser>, text: &'s str) -> File<'s> {
        let tree = parser.as_mut().and_then(|p| p.parse(text, None));
        File { text, tree }
    }

    /// The whole words of the file outside plain comments: strings and doc comments count.
    fn words(&self) -> HashSet<&'s str> {
        match &self.tree {
            Some(tree) => self.words_in(tree.root_node(), 0..self.text.len()),
            None => ascii_words(self.text).collect(),
        }
    }

    /// The whole words of `range`, a part of `node`, outside the plain comments in it.
    fn words_in(&self, node: Node, range: std::ops::Range<usize>) -> HashSet<&'s str> {
        words_outside(self.text, node, range, |n, _| is_plain_comment(n))
    }

    /// What a library file declares: its child modules sit under `dir`, and with `test` the
    /// whole file is test code, being reached through a module declared as test code.
    fn declarations(&self, dir: &Path, test: bool) -> Declarations {
        let mut out = Declarations::default();
        if let Some(tree) = &self.tree {
            self.module(tree.root_node(), dir, test, &mut out);
            self.carriers(tree.root_node(), test, &mut out.carriers);
        }
        out
    }

    /// The carriers under `node`, at any depth, outside test code: each `#[macro_export]`
    /// `macro_rules!` carries the words of its rules under its name, and each exactly-`pub`
    /// `use` carries the last segment of each `<path> as <alias>` under the alias.
    fn carriers(&self, node: Node, test: bool, out: &mut Vec<Carrier>) {
        let test = test || self.opens_with_test(node);
        for (child, attrs) in self.with_attributes(node) {
            let test = test || attrs.iter().any(Attribute::is_test);
            match child.kind() {
                _ if test => {}
                "macro_definition" => {
                    let exported = attrs.iter().any(|a| a.name == "macro_export");
                    if let (true, Some(name)) = (exported, child.child_by_field_name("name")) {
                        let rules = name.end_byte()..child.end_byte();
                        out.push(carrier(self.text(name), self.words_in(child, rules)));
                    }
                }
                "use_declaration" => {
                    if self.public(child) {
                        self.renames(child, out);
                    }
                }
                _ => self.carriers(child, test, out),
            }
        }
    }

    /// The `<path> as <alias>` clauses under `node`, a `pub use`, as carriers.
    fn renames(&self, node: Node, out: &mut Vec<Carrier>) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if child.kind() != "use_as_clause" {
                self.renames(child, out);
                continue;
            }
            let (Some(path), Some(alias)) = (
                child.child_by_field_name("path"),
                child.child_by_field_name("alias"),
            ) else {
                continue;
            };
            let last = match path.kind() {
                "identifier" => Some(path),
                "scoped_identifier" => path.child_by_field_name("name"),
                _ => None,
            };
            let alias = unraw(self.text(alias));
            if let Some(last) = last.filter(|l| l.kind() == "identifier" && alias != "_") {
                out.push(carrier(alias, HashSet::from([unraw(self.text(last))])));
            }
        }
    }

    /// What the module whose body is `node`, with child modules under `dir`, declares directly.
    fn module(&self, node: Node, dir: &Path, test: bool, out: &mut Declarations) {
        // `#![cfg(test)]`: this module and every module below it is test code.
        let test = test || self.opens_with_test(node);
        for (child, attrs) in self.with_attributes(node) {
            let test = test || attrs.iter().any(Attribute::is_test);
            match child.kind() {
                "mod_item" => {
                    let Some(name) = self.name(child) else {
                        continue;
                    };
                    match child.child_by_field_name("body") {
                        Some(body) => self.module(body, &dir.join(name), test, out),
                        None if !attrs.iter().any(|a| a.name == "path") => {
                            out.modules.push(Module {
                                dir: dir.join(name),
                                test,
                            });
                        }
                        None => {}
                    }
                }
                _ if test => {}
                "impl_item" => {
                    let (Some(ty), Some(body)) = (
                        child.child_by_field_name("type"),
                        child.child_by_field_name("body"),
                    ) else {
                        continue;
                    };
                    let ty = self.type_name(ty);
                    // The header carries under the type's name, and a trait `impl` its members'
                    // signatures too; an inherent `impl`'s listed items carry their own.
                    let mut words = self.words_in(child, child.start_byte()..body.start_byte());
                    let inherent = child.child_by_field_name("trait").is_none();
                    for (item, own) in self.with_attributes(body) {
                        if own.iter().any(Attribute::is_test) {
                            continue;
                        }
                        if !inherent {
                            words.extend(self.signature(item));
                        } else if IMPL_ITEMS.contains(&item.kind()) {
                            self.push(item, Some(ty), [&attrs, &own], out);
                        }
                    }
                    out.carriers.push(carrier(ty, words));
                }
                kind if MODULE_ITEMS.contains(&kind) => self.push(child, None, [&attrs, &[]], out),
                _ => {}
            }
        }
    }

    /// Lists `item` when it is exactly `pub`, named after `ty` for an `impl` item, with the
    /// attributes it carries and those of its `impl` block.
    fn push(&self, item: Node, ty: Option<&str>, attrs: [&[Attribute]; 2], out: &mut Declarations) {
        let Some(name) = self.name(item).filter(|n| self.public(item) && *n != "_") else {
            return;
        };
        out.items.push(Declaration {
            name: ty.map_or_else(|| name.to_string(), |ty| format!("{ty}::{name}")),
            word: name.to_string(),
            signature: carrier(name, self.signature(item)),
            line: item.start_position().row + 1,
            attributes: attrs
                .iter()
                .flat_map(|a| a.iter())
                .map(|a| a.name.clone())
                .collect(),
        });
    }

    /// Whether `item`'s visibility is exactly `pub`.
    fn public(&self, item: Node) -> bool {
        let mut cursor = item.walk();
        item.children(&mut cursor)
            .find(|c| c.kind() == "visibility_modifier")
            .is_some_and(|v| self.is_pub(v))
    }

    /// Whether the visibility modifier `node` is exactly `pub`.
    fn is_pub(&self, node: Node) -> bool {
        self.text(node).split_whitespace().collect::<String>() == "pub"
    }

    /// The whole words, outside comments, of the types in `item`'s signature: a function's
    /// generics, parameters, return type and `where` clause; a `const`'s or `static`'s type; a
    /// type alias's or associated type's generics and right-hand side or bounds; a `struct`'s
    /// or `union`'s generics, `where` clause and exactly-`pub` fields' types; an `enum`'s
    /// generics, `where` clause and variants; a `trait`'s generics, supertraits, `where` clause
    /// and members' signatures.
    fn signature(&self, item: Node) -> HashSet<&'s str> {
        let start = item
            .child_by_field_name("name")
            .map_or(item.start_byte(), |n| n.end_byte());
        let body = item.child_by_field_name("body");
        let head = |end: usize| self.words_in(item, start..end);
        match (item.kind(), body) {
            ("function_item" | "function_signature_item", Some(body)) => head(body.start_byte()),
            ("function_item" | "function_signature_item", None) => head(item.end_byte()),
            ("const_item" | "static_item", _) => item
                .child_by_field_name("type")
                .map(|t| self.words_in(t, t.byte_range()))
                .unwrap_or_default(),
            ("struct_item" | "union_item", Some(body)) => {
                let mut words = head(body.start_byte());
                words.extend(self.words_in(item, body.end_byte()..item.end_byte()));
                words.extend(self.public_fields(body));
                words
            }
            ("trait_item", Some(body)) => {
                let mut words = head(body.start_byte());
                for (member, own) in self.with_attributes(body) {
                    if !own.iter().any(Attribute::is_test) {
                        words.extend(self.signature(member));
                    }
                }
                words
            }
            ("struct_item" | "union_item" | "enum_item" | "type_item" | "associated_type", _) => {
                head(item.end_byte())
            }
            _ => HashSet::new(),
        }
    }

    /// The words of the types of the exactly-`pub` fields of `body`, a `struct`'s or `union`'s
    /// named or tuple fields.
    fn public_fields(&self, body: Node) -> HashSet<&'s str> {
        let mut out = HashSet::new();
        // In a tuple struct a field's visibility is a sibling before its type.
        let mut public = false;
        let mut cursor = body.walk();
        for child in body.named_children(&mut cursor) {
            match child.kind() {
                "field_declaration" => {
                    if let Some(ty) = child.child_by_field_name("type")
                        && self.public(child)
                    {
                        out.extend(self.words_in(ty, ty.byte_range()));
                    }
                }
                "visibility_modifier" => public = self.is_pub(child),
                "attribute_item" | "line_comment" | "block_comment" => {}
                _ => {
                    if std::mem::take(&mut public) {
                        out.extend(self.words_in(child, child.byte_range()));
                    }
                }
            }
        }
        out
    }

    /// The named children of `node` other than comments and inner attributes, each with the
    /// outer attributes written before it.
    fn with_attributes<'t>(&self, node: Node<'t>) -> Vec<(Node<'t>, Vec<Attribute>)> {
        let mut out = Vec::new();
        let mut attrs = Vec::new();
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            match child.kind() {
                "attribute_item" => attrs.push(Attribute::of(self.text(child))),
                "line_comment" | "block_comment" | "inner_attribute_item" => {}
                _ => out.push((child, std::mem::take(&mut attrs))),
            }
        }
        out
    }

    /// The last path segment of an `impl`'s type, without generic arguments, references or
    /// `dyn`.
    fn type_name(&self, node: Node) -> &'s str {
        let inner = match node.kind() {
            "generic_type" | "reference_type" | "pointer_type" => node.child_by_field_name("type"),
            "scoped_type_identifier" | "scoped_identifier" => node.child_by_field_name("name"),
            "dynamic_type" => node.child_by_field_name("trait"),
            _ => None,
        };
        match inner {
            Some(inner) => self.type_name(inner),
            None => self.text(node),
        }
    }

    /// An item's name, a raw identifier without its `r#`.
    fn name(&self, node: Node) -> Option<&str> {
        Some(unraw(self.text(node.child_by_field_name("name")?)))
    }

    /// Whether the body `node` opens with a `#![cfg(…)]` that holds only when `test` is set.
    fn opens_with_test(&self, node: Node) -> bool {
        let mut cursor = node.walk();
        node.named_children(&mut cursor)
            .filter(|n| n.kind() == "inner_attribute_item")
            .any(|n| Attribute::of(self.text(n)).is_test())
    }

    fn text(&self, node: Node) -> &'s str {
        &self.text[node.byte_range()]
    }
}

/// What a library file declares.
#[derive(Default)]
struct Declarations {
    items: Vec<Declaration>,
    /// The out-of-line modules it declares (`mod m;`), outside blocks and without `#[path]`.
    modules: Vec<Module>,
    carriers: Vec<Carrier>,
}

/// A listed item: its name below the crate, the word a use spells, its signature as a carrier
/// under that word, its 1-based line, and the names of the attributes it and its `impl` block
/// carry.
struct Declaration {
    name: String,
    word: String,
    signature: Carrier,
    line: usize,
    attributes: Vec<String>,
}

/// A carrier of `words` under `name`.
fn carrier(name: &str, words: HashSet<&str>) -> Carrier {
    Carrier {
        name: name.to_string(),
        words: words.into_iter().map(String::from).collect(),
    }
}

/// An out-of-line module: the directory its name gives (`<dir>/m` for `mod m;`, whose file is
/// `<dir>/m.rs` or `<dir>/m/mod.rs`), and whether it is declared as test code.
struct Module {
    dir: PathBuf,
    test: bool,
}

/// An attribute's name, its last path segment with any `unsafe(…)` removed, and the text of
/// its arguments.
struct Attribute {
    name: String,
    args: String,
}

impl Attribute {
    /// Reads `#[…]` or `#![…]`.
    fn of(text: &str) -> Attribute {
        let inner = text
            .trim_start_matches(['#', '!'])
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .trim();
        let inner = inner
            .strip_prefix("unsafe")
            .map(str::trim_start)
            .and_then(|rest| rest.strip_prefix('('))
            .and_then(|rest| rest.strip_suffix(')'))
            .map_or(inner, str::trim);
        let end = inner
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == ':' || c == ' '))
            .unwrap_or(inner.len());
        let path: String = inner[..end].split_whitespace().collect();
        Attribute {
            name: path.rsplit("::").next().unwrap_or_default().to_string(),
            args: inner[end..].to_string(),
        }
    }

    /// A `#[cfg(…)]` whose predicate holds only when `test` is set.
    fn is_test(&self) -> bool {
        if self.name != "cfg" {
            return false;
        }
        let tokens = tokens(&self.args);
        let [Token::Open, predicate @ .., Token::Close] = tokens.as_slice() else {
            return false;
        };
        let mut rest = predicate;
        test_only(&mut rest) == Some(true) && rest.iter().all(|t| *t == Token::Comma)
    }
}

/// A token of a `cfg` predicate.
#[derive(PartialEq)]
enum Token<'s> {
    Word(&'s str),
    /// A string literal, whatever it holds.
    Str,
    Open,
    Close,
    Comma,
    Eq,
    Other,
}

/// The tokens of `text`, a `cfg` attribute's arguments.
fn tokens(text: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        out.push(match c {
            '(' => Token::Open,
            ')' => Token::Close,
            ',' => Token::Comma,
            '=' => Token::Eq,
            // A raw string, `r"…"` or `r#"…"#`, holds no escapes.
            'r' if matches!(chars.peek(), Some((_, '"' | '#'))) => {
                let mut hashes = 0;
                while chars.next_if(|&(_, c)| c == '#').is_some() {
                    hashes += 1;
                }
                if chars.next_if(|&(_, c)| c == '"').is_none() {
                    Token::Other
                } else {
                    let close = format!("\"{}", "#".repeat(hashes));
                    let body = &text[chars.peek().map_or(text.len(), |&(at, _)| at)..];
                    let end = body.find(&close).map_or(body.len(), |at| at + close.len());
                    let stop = text.len() - body.len() + end;
                    while chars.next_if(|&(at, _)| at < stop).is_some() {}
                    Token::Str
                }
            }
            '"' => {
                while let Some((_, c)) = chars.next() {
                    match c {
                        '\\' => {
                            chars.next();
                        }
                        '"' => break,
                        _ => {}
                    }
                }
                Token::Str
            }
            c if c.is_alphanumeric() || c == '_' => {
                let mut end = start + c.len_utf8();
                while let Some(&(at, c)) = chars.peek() {
                    if !(c.is_alphanumeric() || c == '_') {
                        break;
                    }
                    end = at + c.len_utf8();
                    chars.next();
                }
                Token::Word(&text[start..end])
            }
            c if c.is_whitespace() => continue,
            _ => Token::Other,
        });
    }
    out
}

/// Reads one predicate from the front of `rest`: whether it holds only when `test` is set
/// (`test`, an `all(…)` with such an argument, or a non-empty `any(…)` whose arguments all are
/// such, never a `not(…)`), or `None` when `rest` doesn't start with a predicate.
fn test_only(rest: &mut &[Token]) -> Option<bool> {
    let (Token::Word(name), after) = rest.split_first()? else {
        return None;
    };
    *rest = after;
    match rest.first() {
        Some(Token::Eq) => {
            *rest = rest.get(2..)?;
            Some(false)
        }
        Some(Token::Open) => {
            *rest = &rest[1..];
            let mut args = Vec::new();
            loop {
                match rest.first()? {
                    Token::Close => break,
                    Token::Comma => *rest = &rest[1..],
                    _ => args.push(test_only(rest)?),
                }
            }
            *rest = &rest[1..];
            Some(match *name {
                "all" => args.iter().any(|a| *a),
                "any" => !args.is_empty() && args.iter().all(|a| *a),
                _ => false,
            })
        }
        _ => Some(*name == "test"),
    }
}

/// A comment that isn't a doc comment, as tree-sitter parses it.
fn is_plain_comment(node: Node) -> bool {
    matches!(node.kind(), "line_comment" | "block_comment")
        && node.child_by_field_name("outer").is_none()
        && node.child_by_field_name("inner").is_none()
}

/// An identifier without the `r#` of a raw identifier, as a use spells it in words.
fn unraw(name: &str) -> &str {
    name.strip_prefix("r#").unwrap_or(name)
}
