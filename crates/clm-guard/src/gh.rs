//! Reading a segment's `gh` calls, for `release-guard` and `host-routes-guard`.

/// Each `gh` call of a segment: the tokens after each token that is `gh` or ends in `/gh`,
/// anywhere in the segment, which covers any prefix, and a prefix's value that is `gh` too.
pub fn calls(tokens: &[String]) -> impl Iterator<Item = &[String]> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, t)| *t == "gh" || t.ends_with("/gh"))
        .map(|(i, _)| &tokens[i + 1..])
}

/// A `gh` call's command and, for each command but `api`, its subcommand, each with its
/// index in `args`, read as `gh` finds its command: `--help` and `-h` stand alone; any other
/// `--<name>` without `=`, or `-` and one character, takes the next token as its value;
/// other options stand alone; `--` ends the command words.
pub fn words(args: &[String]) -> Vec<(usize, &str)> {
    let mut words = Vec::new();
    let mut i = 0;
    while let Some(t) = args.get(i).map(String::as_str) {
        let wanted = match words.first() {
            Some(&(_, "api")) => 1,
            _ => 2,
        };
        if words.len() == wanted {
            break;
        }
        match t {
            "--" => break,
            "--help" | "-h" => {}
            _ if t.starts_with("--") && !t.contains('=') => i += 1,
            _ if t.starts_with('-') && t.chars().count() == 2 => i += 1,
            _ if t.starts_with('-') => {}
            _ => words.push((i, t)),
        }
        i += 1;
    }
    words
}

/// Whether `gh api` arguments send fields or a body, or set any method other than `GET`.
pub(crate) fn api_writes(args: &[String]) -> bool {
    api_methods(args).any(|m| !m.eq_ignore_ascii_case("GET"))
        || args.iter().any(|t| {
            ["--field", "--raw-field", "--input", "-f", "-F"]
                .iter()
                .any(|p| t.starts_with(p))
        })
}

/// Every method `gh api` arguments set with `-X` or `--method`, alone or with the value
/// glued on (`-XPOST`, `--method=post`).
fn api_methods(args: &[String]) -> impl Iterator<Item = &str> {
    args.iter()
        .enumerate()
        .filter_map(|(i, t)| match t.as_str() {
            "-X" | "--method" => args.get(i + 1).map(String::as_str),
            _ => t
                .strip_prefix("--method=")
                .or_else(|| t.strip_prefix("-X"))
                .filter(|m| !m.is_empty()),
        })
}

/// The values of `gh api`'s fields: after `-f`, `-F`, `--field` or `--raw-field`, or after
/// the option's `=` or glued on.
pub(crate) fn api_fields(args: &[String]) -> Vec<&str> {
    args.iter()
        .enumerate()
        .filter_map(|(i, t)| match t.as_str() {
            "-f" | "-F" | "--field" | "--raw-field" => args.get(i + 1).map(String::as_str),
            _ => t
                .strip_prefix("--field=")
                .or_else(|| t.strip_prefix("--raw-field="))
                .or_else(|| t.strip_prefix("-f"))
                .or_else(|| t.strip_prefix("-F"))
                .filter(|v| !v.is_empty()),
        })
        .collect()
}
