//! View: `clm status` and `clm view` (see specs/view/SPEC.md), the `spec`, `flight` and
//! `blockers` views, and the `queue` model of a CLM repository, as models (`views-rendered-by-clm`) and as charts.
//!
//! The models ([`spec_view`], [`flight_view`], [`blockers_view`], [`QueueView`]) and the renderers
//! ([`render`]) need neither git nor a checkout, so the crate builds for
//! `wasm32-unknown-unknown` (`view-library`). The commands that read a repository
//! are `clm-view-repo`'s.

mod blockers;
mod chart;
mod flight;
mod queue;
mod spec;
mod status;
mod svg;

pub use blockers::{BlockersView, Held, Root, blockers_view};
pub use clm_orchestrator::ClaimRef;
pub use flight::{Claim, FlightChange, FlightGroup, FlightNode, FlightView, Totals, flight_view};
pub use queue::{Loop, Next, QueueView, Running, Stop, WaitingOnWork, WaitingOnYou};
use spec::States;
pub use spec::{SpecNode, SpecView, queue_states, spec_view};
pub use status::status_text;

/// One of the three views.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    /// The `spec` model, with the queue states its chart and SVG show beside it (from
    /// [`queue_states`]); its JSON is the model alone.
    Spec(SpecView, States),
    Flight(FlightView),
    Blockers(BlockersView),
    /// The build loop's plan; it has no `svg` form, which `clm view` refuses before it
    /// renders, and its JSON carries the checkout like the others.
    Queue(QueueView),
}

/// A view's name, as `clm view <name>` takes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Name {
    Spec,
    Flight,
    Blockers,
}

/// The forms `clm view --format` prints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// A chart in box-drawing characters, no escape sequences.
    Text,
    /// The same chart with ANSI colour.
    Ansi,
    /// A standalone SVG document.
    Svg,
    /// The view's model, serialised.
    Json,
}

/// The default `--width`.
pub const DEFAULT_WIDTH: usize = 100;

/// The commit a command describes (`views-name-their-commit`): `commit`, the first 7 hex
/// digits of `HEAD`'s hash, and `behind`, the number of `origin/main`'s commits `HEAD`
/// doesn't contain, absent when there is no `origin/main`. The command reads it and hands
/// it to the renderers beside the model; the models carry none.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Checkout {
    pub commit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behind: Option<u64>,
}

impl Checkout {
    /// `commit <hash>`, then `  ⚠ <n> behind origin/main` in amber when `n` isn't zero.
    pub(crate) fn line(&self) -> chart::Line {
        let mut line = vec![chart::plain(format!("commit {}", self.commit))];
        if let Some(n) = self.behind.filter(|n| *n > 0) {
            line.push(chart::plain("  "));
            line.push(chart::span(
                format!("⚠ {n} behind origin/main"),
                chart::Style::Amber,
            ));
        }
        line
    }
}

/// The commit `clm view <name> --at <rev>` describes (`view-at-rev`): `rev` as given,
/// `commit`, the first 7 hex digits of its hash, `fetched`, when a `FETCH_HEAD` of the clone
/// was last written in Unix seconds, and `branch`, the checkout's branch, each absent when
/// unknown or detached. `now`, the time the age is measured to, isn't in the JSON.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct At {
    pub rev: String,
    pub commit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetched: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip)]
    pub now: u64,
}

impl At {
    /// `<rev> <hash>`, then ` · fetched <age> ago` when known, then ` · checkout <branch>`
    /// or ` · checkout detached`; never a `⚠`, which is the working tree's.
    pub(crate) fn line(&self) -> chart::Line {
        let mut text = format!("{} {}", self.rev, self.commit);
        if let Some(fetched) = self.fetched {
            let secs = self.now.saturating_sub(fetched);
            let age = if secs < 3600 {
                format!("{}m", secs / 60)
            } else {
                format!("{}h", secs / 3600)
            };
            text.push_str(&format!(" · fetched {age} ago"));
        }
        let branch = self.branch.as_deref().unwrap_or("detached");
        text.push_str(&format!(" · checkout {branch}"));
        vec![chart::plain(text)]
    }
}

/// What a chart begins with: the checkout a command read, or, with `--at`, the commit it
/// describes in its place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Head {
    Checkout(Checkout),
    At(At),
}

impl Head {
    fn line(&self) -> chart::Line {
        match self {
            Head::Checkout(c) => c.line(),
            Head::At(a) => a.line(),
        }
    }
}

/// A model with the command's head beside it, at the top level of its JSON: `checkout`, or
/// `at` in its place.
#[derive(serde::Serialize)]
struct WithHead<'a, T> {
    #[serde(flatten)]
    model: &'a T,
    #[serde(skip_serializing_if = "Option::is_none")]
    checkout: Option<&'a Checkout>,
    #[serde(skip_serializing_if = "Option::is_none")]
    at: Option<&'a At>,
}

fn json<T: serde::Serialize>(model: &T, head: Option<&Head>) -> serde_json::Result<String> {
    let (checkout, at) = match head {
        Some(Head::Checkout(c)) => (Some(c), None),
        Some(Head::At(a)) => (None, Some(a)),
        None => (None, None),
    };
    serde_json::to_string_pretty(&WithHead {
        model,
        checkout,
        at,
    })
}

/// `view` in `format`; charts are cut to `width` columns. With `head`, the charts begin
/// with its line, the SVG holds the same words and the JSON carries it as `checkout`, or,
/// for the commit `--at` names, as `at`.
pub fn render(view: &View, format: Format, width: usize, head: Option<&Head>) -> String {
    let line = head.map(Head::line);
    let lines = || {
        let mut lines: Vec<chart::Line> = line.iter().cloned().collect();
        lines.extend(match view {
            View::Spec(v, states) => spec::chart(v, states),
            View::Flight(v) => flight::chart(v),
            View::Blockers(v) => blockers::chart(v),
            View::Queue(v) => queue::chart(v),
        });
        lines
    };
    match format {
        Format::Text => chart::to_text(&lines(), width),
        Format::Ansi => chart::to_ansi(&lines(), width),
        Format::Svg => match view {
            View::Spec(v, states) => svg::spec(v, states, line.as_ref()),
            _ => svg::text(&lines()),
        },
        Format::Json => {
            let json = match view {
                View::Spec(v, _) => json(v, head),
                View::Flight(v) => json(v, head),
                View::Blockers(v) => json(v, head),
                View::Queue(v) => json(v, head),
            };
            // The models are plain data: a failure here is a bug, and a silent empty line
            // would break the web app and the panel that parse it.
            let mut text = json.expect("a view model serialises");
            text.push('\n');
            text
        }
    }
}
