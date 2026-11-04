//! The renames a change records in `change.md`'s `renames` (`rename-records`).

use std::collections::BTreeSet;
use std::path::Path;

use serde_yaml::Value;

use crate::{Violation, v};

const RULE: &str = "model/rename-records";

/// What a rename names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameKind {
    /// A node, by ID.
    Component,
    /// A requirement, by `<component>/<slug>`.
    Requirement,
    /// A word or phrase.
    Term,
}

const KINDS: [(&str, RenameKind); 3] = [
    ("component", RenameKind::Component),
    ("requirement", RenameKind::Requirement),
    ("term", RenameKind::Term),
];

/// One recorded rename: `name` becomes `to`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rename {
    pub kind: RenameKind,
    pub name: String,
    pub to: String,
}

/// The entries of `renames`, in file order. A live change's entries are checked, and each
/// problem is pushed against `path` with the entry's 1-based position; an archived
/// change's are loaded as written, skipping any that don't have a rename's shape.
pub(crate) fn load_renames(
    value: Option<&Value>,
    path: &Path,
    archived: bool,
    violations: &mut Vec<Violation>,
) -> Vec<Rename> {
    let entries = match value {
        None | Some(Value::Null) => return Vec::new(),
        Some(Value::Sequence(s)) => s.as_slice(),
        Some(_) if archived => return Vec::new(),
        Some(_) => {
            violations.push(v(path, RULE, "`renames` isn't a list".into()));
            return Vec::new();
        }
    };
    let mut out = Vec::new();
    let mut problems: Vec<(usize, String)> = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        match shape(entry) {
            Ok(r) if archived => out.push((i, r)),
            Ok(r) => match check(&r) {
                Ok(()) => out.push((i, r)),
                Err(m) => problems.push((i + 1, m)),
            },
            Err(_) if archived => {}
            Err(m) => problems.push((i + 1, m)),
        }
    }
    if !archived {
        let mut old = BTreeSet::new();
        let mut new = BTreeSet::new();
        for (i, r) in &out {
            let kind = r.kind as u8;
            let target = match r.kind {
                RenameKind::Requirement => {
                    let node = r.name.split_once('/').map_or("", |(n, _)| n);
                    format!("{node}/{}", r.to)
                }
                _ => r.to.clone(),
            };
            if !old.insert((kind, r.name.clone())) {
                problems.push((i + 1, format!("`{}` is renamed twice", r.name)));
            } else if !new.insert((kind, target)) {
                problems.push((i + 1, format!("two renames give the new name `{}`", r.to)));
            }
        }
    }
    violations.extend(
        problems
            .iter()
            .map(|(i, m)| v(path, RULE, format!("entry {i}: {m}"))),
    );
    out.into_iter().map(|(_, r)| r).collect()
}

/// An entry's three parts, or what is wrong with its shape.
fn shape(entry: &Value) -> Result<Rename, String> {
    let Value::Mapping(map) = entry else {
        return Err("isn't a mapping".into());
    };
    let mut kinds = Vec::new();
    let mut to = None;
    for (k, val) in map {
        let key = k.as_str().unwrap_or("");
        let named = key == "to" || KINDS.iter().any(|(n, _)| *n == key);
        if !named {
            return Err(format!("unknown key `{key}`"));
        }
        let text = match val {
            Value::String(s) if !s.is_empty() && !s.contains(['\n', '\r']) => s.clone(),
            _ => return Err(format!("`{key}` isn't a non-empty single-line string")),
        };
        match KINDS.iter().find(|(n, _)| *n == key) {
            Some((_, kind)) => kinds.push((*kind, text)),
            None => to = Some(text),
        }
    }
    let [(kind, name)] = kinds.as_slice() else {
        return Err("needs exactly one of `component`, `requirement` or `term`".into());
    };
    let Some(to) = to else {
        return Err("has no `to`".into());
    };
    Ok(Rename {
        kind: *kind,
        name: name.clone(),
        to,
    })
}

fn check(r: &Rename) -> Result<(), String> {
    match r.kind {
        RenameKind::Component => {
            if !node_id(&r.name) || !node_id(&r.to) {
                return Err("a component and its `to` must be component IDs".into());
            }
            let parent = |s: &str| s.rsplit_once('.').map(|(p, _)| p.to_string());
            if parent(&r.name) != parent(&r.to) {
                return Err("a component keeps its parent: only the last segment changes".into());
            }
        }
        RenameKind::Requirement => {
            let ok = r
                .name
                .split_once('/')
                .is_some_and(|(n, s)| node_id(n) && slug(s));
            if !ok {
                return Err("a requirement must be `<component>/<slug>`".into());
            }
            if !slug(&r.to) {
                return Err("a requirement's `to` must be a slug".into());
            }
        }
        RenameKind::Term => {}
    }
    let same = match r.kind {
        RenameKind::Requirement => r.name.rsplit('/').next() == Some(r.to.as_str()),
        _ => r.name == r.to,
    };
    if same {
        return Err("`to` equals what it renames".into());
    }
    Ok(())
}

/// Lowercase letters, digits and `-`, in non-empty dot-separated segments.
fn node_id(s: &str) -> bool {
    s.split('.').all(|seg| {
        !seg.is_empty()
            && seg
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    })
}

/// `[a-z0-9]+(-[a-z0-9]+)*`.
fn slug(s: &str) -> bool {
    s.split('-').all(|p| {
        !p.is_empty()
            && p.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    })
}
