//! The guard's one scan of a command (`guard-quotes`, `guard-heredocs`): left to right,
//! quoting as the shell does, into segments with their tokens, the separator that ended
//! each, the heredoc bodies each opened, the substitutions inside its double quotes and the
//! words of its here-strings. A text heuristic, not a shell parser.

/// What ended a segment. The command's end counts as a newline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sep {
    AndAnd,
    OrOr,
    Pipe,
    PipeAmp,
    Amp,
    Semi,
    Newline,
    /// `(`: a subshell, group or substitution opens.
    Open,
    /// `)`: one closes, when a `(` opened it.
    Close,
    /// A backtick, which opens a substitution or closes the open one.
    Backtick,
}

impl Sep {
    /// Whether the next segment reads this one's output: the two are in one pipeline.
    pub(crate) fn pipes(self) -> bool {
        matches!(self, Sep::Pipe | Sep::PipeAmp)
    }
}

/// One command segment.
#[derive(Clone, Debug)]
pub(crate) struct Segment {
    /// Its own text as written, trimmed, without its heredoc bodies.
    pub text: String,
    /// Its words, with the quotes and backslashes that quote removed and its redirections
    /// (`>f`, `2>&1`, `<<EOF` and a bare operator's target) left out.
    pub tokens: Vec<String>,
    pub sep: Sep,
    /// The bodies of the heredocs it opened, in order.
    pub bodies: Vec<String>,
    /// The text of each substitution inside its double quotes, between `$(` and the
    /// matching `)` (`guard-quoted-substitutions`), outermost only.
    pub subs: Vec<String>,
    /// The words of its here-strings, with their quotes removed (`guard-here-strings`).
    pub herestrings: Vec<String>,
}

/// `command`'s segments. When the scan ends inside a quote or a substitution, the quotes
/// don't balance and are ignored: the command is split at every separator and space, with
/// every quote character removed, keeping the heredoc bodies the first scan found, so a
/// stray apostrophe never hides a command.
pub fn segments(command: &str) -> Vec<Segment> {
    let first = Scanner::new(command, None).run();
    if !first.unbalanced {
        return first.segments;
    }
    Scanner::new(command, Some(&first.heredocs)).run().segments
}

/// A heredoc the scan found: its operator's offset, where its body starts and ends, and
/// where the scan resumes after its terminator line.
#[derive(Clone)]
struct Heredoc {
    op: usize,
    start: usize,
    end: usize,
    resume: usize,
}

struct Scan {
    segments: Vec<Segment>,
    heredocs: Vec<Heredoc>,
    unbalanced: bool,
}

/// A word as scanned, and whether it holds a redirection operator outside quotes.
struct Word {
    text: String,
    redirect: bool,
}

/// The quote the scan is inside.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Quote {
    Single,
    Double,
    /// `$'...'`, in which a backslash escapes the next character.
    Ansi,
}

/// A finished segment: its start and end offsets, words, separator and substitutions.
struct Done {
    start: usize,
    end: usize,
    words: Vec<Word>,
    sep: Sep,
    subs: Vec<String>,
}

struct Scanner<'a> {
    text: &'a str,
    /// The heredocs of a first scan, for a scan that ignores quotes; `None` for a first
    /// scan, which quotes and finds heredocs itself.
    known: Option<&'a [Heredoc]>,
    heredocs: Vec<Heredoc>,
    /// The heredocs found inside quoted substitutions: theirs, not the segment's, but kept
    /// for a scan that ignores quotes.
    nested: Vec<Heredoc>,
    /// Operators on the current line whose bodies start after it: offset, word, `<<-`.
    pending: Vec<(usize, String, bool)>,
    done: Vec<Done>,
    words: Vec<Word>,
    word: String,
    /// Whether the current word has begun (a quoted empty string is a word).
    started: bool,
    redirect: bool,
    start: usize,
    /// The current segment's quoted substitutions.
    subs: Vec<String>,
    /// Whether a backtick substitution is open.
    tick: bool,
}

impl<'a> Scanner<'a> {
    fn new(text: &'a str, known: Option<&'a [Heredoc]>) -> Self {
        Scanner {
            text,
            known,
            heredocs: Vec::new(),
            nested: Vec::new(),
            pending: Vec::new(),
            done: Vec::new(),
            words: Vec::new(),
            word: String::new(),
            started: false,
            redirect: false,
            start: 0,
            subs: Vec::new(),
            tick: false,
        }
    }

    fn run(mut self) -> Scan {
        let text = self.text;
        let quotes = self.known.is_none();
        let mut quote: Option<Quote> = None;
        let mut i = 0;
        while let Some(c) = text[i..].chars().next() {
            let next = text[i + c.len_utf8()..].chars().next();
            let mut width = c.len_utf8();
            if let Some(q) = quote {
                match (q, c) {
                    (Quote::Single, '\'') | (Quote::Double, '"') | (Quote::Ansi, '\'') => {
                        quote = None
                    }
                    (Quote::Double, '\\')
                        if matches!(next, Some('"' | '\\' | '$' | '`' | '\n')) =>
                    {
                        let n = next.unwrap_or_default();
                        if n != '\n' {
                            self.word.push(n);
                        }
                        width += n.len_utf8();
                    }
                    (Quote::Double, '$') if next == Some('(') => match substitution(text, i + 2) {
                        Some((end, nested)) => {
                            self.word.push_str(&text[i..=end]);
                            self.subs.push(text[i + 2..end].to_string());
                            self.nested.extend(nested);
                            width = end + 1 - i;
                        }
                        // It never closes: the scan ends inside the double quotes, which
                        // don't balance.
                        None => {
                            self.word.push_str(&text[i..]);
                            width = text.len() - i;
                        }
                    },
                    // Only `\'` and `\\` lose their backslash.
                    (Quote::Ansi, '\\') if next.is_some() => {
                        let n = next.unwrap_or_default();
                        if !matches!(n, '\'' | '\\') {
                            self.word.push('\\');
                        }
                        self.word.push(n);
                        width += n.len_utf8();
                    }
                    _ => self.word.push(c),
                }
                i += width;
                continue;
            }
            match c {
                '\'' if quotes => {
                    quote = Some(Quote::Single);
                    self.started = true;
                }
                '"' if quotes => {
                    quote = Some(Quote::Double);
                    self.started = true;
                }
                '$' if quotes && next == Some('\'') => {
                    quote = Some(Quote::Ansi);
                    self.started = true;
                    width += 1;
                }
                '\'' | '"' => {}
                '\\' if quotes && next.is_some() => {
                    let n = next.unwrap_or_default();
                    if n != '\n' {
                        self.word.push(n);
                        self.started = true;
                    }
                    width += n.len_utf8();
                }
                ' ' | '\t' | '\r' => self.end_word(),
                // A `#` that begins a word comments out the rest of the line: its quotes
                // and `<<` count for nothing.
                '#' if !self.started && self.word.is_empty() => {
                    i = text[i..].find('\n').map_or(text.len(), |n| i + n);
                    continue;
                }
                '\n' => {
                    self.end_segment(i, Sep::Newline);
                    i = self.bodies_from(i + 1);
                    self.start = i;
                    continue;
                }
                ';' => self.end_segment(i, Sep::Semi),
                '|' => {
                    let sep = match next {
                        Some('|') => Sep::OrOr,
                        Some('&') => Sep::PipeAmp,
                        _ => Sep::Pipe,
                    };
                    width += usize::from(sep != Sep::Pipe);
                    self.end_segment(i, sep);
                }
                '&' if next == Some('&') => {
                    self.end_segment(i, Sep::AndAnd);
                    width += 1;
                }
                // The `&` of `>&`, `<&` and `&>` is a redirection's.
                '&' if next == Some('>')
                    || (i > 0 && matches!(text.as_bytes()[i - 1], b'>' | b'<')) =>
                {
                    self.redirection('&')
                }
                '&' => self.end_segment(i, Sep::Amp),
                '(' => self.end_segment(i, Sep::Open),
                ')' => self.end_segment(i, Sep::Close),
                '`' => {
                    // The word a substitution opens in keeps a backtick, as `$(` keeps its
                    // `$`, so a value read from one stays a value the guard can't know.
                    if !self.tick {
                        self.word.push('`');
                        self.started = true;
                    }
                    self.tick = !self.tick;
                    self.end_segment(i, Sep::Backtick);
                }
                '<' if quotes && text[i..].starts_with("<<<") => {
                    self.redirection('<');
                    self.word.push_str("<<");
                    width = 3;
                }
                '<' if quotes && text[i..].starts_with("<<") => {
                    let tabs = text[i + 2..].starts_with('-');
                    if let Some(word) = heredoc_word(&text[i + 2 + usize::from(tabs)..]) {
                        self.pending.push((i, word, tabs));
                    }
                    self.redirection('<');
                    self.word.push('<');
                    width = 2;
                    if tabs {
                        self.word.push('-');
                        width += 1;
                    }
                }
                '<' | '>' => self.redirection(c),
                _ => {
                    self.word.push(c);
                    self.started = true;
                }
            }
            i += width;
        }
        self.end_segment(text.len(), Sep::Newline);
        self.finish(quote.is_some())
    }

    /// Adds a redirection operator's character to the current word, which a file
    /// descriptor's digits may begin; a word of anything else ends first.
    fn redirection(&mut self, c: char) {
        if !self.redirect && !self.word.bytes().all(|b| b.is_ascii_digit()) {
            self.end_word();
        }
        self.word.push(c);
        self.started = true;
        self.redirect = true;
    }

    fn end_word(&mut self) {
        if self.started || !self.word.is_empty() {
            self.words.push(Word {
                text: std::mem::take(&mut self.word),
                redirect: self.redirect,
            });
        }
        self.started = false;
        self.redirect = false;
    }

    /// Ends the segment at `at`, by `sep`; the next starts after the separator.
    fn end_segment(&mut self, at: usize, sep: Sep) {
        self.end_word();
        self.done.push(Done {
            start: self.start,
            end: at,
            words: std::mem::take(&mut self.words),
            sep,
            subs: std::mem::take(&mut self.subs),
        });
        let len = match sep {
            Sep::AndAnd | Sep::OrOr | Sep::PipeAmp => 2,
            _ => 1,
        };
        self.start = at + len;
    }

    /// Skips the bodies of the heredocs whose operators were on the line that ends just
    /// before `at`, recording each, and returns where the scan resumes. An operator with no
    /// terminator line opens no heredoc.
    fn bodies_from(&mut self, mut at: usize) -> usize {
        if let Some(known) = self.known {
            while let Some(h) = known.iter().find(|h| h.start == at) {
                self.heredocs.push(h.clone());
                at = h.resume;
            }
            return at;
        }
        for (op, word, tabs) in std::mem::take(&mut self.pending) {
            if let Some(h) = terminated(self.text, op, at, &word, tabs) {
                at = h.resume;
                self.heredocs.push(h);
            }
        }
        at
    }

    fn finish(self, unbalanced: bool) -> Scan {
        let text = self.text;
        let mut segments: Vec<Segment> = self
            .done
            .iter()
            .map(|d| Segment {
                text: text[d.start..d.end.max(d.start)].trim().to_string(),
                tokens: tokens(&d.words),
                sep: d.sep,
                bodies: Vec::new(),
                subs: d.subs.clone(),
                // A scan that ignores quotes runs because they don't balance.
                herestrings: herestrings(&d.words, self.known.is_some()),
            })
            .collect();
        for h in &self.heredocs {
            // The segment holding the operator: the last that starts at or before it.
            if let Some(s) = self.done.iter().rposition(|d| d.start <= h.op) {
                segments[s].bodies.push(text[h.start..h.end].to_string());
            }
        }
        let mut heredocs = self.heredocs;
        heredocs.extend(self.nested);
        Scan {
            segments,
            heredocs,
            unbalanced,
        }
    }
}

/// The heredoc whose operator at `op`, with `word` (and `<<-`'s `tabs`), has its body
/// start at `at`: up to the first line that is `word`, or `None` when there is none.
fn terminated(text: &str, op: usize, at: usize, word: &str, tabs: bool) -> Option<Heredoc> {
    let mut line = at;
    while line <= text.len() {
        let end = text[line..].find('\n').map_or(text.len(), |n| line + n);
        let l = &text[line..end];
        if (if tabs { l.trim_start_matches('\t') } else { l }) == word {
            return Some(Heredoc {
                op,
                start: at,
                end: line.saturating_sub(1).max(at),
                resume: (end + 1).min(text.len()),
            });
        }
        line = end + 1;
    }
    None
}

/// The offset of the `)` that closes a substitution whose text starts at `from`, inside
/// double quotes, and the heredocs found inside it; `None` when it never closes. Its text
/// is scanned as a command of its own: quotes, nested substitutions, `(` and `)`, comments
/// and heredocs, whose bodies count for nothing. A `)` with no `(` ends it, as in a `case`
/// pattern: a gap.
fn substitution(text: &str, from: usize) -> Option<(usize, Vec<Heredoc>)> {
    let mut heredocs = Vec::new();
    let mut pending: Vec<(usize, String, bool)> = Vec::new();
    let mut quote: Option<Quote> = None;
    let mut depth = 0usize;
    let mut word_start = true;
    let mut i = from;
    while let Some(c) = text[i..].chars().next() {
        let next = text[i + c.len_utf8()..].chars().next();
        let escaped = next.map_or(0, char::len_utf8);
        let mut width = c.len_utf8();
        match (quote, c) {
            (Some(Quote::Single), '\'')
            | (Some(Quote::Double), '"')
            | (Some(Quote::Ansi), '\'') => quote = None,
            (Some(Quote::Double | Quote::Ansi) | None, '\\') => width += escaped,
            (Some(Quote::Double), '$') if next == Some('(') => {
                let (end, inner) = substitution(text, i + 2)?;
                heredocs.extend(inner);
                width = end + 1 - i;
            }
            (Some(_), _) => {}
            (None, '\'') => quote = Some(Quote::Single),
            (None, '"') => quote = Some(Quote::Double),
            (None, '$') if next == Some('\'') => {
                quote = Some(Quote::Ansi);
                width += 1;
            }
            (None, '#') if word_start => {
                i = text[i..].find('\n').map_or(text.len(), |n| i + n);
                continue;
            }
            (None, '(') => depth += 1,
            (None, ')') if depth == 0 => return Some((i, heredocs)),
            (None, ')') => depth -= 1,
            (None, '<') if text[i..].starts_with("<<<") => width = 3,
            (None, '<') if text[i..].starts_with("<<") => {
                let tabs = text[i + 2..].starts_with('-');
                if let Some(word) = heredoc_word(&text[i + 2 + usize::from(tabs)..]) {
                    pending.push((i, word, tabs));
                }
                width = 2 + usize::from(tabs);
            }
            (None, '\n') => {
                let mut at = i + 1;
                for (op, word, tabs) in pending.drain(..) {
                    if let Some(h) = terminated(text, op, at, &word, tabs) {
                        at = h.resume;
                        heredocs.push(h);
                    }
                }
                word_start = true;
                i = at;
                continue;
            }
            (None, _) => {}
        }
        word_start = quote.is_none() && (c.is_whitespace() || ";&|()".contains(c));
        i += width;
    }
    None
}

/// A segment's arguments: its words without its redirections, nor the target of an
/// operator written alone (`> f`, `<< EOF`).
fn tokens(words: &[Word]) -> Vec<String> {
    let mut out = Vec::new();
    let mut target = false;
    for w in words {
        if std::mem::take(&mut target) {
            continue;
        }
        if w.redirect {
            target = w.text.ends_with(['<', '>', '&']) || w.text.ends_with("<<-");
            continue;
        }
        out.push(w.text.clone());
    }
    out
}

/// The words of a segment's here-strings (`<<<`): the rest of the operator's word, or the
/// next word when the operator ends it; when the quotes don't balance, every word after the
/// operator to the segment's end.
fn herestrings(words: &[Word], unbalanced: bool) -> Vec<String> {
    let mut out = Vec::new();
    for (i, w) in words.iter().enumerate() {
        let Some(rest) = w
            .text
            .trim_start_matches(|c: char| c.is_ascii_digit())
            .strip_prefix("<<<")
            .filter(|_| w.redirect)
        else {
            continue;
        };
        let mut word: Vec<&str> = Vec::new();
        if !rest.is_empty() {
            word.push(rest);
        }
        let after = words[i + 1..].iter().map(|w| w.text.as_str());
        match (unbalanced, rest.is_empty()) {
            (true, _) => word.extend(after),
            (false, true) => word.extend(after.take(1)),
            (false, false) => {}
        }
        out.push(word.join(" "));
    }
    out
}

/// The word after a heredoc operator, past optional spaces: quoted in `'` or `"`, or bare
/// up to a space or an operator character, with quotes and backslashes removed.
fn heredoc_word(rest: &str) -> Option<String> {
    let rest = rest.trim_start_matches([' ', '\t']);
    let word = match rest.chars().next() {
        Some(q @ ('\'' | '"')) => rest[1..].split(q).next().unwrap_or_default(),
        _ => rest
            .split(|c: char| c.is_whitespace() || ";&|<>()".contains(c))
            .next()
            .unwrap_or_default(),
    };
    let word = word.replace(['\\', '\'', '"'], "");
    (!word.is_empty()).then_some(word)
}
