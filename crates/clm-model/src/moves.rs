//! The merges a change records in `change.md`'s `moves` (`move-records`).

use std::collections::BTreeSet;
use std::path::Path;

use serde_yaml::Value;

use crate::change::Delta;
use crate::{Violation, v};

const RULE: &str = "model/move-records";

/// One recorded move: the change removes `requirement`, merged into `to`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Move {
    /// The removed requirement, `<component>/<slug>`.
    pub requirement: String,
    /// The requirement it merged into, `<component>/<slug>`.
    pub to: String,
}

/// The entries of `moves`, in file order. A live change's entries are checked against its
/// `deltas`, and each problem is pushed against `path` with the entry's 1-based position;
/// an archived change's are loaded as written, leaving out any that isn't a mapping of
/// exactly `requirement` and `to` to strings.
pub(crate) fn load_moves(
    value: Option<&Value>,
    path: &Path,
    archived: bool,
    deltas: &[Delta],
    violations: &mut Vec<Violation>,
) -> Vec<Move> {
    let entries = match value {
        None | Some(Value::Null) => return Vec::new(),
        Some(Value::Sequence(s)) => s.as_slice(),
        Some(_) if archived => return Vec::new(),
        Some(_) => {
            violations.push(v(path, RULE, "`moves` isn't a list".into()));
            return Vec::new();
        }
    };
    let shaped = entries.iter().map(shape);
    if archived {
        return shaped.filter_map(Result::ok).collect();
    }
    let mut out = Vec::new();
    let mut named = BTreeSet::new();
    for (i, entry) in shaped.enumerate() {
        let checked = entry.and_then(|m| {
            let first = named.insert(m.requirement.clone());
            check(&m, first, deltas).map(|()| m)
        });
        match checked {
            Ok(m) => out.push(m),
            Err(e) => violations.push(v(path, RULE, format!("entry {}: {e}", i + 1))),
        }
    }
    out
}

/// An entry's two parts, or what is wrong with its shape.
fn shape(entry: &Value) -> Result<Move, String> {
    let Value::Mapping(map) = entry else {
        return Err("isn't a mapping".into());
    };
    let (mut requirement, mut to) = (None, None);
    for (k, val) in map {
        let key = k.as_str().unwrap_or("");
        let slot = match key {
            "requirement" => &mut requirement,
            "to" => &mut to,
            _ => return Err(format!("unknown key `{key}`")),
        };
        let Value::String(s) = val else {
            return Err(format!("`{key}` isn't a string"));
        };
        *slot = Some(s.clone());
    }
    match (requirement, to) {
        (Some(requirement), Some(to)) => Ok(Move { requirement, to }),
        (None, _) => Err("has no `requirement`".into()),
        (_, None) => Err("has no `to`".into()),
    }
}

/// Whether a live change's entry is one its deltas make; `first` is false when an earlier
/// entry names the same `requirement`.
fn check(m: &Move, first: bool, deltas: &[Delta]) -> Result<(), String> {
    let (Some(from), Some(to)) = (requirement_id(&m.requirement), requirement_id(&m.to)) else {
        return Err("`requirement` and `to` must each be `<component>/<slug>`".into());
    };
    let of = |node| deltas.iter().filter(move |d: &&Delta| d.node == node);
    if !of(from.0).any(|d| d.removed.iter().any(|r| r.slug == from.1)) {
        return Err(format!(
            "the change's deltas don't remove `{}`",
            m.requirement
        ));
    }
    let takes = |d: &Delta| d.added.iter().chain(&d.modified).any(|r| r.slug == to.1);
    if !of(to.0).any(takes) {
        return Err(format!(
            "the change's deltas don't add or modify `{}`",
            m.to
        ));
    }
    if !first {
        return Err(format!("an earlier entry moves `{}`", m.requirement));
    }
    Ok(())
}

/// `<component>/<slug>` split in two, or `None` when it isn't one.
fn requirement_id(s: &str) -> Option<(&str, &str)> {
    s.split_once('/')
        .filter(|(n, s)| crate::renames::node_id(n) && crate::renames::slug(s))
}
