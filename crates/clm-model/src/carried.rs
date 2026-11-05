//! CLM's carried constraints: the entries of CLM's own `ARCHITECTURE.md` that hold a
//! `#### Carries` block, compiled into the crate, and what a repository's own
//! `ARCHITECTURE.md` does with them (`carried-constraints`).

use std::ops::Range;
use std::path::Path;

use crate::layers::Layer;
use crate::{Files, Violation, files, outline, v};

const RULE: &str = "model/human-layers";

/// CLM's own `ARCHITECTURE.md` as it was when this crate was built.
const COMPILED: &str = include_str!("../../../ARCHITECTURE.md");

/// How a carried entry binds a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// A project can't replace it.
    Method,
    /// A project may replace it with an entry that keeps its guarantee.
    Replaceable,
    /// An opinion a project may drop.
    Default,
}

impl Tier {
    fn parse(line: &str) -> Option<Tier> {
        match line.strip_prefix("- ")?.trim() {
            "method" => Some(Tier::Method),
            "replaceable" => Some(Tier::Replaceable),
            "default" => Some(Tier::Default),
            _ => None,
        }
    }
}

/// An architecture entry holding a `#### Carries` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarriedEntry {
    pub slug: String,
    pub tier: Tier,
    /// The entry's text from after its heading to its first `####` heading, trimmed.
    pub statement: String,
    /// A `replaceable` entry's `#### Guarantee` block, trimmed.
    pub guarantee: Option<String>,
}

/// The carried constraints a graph holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Carried {
    /// Whether the repository's own `ARCHITECTURE.md` holds a `#### Carries` block: CLM's
    /// own repository, the source.
    pub source: bool,
    /// The source's own carried entries, or the compiled copy's.
    pub entries: Vec<CarriedEntry>,
    /// Outside the source, `entries` less those a `#### Replaces` block names; in the
    /// source, all of them.
    pub in_force: Vec<CarriedEntry>,
}

/// The compiled copy's carried entries, without loading any repository.
pub fn carried() -> Vec<CarriedEntry> {
    scan(COMPILED, &mut Vec::new()).entries
}

/// One architecture entry as the scan sees it.
struct Raw {
    slug: String,
    has_carries: bool,
    replaces: Vec<Vec<String>>,
}

struct Scan {
    raws: Vec<Raw>,
    entries: Vec<CarriedEntry>,
}

type Blocks = Vec<(usize, Range<usize>)>;

fn nonblank(lines: &[&str], body: &Range<usize>) -> Vec<String> {
    let kept = lines[body.clone()].iter().map(|l| l.trim());
    kept.filter(|l| !l.is_empty()).map(String::from).collect()
}

fn inside(blocks: &Blocks, span: &Range<usize>) -> Blocks {
    let mine = blocks.iter().filter(|(at, _)| span.contains(at));
    mine.cloned().collect()
}

/// Reads each architecture entry's three blocks, and reports the shape problems of
/// `Carries` and `Guarantee` alone.
fn scan(text: &str, violations: &mut Vec<Violation>) -> Scan {
    let path = Path::new(Layer::Architecture.file());
    let text = text.replace("\r\n", "\n");
    let lines: Vec<&str> = text.lines().collect();
    let form = Layer::Architecture.form();
    let within = outline::section_spans(&lines)
        .into_iter()
        .find(|s| s.name == form.section)
        .map(|s| s.heading + 1..s.end);
    let mask = outline::fence_mask(&lines);
    let carries = outline::blocks_named(&lines, "#### Carries");
    let guarantees = outline::blocks_named(&lines, "#### Guarantee");
    let replaces = outline::blocks_named(&lines, "#### Replaces");
    let spans = within
        .map(|w| outline::entry_spans(&lines, w, form.heading))
        .unwrap_or_default();
    let mut raws = Vec::new();
    let mut entries = Vec::new();
    for span in spans {
        let slug = span.slug.clone();
        let (mine_c, mine_g, mine_r) = (
            inside(&carries, &span.lines),
            inside(&guarantees, &span.lines),
            inside(&replaces, &span.lines),
        );
        let mut say = |problem: String| {
            violations.push(v(path, RULE, format!("entry `{slug}` {problem}")));
        };
        if mine_c.len() > 1 {
            say("has more than one `#### Carries` block".into());
        }
        let bodies: Vec<Vec<String>> = mine_c.iter().map(|(_, b)| nonblank(&lines, b)).collect();
        let tier = match bodies.first() {
            Some(body) if body.len() == 1 => {
                let tier = Tier::parse(&body[0]);
                if tier.is_none() {
                    say(format!(
                        "has a `#### Carries` line `{}` that isn't `- method`, `- replaceable` or `- default`",
                        body[0]
                    ));
                }
                tier
            }
            Some(_) => {
                say("has a `#### Carries` block that isn't exactly one line naming a tier".into());
                None
            }
            None => None,
        };
        let guarantee = mine_g
            .first()
            .map(|(_, b)| lines[b.clone()].join("\n").trim().to_string());
        let guarantees = mine_g.len();
        match tier {
            Some(Tier::Replaceable) if guarantees != 1 => say(format!(
                "is `replaceable` and has {guarantees} `#### Guarantee` blocks, not one"
            )),
            Some(Tier::Replaceable) => {}
            Some(_) if guarantees > 0 => {
                say("has a `#### Guarantee` block but isn't `replaceable`".into())
            }
            None if bodies.is_empty() && guarantees > 0 => {
                say("has a `#### Guarantee` block but isn't `replaceable`".into())
            }
            _ => {}
        }
        let first_h4 = (span.lines.start + 1..span.lines.end)
            .find(|&i| !mask[i] && lines[i].starts_with("####"))
            .unwrap_or(span.lines.end);
        let statement = lines[span.lines.start + 1..first_h4]
            .join("\n")
            .trim()
            .to_string();
        entries.extend(tier.map(|tier| CarriedEntry {
            slug: slug.clone(),
            tier,
            statement,
            guarantee: if tier == Tier::Replaceable {
                guarantee
            } else {
                None
            },
        }));
        raws.push(Raw {
            slug,
            has_carries: !mine_c.is_empty(),
            replaces: mine_r.iter().map(|(_, b)| nonblank(&lines, b)).collect(),
        });
    }
    Scan { raws, entries }
}

/// The graph's carried constraints, with the violations of `ARCHITECTURE.md`'s three
/// blocks and of `CAPABILITIES.md` holding any (`carried-constraints`).
pub(crate) fn load(files: &Files, violations: &mut Vec<Violation>) -> Carried {
    check_capabilities(files, violations);
    let compiled = carried();
    let Some(Ok(text)) = files::text(files, Path::new(Layer::Architecture.file())) else {
        return outside(compiled, &[]);
    };
    let own = scan(&text, violations);
    let path = Path::new(Layer::Architecture.file());
    if own.raws.iter().any(|r| r.has_carries) {
        for r in own.raws.iter().filter(|r| !r.replaces.is_empty()) {
            violations.push(v(
                path,
                RULE,
                format!(
                    "entry `{}` has a `#### Replaces` block in CLM's own repository",
                    r.slug
                ),
            ));
        }
        return Carried {
            source: true,
            in_force: own.entries.clone(),
            entries: own.entries,
        };
    }
    let mut named = Vec::new();
    for r in &own.raws {
        let mut say = |problem: String| {
            violations.push(v(path, RULE, format!("entry `{}` {problem}", r.slug)));
        };
        if compiled.iter().any(|c| c.slug == r.slug) {
            say("has the slug of a carried entry".into());
        }
        if r.replaces.len() > 1 {
            say("has more than one `#### Replaces` block".into());
        }
        if r.replaces.first().is_some_and(Vec::is_empty) {
            say("has an empty `#### Replaces` block".into());
        }
        for line in r.replaces.iter().flatten() {
            let slug = line
                .strip_prefix("- ")
                .map(str::trim)
                .filter(|s| !s.is_empty() && !s.contains(|c: char| c.is_whitespace() || c == '`'));
            let Some(slug) = slug else {
                say(format!(
                    "has a `#### Replaces` line that isn't `- <slug>`: `{line}`"
                ));
                continue;
            };
            match compiled.iter().find(|c| c.slug == slug) {
                None => say(format!("replaces `{slug}`, which names no carried entry")),
                Some(c) if c.tier == Tier::Method => say(format!(
                    "replaces `{slug}`, a `method` entry, which a project can't replace"
                )),
                Some(_) => named.push(slug.to_string()),
            }
        }
    }
    outside(compiled, &named)
}

fn outside(compiled: Vec<CarriedEntry>, named: &[String]) -> Carried {
    let in_force = compiled
        .iter()
        .filter(|c| !named.contains(&c.slug))
        .cloned()
        .collect();
    Carried {
        source: false,
        entries: compiled,
        in_force,
    }
}

/// A capability may hold none of the three blocks.
fn check_capabilities(files: &Files, violations: &mut Vec<Violation>) {
    let file = Layer::Capability.file();
    let Some(Ok(text)) = files::text(files, Path::new(file)) else {
        return;
    };
    let text = text.replace("\r\n", "\n");
    let lines: Vec<&str> = text.lines().collect();
    let form = Layer::Capability.form();
    let Some(within) = outline::section_spans(&lines)
        .into_iter()
        .find(|s| s.name == form.section)
        .map(|s| s.heading + 1..s.end)
    else {
        return;
    };
    for heading in ["#### Carries", "#### Guarantee", "#### Replaces"] {
        let blocks = outline::blocks_named(&lines, heading);
        for span in outline::entry_spans(&lines, within.clone(), form.heading) {
            if blocks.iter().any(|(at, _)| span.lines.contains(at)) {
                violations.push(v(
                    Path::new(file),
                    RULE,
                    format!(
                        "capability `{}` holds a `{heading}` block, which only an architecture entry may",
                        span.slug
                    ),
                ));
            }
        }
    }
}
