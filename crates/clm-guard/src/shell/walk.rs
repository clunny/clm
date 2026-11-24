//! The pieces of a command to judge, in order, each with the checkout it runs in
//! (`guard-checkout`).

use std::path::{Path, PathBuf};

use super::command::{self, Command};
use super::scan::{self, Input, Segment, Sep};

/// A segment as judged: its text and tokens, and the checkout it runs in.
pub(crate) struct Judged {
    pub(crate) text: String,
    pub(crate) tokens: Vec<String>,
    pub(crate) checkout: Option<PathBuf>,
    /// The last `cd` that set `checkout` when the guard couldn't follow it, as written.
    pub(crate) lost: Option<String>,
    /// Whether a pipe (`|` or `|&`) ends the segment, so its output goes to a later command.
    pub(crate) piped: bool,
    /// What feeds its standard input (`guard-standard-input`).
    pub(crate) stdin: Stdin,
}

/// What feeds a piece's standard input, as the shell decides it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Stdin {
    /// The command's own: no redirection and no pipe into it.
    Own,
    /// The output of the earlier segment of its pipeline.
    Pipe,
    /// A file, or a descriptor duplicated or closed. Its contents are never read.
    File,
    /// A here-string (`<<<`).
    HereString,
    /// A heredoc: its body as the shell sends it, and whether it sends it as written.
    Heredoc { body: String, as_written: bool },
}

impl Stdin {
    /// The input of `segment`, which a pipe feeds when `piped` (the segment before it ended
    /// with `|` or `|&`): its last input redirection wins.
    fn of(segment: &Segment, piped: bool) -> Stdin {
        match segment.inputs.last() {
            Some(Input::Heredoc { body, as_written }) => Stdin::Heredoc {
                body: body.clone(),
                as_written: *as_written,
            },
            Some(Input::HereString) => Stdin::HereString,
            Some(Input::File) => Stdin::File,
            None if piped => Stdin::Pipe,
            None => Stdin::Own,
        }
    }
}

impl Judged {
    pub(crate) fn command(&self) -> Command<'_> {
        command::parse(&self.tokens)
    }
}

/// Where the segments run: the checkout so far, and the last `cd` that set it when the
/// guard couldn't follow it, as written (`push-guard`).
#[derive(Clone)]
struct Place {
    dir: Option<PathBuf>,
    lost: Option<String>,
}

impl Place {
    /// The place `command`, whose tokens' glob patterns are `globs`, runs in: moved by its
    /// own `env -C` or `sudo -D`, each value's glob expanded from this place, and set
    /// outright, so no `cd` is lost, by an absolute one.
    fn moved(&self, command: &Command, globs: &[Option<String>]) -> Place {
        let mut dir = self.dir.clone();
        let mut absolute = false;
        for d in &command.chdir {
            let pattern = d.token.and_then(|t| globs.get(t).cloned().flatten());
            let value = match pattern.map(|p| glob::matches(self.dir.as_deref(), &p)) {
                Some(found) if found.len() == 1 => found[0].clone(),
                _ => d.value.to_string(),
            };
            if Path::new(&value).is_absolute() {
                absolute = true;
                dir = Some(PathBuf::from(value));
            } else {
                dir = dir.map(|dir| dir.join(value));
            }
        }
        Place {
            dir,
            lost: self.lost.clone().filter(|_| !absolute),
        }
    }
}

/// What a `cd` does to the checkout, as bash reads it (`guard-checkout`).
enum Cd {
    /// It moves to the directory, absolute or not.
    To(PathBuf, bool),
    /// It fails, or the guard can't tell where it goes from no checkout: the checkout stays.
    Stays,
    /// The guard can't follow it: the checkout goes back to where the walk started.
    Resets,
}

/// Reads a `cd` with `args`, whose glob patterns are `globs`, from the checkout `dir`: its
/// options up to and including a `--`, then its directory. A `$`, `~` or backtick resets
/// it, a glob is expanded, a bad option, a second directory or one that isn't there makes
/// it fail, and no directory or `-` resets it.
fn read_cd(args: &[String], globs: &[Option<String>], dir: Option<&Path>) -> Cd {
    let mut k = 0;
    let mut bad = false;
    while let Some(a) = args.get(k).filter(|a| a.starts_with('-') && *a != "-") {
        k += 1;
        if a == "--" {
            break;
        }
        bad |= !a[1..].chars().all(|c| matches!(c, 'L' | 'P' | 'e'));
    }
    let Some(word) = args.get(k) else {
        return if bad { Cd::Stays } else { Cd::Resets };
    };
    if word.contains(['$', '~', '`']) {
        return Cd::Resets;
    }
    let mut to = word.clone();
    let mut extra = args.len() > k + 1;
    if let Some(pattern) = globs.get(k).cloned().flatten() {
        match glob::matches(dir, &pattern).as_slice() {
            [one] => to = one.clone(),
            [] => {}
            _ => extra = true,
        }
    }
    if bad || extra {
        return Cd::Stays;
    }
    if to == "-" {
        return Cd::Resets;
    }
    let absolute = Path::new(&to).is_absolute();
    let to = match dir {
        Some(dir) => dir.join(&to),
        None if absolute => PathBuf::from(to),
        None => return Cd::Stays,
    };
    if to.is_dir() {
        Cd::To(to, absolute)
    } else {
        Cd::Stays
    }
}

/// `tokens`, a segment's tokens with glob patterns `globs`, as bash passes them to `git`
/// (`guard-checkout`): each `-C` value before git's subcommand that holds a glob, and each
/// glued `-C<value>` that does, replaced by its matches from `dir`, the checkout before the
/// segment's own `env -C` or `sudo -D`, sorted, or kept as written when there is none.
fn git_globs(
    command: &Command,
    tokens: &[String],
    globs: &[Option<String>],
    dir: Option<&Path>,
) -> Vec<String> {
    if !command.is("git") {
        return tokens.to_vec();
    }
    let mut k = tokens.len() - command.args.len();
    let mut out = tokens[..k].to_vec();
    let expanded = |at: usize| {
        let found = globs
            .get(at)
            .cloned()
            .flatten()
            .map(|p| glob::matches(dir, &p))
            .unwrap_or_default();
        if found.is_empty() {
            vec![tokens[at].clone()]
        } else {
            found
        }
    };
    while let Some(t) = tokens.get(k) {
        let valued = matches!(
            t.as_str(),
            "-C" | "-c" | "--git-dir" | "--work-tree" | "--namespace" | "--config-env"
        );
        if valued && k + 1 < tokens.len() {
            out.push(t.clone());
            if t == "-C" {
                out.extend(expanded(k + 1));
            } else {
                out.push(tokens[k + 1].clone());
            }
            k += 2;
        } else if t.starts_with("-C") {
            out.extend(expanded(k));
            k += 1;
        } else if t.starts_with('-') {
            out.push(t.clone());
            k += 1;
        } else {
            break;
        }
    }
    out.extend_from_slice(&tokens[k..]);
    out
}

/// The pieces of `command` to judge, in order, starting at `cwd`, the directory the guard
/// starts from, or with no checkout when it has none.
pub(crate) fn walk(command: &str, cwd: Option<&Path>) -> Vec<Judged> {
    let mut judged = Vec::new();
    let start = Place {
        dir: cwd.map(Path::to_path_buf),
        lost: None,
    };
    collect(&scan::segments(command), cwd, start, &mut judged);
    judged
}

/// Collects `segments` in order into `out`, each with the checkout it runs in: `place`'s,
/// moved by each `cd` it can follow that `&&`, `;` or a newline ends, kept by one into a
/// directory that isn't there, and reset to the event's `cwd` by any other `cd`; a `cd`
/// inside `( )` or backticks stays inside them. Each `cd` it couldn't follow is kept, as
/// written, until a later one sets the checkout. A body or `-c` script a shell runs is
/// collected after the segment that runs it, from that shell's checkout, and its `cd`s
/// stay in that shell.
fn collect(segments: &[Segment], cwd: Option<&Path>, place: Place, out: &mut Vec<Judged>) {
    let mut place = place;
    let mut scopes: Vec<Place> = Vec::new();
    let mut backtick: Option<Place> = None;
    for (i, segment) in segments.iter().enumerate() {
        let (tokens, globs) = command::expand(&segment.tokens, &segment.globs);
        let command = command::parse(&tokens);
        let here = place.moved(&command, &globs);
        out.push(Judged {
            text: segment.text.clone(),
            // Bash globs every word where the shell stands, before the segment's own
            // `env -C` or `sudo -D` changes directory.
            tokens: git_globs(&command, &tokens, &globs, place.dir.as_deref()),
            checkout: here.dir.clone(),
            lost: here.lost.clone(),
            piped: segment.sep.pipes(),
            stdin: Stdin::of(segment, i > 0 && segments[i - 1].sep.pipes()),
        });
        let run = |text: &str, from: Place, out: &mut Vec<Judged>| {
            collect(&scan::segments(text), cwd, from, out);
        };
        // The shell expands a quoted substitution before the command, and its own `-C`,
        // runs (`guard-quoted-substitutions`).
        for sub in &segment.subs {
            run(sub, place.clone(), out);
        }
        if command.reads_stdin() {
            for input in segment.bodies.iter().chain(&segment.herestrings) {
                run(input, here.clone(), out);
            }
        }
        if let Some(script) = command.script() {
            run(script, here.clone(), out);
        }
        // A body piped to a later shell of the same pipeline that has no body of its own.
        if !segment.bodies.is_empty() {
            let mut k = i;
            while segments[k].sep.pipes() && k + 1 < segments.len() {
                k += 1;
                let (tokens, globs) = command::expand(&segments[k].tokens, &segments[k].globs);
                let later = command::parse(&tokens);
                if segments[k].bodies.is_empty() && later.reads_stdin() {
                    for body in &segment.bodies {
                        run(body, place.moved(&later, &globs), out);
                    }
                    break;
                }
            }
        }
        // A `cd` ended by `|`, `|&` or `&` runs apart, and one ended by `||` lets the next
        // segment run only when it failed: neither moves the checkout. One ended by an
        // opening `(` or backtick takes a substitution as its directory, which can't be
        // followed.
        let opens =
            segment.sep == Sep::Open || (segment.sep == Sep::Backtick && backtick.is_none());
        let sets = opens || matches!(segment.sep, Sep::AndAnd | Sep::Semi | Sep::Newline);
        if command.name == Some("cd") && sets {
            let mut written = std::iter::once("cd")
                .chain(command.args.iter().map(String::as_str))
                .collect::<Vec<_>>()
                .join(" ");
            // The substitution it takes as its directory lies in the segments that follow.
            match segment.sep {
                Sep::Open if opens => written.push_str("(...)"),
                Sep::Backtick if opens => written.push_str("...`"),
                _ => {}
            }
            let read = if opens {
                Cd::Resets
            } else {
                let globs = &globs[tokens.len() - command.args.len()..];
                read_cd(command.args, globs, place.dir.as_deref())
            };
            place = match read {
                // A relative `cd` from a checkout the guard fell back to is no surer than it.
                Cd::To(to, absolute) => Place {
                    lost: place.lost.filter(|_| !absolute),
                    dir: Some(to),
                },
                // A `cd` that fails leaves the shell where it was, and `;` pushes from there.
                Cd::Stays => Place {
                    dir: place.dir,
                    lost: Some(written),
                },
                Cd::Resets => Place {
                    dir: cwd.map(Path::to_path_buf),
                    lost: Some(written),
                },
            };
        }
        match segment.sep {
            Sep::Open => scopes.push(place.clone()),
            Sep::Close => {
                if let Some(outer) = scopes.pop() {
                    place = outer;
                }
            }
            Sep::Backtick => match backtick.take() {
                Some(outer) => place = outer,
                None => backtick = Some(place.clone()),
            },
            _ => {}
        }
    }
}

/// Bash's default globbing, for a `cd`'s directory and an `env -C`, `sudo -D` or `git -C`
/// value (`guard-checkout`): `*`, `?` and `[...]` (with `[!...]`, `[^...]`, ranges and
/// `[[:class:]]`), matched per path component, a wildcard never matching `/`, a leading `.`
/// matched only when the pattern writes it, an unclosed `[` taken literally and a
/// backslash-escaped character matched as itself.
mod glob {

    use std::path::{Path, PathBuf};

    /// The paths `pattern` matches on the file system, as bash writes them (relative when the
    /// pattern is), sorted by their bytes as bash sorts them in the C locale. A relative pattern
    /// with no `base` to read from matches nothing.
    pub(crate) fn matches(base: Option<&Path>, pattern: &str) -> Vec<String> {
        let (root, rest, mut found) = match pattern.strip_prefix('/') {
            Some(rest) => (PathBuf::from("/"), rest, vec!["/".to_string()]),
            None => match base {
                Some(base) => (base.to_path_buf(), pattern, vec![String::new()]),
                None => return Vec::new(),
            },
        };
        let components: Vec<&str> = rest.split('/').collect();
        for (k, component) in components.iter().enumerate() {
            if component.is_empty() {
                // `a//b`, or a trailing `/`, which only a directory matches.
                if k + 1 == components.len() {
                    found.retain(|f| root.join(f).is_dir());
                    for f in &mut found {
                        f.push('/');
                    }
                }
                continue;
            }
            let parts = parse(component);
            let wild = parts.iter().any(|p| !matches!(p, Part::Lit(_)));
            let mut next = Vec::new();
            for f in &found {
                if wild {
                    let Ok(entries) = std::fs::read_dir(root.join(f)) else {
                        continue;
                    };
                    for entry in entries.flatten() {
                        // A name that isn't UTF-8 still counts toward the matches.
                        let name = entry.file_name();
                        let name = name.to_string_lossy();
                        if matches_name(&parts, &name) {
                            next.push(join(f, &name));
                        }
                    }
                } else {
                    let literal: String = parts
                        .iter()
                        .filter_map(|p| match p {
                            Part::Lit(c) => Some(*c),
                            _ => None,
                        })
                        .collect();
                    let path = join(f, &literal);
                    if root.join(&path).symlink_metadata().is_ok() {
                        next.push(path);
                    }
                }
            }
            found = next;
        }
        found.sort();
        found
    }

    /// `name` under the written path `f`.
    fn join(f: &str, name: &str) -> String {
        match f {
            "" => name.to_string(),
            f if f.ends_with('/') => format!("{f}{name}"),
            f => format!("{f}/{name}"),
        }
    }

    /// One element of a path component's pattern.
    #[derive(Debug, PartialEq)]
    enum Part {
        Lit(char),
        /// `*`
        Any,
        /// `?`
        One,
        /// `[...]`: negated, and its items.
        Class(bool, Vec<Item>),
    }

    /// One item of a bracket expression.
    #[derive(Debug, PartialEq)]
    enum Item {
        Char(char),
        Range(char, char),
        Named(String),
    }

    /// A path component's pattern, read as bash reads it.
    fn parse(component: &str) -> Vec<Part> {
        let chars: Vec<char> = component.chars().collect();
        let mut out = Vec::new();
        let mut i = 0;
        while let Some(&c) = chars.get(i) {
            i += 1;
            out.push(match c {
                '\\' => match chars.get(i) {
                    Some(&n) => {
                        i += 1;
                        Part::Lit(n)
                    }
                    None => Part::Lit('\\'),
                },
                '*' => Part::Any,
                '?' => Part::One,
                '[' => match class(&chars, i) {
                    Some((part, next)) => {
                        i = next;
                        part
                    }
                    None => Part::Lit('['),
                },
                c => Part::Lit(c),
            });
        }
        out
    }

    /// The bracket expression whose text starts at `j`, just after its `[`, and the index after
    /// its `]`; `None` when it never closes, so the `[` is a literal.
    fn class(chars: &[char], mut j: usize) -> Option<(Part, usize)> {
        let negated = matches!(chars.get(j), Some('!' | '^'));
        j += usize::from(negated);
        let mut items = Vec::new();
        let mut first = true;
        loop {
            let &c = chars.get(j)?;
            if c == ']' && !first {
                return Some((Part::Class(negated, items), j + 1));
            }
            first = false;
            if c == '[' && chars.get(j + 1) == Some(&':') {
                let name: String = chars[j + 2..].iter().collect();
                if let Some(end) = name.find(":]") {
                    items.push(Item::Named(name[..end].to_string()));
                    j += 2 + name[..end].chars().count() + 2;
                    continue;
                }
            }
            let (low, after) = literal(chars, j)?;
            j = after;
            if chars.get(j) == Some(&'-') && chars.get(j + 1).is_some_and(|&n| n != ']') {
                let (high, after) = literal(chars, j + 1)?;
                items.push(Item::Range(low, high));
                j = after;
            } else {
                items.push(Item::Char(low));
            }
        }
    }

    /// The character at `j` in a bracket expression, a backslash escaping the next, and the
    /// index after it.
    fn literal(chars: &[char], j: usize) -> Option<(char, usize)> {
        match chars.get(j)? {
            '\\' => chars.get(j + 1).map(|&c| (c, j + 2)),
            &c => Some((c, j + 1)),
        }
    }

    /// Whether `name`, an entry of a directory, matches `parts`: a leading `.` only when the
    /// pattern writes it.
    fn matches_name(parts: &[Part], name: &str) -> bool {
        if name.starts_with('.') && parts.first() != Some(&Part::Lit('.')) {
            return false;
        }
        let name: Vec<char> = name.chars().collect();
        matches_from(parts, &name)
    }

    /// Whether `name` matches `parts`, backtracking only to the last `*`, so it takes time in
    /// proportion to their lengths' product however many `*` the pattern holds.
    fn matches_from(parts: &[Part], name: &[char]) -> bool {
        let (mut p, mut n) = (0, 0);
        // The last `*` and the name's position it was tried at.
        let mut star: Option<(usize, usize)> = None;
        while n < name.len() {
            let one = match parts.get(p) {
                Some(Part::Any) => {
                    star = Some((p, n));
                    p += 1;
                    continue;
                }
                Some(Part::One) => true,
                Some(Part::Lit(c)) => name[n] == *c,
                Some(Part::Class(negated, items)) => {
                    items.iter().any(|item| holds(item, name[n])) != *negated
                }
                None => false,
            };
            if one {
                p += 1;
                n += 1;
            } else if let Some((s, at)) = star {
                p = s + 1;
                n = at + 1;
                star = Some((s, at + 1));
            } else {
                return false;
            }
        }
        parts[p..].iter().all(|part| *part == Part::Any)
    }

    /// Whether bracket item `item` holds `c`.
    fn holds(item: &Item, c: char) -> bool {
        match item {
            Item::Char(x) => c == *x,
            Item::Range(low, high) => (*low..=*high).contains(&c),
            Item::Named(name) => match name.as_str() {
                "alpha" => c.is_ascii_alphabetic(),
                "digit" => c.is_ascii_digit(),
                "alnum" => c.is_ascii_alphanumeric(),
                "upper" => c.is_ascii_uppercase(),
                "lower" => c.is_ascii_lowercase(),
                "space" => c.is_ascii_whitespace() || c == '\x0b',
                "blank" => c == ' ' || c == '\t',
                "punct" => c.is_ascii_punctuation(),
                "xdigit" => c.is_ascii_hexdigit(),
                "cntrl" => c.is_ascii_control(),
                "graph" => c.is_ascii_graphic(),
                "print" => c.is_ascii_graphic() || c == ' ',
                "word" => c.is_ascii_alphanumeric() || c == '_',
                _ => false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The standard input of each piece of `command` whose first token is `name`.
    fn inputs(command: &str, name: &str) -> Vec<Stdin> {
        walk(command, None)
            .into_iter()
            .filter(|j| j.tokens.first().map(String::as_str) == Some(name))
            .map(|j| j.stdin)
            .collect()
    }

    fn heredoc(body: &str, as_written: bool) -> Stdin {
        Stdin::Heredoc {
            body: body.to_string(),
            as_written,
        }
    }

    /// The `git` piece of `command`, walked from `cwd`.
    fn git_piece(command: &str, cwd: Option<&Path>) -> Judged {
        walk(command, cwd)
            .into_iter()
            .find(|j| j.command().is("git"))
            .unwrap()
    }

    /// A directory holding the empty directories `names`.
    fn tree(names: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        for name in names {
            std::fs::create_dir_all(dir.path().join(name)).unwrap();
        }
        dir
    }

    // req:plugin.hooks.guard.shell/guard-checkout@dd60352e
    #[test]
    fn globbing_matches_as_bash_does() {
        let dir = tree(&["m", "x1", "x2", "xa", ".hid", "-Cm", "a[b", "a*b", "sub/y1"]);
        let at = |p: &str| glob::matches(Some(dir.path()), p);
        assert_eq!(at("x*"), ["x1", "x2", "xa"]);
        assert_eq!(at("x?"), ["x1", "x2", "xa"]);
        assert_eq!(at("x[0-9]"), ["x1", "x2"]);
        assert_eq!(at("x[!0-9]"), ["xa"]);
        assert_eq!(at("x[^12]"), ["xa"]);
        assert_eq!(at("x[[:digit:]]"), ["x1", "x2"]);
        assert_eq!(at("x[[:alpha:]]"), ["xa"]);
        assert_eq!(at("[m]"), ["m"]);
        assert_eq!(at("*1"), ["x1"]);
        assert_eq!(at("x*1*"), ["x1"]);
        assert_eq!(at("*x*"), ["x1", "x2", "xa"]);
        // A leading `.` only when the pattern writes it.
        assert!(!at("*").contains(&".hid".to_string()));
        assert_eq!(at(".h*"), [".hid"]);
        // An unclosed `[` is a literal.
        assert_eq!(at("a[*"), ["a[b"]);
        // An escaped wildcard is a literal.
        assert_eq!(at("a\\*?"), ["a*b"]);
        // Per path component; a wildcard never matches `/`.
        assert_eq!(at("s*/y*"), ["sub/y1"]);
        assert!(at("s*y1").is_empty());
        assert_eq!(at("m/../x[12]"), ["m/../x1", "m/../x2"]);
        // A glued `-C` value globs as the whole word.
        assert_eq!(at("-Cm*"), ["-Cm"]);
        // Absolute patterns read from the root; relative ones need a base.
        let absolute = format!("{}/x[a]", dir.path().display());
        assert_eq!(
            glob::matches(None, &absolute),
            [absolute.replace("[a]", "a")]
        );
        assert!(glob::matches(None, "x*").is_empty());
        assert!(at("nothing*").is_empty());
        // Many wildcards against a long name end at once.
        let long = tree(&[&"a".repeat(200)]);
        let many = format!("{}b", "*a".repeat(30));
        assert!(glob::matches(Some(long.path()), &many).is_empty());
    }

    // req:plugin.hooks.guard.shell/guard-checkout@dd60352e
    #[test]
    fn no_directory_to_start_from() {
        let dir = tree(&["m"]);
        let m = dir.path().join("m");
        for command in [
            format!("cd {} && git push", m.display()),
            format!("env -C {} git push", m.display()),
            format!("cd gone; cd {} && git push", m.display()),
        ] {
            let git = git_piece(&command, None);
            assert_eq!(git.checkout.as_deref(), Some(m.as_path()), "{command}");
        }
        let git = git_piece("cd m && git push", None);
        assert_eq!(git.checkout, None);
        assert_eq!(git.lost.as_deref(), Some("cd m"));
        assert_eq!(git_piece("env -C m git push", None).checkout, None);
        // A relative glob stays as written.
        assert_eq!(
            git_piece("git -C m* push", None).tokens,
            ["git", "-C", "m*", "push"]
        );
    }

    // req:plugin.hooks.guard.shell/guard-checkout@dd60352e
    #[test]
    fn git_c_globs_yield_every_match() {
        let dir = tree(&["m", "x1", "x2"]);
        let at = |command: &str| git_piece(command, Some(dir.path())).tokens;
        assert_eq!(at("git -C x* push"), ["git", "-C", "x1", "x2", "push"]);
        assert_eq!(at("git -C m* push"), ["git", "-C", "m", "push"]);
        assert_eq!(
            at("/usr/bin/git -C [m] push"),
            ["/usr/bin/git", "-C", "m", "push"]
        );
        assert_eq!(
            at("git --config-env a=B -C m* push"),
            ["git", "--config-env", "a=B", "-C", "m", "push"]
        );
        // A glued `-C` globs as the whole word: no entry starts with `-C`.
        assert_eq!(at("git -Cm* push"), ["git", "-Cm*", "push"]);
        // Quoted, none, or after the subcommand: as written.
        assert_eq!(at("git -C 'm*' push"), ["git", "-C", "m*", "push"]);
        assert_eq!(at("git -C z* push"), ["git", "-C", "z*", "push"]);
        assert_eq!(at("git log -C m*"), ["git", "log", "-C", "m*"]);
        // `env -S` splits its value but globs nothing in it.
        assert_eq!(
            at("env -S 'git -C m* push'"),
            ["env", "git", "-C", "m*", "push"]
        );
        // From the checkout so far, before the segment's own `env -C` or `sudo -D`.
        let s = tree(&["m", "s/m", "s/m2"]);
        for command in ["env -C s git -C m* push", "sudo -D s git -C m* push"] {
            let git = git_piece(command, Some(s.path()));
            assert_eq!(
                git.tokens[git.tokens.len() - 4..],
                ["git", "-C", "m", "push"]
            );
            assert_eq!(git.checkout.as_deref(), Some(s.path().join("s").as_path()));
        }
        let deeper = format!("cd {} && git -C ../x* push", dir.path().join("m").display());
        assert_eq!(
            git_piece(&deeper, None).tokens,
            ["git", "-C", "../x1", "../x2", "push"]
        );
        let glued = tree(&["-Cm"]);
        assert_eq!(
            git_piece("git -Cm* push", Some(glued.path())).tokens,
            ["git", "-Cm", "push"]
        );
    }

    // req:plugin.hooks.guard.shell/guard-checkout@dd60352e
    #[test]
    fn a_cd_reads_its_options_and_globs_as_bash_does() {
        let dir = tree(&["m", "x1", "x2", ".hid", "a[b"]);
        let checkout = |command: &str| git_piece(command, Some(dir.path())).checkout.unwrap();
        let m = dir.path().join("m");
        for command in [
            "cd -P m; git push",
            "cd -LPe m; git push",
            "cd -- m; git push",
            "cd m*; git push",
            "cd [m]; git push",
            "cd [!x.a]; git push",
            "cd m; cd ../x*; git push",
            "cd m; cd m x; git push",
            "cd m; cd -x .; git push",
            "cd m; cd -@ .; git push",
            "cd m; cd -x; git push",
            "env -C m* git push",
            "sudo -D [m] git push",
        ] {
            assert_eq!(checkout(command), m, "{command}");
        }
        assert_eq!(checkout("cd .h*; git push"), dir.path().join(".hid"));
        assert_eq!(checkout("cd a[b; git push"), dir.path().join("a[b"));
        // A quoted glob is the word as written; none there, so the `cd` fails.
        assert_eq!(checkout("cd 'm*'; git push"), dir.path());
        assert_eq!(checkout("cd m\\*; git push"), dir.path());
        // Several matches, or a file, fail the `cd`.
        assert_eq!(checkout("cd x*; git push"), dir.path());
        std::fs::write(dir.path().join("f"), "").unwrap();
        assert_eq!(checkout("cd f; git push"), dir.path());
        // `$`, `~` and backticks reset, unexpanded, before any glob.
        for command in ["cd m; cd $D*; git push", "cd m; cd ~/m*; git push"] {
            assert_eq!(checkout(command), dir.path(), "{command}");
        }
        // No directory, or `-`, resets too.
        for command in [
            "cd m; cd; git push",
            "cd m; cd -; git push",
            "cd m; cd -P; git push",
        ] {
            assert_eq!(checkout(command), dir.path(), "{command}");
        }
    }

    // req:plugin.hooks.guard.shell/guard-standard-input@a171c29b
    #[test]
    fn a_quoted_heredoc_is_sent_as_written() {
        assert_eq!(
            inputs("clm defect --title x <<'EOF'\ncost $HOME\nEOF", "clm"),
            [heredoc("cost $HOME", true)]
        );
    }

    // req:plugin.hooks.guard.shell/guard-standard-input@a171c29b
    #[test]
    fn an_unquoted_heredoc_is_sent_as_written_only_without_expansions() {
        assert_eq!(
            inputs("clm defect <<EOF\nplain words\nEOF", "clm"),
            [heredoc("plain words", true)]
        );
        assert_eq!(
            inputs("clm defect <<EOF\ncost $HOME\nEOF", "clm"),
            [heredoc("cost $HOME", false)]
        );
        assert_eq!(
            inputs("clm defect <<E\\OF\ncost $HOME\nEOF", "clm"),
            [heredoc("cost $HOME", true)]
        );
        assert_eq!(
            inputs("clm defect <<EOF\ncost `date`\nEOF", "clm"),
            [heredoc("cost `date`", false)]
        );
        assert_eq!(
            inputs("clm defect <<EOF\ncost \\$5\nEOF", "clm"),
            [heredoc("cost \\$5", false)]
        );
    }

    // req:plugin.hooks.guard.shell/guard-standard-input@a171c29b
    #[test]
    fn a_dash_heredoc_loses_its_leading_tabs() {
        assert_eq!(
            inputs("clm defect <<-'EOF'\n\ta\n\t\tb\n\tEOF", "clm"),
            [heredoc("a\nb", true)]
        );
    }

    // req:plugin.hooks.guard.shell/guard-standard-input@a171c29b
    #[test]
    fn the_last_redirection_wins() {
        assert_eq!(
            inputs("clm defect <<'EOF' < f\nr\nEOF", "clm"),
            [Stdin::File]
        );
        assert_eq!(
            inputs("cat r | clm defect <<'EOF'\nr\nEOF", "clm"),
            [heredoc("r", true)]
        );
        assert_eq!(
            inputs("clm defect < f <<< x <<'EOF'\nr\nEOF", "clm"),
            [heredoc("r", true)]
        );
        // another descriptor
        assert_eq!(inputs("clm defect 3< f", "clm"), [Stdin::Own]);
        assert_eq!(inputs("clm defect 3<<'EOF'\nr\nEOF", "clm"), [Stdin::Own]);
        assert_eq!(inputs("clm defect 0< f", "clm"), [Stdin::File]);
        assert_eq!(inputs("clm defect 0<&3", "clm"), [Stdin::File]);
        assert_eq!(inputs("clm defect <> f", "clm"), [Stdin::File]);
    }

    // req:plugin.hooks.guard.shell/guard-standard-input@a171c29b
    #[test]
    fn other_sources() {
        assert_eq!(inputs("clm defect < r.md", "clm"), [Stdin::File]);
        assert_eq!(inputs("cat r.md | clm defect", "clm"), [Stdin::Pipe]);
        assert_eq!(inputs("cat r.md |& clm defect", "clm"), [Stdin::Pipe]);
        assert_eq!(inputs("clm defect <<< \"$R\"", "clm"), [Stdin::HereString]);
        assert_eq!(inputs("clm defect", "clm"), [Stdin::Own]);
        assert_eq!(inputs("clm defect <(echo r)", "clm"), [Stdin::Own]);
        assert_eq!(inputs("cat r.md; clm defect", "clm"), [Stdin::Own]);
    }

    // req:plugin.hooks.guard.shell/guard-standard-input@a171c29b
    #[test]
    fn a_heredoc_inside_a_substitution_belongs_to_its_own_piece() {
        let walked = walk("git commit -m \"$(cat <<'EOF'\nmsg\nEOF\n)\"", None);
        let of = |name: &str| {
            walked
                .iter()
                .find(|j| j.tokens.first().map(String::as_str) == Some(name))
                .map(|j| j.stdin.clone())
        };
        assert_eq!(of("git"), Some(Stdin::Own));
        assert_eq!(of("cat"), Some(heredoc("msg", true)));
    }

    // req:plugin.hooks.guard.shell/guard-standard-input@a171c29b
    #[test]
    fn a_script_and_a_body_judged_as_commands_keep_their_own_input() {
        assert_eq!(
            inputs("bash -c 'clm defect < f' <<'EOF'\nx\nEOF", "clm"),
            [Stdin::File]
        );
        assert_eq!(
            inputs("bash -c 'clm defect < f' <<'EOF'\nx\nEOF", "bash"),
            [heredoc("x", true)]
        );
        assert_eq!(inputs("bash -c 'clm defect < f'", "bash"), [Stdin::Own]);
        assert_eq!(inputs("bash <<'EOF'\nclm defect\nEOF", "clm"), [Stdin::Own]);
    }
}
