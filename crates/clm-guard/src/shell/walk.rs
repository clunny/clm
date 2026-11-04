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
    // The defect ask reads it (`plugin.hooks.guard/defect-approval`); until then only the
    // walk's own tests do.
    #[cfg_attr(not(test), allow(dead_code))]
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
    /// The place `command` runs in: moved by its own `env -C` or `sudo -D`, and set
    /// outright, so no `cd` is lost, by an absolute one.
    fn moved(&self, command: &Command) -> Place {
        let absolute = command.chdir.iter().any(|d| Path::new(d).is_absolute());
        Place {
            dir: moved(&self.dir, command),
            lost: self.lost.clone().filter(|_| !absolute),
        }
    }
}

/// The pieces of `command` to judge, in order, starting at the event's `cwd`.
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
        let tokens = command::expand(&segment.tokens);
        let command = command::parse(&tokens);
        let here = place.moved(&command);
        out.push(Judged {
            text: segment.text.clone(),
            tokens: tokens.clone(),
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
                let tokens = command::expand(&segments[k].tokens);
                let later = command::parse(&tokens);
                if segments[k].bodies.is_empty() && later.reads_stdin() {
                    for body in &segment.bodies {
                        run(body, place.moved(&later), out);
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
            let follows = |a: &String| a != "-" && !a.contains(['$', '~', '`']);
            let to = match command.args {
                [to] if !opens && follows(to) => place.dir.as_ref().map(|d| d.join(to)),
                _ => None,
            };
            place = match to {
                // A relative `cd` from a checkout the guard fell back to is no surer than it.
                Some(to) if to.is_dir() => Place {
                    lost: place.lost.filter(|_| !command.args[0].starts_with('/')),
                    dir: Some(to),
                },
                // A `cd` that fails leaves the shell where it was, and `;` pushes from there.
                Some(_) => Place {
                    dir: place.dir,
                    lost: Some(written),
                },
                None => Place {
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

/// `dir` moved by the command's own `env -C` or `sudo -D`.
fn moved(dir: &Option<PathBuf>, command: &Command) -> Option<PathBuf> {
    let mut dir = dir.clone()?;
    for d in &command.chdir {
        dir.push(d);
    }
    Some(dir)
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

    // req:plugin.hooks.guard.shell/guard-standard-input@1b42414f
    #[test]
    fn a_quoted_heredoc_is_sent_as_written() {
        assert_eq!(
            inputs("clm defect --title x <<'EOF'\ncost $HOME\nEOF", "clm"),
            [heredoc("cost $HOME", true)]
        );
    }

    // req:plugin.hooks.guard.shell/guard-standard-input@1b42414f
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
    }

    // req:plugin.hooks.guard.shell/guard-standard-input@1b42414f
    #[test]
    fn a_dash_heredoc_loses_its_leading_tabs() {
        assert_eq!(
            inputs("clm defect <<-'EOF'\n\ta\n\t\tb\n\tEOF", "clm"),
            [heredoc("a\nb", true)]
        );
    }

    // req:plugin.hooks.guard.shell/guard-standard-input@1b42414f
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
        assert_eq!(inputs("clm defect 3< f", "clm"), [Stdin::Own]);
        assert_eq!(inputs("clm defect 0<&3", "clm"), [Stdin::File]);
        assert_eq!(inputs("clm defect <> f", "clm"), [Stdin::File]);
    }

    // req:plugin.hooks.guard.shell/guard-standard-input@1b42414f
    #[test]
    fn other_sources() {
        assert_eq!(inputs("clm defect < r.md", "clm"), [Stdin::File]);
        assert_eq!(inputs("cat r.md | clm defect", "clm"), [Stdin::Pipe]);
        assert_eq!(inputs("cat r.md |& clm defect", "clm"), [Stdin::Pipe]);
        assert_eq!(inputs("clm defect <<< \"$R\"", "clm"), [Stdin::HereString]);
        assert_eq!(inputs("clm defect", "clm"), [Stdin::Own]);
        assert_eq!(inputs("cat r.md; clm defect", "clm"), [Stdin::Own]);
    }

    // req:plugin.hooks.guard.shell/guard-standard-input@1b42414f
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

    // req:plugin.hooks.guard.shell/guard-standard-input@1b42414f
    #[test]
    fn a_script_and_a_body_judged_as_commands_keep_their_own_input() {
        assert_eq!(
            inputs("bash -c 'clm defect < f' <<'EOF'\nx\nEOF", "clm"),
            [Stdin::File]
        );
        assert_eq!(inputs("bash <<'EOF'\nclm defect\nEOF", "clm"), [Stdin::Own]);
    }
}
