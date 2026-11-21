//! `clm upgrade` and its flags: what each prints, and what it writes in the working tree. Each
//! returns the text to print, or the refusal to print with exit status 1.

use std::fs;
use std::path::Path;

use crate::pending::{pending_in, skipped, strings, superseded, table};
use crate::{Kind, Level, Migration, latest_in};

fn pending_or_refuse<'a>(
    list: &'a [Migration],
    supersedes: &[(&str, &str)],
    root: &Path,
) -> Result<Vec<&'a Migration>, String> {
    pending_in(list, supersedes, root).map_err(|too_new| too_new.message())
}

/// `clm upgrade` with no flag: one line per pending migration of `list`, or `up to date`.
pub fn list_in(
    list: &[Migration],
    supersedes: &[(&str, &str)],
    root: &Path,
) -> Result<String, String> {
    let pending = pending_or_refuse(list, supersedes, root)?;
    if pending.is_empty() {
        return Ok("up to date\n".to_string());
    }
    let skipped = skipped(root);
    let mut out = String::new();
    for m in pending {
        let level = match m.level {
            Level::Required => "required",
            Level::Advised => "advised",
        };
        out.push_str(m.id);
        out.push(' ');
        out.push_str(level);
        match &m.kind {
            Kind::Apply(_, description) => {
                out.push_str(" apply");
                if let Some(first) = description.lines().next() {
                    out.push(' ');
                    out.push_str(first);
                }
            }
            Kind::Note(n) => {
                out.push_str(" note ");
                out.push_str(n.text.lines().next().unwrap_or(""));
            }
        }
        if m.level == Level::Required && skipped.iter().any(|s| s == m.id) {
            out.push_str(" skipped");
        }
        out.push('\n');
    }
    Ok(out)
}

/// `clm upgrade --apply`: run the pending apply migrations of `list`, rewrite unedited note files,
/// stamp `upgraded` with the last of `list`.
pub fn apply_in(
    list: &[Migration],
    supersedes: &[(&str, &str)],
    root: &Path,
) -> Result<String, String> {
    check_clm_toml(root)?;
    let pending = pending_or_refuse(list, supersedes, root)?;
    let mut out = String::new();
    let mut failures = String::new();
    for m in &pending {
        let result = match &m.kind {
            Kind::Apply(run, _) => run(root).map(|()| true),
            Kind::Note(n) => match n.new {
                Some(new)
                    if fs::read_to_string(root.join(n.file))
                        .is_ok_and(|text| n.old.contains(&text.as_str())) =>
                {
                    fs::write(root.join(n.file), new)
                        .map(|()| true)
                        .map_err(|e| format!("can't write {}: {e}", n.file))
                }
                _ => Ok(false),
            },
        };
        match result {
            Ok(true) => out.push_str(&format!("applied {}\n", m.id)),
            Ok(false) => {}
            Err(e) => failures.push_str(&format!("migration `{}` failed: {e}\n", m.id)),
        }
    }
    if !failures.is_empty() {
        // Never stop early, but never stamp a project a migration failed on.
        return Err(format!("{out}{failures}"));
    }
    // Judged before the stamp is written, so a note the stamp takes is shown on the run that
    // takes it.
    for m in pending {
        if matches!(m.kind, Kind::Note(_))
            && !(m.done)(root)
            && !superseded(list, supersedes, m.id, root)
        {
            out.push_str(&format!("note {}\n", m.id));
        }
    }
    if let Some(latest) = latest_in(list) {
        set_key(root, "upgraded", &quote(latest)).map_err(|e| format!("{out}{e}"))?;
    }
    Ok(out)
}

/// Refuses a `clm.toml` that exists and can't be read as UTF-8 text or doesn't parse as TOML,
/// so nothing runs on settings `--apply` couldn't stamp. A missing one is fine.
fn check_clm_toml(root: &Path) -> Result<(), String> {
    let bytes = match fs::read(root.join("clm.toml")) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(format!("can't read clm.toml: {e}")),
    };
    let text = String::from_utf8(bytes).map_err(|_| "clm.toml isn't UTF-8 text".to_string())?;
    text.parse::<toml::Table>()
        .map(|_| ())
        .map_err(|e| format!("clm.toml doesn't parse as TOML: {e}"))
}

/// `clm upgrade --note <id>`: the migration's note.
pub fn note_in(
    list: &[Migration],
    supersedes: &[(&str, &str)],
    root: &Path,
    id: &str,
) -> Result<String, String> {
    pending_or_refuse(list, supersedes, root)?;
    match list.iter().find(|m| m.id == id).map(|m| &m.kind) {
        None => Err(format!("`{id}` is not a migration")),
        Some(Kind::Apply(..)) => Err(format!("`{id}` is an apply migration and has no note")),
        Some(Kind::Note(n)) => Ok(format!("{}\n", n.text.trim_end())),
    }
}

/// `clm upgrade --decline <id>`: add a pending note migration to `declined_migrations`.
pub fn decline_in(
    list: &[Migration],
    supersedes: &[(&str, &str)],
    root: &Path,
    id: &str,
) -> Result<String, String> {
    let pending = pending_or_refuse(list, supersedes, root)?;
    if !list.iter().any(|m| m.id == id) {
        return Err(format!("`{id}` is not a migration"));
    }
    if !pending
        .iter()
        .any(|m| m.id == id && matches!(m.kind, Kind::Note(_)))
    {
        return Err(format!("`{id}` is not a pending note migration"));
    }
    let config = table(&root.join("clm.toml"))
        .ok_or_else(|| "clm.toml is missing or doesn't parse; fix it first".to_string())?;
    let mut declined = strings(&config, "declined_migrations");
    declined.push(id.to_string());
    let value = format!(
        "[{}]",
        declined
            .iter()
            .map(|d| quote(d))
            .collect::<Vec<_>>()
            .join(", ")
    );
    set_key(root, "declined_migrations", &value)?;
    Ok(format!("declined {id}\n"))
}

fn quote(s: &str) -> String {
    toml::Value::String(s.to_string()).to_string()
}

/// Sets top-level `key` in `clm.toml` to the TOML `value`, writing only when that changes it.
fn set_key(root: &Path, key: &str, value: &str) -> Result<(), String> {
    let path = root.join("clm.toml");
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(format!("can't read clm.toml: {e}")),
    };
    let new = set_top_level(&text, key, value)?;
    if new != text {
        fs::write(&path, new).map_err(|e| format!("can't write clm.toml: {e}"))?;
    }
    Ok(())
}

fn is_header(line: &str) -> bool {
    line.trim_start().starts_with('[')
}

/// The byte spans of the values that set top-level `key`, however its key is written: one for a
/// plain or quoted key, one per leaf for a dotted one (`key.part = ...`). A header table of that
/// name sets nothing at the top level and gives none.
fn value_spans(doc: &toml_edit::Table, key: &str) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    if let Some(item) = doc.get(key) {
        leaves(item, &mut out);
    }
    out
}

/// The spans of the values a top-level item sets, following dotted keys.
fn leaves(item: &toml_edit::Item, out: &mut Vec<std::ops::Range<usize>>) {
    match item {
        toml_edit::Item::Table(t) if t.is_dotted() => {
            for (_, child) in t.iter() {
                leaves(child, out);
            }
        }
        toml_edit::Item::Value(v) => out.extend(v.span()),
        _ => {}
    }
}

/// `text` with the lines that set top-level `key` replaced by one line `key = value`, where the
/// first of them was; or else that line added above the first table header's lead-in (its
/// directly preceding comment lines and the blank lines above them), or at the end when there is
/// none. Every other line is left as it is. The lines are found from the parse, so a bracket in a
/// comment or a string ends nothing. Fails, naming `clm.toml`, when `text` isn't TOML.
pub(crate) fn set_top_level(text: &str, key: &str, value: &str) -> Result<String, String> {
    let new = edit_top_level(text, key, value)?;
    // A table of the same name (`[key]`, `[key.part]`) can't be replaced line by line; refuse
    // rather than write a clm.toml that no longer parses.
    toml_edit::ImDocument::parse(new.as_str()).map_err(|_| {
        format!("can't set `{key}` in clm.toml: a table of that name is in the way; fix it by hand")
    })?;
    Ok(new)
}

fn edit_top_level(text: &str, key: &str, value: &str) -> Result<String, String> {
    let doc = toml_edit::ImDocument::parse(text)
        .map_err(|e| format!("clm.toml doesn't parse as TOML: {e}"))?;
    let new_line = format!("{key} = {value}\n");
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let line_of = |at: usize| text[..at].matches('\n').count();
    let mut taken = vec![false; lines.len()];
    for span in value_spans(doc.as_table(), key) {
        // A key and its value start on one line; the value may run on over several.
        let last = line_of(span.end.saturating_sub(1).max(span.start));
        taken[line_of(span.start)..=last].fill(true);
    }
    if let Some(first) = taken.iter().position(|t| *t) {
        let mut out = String::new();
        for (i, line) in lines.iter().enumerate() {
            if i == first {
                out.push_str(&new_line);
            }
            if !taken[i] {
                out.push_str(line);
            }
        }
        return Ok(out);
    }
    // A line a top-level value runs on over, such as one of a multi-line array or string, is no
    // header whatever it starts with.
    let mut inside = vec![false; lines.len()];
    let mut spans = Vec::new();
    for (_, item) in doc.as_table().iter() {
        leaves(item, &mut spans);
    }
    for span in spans {
        let last = line_of(span.end.saturating_sub(1).max(span.start));
        inside[line_of(span.start) + 1..=last].fill(true);
    }
    let first_header = (0..lines.len()).find(|&i| !inside[i] && is_header(lines[i]));
    Ok(match first_header {
        Some(header) => {
            // The header's lead-in: the comment lines directly above it, then the blank lines
            // above those.
            let mut at = header;
            while at > 0 && lines[at - 1].trim_start().starts_with('#') {
                at -= 1;
            }
            while at > 0 && lines[at - 1].trim().is_empty() {
                at -= 1;
            }
            let blank = if lines[at].trim().is_empty() {
                ""
            } else {
                "\n"
            };
            [&lines[..at], &[new_line.as_str(), blank], &lines[at..]]
                .concat()
                .concat()
        }
        None if text.is_empty() || text.ends_with('\n') => format!("{text}{new_line}"),
        None => format!("{text}\n{new_line}"),
    })
}
