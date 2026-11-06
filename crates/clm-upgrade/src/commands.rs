//! `clm upgrade` and its flags: what each prints, and what it writes in the working tree. Each
//! returns the text to print, or the refusal to print with exit status 1.

use std::fs;
use std::path::Path;

use crate::pending::{pending_in, skipped, strings, table};
use crate::{Kind, Level, Migration, migrations};

/// `clm upgrade` with no flag: one line per pending migration, or `up to date`.
pub fn list(root: &Path) -> Result<String, String> {
    list_in(migrations(), root)
}

/// `clm upgrade --apply`: run the pending apply migrations, rewrite unedited note files, stamp
/// `upgraded`.
pub fn apply(root: &Path) -> Result<String, String> {
    apply_in(migrations(), root)
}

/// `clm upgrade --note <id>`: the migration's note.
pub fn note(root: &Path, id: &str) -> Result<String, String> {
    note_in(migrations(), root, id)
}

/// `clm upgrade --decline <id>`: add a pending note migration to `declined_migrations`.
pub fn decline(root: &Path, id: &str) -> Result<String, String> {
    decline_in(migrations(), root, id)
}

fn pending_or_refuse<'a>(list: &'a [Migration], root: &Path) -> Result<Vec<&'a Migration>, String> {
    pending_in(list, root).map_err(|too_new| too_new.message())
}

pub(crate) fn list_in(list: &[Migration], root: &Path) -> Result<String, String> {
    let pending = pending_or_refuse(list, root)?;
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
            Kind::Apply(_) => out.push_str(" apply"),
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

pub(crate) fn apply_in(list: &[Migration], root: &Path) -> Result<String, String> {
    let pending = pending_or_refuse(list, root)?;
    let mut out = String::new();
    for m in &pending {
        match &m.kind {
            Kind::Apply(run) => {
                run(root).map_err(|e| format!("migration `{}` failed: {e}", m.id))?;
                out.push_str(&format!("applied {}\n", m.id));
            }
            Kind::Note(n) => {
                let Some(new) = n.new else { continue };
                let path = root.join(n.file);
                if fs::read_to_string(&path).is_ok_and(|text| n.old.contains(&text.as_str())) {
                    fs::write(&path, new).map_err(|e| format!("can't write {}: {e}", n.file))?;
                    out.push_str(&format!("applied {}\n", m.id));
                }
            }
        }
    }
    if let Some(latest) = list.last() {
        set_key(root, "upgraded", &quote(latest.id))?;
    }
    for m in pending {
        if matches!(m.kind, Kind::Note(_)) && !(m.done)(root) {
            out.push_str(&format!("note {}\n", m.id));
        }
    }
    Ok(out)
}

pub(crate) fn note_in(list: &[Migration], root: &Path, id: &str) -> Result<String, String> {
    pending_or_refuse(list, root)?;
    match list.iter().find(|m| m.id == id).map(|m| &m.kind) {
        None => Err(format!("`{id}` is not a migration")),
        Some(Kind::Apply(_)) => Err(format!("`{id}` is an apply migration and has no note")),
        Some(Kind::Note(n)) => Ok(format!("{}\n", n.text.trim_end())),
    }
}

pub(crate) fn decline_in(list: &[Migration], root: &Path, id: &str) -> Result<String, String> {
    let pending = pending_or_refuse(list, root)?;
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
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(());
    };
    let new = set_top_level(&text, key, value);
    if new != text {
        fs::write(&path, new).map_err(|e| format!("can't write clm.toml: {e}"))?;
    }
    Ok(())
}

fn is_header(line: &str) -> bool {
    line.trim_start().starts_with('[')
}

fn bracket_depth(line: &str) -> i32 {
    line.chars().fold(0, |depth, c| match c {
        '[' => depth + 1,
        ']' => depth - 1,
        _ => depth,
    })
}

/// `text` with the top-level `key = value` line replaced (with the lines of a multi-line array it
/// began), or else added before the first table header, or at the end when there is none. Every
/// other line is left as it is.
pub(crate) fn set_top_level(text: &str, key: &str, value: &str) -> String {
    let new_line = format!("{key} = {value}\n");
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let first_header = lines.iter().position(|l| is_header(l));
    let top = &lines[..first_header.unwrap_or(lines.len())];
    let existing = top.iter().position(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with(key) && trimmed[key.len()..].trim_start().starts_with('=')
    });
    if let Some(i) = existing {
        let mut end = i + 1;
        let mut depth = bracket_depth(lines[i]);
        while depth > 0 && end < lines.len() {
            depth += bracket_depth(lines[end]);
            end += 1;
        }
        return [&lines[..i], &[new_line.as_str()], &lines[end..]]
            .concat()
            .concat();
    }
    match first_header {
        Some(header) => {
            let mut at = header;
            while at > 0 && lines[at - 1].trim().is_empty() {
                at -= 1;
            }
            let blank = if at == header { "\n" } else { "" };
            [&lines[..at], &[new_line.as_str(), blank], &lines[at..]]
                .concat()
                .concat()
        }
        None if text.is_empty() || text.ends_with('\n') => format!("{text}{new_line}"),
        None => format!("{text}\n{new_line}"),
    }
}
