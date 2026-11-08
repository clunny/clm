//! What a land says of its check's time (`land/check-time-report`): the costliest steps with
//! the chain that selected each, and how recent lands fared. None of it changes the land.

use clm_ledger::{TimedLand, TimedStep};

/// How many steps the report names, and how many earlier timed lands it counts.
const STEPS: usize = 3;
pub(crate) const LANDS: usize = 19;

/// Why a step was selected, as `clm affected --why` printed it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Chain {
    /// The step named no selecting file.
    NoFile,
    Lines(Vec<String>),
    /// The command failed, for this reason.
    Failed(String),
}

/// A costly step with its chain.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Slow {
    name: String,
    seconds: u64,
    chain: Chain,
}

/// The up to three steps with the most seconds, most first and ties in the check's order, each
/// with the chain `chain` gives for its first `selected_by` path.
pub(crate) fn slowest(
    steps: &[TimedStep],
    mut chain: impl FnMut(&str) -> Result<Vec<String>, String>,
) -> Vec<Slow> {
    let mut order: Vec<&TimedStep> = steps.iter().collect();
    order.sort_by_key(|s| std::cmp::Reverse(s.seconds));
    order
        .into_iter()
        .take(STEPS)
        .map(|s| Slow {
            name: s.name.clone(),
            seconds: s.seconds,
            chain: match s.selected_by.first() {
                None => Chain::NoFile,
                Some(path) => match chain(path) {
                    Ok(lines) => Chain::Lines(lines),
                    Err(why) => Chain::Failed(why),
                },
            },
        })
        .collect()
}

/// The report's lines for a land whose check took `seconds` against `target`; `slow` is given
/// when it went over, with this land's `steps` and the `past` timed lands before it.
pub(crate) fn lines(
    seconds: u64,
    target: u64,
    slow: Option<(&[Slow], &[TimedStep])>,
    past: &[TimedLand],
) -> Vec<String> {
    let mut out = vec![format!("check {seconds}s, target {target}s")];
    let Some((slow, steps)) = slow else {
        return out;
    };
    for s in slow {
        match &s.chain {
            Chain::NoFile => out.push(format!("  {} {}s, selected by no file", s.name, s.seconds)),
            Chain::Lines(lines) => {
                out.push(format!("  {} {}s", s.name, s.seconds));
                out.extend(lines.iter().map(|l| format!("    {l}")));
            }
            Chain::Failed(why) => {
                out.push(format!("  {} {}s", s.name, s.seconds));
                out.push(format!("    (no chain: {why})"));
            }
        }
    }
    let over = past.iter().filter(|l| l.check_seconds > target).count() + 1;
    let mut totals: Vec<(&str, u64)> = Vec::new();
    for step in steps.iter().chain(past.iter().flat_map(|l| &l.steps)) {
        match totals.iter_mut().find(|(name, _)| *name == step.name) {
            Some((_, total)) => *total += step.seconds,
            None => totals.push((&step.name, step.seconds)),
        }
    }
    totals.sort_by_key(|(_, total)| std::cmp::Reverse(*total));
    let most: Vec<String> = totals
        .iter()
        .take(STEPS)
        .map(|(name, total)| format!("{name} {total}s"))
        .collect();
    out.push(format!(
        "last {} timed lands: {over} over target; most time in {}",
        past.len() + 1,
        most.join(", ")
    ));
    out.push("see clm guide ci-and-deployment, where slow tests go".into());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(name: &str, seconds: u64, by: &[&str]) -> TimedStep {
        TimedStep {
            name: name.into(),
            seconds,
            selected_by: by.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn the_three_costliest_steps_with_ties_in_check_order() {
        // req:land/check-time-report@ddbc0e19
        let steps = [
            step("a", 5, &["p", "q"]),
            step("b", 9, &[]),
            step("c", 5, &["r"]),
            step("d", 5, &["s"]),
            step("e", 1, &["t"]),
        ];
        let got = slowest(&steps, |p| match p {
            "r" => Err("boom".into()),
            p => Ok(vec![p.to_string()]),
        });
        let names: Vec<_> = got.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["b", "a", "c"]);
        assert_eq!(got[0].chain, Chain::NoFile);
        assert_eq!(got[1].chain, Chain::Lines(vec!["p".into()]));
        assert_eq!(got[2].chain, Chain::Failed("boom".into()));
    }

    #[test]
    fn under_target_says_one_line_and_over_says_the_rest() {
        // req:land/check-time-report@ddbc0e19
        assert_eq!(lines(3, 3, None, &[]), ["check 3s, target 3s"]);
        let steps = [step("x", 2, &[]), step("y", 1, &[])];
        let slow = slowest(&steps, |_| Ok(vec![]));
        let past = [
            TimedLand {
                commit: "c".into(),
                at: String::new(),
                check_seconds: 1,
                steps: vec![step("y", 4, &[])],
            },
            TimedLand {
                commit: "d".into(),
                at: String::new(),
                check_seconds: 9,
                steps: vec![step("y", 4, &[])],
            },
        ];
        let got = lines(3, 2, Some((&slow, &steps)), &past);
        assert_eq!(got[0], "check 3s, target 2s");
        assert_eq!(got[1], "  x 2s, selected by no file");
        assert_eq!(
            got[3],
            "last 3 timed lands: 2 over target; most time in y 9s, x 2s"
        );
        assert_eq!(got.len(), 5);
        assert!(got[4].contains("clm guide ci-and-deployment"));
    }
}
