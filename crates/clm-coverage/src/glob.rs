//! A node's `paths` matched against a file in memory, as git matches `:(glob)` pathspecs,
//! so `pending` and `owner` need no checkout (`pending-report`, `owner`).

/// Whether `path` matches `pattern` as the pathspec `:(glob)<pattern>` would: git's
/// `wildmatch` with `WM_PATHNAME`, so `*`, `?` and `[...]` never match `/`, and `**`
/// crosses directories only as a whole segment (`**/`, `/**/`, `/**`); and, as git compares
/// every pathspec literally first, a pattern read literally also matches every path below
/// it.
pub fn matches(pattern: &str, path: &str) -> bool {
    let dir = pattern.trim_end_matches('/');
    path == pattern
        || path.strip_prefix(dir).is_some_and(|r| r.starts_with('/'))
        || wildmatch(pattern.as_bytes(), 0, path.as_bytes())
}

/// Whether `t` matches `p[i..]`; `p[..i]` is what came before, to tell a whole-segment
/// `**` from two stars.
fn wildmatch(p: &[u8], i: usize, t: &[u8]) -> bool {
    let Some(&c) = p.get(i) else {
        return t.is_empty();
    };
    let rest = |n: usize| &t[n..];
    match c {
        b'*' if p.get(i + 1) == Some(&b'*') && (i == 0 || p[i - 1] == b'/') => match p.get(i + 2) {
            None => true,
            // `**/`: zero or more whole directories.
            Some(b'/') => {
                wildmatch(p, i + 3, t)
                    || (0..t.len()).any(|n| t[n] == b'/' && wildmatch(p, i + 3, rest(n + 1)))
            }
            Some(_) => star(p, i, t),
        },
        b'*' => star(p, i, t),
        b'?' => t.first().is_some_and(|&b| b != b'/') && wildmatch(p, i + 1, rest(1)),
        // An unclosed `[` matches nothing, as git aborts on it.
        b'[' => t.first().is_some_and(|&b| {
            b != b'/'
                && class(p, i + 1, b).is_some_and(|(hit, next)| hit && wildmatch(p, next, rest(1)))
        }),
        b'\\' if i + 1 < p.len() => t.first() == Some(&p[i + 1]) && wildmatch(p, i + 2, rest(1)),
        _ => t.first() == Some(&c) && wildmatch(p, i + 1, rest(1)),
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

/// Whether `b` is in the POSIX character class `[:<name>:]`; `None` for an unknown name,
/// on which git aborts the match.
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
        b"space" => b.is_ascii_whitespace() || b == 0x0b,
        b"upper" => b.is_ascii_uppercase(),
        b"xdigit" => b.is_ascii_hexdigit(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn double_star() {
        assert!(matches("crates/v/**", "crates/v/tests/t.rs"));
        assert!(matches("crates/v/**", "crates/v/t.rs"));
        assert!(!matches("crates/v/**", "crates/vv/t.rs"));
        assert!(matches("**/t.rs", "t.rs"));
        assert!(matches("**/t.rs", "a/b/t.rs"));
        assert!(matches("a/**/b.rs", "a/b.rs"));
        assert!(matches("a/**/b.rs", "a/x/y/b.rs"));
        assert!(!matches("a/**/b.rs", "ab.rs"));
        assert!(matches("**", "a/b"));
    }

    #[test]
    fn star_and_question_mark_stay_in_a_segment() {
        assert!(!matches("crates/*", "crates/v/t.rs"));
        assert!(matches("crates/*", "crates/t.rs"));
        assert!(matches("crates/*.rs", "crates/t.rs"));
        assert!(!matches("*.rs", "a/t.rs"));
        assert!(matches("a?c", "abc"));
        assert!(!matches("a?c", "a/c"));
        assert!(matches("a**c", "abbc"));
        assert!(!matches("a**c", "a/c"));
    }

    #[test]
    fn literal_patterns_match_leading_directories() {
        assert!(matches("crates/v", "crates/v/t.rs"));
        assert!(matches("crates/v/", "crates/v/t.rs"));
        assert!(matches("crates/v", "crates/v"));
        assert!(!matches("crates/v", "crates/vv/t.rs"));
        assert!(!matches("crates/v/t.rs", "crates/v/t.rsx"));
    }

    #[test]
    fn brackets() {
        assert!(matches("[ab].rs", "a.rs"));
        assert!(!matches("[ab].rs", "c.rs"));
        assert!(matches("[!a].rs", "c.rs"));
        assert!(matches("[^a].rs", "c.rs"));
        assert!(!matches("[!a].rs", "a.rs"));
        assert!(matches("[a-c].rs", "b.rs"));
        assert!(!matches("a[!x]b", "a/b"));
        assert!(matches("a[]]b", "a]b"));
        assert!(matches("a\\*b", "a*b"));
        assert!(!matches("a\\*b", "axb"));
        assert!(!matches("a[b*", "a[bc"));
        assert!(matches("x/[[:alpha:]].rs", "x/a.rs"));
        assert!(!matches("x/[[:digit:]].rs", "x/a.rs"));
        assert!(matches("x/[[:digit:]a].rs", "x/a.rs"));
        assert!(!matches("x/[[:nope:]a].rs", "x/a.rs"));
    }

    #[test]
    fn a_wildcard_pattern_is_also_compared_literally() {
        assert!(matches("docs/[draft]", "docs/[draft]/t.md"));
        assert!(matches("docs/[draft]", "docs/[draft]"));
    }
}
