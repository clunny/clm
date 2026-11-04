//! The lines `clm capabilities` and `clm architecture` print, and their pages
//! (`effort/capabilities-list`, `effort/capability-page`, `effort/architecture-list`,
//! `effort/architecture-page`).

use crate::{Constraint, Cost, Decided, Flag, Report, Spend};

/// `850`, `85k` or `1.3M`, rounded first as the panel rounds them.
fn tokens(n: f64) -> String {
    if n >= 999_500.0 {
        format!("{:.1}M", n / 1e6)
    } else if n >= 999.5 {
        format!("{}k", (n / 1e3).round())
    } else {
        format!("{}", n.round())
    }
}

/// `$` and the dollars: to the cent under $10, to the tenth above.
fn usd(dollars: f64) -> String {
    let cents = (dollars * 100.0).round() / 100.0;
    if cents < 10.0 {
        format!("${cents:.2}")
    } else {
        format!("${dollars:.1}")
    }
}

/// A number only history gives, or `-`.
fn or_dash<T: ToString>(n: Option<T>) -> String {
    n.map_or_else(|| "-".into(), |n| n.to_string())
}

fn day(at: &str) -> &str {
    at.get(..10).unwrap_or(at)
}

/// The `workarounds` section: a heading and a flag a line with its text indented under it.
fn flag_lines(flags: &[Flag]) -> Vec<String> {
    let unsettled = flags.iter().filter(|f| !f.settled).count();
    let mut lines = vec![format!(
        "workarounds ({} open, {unsettled} unsettled)",
        flags.len()
    )];
    for f in flags {
        let added = f
            .added
            .as_ref()
            .map_or_else(String::new, |d| format!(" (added {d})"));
        lines.push(format!(
            "  {}{} {} {}{added}",
            if f.settled { "  " } else { "⚑ " },
            f.slug,
            f.requirement,
            f.title
        ));
        let text = f.text.lines().map(str::trim_end).filter(|l| !l.is_empty());
        lines.extend(text.map(|l| format!("    {l}")));
    }
    lines
}

/// The `decisions` section.
fn decision_lines(decisions: &[Decided]) -> Vec<String> {
    let mut lines = vec!["decisions".to_string()];
    if decisions.is_empty() {
        lines.push("  never decided".into());
    }
    for d in decisions {
        lines.push(format!(
            "  {} {} (value {}) by {}",
            day(&d.at),
            d.decision.as_str(),
            d.value.as_str(),
            d.writer
        ));
    }
    lines
}

/// An entry's latest decision, for a list row.
fn latest(decisions: &[Decided]) -> String {
    decisions.last().map_or_else(
        || "never decided".to_string(),
        |d| {
            let (decision, value) = (d.decision.as_str(), d.value.as_str());
            format!("{decision} {value} {}", day(&d.at))
        },
    )
}

/// The `≈$<d> · <tokens> tokens · <h>h active · <cache> cache reads` line.
fn spend_line(s: &Spend) -> String {
    format!(
        "  ≈{} · {} tokens · {:.1}h active · {} cache reads",
        usd(s.dollars),
        tokens(s.work_tokens),
        s.active_seconds / 3600.0,
        tokens(s.cache_read_tokens)
    )
}

/// `declared burdens: <list>` or `none`.
fn burdens_line(burdens: &[String]) -> String {
    let list = if burdens.is_empty() {
        "none".to_string()
    } else {
        burdens.join(", ")
    };
    format!("  declared burdens: {list}")
}

/// `components a, b` or `no components`.
fn components_text(components: &[String]) -> String {
    if components.is_empty() {
        "no components".to_string()
    } else {
        format!("components {}", components.join(", "))
    }
}

const ESTIMATE: &str = "a component's cost is split by its rules' share";

/// `clm capabilities`.
pub fn list_text(report: &Report) -> Vec<String> {
    let caps = &report.capabilities;
    let flagged = caps.iter().filter(|c| c.flagged).count();
    let mut lines = vec![format!(
        "capabilities {} · {flagged} ⚑ · costs are estimates: {ESTIMATE}",
        caps.len()
    )];
    let width = caps
        .iter()
        .map(|c| c.slug.chars().count())
        .max()
        .unwrap_or(0);
    for c in caps {
        let unsettled = c.flags.iter().filter(|f| !f.settled).count();
        lines.push(format!(
            "{}{:<width$}  ≈{}  {} tok  {} commits  {} reviews  {unsettled} workarounds  {}",
            if c.flagged { "⚑ " } else { "  " },
            c.slug,
            usd(c.spend.dollars),
            tokens(c.spend.work_tokens),
            or_dash(c.commits),
            c.approvals,
            latest(&c.decisions),
        ));
    }
    let label = (width + 2).max("architecture only".len());
    for (name, spend) in [
        ("unattributed", &report.unattributed),
        ("architecture only", &report.architecture_only),
    ] {
        lines.push(format!(
            "{name:<label$}  ≈{}  {} tok",
            usd(spend.dollars),
            tokens(spend.work_tokens)
        ));
    }
    lines
}

/// `clm capabilities <slug>`.
pub fn page_text(cost: &Cost) -> Vec<String> {
    let c = cost;
    let mut lines = vec![
        format!("{}{}", c.slug, if c.flagged { " ⚑" } else { "" }),
        c.statement.clone(),
        format!("{} rules · {}", c.rules.len(), components_text(&c.nodes)),
        String::new(),
        "why we need it".into(),
        format!("  {}", c.why),
        format!("what it costs (an estimate: {ESTIMATE})"),
        spend_line(&c.spend),
        format!("  {} commits · {} changes", or_dash(c.commits), c.changes),
        format!(
            "  your attention: {} approvals · {} questions answered · {} flags settled",
            c.approvals,
            or_dash(c.questions_answered),
            c.flags_settled
        ),
        format!(
            "  blocking: {} items held its rules up for {} days",
            or_dash(c.blocking.map(|b| b.0)),
            or_dash(c.blocking.map(|b| b.1))
        ),
        format!(
            "  size: {} rules · {} tokens",
            c.rules.len(),
            c.size_tokens.map_or_else(|| "-".into(), tokens)
        ),
        burdens_line(&c.burdens),
    ];
    lines.extend(flag_lines(&c.flags));
    lines.push("can we live without it?".into());
    let o = &c.overlap;
    lines.push(if o.shared.is_empty() {
        "  all its rules serve only it".into()
    } else {
        let shared: Vec<String> = o
            .shared
            .iter()
            .map(|(id, k)| format!("{} ({k})", id.trim_start_matches("capability/")))
            .collect();
        format!(
            "  {} of its {} rules serve only it; the rest also serve {}",
            o.only,
            c.rules.len(),
            shared.join(", ")
        )
    });
    lines.push("rules".into());
    lines.extend(c.rules.iter().map(|r| format!("  {r}")));
    lines.extend(decision_lines(&c.decisions));
    lines.push(format!(
        "decide: clm capabilities decide {} --value high|medium|low --decision keep|cut|redesign|accept",
        c.slug
    ));
    lines
}

const ARCHITECTURE_SHARE: &str = "its enforcing rules' share, counted apart from the capabilities'";

/// How many of an entry's enforcing rules are current.
fn held(c: &Constraint) -> usize {
    c.enforced_by
        .iter()
        .filter(|e| e.current == Some(true))
        .count()
}

/// `held <c>/<n>`, with ` + untested` or alone `untested`.
fn proof(c: &Constraint) -> String {
    match (c.enforced_by.is_empty(), c.untested.is_some()) {
        (true, true) => "untested".into(),
        (_, untested) => {
            let rest = if untested { " + untested" } else { "" };
            format!("held {}/{}{rest}", held(c), c.enforced_by.len())
        }
    }
}

/// `clm architecture`.
pub fn architecture_text(constraints: &[Constraint]) -> Vec<String> {
    let all_held = |c: &&Constraint| !c.enforced_by.is_empty() && held(c) == c.enforced_by.len();
    let flagged = constraints.iter().filter(|c| c.flagged).count();
    let mut lines = vec![format!(
        "architecture {} · {} held · {flagged} ⚑ · costs are estimates: an entry's cost is {ARCHITECTURE_SHARE}",
        constraints.len(),
        constraints.iter().filter(all_held).count(),
    )];
    let width = |f: &dyn Fn(&Constraint) -> usize| constraints.iter().map(f).max().unwrap_or(0);
    let slugs = width(&|c| c.slug.chars().count());
    let proofs = width(&|c| proof(c).chars().count());
    for c in constraints {
        let unsettled = c.flags.iter().filter(|f| !f.settled).count();
        lines.push(format!(
            "{}{:<slugs$}  {:<proofs$}  ≈{}  {} tok  {} reviews  {unsettled} workarounds  {}",
            if c.flagged { "⚑ " } else { "  " },
            c.slug,
            proof(c),
            usd(c.spend.dollars),
            tokens(c.spend.work_tokens),
            c.approvals,
            latest(&c.decisions),
        ));
    }
    lines
}

/// `clm architecture <slug>`.
pub fn constraint_text(constraint: &Constraint) -> Vec<String> {
    let c = constraint;
    let n = c.enforced_by.len();
    let mut lines = vec![
        format!("{}{}", c.slug, if c.flagged { " ⚑" } else { "" }),
        c.statement.clone(),
        format!("{n} enforcing rules · {}", components_text(&c.nodes)),
        String::new(),
        "is it held?".into(),
    ];
    for e in &c.enforced_by {
        let state = match e.current {
            Some(true) => "current",
            Some(false) => "pending",
            None => "unknown",
        };
        lines.push(format!("  {state} {}", e.rule));
    }
    if let Some(text) = &c.untested {
        lines.push(format!("  untested: {text}"));
    }
    lines.push(format!("what it costs (an estimate: {ARCHITECTURE_SHARE})"));
    lines.push(spend_line(&c.spend));
    lines.push(format!("  {} changes", c.changes));
    lines.push(format!(
        "  your attention: {} approvals · {} flags settled",
        c.approvals, c.flags_settled
    ));
    lines.push(burdens_line(&c.burdens));
    lines.extend(flag_lines(&c.flags));
    lines.push("can we live without it?".into());
    let o = &c.overlap;
    lines.push(if n == 0 {
        "  no rule enforces it".into()
    } else if o.shared.is_empty() {
        "  all its enforcing rules enforce only it".into()
    } else {
        let shared: Vec<String> = o
            .shared
            .iter()
            .map(|(id, k)| format!("{} ({k})", id.trim_start_matches("architecture/")))
            .collect();
        format!(
            "  {} of its {n} enforcing rules enforce only it; the rest also enforce {}",
            o.only,
            shared.join(", ")
        )
    });
    lines.extend(decision_lines(&c.decisions));
    lines.push(format!(
        "decide: clm architecture decide {} --value high|medium|low --decision keep|cut|redesign|accept",
        c.slug
    ));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Overlap, Spend};

    fn cost(slug: &str) -> Cost {
        Cost {
            id: format!("capability/{slug}"),
            slug: slug.into(),
            version: "00000000".into(),
            statement: "S.".into(),
            why: "W.".into(),
            rules: Vec::new(),
            nodes: Vec::new(),
            spend: Spend::default(),
            changes: 0,
            approvals: 0,
            flags_settled: 0,
            burdens: Vec::new(),
            commits: None,
            questions_answered: None,
            blocking: None,
            size_tokens: None,
            flags: Vec::new(),
            overlap: Overlap::default(),
            decisions: Vec::new(),
            flagged: false,
        }
    }

    #[test]
    fn numbers_read_as_the_panel_reads_them() {
        // req:effort/capabilities-list@daa12c28
        let cases = [
            (850.0, "850"),
            (999.6, "1k"),
            (85_000.0, "85k"),
            (999_600.0, "1.0M"),
        ];
        for (n, text) in cases {
            assert_eq!(tokens(n), text);
        }
        assert_eq!(tokens(1_300_000.0), "1.3M");
        assert_eq!(
            (usd(3.0), usd(9.999), usd(12.34)),
            ("$3.00".into(), "$10.0".into(), "$12.3".into())
        );
    }

    #[test]
    fn footer_padded_to_a_wide_slug() {
        // req:effort/capabilities-list@daa12c28
        let report = Report {
            capabilities: vec![cost("a-very-long-capability-slug")],
            ..Report::default()
        };
        let lines = list_text(&report);
        let pad = " ".repeat("a-very-long-capability-slug".len() + 2 - "unattributed".len());
        assert_eq!(lines[2], format!("unattributed{pad}  ≈$0.00  0 tok"));
    }

    #[test]
    fn page_without_rules_or_burdens() {
        // req:effort/capability-page@dd387c2f
        let page = page_text(&cost("p"));
        for line in [
            "0 rules · no components",
            "  declared burdens: none",
            "workarounds (0 open, 0 unsettled)",
            "  all its rules serve only it",
            "  never decided",
        ] {
            assert!(page.contains(&line.to_string()), "{line}: {page:?}");
        }
    }
}
