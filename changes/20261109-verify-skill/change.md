---
status: proposed
---

## Intent

A person can verify `origin/main` as it is now with `/clm:verify`, without waiting for one of `[verify]`'s triggers. The skill runs `clm verify --commit origin/main`. On a red result, it shows the failing step and its last lines, re-runs the failing tests when it can tell which they are and the project can run one alone, and offers to take the failure in at `P1` right then. `clm verify` itself prints that report of a red run, the failing step and the last lines of output, so every caller gets the evidence, not only the skill.

## Problem

`clm verify --due` runs only when a trigger fires (`change-done`, `queue-drained`, `work-done`, `every`), and `/clm:build` and `/clm:triage` run it only at their end. A person who wants to know now whether `main` is green, before a release or after a run of lands, has to know that `clm verify --commit <sha>` exists and look up the head's ID. `--commit` already accepts `origin/main`: it fetches first and resolves the revision with `git rev-parse`, but no requirement says so.

When the run is red, `clm verify` prints the outcome line and the log's path, and nothing about what failed. The shared check runner already returns the step open when the check's output ended and its last 40 lines (`land/shared-check-runner`), and `clm land`'s refusal shows them (`land/check-log`), but `verify` drops them. Whoever takes the failure in has to open the log to learn what broke, and nothing tells a flaky test from a real break: CLM sees only the check's exit status, and one failing run is recorded red.

## Approaches

- **CLM keeps and shows the evidence; the session judges flakiness (chosen).** `clm verify` prints a red run's failing step and last 40 lines on stderr after its record, as `clm land`'s refusal does, from what the shared runner already returns, asking nothing more of projects: step markers stay optional. The skill reads them and, only when it can tell which tests failed and the project can run one test alone, re-runs them at the failing commit and says whether each failed again or passed. That advises; the human decides what to take in (`deterministic-enforcement`, `verify-after-land`). A flaky test the re-run shows is offered as a bug in the test, which holds no land, not as a `verify_failures` item.
- **`clm verify` re-runs a failed `[verify] check` once before recording red, behind a `[verify] retries` setting.** Deterministic and the same for every caller, but it re-runs the whole suite on every real break, and a second whole run says nothing about which test flaked. The human didn't want to impose more on projects. Rejected for now; the bisect probe case, where a flake blames an innocent land, goes to the backlog as `verify-bisect-blames-a-flaky-probe`.
- **Leave retries to the project's script** (per-test retries in its runner, as `ci-check.sh` does for Playwright). That stays open to any project and the guide already allows it, but CLM can't count on it, so it's no answer on its own.
- **A new CLI form (`clm verify --head`).** Not needed: `--commit origin/main` does it after the fetch. The change writes that down in `verify-commit` instead (`code-fix-needs-a-written-rule`).

## Outcome

- `verify/verify-run`: a red run prints, after its record, the last 40 lines of the first run's output under a line naming the check, the step open when it ended with its line range when there was one, and the log's path, on stderr.
- `verify/verify-commit`: `--commit` takes any revision that names a commit after the fetch, `origin/main` among them.
- `plugin/verify-skill`: the new skill, its file in `plugin`'s `paths`.
- `land`'s Design no longer says `verify` names no step.
- Decision `verify-skill-reruns-advise`.

## Constraints

- The re-run never changes a `verified` record and never decides anything: only the human's yes takes a failure in (`verify-after-land`, `deterministic-enforcement`).
- The skill pushes nothing to `main` but through `clm land` (`land-locally`), and works in a worktree of its own (`session-worktrees`).
- `[verify] check` runs under the clone's check lock, so the clone's lands wait while `/clm:verify` runs (`check-runs-one-at-a-time`); the skill says so.
