//! A segment's command (`guard-command-position`): its first token past the assignments,
//! shell words and prefix commands that only run it.

/// What a segment runs.
pub struct Command<'a> {
    /// The command's name, or `None` for a segment of only prefixes.
    pub name: Option<&'a str>,
    /// The tokens after the name.
    pub args: &'a [String],
    /// Every `NAME=value` among the skipped tokens, in order; the last of a name wins.
    pub assigns: Vec<(&'a str, &'a str)>,
    /// The directories `env -C` or `sudo -D` move the command's checkout by, in order.
    pub chdir: Vec<&'a str>,
}

impl Command<'_> {
    /// Whether the command is `name`, alone or as a path ending in `/<name>`.
    pub fn is(&self, name: &str) -> bool {
        self.name
            .is_some_and(|n| n == name || n.strip_suffix(name).is_some_and(|p| p.ends_with('/')))
    }

    /// The value the segment assigns to `name` for its command.
    pub(crate) fn assigned(&self, name: &str) -> Option<&str> {
        self.assigns
            .iter()
            .rev()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| *v)
    }
}

/// The prefix commands, each with its options that take a value and which of those
/// change directory.
const PREFIXES: [(&str, &[&str], &[&str]); 10] = [
    (
        "env",
        &["-u", "-C", "-S", "--unset", "--chdir", "--split-string"],
        &["-C", "--chdir"],
    ),
    (
        "sudo",
        &[
            "-u",
            "-g",
            "-h",
            "-p",
            "-C",
            "-D",
            "-r",
            "-t",
            "-U",
            "--user",
            "--group",
            "--host",
            "--prompt",
            "--chdir",
            "--close-from",
            "--role",
            "--type",
            "--other-user",
        ],
        &["-D", "--chdir"],
    ),
    ("doas", &["-u", "-C"], &[]),
    ("command", &[], &[]),
    ("exec", &["-a"], &[]),
    ("nohup", &[], &[]),
    ("nice", &["-n", "--adjustment"], &[]),
    ("ionice", &["-c", "-n", "-p"], &[]),
    ("stdbuf", &["-i", "-o", "-e"], &[]),
    (
        "xargs",
        &["-I", "-n", "-P", "-L", "-s", "-d", "-a", "-E"],
        &[],
    ),
];

/// `timeout`'s options that take a value.
const TIMEOUT_VALUES: [&str; 4] = ["-s", "-k", "--signal", "--kill-after"];

/// `env`'s option whose value holds its further arguments (`-S`, `--split-string`).
const SPLIT: [&str; 2] = ["-S", "--split-string"];

/// One prefix option that takes a value, as read from the tokens.
struct Valued<'a> {
    /// The option, `-x` or `--name`.
    name: &'a str,
    value: Option<&'a str>,
    /// The letters of a cluster before the option, as an option of their own (`-i` of
    /// `-iS`), when there are any.
    before: Option<&'a str>,
}

/// Reads the option token `tokens[i]` of a prefix whose options taking a value are
/// `values`, and returns the option that takes one, if any, and the index after it and
/// its value. A short option's cluster (`-Eu root`, `-uroot`) is read as `getopt` reads
/// it: the first letter that takes a value takes the rest of the token, or the next token
/// when it is the last letter.
fn option<'a>(tokens: &'a [String], i: usize, values: &[&'a str]) -> (Option<Valued<'a>>, usize) {
    let o = tokens[i].as_str();
    // The option's value: `glued` when the token holds it, else the next token.
    let valued = |name, glued: Option<&'a str>, before| {
        let value = glued.or(tokens.get(i + 1).map(String::as_str));
        let next = if glued.is_some() { i + 1 } else { i + 2 };
        (
            Some(Valued {
                name,
                value,
                before,
            }),
            next,
        )
    };
    if o.starts_with("--") {
        return match o.split_once('=') {
            Some((name, value)) if values.contains(&name) => valued(name, Some(value), None),
            None if values.contains(&o) => valued(o, None, None),
            _ => (None, i + 1),
        };
    }
    for (k, letter) in o.char_indices().skip(1) {
        let at = k + letter.len_utf8();
        let short = |v: &&&str| v.len() == at - k + 1 && v[1..] == o[k..at];
        if let Some(name) = values.iter().find(short) {
            let rest = Some(&o[at..]).filter(|r| !r.is_empty());
            return valued(name, rest, (k > 1).then_some(&o[..k]));
        }
    }
    (None, i + 1)
}

/// An `env -S` value to split in place: the tokens `at..end` it spans, the cluster's
/// letters before it, and the value.
struct Split<'a> {
    at: usize,
    end: usize,
    before: Option<&'a str>,
    value: &'a str,
}

/// The command of a segment with `tokens`.
pub fn parse(tokens: &[String]) -> Command<'_> {
    walk(tokens).0
}

/// `tokens` with each `env -S`/`--split-string` value among the command's prefixes split
/// as `guard-quotes` splits a segment and read in the option's place, as `env` reads it
/// (`guard-command-position`). Every rule reads a segment's tokens this way.
pub fn expand(tokens: &[String]) -> Vec<String> {
    let mut tokens = tokens.to_vec();
    // A value may hold another `env -S`. Each round replaces the option and its value with
    // fewer characters than they had, so the rounds end.
    while let Some(split) = walk(&tokens).1 {
        let mut out = tokens[..split.at].to_vec();
        out.extend(split.before.map(String::from));
        out.extend(
            crate::shell::scan::segments(split.value)
                .into_iter()
                .flat_map(|s| s.tokens),
        );
        out.extend_from_slice(&tokens[split.end..]);
        tokens = out;
    }
    tokens
}

/// The command of a segment with `tokens`, and the first `env -S` value among its
/// prefixes.
fn walk(tokens: &[String]) -> (Command<'_>, Option<Split<'_>>) {
    let mut assigns = Vec::new();
    let mut chdir = Vec::new();
    let mut i = 0;
    while let Some(t) = tokens.get(i) {
        let t = t.as_str();
        i += 1;
        if let Some(assign) = assignment(t) {
            assigns.push(assign);
            continue;
        }
        let is_option = |at: usize| tokens.get(at).is_some_and(|o| o.starts_with('-'));
        match t {
            "!" | "{" | "if" | "then" | "elif" | "else" | "do" | "while" | "until" => {}
            "time" => i += usize::from(tokens.get(i).is_some_and(|o| o == "-p")),
            "timeout" => {
                while is_option(i) {
                    i = option(tokens, i, &TIMEOUT_VALUES).1;
                }
                // The duration.
                i += 1;
            }
            _ => {
                let Some((prefix, values, dirs)) = PREFIXES.iter().find(|(p, ..)| *p == t) else {
                    let command = Command {
                        name: Some(t),
                        args: &tokens[i..],
                        assigns,
                        chdir,
                    };
                    return (command, None);
                };
                let env = *prefix == "env";
                while is_option(i) {
                    let at = i;
                    let (valued, next) = option(tokens, i, values);
                    i = next;
                    let Some(valued) = valued else { continue };
                    if env && SPLIT.contains(&valued.name) {
                        let split = valued.value.map(|value| Split {
                            at,
                            end: i,
                            before: valued.before,
                            value,
                        });
                        let command = Command {
                            name: None,
                            args: &[],
                            assigns,
                            chdir,
                        };
                        return (command, split);
                    }
                    if dirs.contains(&valued.name) {
                        chdir.extend(valued.value);
                    }
                }
            }
        }
    }
    let command = Command {
        name: None,
        args: &[],
        assigns,
        chdir,
    };
    (command, None)
}

/// A `NAME=value` assignment's name and value.
pub fn assignment(t: &str) -> Option<(&str, &str)> {
    let (name, value) = t.split_once('=')?;
    let valid = name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    valid.then_some((name, value))
}

/// The shells whose script or standard input the guard judges as commands.
const SHELLS: [&str; 5] = ["bash", "sh", "zsh", "dash", "ksh"];

impl Command<'_> {
    fn is_shell(&self) -> bool {
        SHELLS.iter().any(|s| self.is(s))
    }

    /// Whether the command runs a shell over its standard input (`guard-heredocs`): a shell
    /// none of whose leading short options holds `c`, or `source`/`.` of standard input.
    pub(crate) fn reads_stdin(&self) -> bool {
        if self.is_shell() {
            return !self
                .args
                .iter()
                .take_while(|a| a.starts_with('-'))
                .any(|a| !a.starts_with("--") && a.contains('c'));
        }
        matches!(self.name, Some("source" | "."))
            && self.args.first().is_some_and(|a| {
                matches!(a.as_str(), "/dev/stdin" | "/dev/fd/0" | "/proc/self/fd/0")
            })
    }

    /// The script a shell runs with `-c` (`guard-shell-scripts`): the first token that
    /// doesn't start with `-` after a token starting with a single `-` that holds `c`.
    pub fn script(&self) -> Option<&str> {
        if !self.is_shell() {
            return None;
        }
        let c = self
            .args
            .iter()
            .position(|a| a.starts_with('-') && !a.starts_with("--") && a.contains('c'))?;
        self.args[c + 1..]
            .iter()
            .find(|a| !a.starts_with('-'))
            .map(String::as_str)
    }
}
