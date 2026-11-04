//! Markdown code spans, for the rules that read the names a spec's text mentions
//! (`asset-references`, `removed-requirement-names`).

/// The contents of Markdown code spans, each with its byte offset in `text`, read as
/// CommonMark reads them within a block of lines between blank lines: a run of backticks
/// up to the next run of the same length, less one space on each side when both sides
/// have one. A backslash escapes one backtick outside a span, never inside one. Fenced
/// code blocks hold none; headings, list items and indented code are not told apart from
/// paragraphs.
pub(crate) fn code_spans(text: &str) -> impl Iterator<Item = (usize, &str)> {
    paragraphs(text).into_iter().flat_map(|(start, para)| {
        let bytes = para.as_bytes();
        let mut spans = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'\\' => i += 2,
                b'`' => {
                    let len = run_len(bytes, i);
                    let mut j = i + len;
                    while j < bytes.len() && !(bytes[j] == b'`' && run_len(bytes, j) == len) {
                        j += if bytes[j] == b'`' {
                            run_len(bytes, j)
                        } else {
                            1
                        };
                    }
                    if j < bytes.len() {
                        let (pad, span) = unpad(&para[i + len..j]);
                        spans.push((start + i + len + pad, span));
                        i = j + len;
                    } else {
                        i += len;
                    }
                }
                _ => i += 1,
            }
        }
        spans
    })
}

/// The fence a line opens or closes, as (marker byte, length): at least three backticks
/// or tildes after any indentation. A backtick run followed by another backtick on the
/// line opens no fence (CommonMark), so ```` ```x``` ```` starts a paragraph.
fn fence(line: &str) -> Option<(u8, usize)> {
    let trimmed = line.trim_start();
    let marker = *trimmed
        .as_bytes()
        .first()
        .filter(|&&b| b == b'`' || b == b'~')?;
    let len = run_len(trimmed.as_bytes(), 0);
    let info = &trimmed[len..];
    (len >= 3 && !(marker == b'`' && info.contains('`'))).then_some((marker, len))
}

/// Blocks of `text` separated by blank lines, each with its byte offset, leaving out
/// fenced code blocks (from a fence line to a line holding only a fence of the same
/// marker, at least as long); code spans never cross a blank line, and fenced code holds
/// none.
fn paragraphs(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut open: Option<(u8, usize)> = None;
    let mut start = None;
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        let breaks = match open {
            Some((marker, len)) => {
                let closes = fence(line).is_some_and(|(m, l)| {
                    m == marker && l >= len && line.trim().bytes().all(|b| b == marker)
                });
                if closes {
                    open = None;
                }
                true
            }
            None => {
                open = fence(line);
                open.is_some() || line.trim().is_empty()
            }
        };
        match (breaks, start) {
            (true, Some(s)) => {
                out.push((s, &text[s..at]));
                start = None;
            }
            (false, None) => start = Some(at),
            _ => {}
        }
        at += line.len();
    }
    if let Some(s) = start {
        out.push((s, &text[s..]));
    }
    out
}

/// `span` less one space on each side when both sides have one and it isn't all spaces,
/// with the number of bytes taken from its start.
fn unpad(span: &str) -> (usize, &str) {
    match span.strip_prefix(' ').and_then(|s| s.strip_suffix(' ')) {
        Some(inner) if !inner.trim().is_empty() => (1, inner),
        _ => (0, span),
    }
}

/// The length of the run of `bytes[at]` starting at `at`.
fn run_len(bytes: &[u8], at: usize) -> usize {
    bytes[at..].iter().take_while(|&&b| b == bytes[at]).count()
}
