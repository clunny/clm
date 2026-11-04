//! The human layers: the entries of `ARCHITECTURE.md` and `CAPABILITIES.md` at the
//! repository root, loaded as requirements with IDs `architecture/<slug>` and
//! `capability/<slug>` (`human-layers`).

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::path::Path;

use crate::{Files, Graph, IdBlock, Requirement, Violation, content_hash, files, outline, v};

/// One of the two human layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Layer {
    Architecture,
    Capability,
}

impl Layer {
    pub const ALL: [Layer; 2] = [Layer::Architecture, Layer::Capability];

    /// The prefix of its entries' IDs, and the top-level node ID that is either reserved
    /// (`architecture`) or the capability layer's node (`capability`, `reserved-ids`).
    pub fn id(self) -> &'static str {
        match self {
            Layer::Architecture => "architecture",
            Layer::Capability => "capability",
        }
    }

    /// The root file that holds it.
    pub fn file(self) -> &'static str {
        match self {
            Layer::Architecture => "ARCHITECTURE.md",
            Layer::Capability => "CAPABILITIES.md",
        }
    }

    /// The layer an ID `<layer>/<slug>` names, by its prefix.
    pub fn of(id: &str) -> Option<Layer> {
        let (prefix, _) = id.split_once('/')?;
        Self::ALL.into_iter().find(|l| l.id() == prefix)
    }

    /// Whether node ID `id` is one the human layers reserve: the architecture entries'
    /// prefix, which no node may take (`reserved-ids`).
    pub(crate) fn reserves(id: &str) -> bool {
        id == Layer::Architecture.id()
    }

    /// The section its file holds its entries in, and the heading that starts an entry
    /// (`human-layers`).
    fn form(self) -> Form {
        match self {
            Layer::Capability => Form {
                section: "Capabilities",
                heading: "### Capability:",
            },
            Layer::Architecture => Form {
                section: "Constraints",
                heading: "### Constraint:",
            },
        }
    }

    /// The heading its file's entries are written under.
    pub(crate) fn heading(self) -> &'static str {
        self.form().heading
    }
}

/// The section and entry heading a layer's file is read in (`human-layers`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Form {
    /// The `##` section that holds the entries.
    section: &'static str,
    /// The heading that starts an entry, before its slug.
    heading: &'static str,
}

const RULE: &str = "model/human-layers";

/// An architecture entry's `#### Enforced by` block: the rules that enforce it.
const ENFORCED_BY: IdBlock = IdBlock {
    name: "Enforced by",
    rule: RULE,
    shape: "<component>/<slug>",
    nonempty: true,
};

/// Reports an architecture entry's `#### Enforced by` ID that names a human-layer entry,
/// or no requirement the graph holds or a live change adds (`human-layers`). Part of the graph check, since it
/// joins entries to nodes' rules (`graph-check`).
pub(crate) fn check_enforced_by(graph: &Graph, violations: &mut Vec<Violation>) {
    let path = Path::new(Layer::Architecture.file());
    let entries = graph.layers.get(&Layer::Architecture).into_iter().flatten();
    for entry in entries {
        let slug = entry.id.split_once('/').map_or("", |(_, s)| s);
        for id in &entry.enforced_by {
            let problem = if Layer::of(id).is_some() {
                "a human-layer entry, not a rule"
            } else if !crate::records::blockable(graph, id) {
                "no requirement that exists or that a live change adds"
            } else {
                continue;
            };
            violations.push(v(
                path,
                RULE,
                format!("entry `{slug}` is enforced by `{id}`, which names {problem}"),
            ));
        }
    }
}

/// Both layers' entries, in file order, from `files`' root `ARCHITECTURE.md` and
/// `CAPABILITIES.md`. An absent file has none.
pub(crate) fn load_layers(
    files: &Files,
    violations: &mut Vec<Violation>,
) -> BTreeMap<Layer, Vec<Requirement>> {
    let mut entries = BTreeMap::new();
    for layer in Layer::ALL {
        let path = Path::new(layer.file());
        let found = match files::text(files, path) {
            None => Vec::new(),
            Some(Err(e)) => {
                violations.push(v(path, RULE, format!("unreadable: {e}")));
                Vec::new()
            }
            Some(Ok(text)) => parse(layer, &text, violations),
        };
        entries.insert(layer, found);
    }
    entries
}

/// A layer's entries.
fn parse(layer: Layer, text: &str, violations: &mut Vec<Violation>) -> Vec<Requirement> {
    let path = Path::new(layer.file());
    let text = text.replace("\r\n", "\n");
    let lines: Vec<&str> = text.lines().collect();
    let mask = outline::fence_mask(&lines);
    let form = layer.form();
    let within = outline::section_spans(&lines)
        .into_iter()
        .find(|s| s.name == form.section)
        .map(|s| s.heading + 1..s.end);
    for (i, line) in lines.iter().enumerate() {
        if mask[i] {
            continue;
        }
        if within.as_ref().is_some_and(|w| w.contains(&i)) {
            continue;
        }
        if let Some((slug, _)) = outline::entry_heading(line, form.heading) {
            violations.push(v(
                path,
                RULE,
                format!(
                    "entry `{slug}` is outside the `## {}` section",
                    form.section
                ),
            ));
        }
    }
    let Some(within) = within else {
        return Vec::new();
    };
    entries(layer, form, &lines, &mask, within, violations)
}

fn entries(
    layer: Layer,
    form: Form,
    lines: &[&str],
    mask: &[bool],
    within: Range<usize>,
    violations: &mut Vec<Violation>,
) -> Vec<Requirement> {
    let path = Path::new(layer.file());
    let blocks: Vec<(usize, &str)> = ["#### Needs", "#### Serves"]
        .into_iter()
        .flat_map(|h| {
            outline::blocks_named(lines, h)
                .into_iter()
                .map(move |(at, _)| (at, h))
        })
        .collect();
    let enforced_blocks = outline::blocks_named(lines, "#### Enforced by");
    let untested_blocks = outline::blocks_named(lines, "#### Untested");
    let mut seen = BTreeSet::new();
    outline::entry_spans(lines, within, form.heading)
        .into_iter()
        .filter_map(|span| {
            let slug = &span.slug;
            let kebab = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
            if slug.is_empty() || !slug.chars().all(kebab) {
                let problem = format!(
                    "entry `{slug}` needs a slug of lowercase ASCII letters, digits and `-`"
                );
                violations.push(v(path, RULE, problem));
                return None;
            }
            let block = outline::join(&lines[span.lines.clone()]);
            let scenarios = outline::scenario_count(lines, span.lines.clone());
            let mut problems = Vec::new();
            if !seen.insert(slug.clone()) {
                problems.push(format!("duplicate entry `{slug}`"));
            }
            if scenarios == 0 {
                problems.push(format!("entry `{slug}` has no scenario"));
            }
            if layer == Layer::Architecture && !block.contains("SHALL") && !block.contains("MUST") {
                problems.push(format!("entry `{slug}` uses neither SHALL nor MUST"));
            }
            let whys = span
                .lines
                .clone()
                .filter(|i| !mask[*i] && lines[*i].trim_end() == "#### Why")
                .count();
            if layer == Layer::Capability && whys != 1 {
                problems.push(format!(
                    "entry `{slug}` has {whys} `#### Why` blocks, not one"
                ));
            }
            for (_, heading) in blocks.iter().filter(|(at, _)| span.lines.contains(at)) {
                problems.push(format!("entry `{slug}` holds a `{heading}` block"));
            }
            let mine = |blocks: &[(usize, Range<usize>)]| -> Vec<(usize, Range<usize>)> {
                let inside = blocks.iter().filter(|(at, _)| span.lines.contains(at));
                inside.cloned().collect()
            };
            let (enforcing, untested) = (mine(&enforced_blocks), mine(&untested_blocks));
            if untested.len() > 1 {
                problems.push(format!(
                    "entry `{slug}` has more than one `#### Untested` block"
                ));
            }
            if let Some((_, body)) = untested.first()
                && lines[body.clone()].iter().all(|l| l.trim().is_empty())
            {
                problems.push(format!("entry `{slug}` has an empty `#### Untested` block"));
            }
            let enforced_by = if layer == Layer::Capability && !enforcing.is_empty() {
                problems.push(format!(
                    "entry `{slug}` holds an `#### Enforced by` block, which only an architecture entry may"
                ));
                Vec::new()
            } else {
                let owner = format!("entry `{slug}`");
                crate::parse_ids(lines, enforcing, &ENFORCED_BY, &owner, path, violations)
            };
            violations.extend(problems.into_iter().map(|p| v(path, RULE, p)));
            Some(Requirement {
                id: format!("{}/{slug}", layer.id()),
                scenarios,
                hash: content_hash(&block),
                text: block,
                needs: Vec::new(),
                serves: Vec::new(),
                enforced_by,
                untested: !untested.is_empty(),
                untested_text: untested.first().map(|(_, body)| lines[body.clone()].join("\n").trim().to_string()),
            })
        })
        .collect()
}
