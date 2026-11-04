//! `linguist_excluded`: gitattributes read without git, for a listing that has no working
//! tree (`linguist-attributes`, see specs/check/SPEC.md).
//!
//! It follows git's `attr.c`: the `.gitattributes` files of a path's directory and its
//! ancestors, deeper files over shallower ones and later lines over earlier ones, each line's
//! attributes read right to left, an attribute decided by the first assignment met that way,
//! and a macro's attributes filled in where it is set. Macros are defined only in the root
//! `.gitattributes`.

use std::collections::BTreeMap;

/// An attribute's state on one line.
#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
    /// `attr`
    Set,
    /// `-attr`
    Unset,
    /// `!attr`
    Unspecified,
    /// `attr=value`
    Value(String),
}

/// One line: a pattern, or a macro's name, with its attributes in the order written.
struct Line<'a> {
    pattern: Pattern,
    attrs: Vec<(&'a str, State)>,
    base: &'a str,
}

enum Pattern {
    Path(String),
    Macro(String),
}

/// Whether `path`, relative to the top of the tree, has `linguist-generated` or
/// `linguist-vendored` set or `true`, given each `.gitattributes` file's text with its
/// directory (`""` for the root), as `git check-attr` would say with no other attribute
/// source. git's built-in `binary` macro sets no linguist attribute, so it isn't needed here.
pub fn linguist_excluded<D: AsRef<str>, T: AsRef<str>>(attributes: &[(D, T)], path: &str) -> bool {
    // The files that apply to `path`, shallowest first.
    let mut files: Vec<(&str, &str)> = attributes
        .iter()
        .map(|(d, t)| {
            let d = d.as_ref();
            (
                d.strip_prefix("./").unwrap_or(d).trim_matches('/'),
                t.as_ref(),
            )
        })
        .map(|(d, t)| (if d == "." { "" } else { d }, t))
        .filter(|(d, _)| d.is_empty() || path.strip_prefix(*d).is_some_and(|r| r.starts_with('/')))
        .collect();
    files.sort_by_key(|(d, _)| d.split('/').filter(|s| !s.is_empty()).count());
    let lines: Vec<Line> = files
        .iter()
        .flat_map(|(base, text)| text.lines().filter_map(move |l| parse(l, base)))
        .collect();

    let macros: BTreeMap<&str, &Line> = lines
        .iter()
        .filter_map(|l| match &l.pattern {
            Pattern::Macro(name) if l.base.is_empty() => Some((name.as_str(), l)),
            _ => None,
        })
        .collect();

    let mut decided: BTreeMap<&str, State> = BTreeMap::new();
    for line in lines.iter().rev() {
        if let Pattern::Path(pattern) = &line.pattern
            && path_matches(pattern, line.base, path)
        {
            fill(line, &macros, &mut decided);
        }
    }
    ["linguist-generated", "linguist-vendored"]
        .iter()
        .any(|a| match decided.get(a) {
            Some(State::Set) => true,
            Some(State::Value(v)) => v == "true",
            _ => false,
        })
}

/// Assigns `line`'s attributes not yet decided, right to left, expanding a macro set here.
fn fill<'a>(
    line: &Line<'a>,
    macros: &BTreeMap<&str, &Line<'a>>,
    decided: &mut BTreeMap<&'a str, State>,
) {
    for (name, state) in line.attrs.iter().rev() {
        if decided.contains_key(name) {
            continue;
        }
        decided.insert(name, state.clone());
        if *state == State::Set
            && let Some(m) = macros.get(name)
        {
            fill(m, macros, decided);
        }
    }
}

/// One line of a `.gitattributes` file in `base`; `None` for a blank line, a comment, a
/// negative pattern (which git forbids) or a macro outside the root.
fn parse<'a>(line: &'a str, base: &'a str) -> Option<Line<'a>> {
    let line = line.trim_start_matches(blank);
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (pattern, rest) = if line.starts_with('"') {
        unquote(line)?
    } else {
        let end = line.find(blank).unwrap_or(line.len());
        (line[..end].to_string(), &line[end..])
    };
    let pattern = match pattern.strip_prefix("[attr]") {
        Some(name) if base.is_empty() => Pattern::Macro(name.to_string()),
        Some(_) => return None,
        None if pattern.starts_with('!') => return None,
        None => Pattern::Path(pattern),
    };
    let attrs = rest
        .split(blank)
        .filter(|a| !a.is_empty())
        .map(|a| {
            if let Some(n) = a.strip_prefix('-') {
                (n, State::Unset)
            } else if let Some(n) = a.strip_prefix('!') {
                (n, State::Unspecified)
            } else if let Some((n, v)) = a.split_once('=') {
                (n, State::Value(v.to_string()))
            } else {
                (a, State::Set)
            }
        })
        .collect();
    Some(Line {
        pattern,
        attrs,
        base,
    })
}

fn blank(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

/// A C-quoted pattern at the start of `line` and the text after it, unquoted as git's
/// `unquote_c_style` does: `\a`, `\b`, `\f`, `\n`, `\r`, `\t`, `\v`, three octal digits for
/// a byte, and any other escaped character as itself.
fn unquote(line: &str) -> Option<(String, &str)> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 1;
    loop {
        match *bytes.get(i)? {
            b'"' => return Some((String::from_utf8_lossy(&out).into_owned(), &line[i + 1..])),
            b'\\' => {
                let c = *bytes.get(i + 1)?;
                i += 2;
                out.push(match c {
                    b'a' => 7,
                    b'b' => 8,
                    b'f' => 12,
                    b'n' => b'\n',
                    b'r' => b'\r',
                    b't' => b'\t',
                    b'v' => 11,
                    b'0'..=b'3' => {
                        let digits = bytes.get(i..i + 2)?;
                        if !digits.iter().all(|d| (b'0'..=b'7').contains(d)) {
                            return None;
                        }
                        i += 2;
                        ((c - b'0') << 6) | ((digits[0] - b'0') << 3) | (digits[1] - b'0')
                    }
                    c => c,
                });
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
}

/// Whether `pattern`, from the `.gitattributes` of directory `base`, matches the file
/// `path`. A trailing `/` matches directories only, so no file; a pattern without `/`
/// matches the file's name at any depth; any other is matched against the path relative to
/// `base`, a leading `/` only anchoring it.
fn path_matches(pattern: &str, base: &str, path: &str) -> bool {
    if pattern.ends_with('/') {
        return false;
    }
    let rel = if base.is_empty() {
        path
    } else {
        &path[base.len() + 1..]
    };
    if !pattern.contains('/') {
        let name = rel.rsplit('/').next().unwrap_or(rel);
        return wildmatch(pattern.as_bytes(), 0, name.as_bytes());
    }
    let pattern = pattern.strip_prefix('/').unwrap_or(pattern);
    wildmatch(pattern.as_bytes(), 0, rel.as_bytes())
}

/// git's `wildmatch` with `WM_PATHNAME`: `*`, `?` and `[...]` never match `/`, and `**`
/// crosses directories only as a whole segment. `p[..i]` is what came before.
fn wildmatch(p: &[u8], i: usize, t: &[u8]) -> bool {
    let Some(&c) = p.get(i) else {
        return t.is_empty();
    };
    match c {
        b'*' if p.get(i + 1) == Some(&b'*') && (i == 0 || p[i - 1] == b'/') => match p.get(i + 2) {
            None => true,
            Some(b'/') => {
                wildmatch(p, i + 3, t)
                    || (0..t.len()).any(|n| t[n] == b'/' && wildmatch(p, i + 3, &t[n + 1..]))
            }
            Some(_) => star(p, i, t),
        },
        b'*' => star(p, i, t),
        b'?' => t.first().is_some_and(|&b| b != b'/') && wildmatch(p, i + 1, &t[1..]),
        b'[' => t.first().is_some_and(|&b| {
            b != b'/'
                && class(p, i + 1, b).is_some_and(|(hit, next)| hit && wildmatch(p, next, &t[1..]))
        }),
        b'\\' if i + 1 < p.len() => t.first() == Some(&p[i + 1]) && wildmatch(p, i + 2, &t[1..]),
        _ => t.first() == Some(&c) && wildmatch(p, i + 1, &t[1..]),
    }
}

/// One or more stars that don't cross `/`.
fn star(p: &[u8], i: usize, t: &[u8]) -> bool {
    let next = i + p[i..].iter().take_while(|&&b| b == b'*').count();
    let segment = t.iter().position(|&b| b == b'/').unwrap_or(t.len());
    (0..=segment).any(|n| wildmatch(p, next, &t[n..]))
}

/// The bracket expression starting at `p[i]`, just after its `[`: whether it matches `b`,
/// and the index just after its `]`; `None` when it is never closed.
fn class(p: &[u8], mut i: usize, b: u8) -> Option<(bool, usize)> {
    let negated = matches!(p.get(i), Some(b'!' | b'^'));
    if negated {
        i += 1;
    }
    let mut hit = false;
    let mut first = true;
    loop {
        let mut lo = *p.get(i)?;
        if lo == b']' && !first {
            return Some((hit != negated, i + 1));
        }
        first = false;
        if lo == b'[' && p.get(i + 1) == Some(&b':') {
            let name = &p[i + 2..];
            let end = name.windows(2).position(|w| w == b":]")?;
            hit |= posix(&name[..end], b)?;
            i += end + 4;
            continue;
        }
        if lo == b'\\' {
            i += 1;
            lo = *p.get(i)?;
        }
        if p.get(i + 1) == Some(&b'-') && p.get(i + 2).is_some_and(|&c| c != b']') {
            let mut hi_at = i + 2;
            if p[hi_at] == b'\\' {
                hi_at += 1;
            }
            let hi = *p.get(hi_at)?;
            hit |= (lo..=hi).contains(&b);
            i = hi_at + 1;
        } else {
            hit |= lo == b;
            i += 1;
        }
    }
}

/// Whether `b` is in the POSIX class `name`; `None` for a name git doesn't know, on which
/// it matches nothing.
fn posix(name: &[u8], b: u8) -> Option<bool> {
    Some(match name {
        b"alnum" => b.is_ascii_alphanumeric(),
        b"alpha" => b.is_ascii_alphabetic(),
        b"blank" => b == b' ' || b == b'\t',
        b"cntrl" => b.is_ascii_control(),
        b"digit" => b.is_ascii_digit(),
        b"graph" => b.is_ascii_graphic(),
        b"lower" => b.is_ascii_lowercase(),
        b"print" => b.is_ascii_graphic() || b == b' ',
        b"punct" => b.is_ascii_punctuation(),
        b"space" => b.is_ascii_whitespace() || b == 11,
        b"upper" => b.is_ascii_uppercase(),
        b"xdigit" => b.is_ascii_hexdigit(),
        _ => return None,
    })
}
