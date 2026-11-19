---
status: applied
resolves:
  - the-build-loop-loses-a-red-verify-reported-by-its-last-line
---

## Intent

The build loop learns a `clm verify` run's outcome whatever order its output's lines come in. `plan-verify` says the session reports the run's outcome line, the last line of its output that starts `verify ` (or `none` when no line does), instead of its last line, and `clm build returned verify` refuses any other line, so a wrong report fails loudly instead of dropping a red result from the human's queue. The fix is outside the plan's freeze, so a question item holds its build back until the human says whether to make the exception; the build skill's sentence waits in a deferred item on `plugin`, which fixes the lost result in practice with the loop as built now.

## Problem

`plan-verify` prints `run verify clm verify --due ...`, and says the run's last line is reported with `clm build returned verify --result <line>`; the build skill says the same, and the session runs the command as background work, whose output holds stdout and stderr as one. When the verify fails, `clm verify` prints `verify failed <sha> culprit ... components ...` on stdout and then its notes on stderr, the last being `clm verify: output is at <log>`, as `verify-run` and `culprit-recheck` require ("stderr, after it"). The last line is that note, `returned` records only a line starting `verify failed`, and so the loop records no red result and puts nothing in the human's queue. A passed verify whose mutation pass prints notes after the outcome loses nothing, since any other line reads as green.

The design is wrong, not the code: `plan-verify` names the wrong line. No requirement states the behaviour the fix restores, so it is a requirement of this change (`code-fix-needs-a-written-rule`).

No change or code fix since 2026-11-11 fixes it: `20261118-the-build-skill-gives` filed the item and changed only the skill's `--cause` wording, and the pending changes on `orchestrator.loop` (`20261116-the-build-loop-shows`, `20261117-a-deleted-runs-task`, `20261117-the-build-loop-drafts`) don't touch `plan-verify`.

## Approaches

- **Taken: the session reports the outcome line, and `returned verify` refuses any other line but `none`.** `verify`'s Contract already says every outcome is one line on stdout starting `verify `, so the line is named by a fact of that Contract, not by the session's judgement. The refusal is the checked half: a session that reports a note, as every one does today, is told which line to report, and nothing is recorded until it does.
- **Not taken: `clm verify` prints its outcome line last.** `verify-run`, `verify-bisect` and `culprit-recheck` require the notes on stderr after the outcome line, so a human reading a red verify's tail sees the log's path last; reordering moves rules in a component the loop doesn't own for a reader the loop already reads through its Contract.
- **Not taken: `returned verify` reads the outcome from the ledger's `verified` record.** The loop would gain a reader of `ledger` records it doesn't depend on, and would have to tell which record the run wrote when `main` moved under it; `verify`'s outcome line already carries what the queue shows (`yagni`, `reuse-first`).
- **Not taken: the loop runs `clm verify` itself.** A verify runs for many minutes, and the loop's commands return at once; running work is the session's (`build-loop-in-clm`).
- **Not taken: accepting the defect,** the plan's default answer while the freeze holds. A red verify is how a slow test's break reaches the human, and losing it in silence is what `slow-tests-after-land` exists to prevent; whether to build it now is the human's, so a question item asks.

## Outcome

- `orchestrator.loop/plan-verify` (modified): the session reports the run's outcome line, the last line of its output starting `verify `, or `none`; `clm build returned verify` exits 1, recording nothing, for any other line, after the late-return rule of `returned-ignores-an-earlier-task`.
- `backlog/the-fix-for-a-red-verify-lost-to-a-note-is-outside-the-plan-s-freeze` (`kind: question`) blocks `plan-verify` until the human answers.
- `backlog/the-build-skill-reports-a-verify-run-s-last-line-not-its-outcome` (`kind: deferred`, `plugin`) changes the build skill's `run` sentence, due as soon as this change is on `main`. It also blocks `plan-verify`, so the skill reports the outcome line before the loop refuses any other (`expand-contract`): the architecture audit found nothing else ordered the two.
- `plan-verify`'s build folds in the stale "last line, as it stands" wording of `returned`'s doc comment (`crates/clm-loop/src/returned.rs`), and re-cites `crates/clm-loop/tests/plan_verify.rs`. No capability test sees a difference: none reports a `verify` return.

## Constraints

- `verify`'s output is unchanged, and so are its requirements.
- `return-rules` and `stop-causes` are unchanged for every other run; a `land:` run's last line is still reported as it stands.

## Edge cases

- **A verify that prints no outcome line** (killed, or failing before it decides) is reported as `none`, and the loop records it returned with no red result, as today; it wrote no `verified` record, so the next verify is due again.
- **A `--cause stopped` verify** is reported with `none` or its outcome line like any other; the refusal doesn't read the cause.

## Open questions

- The `--result` help text of `clm build returned` in `crates/clm/src/main.rs` ("The run's last line, as it stands.") lies outside `orchestrator.loop`'s `paths`, so the loop's build can't change it (`path-ownership`); it stays true for every run but `verify`, whose refusal names the right line on stderr. Left for the root component's next fix rather than a coupled step here.
