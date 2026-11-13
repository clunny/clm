//! The `queue` view: the build loop's plan (`queue-view`). The model is plain data that
//! `orchestrator.loop` computes and hands in, so this crate depends on no git.

use serde::Serialize;

use crate::chart::{self, Cell, Line, heading, plain, span};

/// What the loop is doing, as the line after the checkout line says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Loop {
    /// `loop held, renewed <n>m ago`.
    Held { renewed_minutes: u64 },
    /// `loop not held`.
    NotHeld,
    /// `stop requested: drain|now`.
    Stop { mode: Stop },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stop {
    Drain,
    Now,
}

/// A run the loop has launched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Running {
    pub name: String,
    pub kind: String,
    pub model: String,
    pub round: u32,
    /// How long it has run.
    pub minutes: u64,
    /// Held by a session that was taken over.
    pub taken_over: bool,
}

/// What the loop launches next, in its order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Next {
    pub name: String,
    pub kind: String,
    /// The effective priority, such as `P2`.
    pub priority: String,
    /// Why it holds its place, such as `waits for a place`.
    pub reason: String,
}

/// An entry that waits on the human.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WaitingOnYou {
    pub name: String,
    pub kind: String,
    pub what: String,
    pub since_minutes: u64,
    /// What the human does about it, such as `say "approve x"`.
    pub action: String,
}

/// An entry that waits on other work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WaitingOnWork {
    pub name: String,
    pub kind: String,
    pub waits_on: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QueueView {
    /// Always `queue`.
    pub view: &'static str,
    #[serde(rename = "loop")]
    pub loop_state: Loop,
    pub running: Vec<Running>,
    pub next: Vec<Next>,
    pub waiting_on_you: Vec<WaitingOnYou>,
    pub waiting_on_work: Vec<WaitingOnWork>,
}

/// `12m`, or `2h` from an hour on.
fn age(minutes: u64) -> String {
    if minutes < 60 {
        format!("{minutes}m")
    } else {
        format!("{}h", minutes / 60)
    }
}

fn cell(text: impl Into<String>) -> Cell {
    vec![plain(text)]
}

fn section(title: &str, rows: Vec<Vec<Cell>>) -> Vec<Line> {
    let mut lines = vec![heading(title)];
    if rows.is_empty() {
        lines.push(vec![plain("  none")]);
    } else {
        for mut line in chart::align(rows) {
            line.insert(0, plain("  "));
            lines.push(line);
        }
    }
    lines
}

/// The chart below the checkout line: the loop line, then the four sections.
pub(crate) fn chart(v: &QueueView) -> Vec<Line> {
    let loop_line = match &v.loop_state {
        Loop::Held { renewed_minutes } => {
            vec![plain(format!("loop held, renewed {renewed_minutes}m ago"))]
        }
        Loop::NotHeld => vec![span("loop not held", chart::Style::Amber)],
        Loop::Stop { mode } => vec![span(
            format!(
                "stop requested: {}",
                match mode {
                    Stop::Drain => "drain",
                    Stop::Now => "now",
                }
            ),
            chart::Style::Amber,
        )],
    };
    let mut lines = vec![loop_line];
    lines.extend(section(
        "running",
        v.running
            .iter()
            .map(|r| {
                let mut row = vec![
                    cell(&r.name),
                    cell(&r.model),
                    cell(format!("round {}", r.round)),
                    cell(age(r.minutes)),
                ];
                if r.taken_over {
                    row.push(vec![span("taken over", chart::Style::Amber)]);
                }
                row
            })
            .collect(),
    ));
    lines.extend(section(
        "next up",
        v.next
            .iter()
            .map(|n| vec![cell(&n.name), cell(&n.priority), cell(&n.reason)])
            .collect(),
    ));
    lines.extend(section(
        "waiting on you",
        v.waiting_on_you
            .iter()
            .map(|w| {
                vec![
                    cell(&w.name),
                    cell(&w.what),
                    cell(format!("since {}", age(w.since_minutes))),
                    cell(&w.action),
                ]
            })
            .collect(),
    ));
    lines.extend(section(
        "waiting on work",
        v.waiting_on_work
            .iter()
            .map(|w| vec![cell(&w.name), cell(&w.waits_on)])
            .collect(),
    ));
    lines
}
