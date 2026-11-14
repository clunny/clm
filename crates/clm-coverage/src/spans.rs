//! Where the tests of a file are (`test-spans`): a line scanner for three language
//! families, with no parser, so the crate builds for `wasm32-unknown-unknown`. It is
//! approximate on purpose: a brace or parenthesis inside a string can end a span early or
//! late, which costs a marker its standing and shows as `uncounted`, never a crash.

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
    if family(path).is_none() {
        return Vec::new();
    }
    let tests = tests_in(path, text);
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if tests.iter().any(|t| t.first <= n && n <= t.last) {
            continue;
        }
        out.extend(parse_markers(line).into_iter().map(|s| Marker {
            id: s.id,
            version: s.version,
            path: path.to_path_buf(),
            line: n,
        }));
    }
    out
}

/// `line` up to a `//` comment (Rust, TypeScript, JavaScript).
fn code(line: &str) -> &str {
    line.find("//").map_or(line, |at| &line[..at])
}

/// Whether the line is a comment (`//`, `/*`, `*`).
fn is_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with("/*") || t.starts_with('*')
}

/// The first line (0-based) of the run of lines above `at` that `above` accepts, with `at`
/// itself when none does: no blank line, no code line between.
fn first_above(lines: &[&str], at: usize, above: impl Fn(&str) -> bool) -> usize {
    let mut first = at;
    while first > 0 && above(lines[first - 1]) {
        first -= 1;
    }
    first
}

/// The line (0-based) on which the `open`/`close` characters first balance after `at`,
/// counting outside `//` comments; the end of the file when they never do, and `at` when
/// nothing opens.
fn closing(lines: &[&str], at: usize, open: char, close: char) -> usize {
    let mut depth = 0usize;
    let mut opened = false;
    for (i, line) in lines.iter().enumerate().skip(at) {
        for c in code(line).chars() {
            if c == open {
                depth += 1;
                opened = true;
            } else if c == close {
                depth = depth.saturating_sub(1);
                if opened && depth == 0 {
                    return i;
                }
            }
        }
        if !opened && code(line).trim_end().ends_with(';') {
            return i;
        }
    }
    lines.len().saturating_sub(1)
}

// Rust

/// The attribute's path's last segment is `test`, `rstest` or `test_case`, or ends `_test`.
fn is_test_attribute(line: &str) -> bool {
    let Some(rest) = line.trim_start().strip_prefix("#[") else {
        return false;
    };
    let path: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
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
        if !before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            let name: String = rest_after
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
        rest = rest_after;
    }
    None
}

fn rust_tests(lines: &[&str]) -> Vec<TestSpan> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if !is_test_attribute(lines[i]) {
            i += 1;
            continue;
        }
        let attribute = i;
        // From the attribute to the function: more attributes (a multi-line one included),
        // comments and blank lines may lie between.
        let mut j = i;
        let mut brackets = 0i32;
        let mut function = None;
        while j < lines.len() {
            let line = lines[j];
            let balance_before = brackets;
            brackets +=
                code(line).matches('[').count() as i32 - code(line).matches(']').count() as i32;
            let in_attribute = balance_before > 0 || line.trim_start().starts_with("#[");
            if brackets <= 0 {
                if let Some(name) = fn_name(code(line)) {
                    function = Some((j, name));
                    break;
                }
                if !in_attribute && !(line.trim().is_empty() || is_comment(line)) {
                    break;
                }
            }
            j += 1;
        }
        let Some((at, name)) = function else {
            i += 1;
            continue;
        };
        let first = first_above(lines, at, |l| {
            l.trim_start().starts_with("#[") || is_comment(l)
        })
        .min(attribute);
        let last = closing(lines, at, '{', '}');
        out.push(TestSpan {
            name,
            group: false,
            first: first + 1,
            last: last + 1,
        });
        i = last.max(at) + 1;
    }
    out
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
/// modifiers: its group flag, whether its name is in a second group of arguments
/// (`it.each(table)('name', fn)`), and the text from its first `(`.
fn script_call(line: &str) -> Option<(bool, bool, &str)> {
    let mut rest = line.trim_start();
    let head = ["describe", "test", "it"]
        .into_iter()
        .find(|h| rest.starts_with(h))?;
    rest = &rest[head.len()..];
    let mut group = head == "describe";
    let mut curried = false;
    while let Some(after) = rest.strip_prefix('.') {
        let word: String = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if head == "test" && word == "describe" && !group && !curried {
            group = true;
        } else if MODIFIERS.contains(&word.as_str()) {
            curried |= word == "each" || word == "skipIf";
        } else {
            return None;
        }
        rest = &after[word.len()..];
    }
    let rest = rest.trim_start();
    rest.starts_with('(').then_some((group, curried, rest))
}

/// The first argument of the call starting at `args` (its `(`) when it is a string literal.
fn script_name(curried: bool, args: &str) -> String {
    let mut args = args;
    // `it.each(table)('name', ...)` and `it.skipIf(cond)('name', ...)` name the test in
    // the second call.
    if curried {
        let mut depth = 0;
        let mut end = None;
        for (at, c) in args.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(at + 1);
                        break;
                    }
                }
                _ => {}
            }
        }
        match end.map(|e| args[e..].trim_start()) {
            Some(next) if next.starts_with('(') => args = next,
            _ => return String::new(),
        }
    }
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
    let mut out = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        let Some((group, curried, args)) = script_call(line) else {
            continue;
        };
        let first = first_above(lines, at, is_comment);
        out.push(TestSpan {
            name: script_name(curried, args),
            group,
            first: first + 1,
            last: script_close(lines, at) + 1,
        });
    }
    out
}

/// The line that closes the call starting on `at`: where its parentheses balance, and no
/// further group of arguments follows on that line (`it.each(table)('name', fn)`).
fn script_close(lines: &[&str], at: usize) -> usize {
    let mut depth = 0usize;
    for (i, line) in lines.iter().enumerate().skip(at) {
        let text = code(line);
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
    lines.len().saturating_sub(1)
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
        .take_while(|c| c.is_alphanumeric() || *c == '_')
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
        let first = first_above(lines, at, |l| {
            let t = l.trim_start();
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
