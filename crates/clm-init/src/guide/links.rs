//! Links stay inside the guide (`guide-links-inside`): every inline link or image target of
//! a page is an absolute URL, an anchor, or a relative path to another page's file, so the
//! page reads the same in `clm guide`'s output and on GitHub.

/// The inline link and image targets of the page at `file` (its path from the repository's
/// root) that leave the guide, each as `<page>: <target>`, `<page>` being the file's name
/// without `.md` (`index` for `README.md`). `pages` holds every page's file.
pub(super) fn links_out(file: &str, markdown: &str, pages: &[&str]) -> Vec<String> {
    let (dir, name) = file.rsplit_once('/').unwrap_or(("", file));
    let page = match name.strip_suffix(".md").unwrap_or(name) {
        "README" => "index",
        page => page,
    };
    targets(&outside_fences(markdown))
        .into_iter()
        .filter(|target| !stays(dir, target, pages))
        .map(|target| format!("{page}: {target}"))
        .collect()
}

/// `markdown` with every line of a fenced code block (```` ``` ```` or `~~~`, fences
/// included) blanked, so the scanner sees only the rest.
fn outside_fences(markdown: &str) -> String {
    let mut fence: Option<(u8, usize)> = None;
    let mut out = String::with_capacity(markdown.len());
    for line in markdown.split_inclusive('\n') {
        let text = line.trim_start_matches(' ');
        let indent = line.len() - text.len();
        let run = |c: u8| text.bytes().take_while(|&b| b == c).count();
        match fence {
            None => {
                // At most three spaces in, and no backtick in a backtick fence's info
                // string, so a line opening with a code span isn't a fence.
                let opens = b"`~".iter().copied().map(|c| (c, run(c))).find(|&(c, n)| {
                    n >= 3 && indent <= 3 && (c == b'~' || !text[n..].contains('`'))
                });
                match opens {
                    Some(open) => fence = Some(open),
                    None => {
                        out.push_str(line);
                        continue;
                    }
                }
            }
            Some((c, n)) => {
                if run(c) >= n && text[run(c)..].trim().is_empty() {
                    fence = None;
                }
            }
        }
        out.push('\n');
    }
    out
}

/// The inline link and image targets in `text`, outside code spans, in order. A target in
/// `<...>` is what the brackets hold; any title after the target is dropped.
fn targets(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'`' => i = past_code_span(bytes, i),
            b'[' => {
                if let Some(target) = link_at(bytes, i) {
                    found.push(target);
                }
                // Scan the link text too, for an image inside a link.
                i += 1;
            }
            _ => i += 1,
        }
    }
    found
}

/// Where the code span opening at `start` ends, or just past its backticks when no run of
/// as many closes it, in which case they are literal.
fn past_code_span(bytes: &[u8], start: usize) -> usize {
    let n = bytes[start..].iter().take_while(|&&b| b == b'`').count();
    let mut i = start + n;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let m = bytes[i..].iter().take_while(|&&b| b == b'`').count();
            if m == n {
                return i + m;
            }
            i += m;
        } else {
            i += 1;
        }
    }
    start + n
}

/// The target of the inline link whose text opens at `open`, if one does: the text's
/// brackets may nest, and the target may hold balanced parentheses.
fn link_at(bytes: &[u8], open: usize) -> Option<String> {
    let mut depth = 0;
    let mut i = open;
    let close = loop {
        match bytes.get(i)? {
            b'\\' => i += 1,
            b'`' => {
                i = past_code_span(bytes, i);
                continue;
            }
            b'[' => depth += 1,
            // A blank line ends the paragraph, and the link text with it.
            b'\n'
                if bytes[i + 1..]
                    .iter()
                    .find(|b| !b" \t\r".contains(b))
                    .is_some_and(|&b| b == b'\n') =>
            {
                return None;
            }
            b']' => {
                depth -= 1;
                if depth == 0 {
                    break i;
                }
            }
            _ => {}
        }
        i += 1;
    };
    if bytes.get(close + 1) != Some(&b'(') {
        return None;
    }
    let mut i = skip_space(bytes, close + 2);
    let start = i;
    let target = if bytes.get(i) == Some(&b'<') {
        let end = start
            + 1
            + bytes[start + 1..]
                .iter()
                .position(|&b| b == b'>' || b == b'\n')?;
        if bytes[end] != b'>' {
            return None;
        }
        i = end + 1;
        &bytes[start + 1..end]
    } else {
        let mut parens = 0;
        while let Some(&b) = bytes.get(i) {
            match b {
                b'(' => parens += 1,
                b')' if parens == 0 => break,
                b')' => parens -= 1,
                b if b.is_ascii_whitespace() => break,
                _ => {}
            }
            i += 1;
        }
        &bytes[start..i]
    };
    // A title, if any, then the closing parenthesis.
    i = skip_space(bytes, i);
    if let Some(&quote) = bytes.get(i).filter(|b| b"\"'(".contains(b)) {
        let closer = if quote == b'(' { b')' } else { quote };
        i += 1 + bytes[i + 1..].iter().position(|&b| b == closer)? + 1;
        i = skip_space(bytes, i);
    }
    (bytes.get(i) == Some(&b')')).then(|| String::from_utf8_lossy(target).into_owned())
}

/// Past the whitespace at `i`, a line break included.
fn skip_space(bytes: &[u8], mut i: usize) -> usize {
    while bytes.get(i).is_some_and(|b| b.is_ascii_whitespace()) {
        i += 1;
    }
    i
}

/// Whether `target`, in a page whose file is in `dir`, stays inside the guide: an absolute
/// URL, an anchor, or a relative path naming one of `pages` once its anchor is dropped and
/// it is resolved lexically against `dir`.
fn stays(dir: &str, target: &str, pages: &[&str]) -> bool {
    if target.starts_with('#') || has_scheme(target) {
        return true;
    }
    let path = target.split_once('#').map_or(target, |(path, _)| path);
    if path.is_empty() || path.starts_with('/') || path.contains('?') {
        return false;
    }
    let mut parts: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return false;
                }
            }
            part => parts.push(part),
        }
    }
    pages.contains(&parts.join("/").as_str())
}

/// Whether `target` starts with a URL scheme, `[A-Za-z][A-Za-z0-9+.-]*:`.
fn has_scheme(target: &str) -> bool {
    let Some((scheme, _)) = target.split_once(':') else {
        return false;
    };
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_out_of_the_guide() {
        // req:init/guide-links-inside@863332b6
        let page = "See [Decisions](../decisions/), [ARCHITECTURE.md](../../ARCHITECTURE.md) \
                    and [gone](missing.md).\n";
        let pages = ["docs/guide/reference.md", "docs/spec-format.md"];
        assert_eq!(
            links_out("docs/guide/reference.md", page, &pages),
            [
                "reference: ../decisions/",
                "reference: ../../ARCHITECTURE.md",
                "reference: missing.md",
            ]
        );
    }

    #[test]
    fn links_that_stay() {
        // req:init/guide-links-inside@863332b6
        let page = "[Spec format](../spec-format.md), [Reference](reference.md#skills), \
                    [top](#top), [Claude Code](https://claude.com/claude-code) and \
                    `[x](../decisions/)`.\n";
        let pages = [
            "docs/guide/using-clm.md",
            "docs/guide/reference.md",
            "docs/spec-format.md",
        ];
        assert_eq!(
            links_out("docs/guide/using-clm.md", page, &pages),
            Vec::<String>::new()
        );
    }

    /// The targets `links_out` reports for `page` at `docs/guide/a.md`, among the pages
    /// `docs/guide/a.md`, `docs/guide/b.md`, `docs/guide/README.md` and `docs/spec-format.md`.
    fn out(page: &str) -> Vec<String> {
        let pages = [
            "docs/guide/a.md",
            "docs/guide/b.md",
            "docs/guide/README.md",
            "docs/spec-format.md",
        ];
        links_out("docs/guide/a.md", page, &pages)
            .into_iter()
            .map(|l| l.strip_prefix("a: ").unwrap().to_owned())
            .collect()
    }

    #[test]
    fn forms_of_link() {
        // req:init/guide-links-inside@863332b6
        // Images, titles, angle brackets, nested brackets and balanced parentheses.
        assert_eq!(
            out("![d](../d.png) [t](x.md \"title\") [t](<y.md>)"),
            ["../d.png", "x.md", "y.md"]
        );
        assert_eq!(
            out("[a [nested] text](z.md) [p](f(1).md)"),
            ["z.md", "f(1).md"]
        );
        assert_eq!(
            out("[ok](b.md \"t\") [ok](<./b.md>) [ok](README.md)"),
            Vec::<String>::new()
        );
        // Normalised lexically; a query or a climb above the root fails.
        assert_eq!(
            out("[n](./x/../b.md) [s](../guide/b.md)"),
            Vec::<String>::new()
        );
        assert_eq!(
            out("[q](b.md?x=1) [up](../../../docs/guide/b.md) [abs](/docs/guide/b.md)"),
            ["b.md?x=1", "../../../docs/guide/b.md", "/docs/guide/b.md"]
        );
        // Schemes and anchors pass; a link with no target fails.
        assert_eq!(
            out("[m](mailto:a@b.c) [x](HTTP+x.y-z:q) [#](#a)"),
            Vec::<String>::new()
        );
        assert_eq!(out("[e]()"), [""]);
        // Not links: no closing parenthesis after the title, a `<` target across a line
        // break, and brackets that span a blank line.
        assert_eq!(
            out("[x](foo bar baz\nlater (a) [y](b.md)"),
            Vec::<String>::new()
        );
        assert_eq!(out("[t](<a\n> b) [y](b.md)"), Vec::<String>::new());
        assert_eq!(out("items [draft\n\ndone](notes)"), Vec::<String>::new());
        assert_eq!(out("[t](x.md 'one' ) [u](y.md (two))"), ["x.md", "y.md"]);
    }

    #[test]
    fn code_is_not_read() {
        // req:init/guide-links-inside@863332b6
        let page = "``a `[x](no.md)` b`` `[y](no.md)`\n\
                    ```\n[f](no.md)\n```\n\
                    ~~~~md\n[g](no.md)\n```\n~~~~\n\
                    \\[not](a link) [ref]: no.md <https://x.y>\n\
                    [after](gone.md)\n";
        assert_eq!(out(page), ["gone.md"]);
        // A line opening with a code span, or indented four spaces, opens no fence.
        let page = "```clm guide``` prints\n    ```\n[a](one.md)\n";
        assert_eq!(out(page), ["one.md"]);
    }
}
