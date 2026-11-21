//! Where the tests of a file are (`test-spans`): a line scanner for three language
//! families, with no parser, so the crate builds for `wasm32-unknown-unknown`. It skips
//! string and character literals and line and block comments, and nothing else: a
//! regular expression holding a quote, a parenthesis or `/*`, or a brace it still
//! miscounts, can end a span late. A test's span ends no later than the line above the next test (or, in
//! TypeScript and JavaScript, group), so a misread costs that test's markers their
//! standing and shows as `uncounted`, never the tests after it.

use std::path::Path;

use crate::coverage::{Marker, parse_markers};

/// A test, or a named group of tests, in one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestSpan {
    /// The function's, class's or call's first argument's name; empty when a TypeScript or
    /// JavaScript call's first argument isn't a string literal.
    pub name: String,
    /// A `describe`, `test.describe` or Python `Test` class.
    pub group: bool,
    /// First line (1-based): the first of the comment and attribute lines directly above.
    pub first: usize,
    /// Last line (1-based), the one that closes it.
    pub last: usize,
}

#[derive(Clone, Copy)]
enum Family {
    Rust,
    Script,
    Python,
}

fn family(path: &Path) -> Option<Family> {
    match path.extension()?.to_str()? {
        "rs" => Some(Family::Rust),
        "ts" | "tsx" | "mts" | "cts" | "js" | "jsx" | "mjs" | "cjs" => Some(Family::Script),
        "py" => Some(Family::Python),
        _ => None,
    }
}

/// The tests and named groups of tests in one file's text, in the order they start.
pub fn tests_in(path: &Path, text: &str) -> Vec<TestSpan> {
    let lines: Vec<&str> = text.lines().collect();
    let mut found = match family(path) {
        Some(Family::Rust) => rust_tests(&lines),
        Some(Family::Script) => script_tests(&lines),
        Some(Family::Python) => python_tests(&lines),
        None => Vec::new(),
    };
    found.sort_by_key(|t| t.first);
    found
}

/// The `req:` markers in `text` on no line of a test or group, as `Marker`s: nothing for a
/// Markdown file or a language `tests_in` doesn't read.
pub fn markers_outside_tests(path: &Path, text: &str) -> Vec<Marker> {
    split_markers(path, text).map_or_else(Vec::new, |(_, outside)| outside)
}

/// The `req:` markers of a file of a language `tests_in` reads, split into those on a line
/// of a test or group and those on none; `None` for any other file.
pub(crate) fn split_markers(path: &Path, text: &str) -> Option<(Vec<Marker>, Vec<Marker>)> {
    family(path)?;
    let tests = tests_in(path, text);
    let (mut inside, mut outside) = (Vec::new(), Vec::new());
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        let into = if tests.iter().any(|t| t.first <= n && n <= t.last) {
            &mut inside
        } else {
            &mut outside
        };
        into.extend(parse_markers(line).into_iter().map(|s| Marker {
            id: s.id,
            version: s.version,
            path: path.to_path_buf(),
            line: n,
        }));
    }
    Some((inside, outside))
}

/// Whether the line is a comment (`//`, `/*`, `*`).
fn is_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with("/*") || t.starts_with('*')
}

/// The first line (0-based) of the run of lines above `at`, down to `floor`, that `above`
/// accepts, with `at` itself when none does: no blank line, no code line between.
fn first_above(at: usize, floor: usize, above: impl Fn(usize) -> bool) -> usize {
    let mut first = at;
    while first > floor && above(first - 1) {
        first -= 1;
    }
    first
}

// Rust

/// A Rust line as the scanner reads it: its code, with a `//` comment dropped and every
/// block comment and string and character literal blanked, and what it starts inside.
struct RustLine {
    code: String,
    start: RustState,
}

/// What the scanner is inside: code, a `"…"` string (with escapes), a raw string with its
/// number of `#`, or block comments to the depth given.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RustState {
    None,
    Plain,
    Raw(usize),
    Comment(usize),
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The file's lines read for literals and block comments from its first line, each running
/// across lines, a block comment nesting.
fn rust_lines(lines: &[&str]) -> Vec<RustLine> {
    let mut string = RustState::None;
    lines
        .iter()
        .map(|line| {
            let start = string;
            let chars: Vec<char> = line.chars().collect();
            let mut code = String::with_capacity(line.len());
            let blank = |code: &mut String, n: usize| code.extend((0..n).map(|_| ' '));
            let mut i = 0;
            while i < chars.len() {
                let c = chars[i];
                let next = chars.get(i + 1).copied();
                match string {
                    RustState::Comment(depth) => {
                        let n = match (c, next) {
                            ('/', Some('*')) => {
                                string = RustState::Comment(depth + 1);
                                2
                            }
                            ('*', Some('/')) => {
                                string = match depth {
                                    1 => RustState::None,
                                    _ => RustState::Comment(depth - 1),
                                };
                                2
                            }
                            _ => 1,
                        };
                        blank(&mut code, n);
                        i += n;
                    }
                    RustState::Plain => {
                        let n = if c == '\\' { 2 } else { 1 };
                        if c == '"' {
                            string = RustState::None;
                        }
                        blank(&mut code, n.min(chars.len() - i));
                        i += n;
                    }
                    RustState::Raw(hashes) => {
                        let closes = c == '"'
                            && chars.len() > i + hashes
                            && chars[i + 1..=i + hashes].iter().all(|&h| h == '#');
                        let n = if closes {
                            string = RustState::None;
                            1 + hashes
                        } else {
                            1
                        };
                        blank(&mut code, n);
                        i += n;
                    }
                    RustState::None => {
                        if c == '/' && next == Some('/') {
                            break;
                        }
                        if c == '/' && next == Some('*') {
                            string = RustState::Comment(1);
                            blank(&mut code, 2);
                            i += 2;
                        } else if c == '"' {
                            string = RustState::Plain;
                            blank(&mut code, 1);
                            i += 1;
                        } else if let Some(hashes) = raw_string_start(&chars, i) {
                            string = RustState::Raw(hashes);
                            blank(&mut code, 2 + hashes);
                            i += 2 + hashes;
                        } else if c == '\'' {
                            let n = char_literal(&chars, i).unwrap_or(1);
                            if n > 1 {
                                blank(&mut code, n);
                            } else {
                                code.push(c);
                            }
                            i += n;
                        } else {
                            code.push(c);
                            i += 1;
                        }
                    }
                }
            }
            RustLine { code, start }
        })
        .collect()
}

/// The number of `#` of a raw string whose `r` is at `at` (`r"`, `r#"`, with any `b` or `c`
/// prefix before it), when one starts there.
fn raw_string_start(chars: &[char], at: usize) -> Option<usize> {
    if chars[at] != 'r' {
        return None;
    }
    let free = |i: usize| i == 0 || !is_ident(chars[i - 1]);
    let prefixed = at > 0 && matches!(chars[at - 1], 'b' | 'c') && free(at - 1);
    if !(free(at) || prefixed) {
        return None;
    }
    let hashes = chars[at + 1..].iter().take_while(|&&c| c == '#').count();
    (chars.get(at + 1 + hashes) == Some(&'"')).then_some(hashes)
}

/// The length of a character literal starting at `at` (`'{'`, `'\''`, `'\u{7f}'`); `None`
/// for a lifetime or a label (`'a`), which has no closing quote.
fn char_literal(chars: &[char], at: usize) -> Option<usize> {
    match chars.get(at + 1)? {
        '\\' => {
            let close = chars.get(at + 3..)?.iter().position(|&c| c == '\'')?;
            Some(3 + close + 1)
        }
        '\'' => None,
        _ => (chars.get(at + 2) == Some(&'\'')).then_some(3),
    }
}

/// The line (0-based) on which the braces first balance after `at`; `bound` when they
/// don't by then, and `at` when nothing opens and the line ends with `;`.
fn closing(lines: &[RustLine], at: usize, bound: usize) -> usize {
    let mut depth = 0usize;
    let mut opened = false;
    for (i, line) in lines.iter().enumerate().take(bound + 1).skip(at) {
        for c in line.code.chars() {
            if c == '{' {
                depth += 1;
                opened = true;
            } else if c == '}' {
                depth = depth.saturating_sub(1);
                if opened && depth == 0 {
                    return i;
                }
            }
        }
        if !opened && line.code.trim_end().ends_with(';') {
            return i;
        }
    }
    bound
}

/// The attribute's path's last segment is `test`, `rstest` or `test_case`, or ends `_test`.
fn is_test_attribute(line: &str) -> bool {
    let Some(rest) = line.trim_start().strip_prefix("#[") else {
        return false;
    };
    let path: String = rest
        .chars()
        .take_while(|&c| is_ident(c) || c == ':')
        .collect();
    let last = path.rsplit("::").next().unwrap_or("");
    matches!(last, "test" | "rstest" | "test_case") || last.ends_with("_test")
}

/// The name after a `fn` keyword on the line.
fn fn_name(line: &str) -> Option<String> {
    let mut rest = line;
    while let Some(at) = rest.find("fn ") {
        let before = rest[..at].chars().next_back();
        let rest_after = &rest[at + 3..];
        if !before.is_some_and(is_ident) {
            let name: String = rest_after
                .trim_start()
                .chars()
                .take_while(|&c| is_ident(c))
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
        rest = rest_after;
    }
    None
}

/// A Rust test found: the line of its function, its name and its first line (0-based).
struct RustTest {
    at: usize,
    name: String,
    first: usize,
}

fn rust_tests(lines: &[&str]) -> Vec<TestSpan> {
    let read = rust_lines(lines);
    // A comment line or an attribute line, which a test's first lines may be: never a line
    // that starts inside a string, and one that starts inside a block comment unless code
    // follows its close.
    let above = |i: usize| match read[i].start {
        RustState::Comment(_) => {
            let code = read[i].code.trim_start();
            code.is_empty() || code.starts_with("#[")
        }
        RustState::None => read[i].code.trim_start().starts_with("#[") || is_comment(lines[i]),
        RustState::Plain | RustState::Raw(_) => false,
    };
    let mut found: Vec<RustTest> = Vec::new();
    let mut i = 0;
    while i < read.len() {
        if !is_test_attribute(&read[i].code) {
            i += 1;
            continue;
        }
        let attribute = i;
        // From the attribute to the function: more attributes (a multi-line one included),
        // comments and blank lines may lie between.
        let mut j = i;
        let mut brackets = 0i32;
        let mut function = None;
        while j < read.len() {
            let code = &read[j].code;
            let balance_before = brackets;
            brackets += code.matches('[').count() as i32 - code.matches(']').count() as i32;
            let in_attribute = balance_before > 0 || code.trim_start().starts_with("#[");
            if brackets <= 0 {
                if let Some(name) = fn_name(code) {
                    function = Some((j, name));
                    break;
                }
                if !in_attribute && !(code.trim().is_empty() || is_comment(lines[j])) {
                    break;
                }
            }
            j += 1;
        }
        let Some((at, name)) = function else {
            i += 1;
            continue;
        };
        // A test's first lines never reach back onto the line of the test before it.
        let floor = found.last().map_or(0, |t| t.at + 1);
        let first = first_above(at, floor, above).min(attribute);
        found.push(RustTest { at, name, first });
        i = at + 1;
    }
    // A span ends where its braces balance, and no later than the line above the next
    // test's first line, so a brace the scanner misreads costs only its own test.
    let bounds: Vec<usize> = found
        .iter()
        .skip(1)
        .map(|t| t.first - 1)
        .chain([read.len().saturating_sub(1)])
        .collect();
    found
        .into_iter()
        .zip(bounds)
        .map(|(t, bound)| TestSpan {
            last: closing(&read, t.at, bound) + 1,
            name: t.name,
            group: false,
            first: t.first + 1,
        })
        .collect()
}

// TypeScript and JavaScript

const MODIFIERS: [&str; 7] = [
    "skip",
    "skipIf",
    "todo",
    "only",
    "each",
    "concurrent",
    "serial",
];

/// A call of `it`, `test` or `describe` (`test.describe`) at the start of a line, with the
/// modifiers: its group flag and the text from its first `(`.
fn script_call(line: &str) -> Option<(bool, &str)> {
    let mut rest = line.trim_start();
    let head = ["describe", "test", "it"]
        .into_iter()
        .find(|h| rest.starts_with(h))?;
    rest = &rest[head.len()..];
    let mut group = head == "describe";
    while let Some(after) = rest.strip_prefix('.') {
        let word: String = after.chars().take_while(|&c| is_ident(c)).collect();
        if head == "test" && word == "describe" && !group {
            group = true;
        } else if MODIFIERS.contains(&word.as_str()) {
        } else {
            return None;
        }
        rest = &after[word.len()..];
    }
    let rest = rest.trim_start();
    rest.starts_with('(').then_some((group, rest))
}

/// The first argument of the call starting at `args` (its `(`) when it is a string literal.
fn script_name(args: &str) -> String {
    let inner = args[1..].trim_start();
    let Some(quote) = inner
        .chars()
        .next()
        .filter(|c| matches!(c, '\'' | '"' | '`'))
    else {
        return String::new();
    };
    let body = &inner[1..];
    body.find(quote)
        .map_or_else(String::new, |end| body[..end].to_string())
}

fn script_tests(lines: &[&str]) -> Vec<TestSpan> {
    // The comment lines a test's first lines may be: a line starting inside a block comment
    // a line above opened is one unless code follows its close, whether or not it starts
    // with `*`.
    let mut open = ScriptOpen::None;
    let comment: Vec<bool> = lines
        .iter()
        .map(|line| {
            let starts = open == ScriptOpen::Comment;
            let code = script_code(line, &mut open);
            if starts {
                code.trim().is_empty()
            } else {
                is_comment(line)
            }
        })
        .collect();
    // Each call, by the line (0-based) it starts on; its last line is found below. A
    // call's first lines never reach back onto the line of the call before it.
    let mut floor = 0;
    let mut calls: Vec<(usize, TestSpan)> = lines
        .iter()
        .enumerate()
        .filter_map(|(at, line)| {
            let (group, args) = script_call(line)?;
            let first = first_above(at, floor, |i| comment[i]) + 1;
            floor = at + 1;
            let name = script_name(args);
            let span = TestSpan {
                name,
                group,
                first,
                last: first,
            };
            Some((at, span))
        })
        .collect();
    let firsts: Vec<usize> = calls.iter().map(|(_, t)| t.first).collect();
    let end = lines.len().saturating_sub(1);
    for (k, (at, t)) in calls.iter_mut().enumerate() {
        // A test, not a group, ends no later than the line above the next call's first
        // line, so what the scanner misreads costs only that test.
        let bound = match firsts.get(k + 1) {
            Some(&next) if !t.group => next - 2,
            _ => end,
        };
        t.last = script_close(lines, *at, bound) + 1;
    }
    calls.into_iter().map(|(_, t)| t).collect()
}

/// The line that closes the call starting on `at`: where its parentheses balance, and no
/// further group of arguments follows on that line (`it.each(table)('name', fn)`); `bound`
/// when they don't by then. Its reading starts on its own line, whatever a line above left
/// open.
fn script_close(lines: &[&str], at: usize, bound: usize) -> usize {
    let mut depth = 0usize;
    let mut open = ScriptOpen::None;
    for (i, line) in lines.iter().enumerate().take(bound + 1).skip(at) {
        let text = script_code(line, &mut open);
        for (pos, c) in text.char_indices() {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth = depth.saturating_sub(1);
                if depth == 0 && !text[pos + 1..].trim_start().starts_with('(') {
                    return i;
                }
            }
        }
    }
    bound
}

/// What the TypeScript and JavaScript scanner is inside: code, a string or template literal
/// by its quote, or a block comment.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ScriptOpen {
    None,
    Quote(char),
    Comment,
}

/// `line` with a `//` comment dropped and its block comments and string and template
/// literals blanked: a `'` or `"` string ends at the end of its line, and a block comment
/// and a template literal, `${…}` included, run across lines, `open` saying what the line
/// starts inside.
fn script_code(line: &str, open: &mut ScriptOpen) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match *open {
            ScriptOpen::Comment => {
                out.push(' ');
                if c == '*' && chars.next_if_eq(&'/').is_some() {
                    out.push(' ');
                    *open = ScriptOpen::None;
                }
            }
            ScriptOpen::Quote(q) => {
                out.push(' ');
                if c == '\\' {
                    if chars.next().is_some() {
                        out.push(' ');
                    }
                } else if c == q {
                    *open = ScriptOpen::None;
                }
            }
            ScriptOpen::None => match c {
                '/' if chars.peek() == Some(&'/') => break,
                '/' if chars.next_if_eq(&'*').is_some() => {
                    out.push_str("  ");
                    *open = ScriptOpen::Comment;
                }
                '\'' | '"' | '`' => {
                    *open = ScriptOpen::Quote(c);
                    out.push(' ');
                }
                _ => out.push(c),
            },
        }
    }
    if matches!(*open, ScriptOpen::Quote('\'' | '"')) {
        *open = ScriptOpen::None;
    }
    out
}

// Python

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The name of a `def`, `async def` or `class` on the line, and whether it is a class.
fn python_header(line: &str) -> Option<(String, bool)> {
    let t = line.trim_start();
    let (rest, class) = if let Some(r) = t.strip_prefix("class ") {
        (r, true)
    } else if let Some(r) = t.strip_prefix("def ") {
        (r, false)
    } else {
        (t.strip_prefix("async def ")?, false)
    };
    let name: String = rest
        .trim_start()
        .chars()
        .take_while(|&c| is_ident(c))
        .collect();
    let wanted = if class {
        name.starts_with("Test")
    } else {
        name.starts_with("test")
    };
    wanted.then_some((name, class))
}

fn python_tests(lines: &[&str]) -> Vec<TestSpan> {
    let mut out = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        let Some((name, group)) = python_header(line) else {
            continue;
        };
        let first = first_above(at, 0, |i| {
            let t = lines[i].trim_start();
            t.starts_with('@') || t.starts_with('#')
        });
        // The header may span lines; the body is what is indented deeper than it.
        let mut depth = 0i32;
        let mut header_end = at;
        for (i, l) in lines.iter().enumerate().skip(at) {
            depth += l.matches(['(', '[', '{']).count() as i32
                - l.matches([')', ']', '}']).count() as i32;
            header_end = i;
            if depth <= 0 {
                break;
            }
        }
        let own = indent(line);
        let mut last = header_end;
        for (i, l) in lines.iter().enumerate().skip(header_end + 1) {
            if l.trim().is_empty() {
                continue;
            }
            if indent(l) <= own {
                break;
            }
            last = i;
        }
        out.push(TestSpan {
            name,
            group,
            first: first + 1,
            last: last + 1,
        });
    }
    out
}
