//! The lines of a `change.md` the command touches, and whether it records renames.

use serde_yaml::Value;

/// The frontmatter lines of `source` and the index of each line's end, when it has any.
fn frontmatter(source: &str) -> Option<(Vec<&str>, usize)> {
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    if lines.first()?.trim_end() != "---" {
        return None;
    }
    let end = lines[1..].iter().position(|l| l.trim_end() == "---")? + 1;
    Some((lines, end))
}

/// Whether `source`'s frontmatter sets `renames`, read without a graph (`replay`).
pub(crate) fn records_renames(source: &str) -> bool {
    let Some((lines, end)) = frontmatter(source) else {
        return false;
    };
    let yaml: String = lines[1..end].concat();
    serde_yaml::from_str::<Value>(&yaml)
        .ok()
        .and_then(|v| {
            v.get("renames")
                .and_then(Value::as_sequence)
                .map(|s| !s.is_empty())
        })
        .unwrap_or(false)
}

/// `source` with its `status:` line set to `status`, and nothing else changed.
pub(crate) fn with_status(source: &str, status: &str) -> String {
    let Some((lines, end)) = frontmatter(source) else {
        return source.to_string();
    };
    let mut done = false;
    let mut out = String::new();
    for (n, line) in lines.iter().enumerate() {
        if n > 0 && n < end && !done && line.starts_with("status:") {
            done = true;
            out.push_str(&format!("status: {status}\n"));
        } else {
            out.push_str(line);
        }
    }
    out
}
