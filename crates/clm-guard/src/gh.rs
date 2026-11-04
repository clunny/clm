//! Reading a segment's `gh` call, for `release-guard` and `push-guard`.

/// The arguments of a segment's `gh` call: the tokens after the first that is `gh` or ends
/// in `/gh`, anywhere in the segment, which covers any prefix.
pub fn args(tokens: &[String]) -> Option<&[String]> {
    let gh = tokens
        .iter()
        .position(|t| t == "gh" || t.ends_with("/gh"))?;
    Some(&tokens[gh + 1..])
}

/// A `gh` call's command words and positional arguments, skipping flags and the values of
/// `-R`/`--repo`, which gh also takes between a command group and its subcommand
/// (`gh release -R o/r create`), and of `-j`/`--job`.
pub fn words(args: &[String]) -> Vec<&str> {
    args.iter()
        .enumerate()
        .filter(|&(i, t)| {
            !t.starts_with('-')
                && (i == 0 || !matches!(args[i - 1].as_str(), "-R" | "--repo" | "-j" | "--job"))
        })
        .map(|(_, t)| t.as_str())
        .collect()
}

/// Whether `gh api` arguments send fields or a body, or set a method other than `GET`.
pub(crate) fn api_writes(args: &[String]) -> bool {
    api_method(args).is_some_and(|m| !m.eq_ignore_ascii_case("GET"))
        || args.iter().any(|t| {
            ["--field", "--raw-field", "--input", "-f", "-F"]
                .iter()
                .any(|p| t.starts_with(p))
        })
}

/// The method `gh api` arguments set with `-X` or `--method`, alone or with the value glued
/// on (`-XPOST`, `--method=post`).
fn api_method(args: &[String]) -> Option<&str> {
    args.iter().enumerate().find_map(|(i, t)| match t.as_str() {
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
