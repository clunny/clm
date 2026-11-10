---
title: "/clm:verify verifies main on demand; clm verify shows a red run's evidence and the session judges flakiness"
date: 2026-11-09
status: accepted
---

Change `20261109-verify-skill`. Extends `verify-after-land`.

**On demand.** `/clm:verify` runs `clm verify --commit origin/main`, which fetches, resolves the revision and verifies `main`'s head whatever `[verify]`'s triggers say, or answers from the ledger when a record already names it. No new CLI form: `--commit` already does it, and `verify-commit` now says it takes a revision. A red result is offered for intake at `P1` right then, since the human is there to say yes.

**CLM's part is the evidence.** `[verify] check` is the project's own script, and CLM sees only its exit status, so it can't know which test failed or re-run one. What it can give every caller, with nothing more asked of projects, is what the shared check runner already returns: the step open when the output ended (from the optional `::group::` markers, `check-step-markers`) and the last 40 lines. `clm verify` now prints those after a red run's record, as `clm land`'s refusal does, beside the kept log.

**The session's part is the judgement.** When the evidence shows which tests failed and the project can run one alone, `/clm:verify` re-runs them at the failing commit and says whether each failed again. That advises (`deterministic-enforcement`): it changes no record, and the human's yes is still what makes a failure real (`verify-after-land`). A failure whose tests all pass on the re-run is offered as a bug in those tests, which holds no land, not as a `verify_failures` item.

**Considered and rejected.**
- *`clm verify` re-runs a failed check once, behind `[verify] retries`.* Deterministic, but it re-runs the whole suite on every real break, says nothing of which test flaked, and asks a setting of every project. A flaky bisect probe, which this would also fix, is kept as `backlog/verify-bisect-blames-a-flaky-probe` until it shows up.
- *Per-test retries in the project's script only.* Still open to projects, as `ci-check.sh` does for its browser suite (`capability-web-retries`), but CLM can't count on it.
