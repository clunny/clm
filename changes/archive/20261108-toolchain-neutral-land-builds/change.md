---
status: done
resolves:
  - a-land-check-that-builds-nothing-still-waits-for-one-that-does
  - only-cargo-projects-get-a-shared-land-build-and-its-lock
---

## Intent

`main` moves quickly, and every move makes each waiting land re-check nearly everything, one land at a time. A land whose build already ran its full check should run only the tests where its own changes and what `main` gained since that check can affect each other, and none at all when they can't. CLM states the intent of a land check as a rule every CLM-built project carries (`land-checks-stay-fast`): fast, deterministic, safe beside other runs and honest about what each test reads, with slow or exclusive tests after the land in `[verify]`. CLM decides when and where a check runs; the project decides how its tests run, so CLM hands a land's check a tool-neutral place to build in (`CLM_LAND_DIR`) instead of knowing about cargo, starting with the expand step of that move.

## Problem

A build's `clm land --check-only` runs every test and records its head's code key as checked (`worktree-check-counts`). Its land reuses that only while `main` stays where the build saw it. With several builds landing, `main` has always moved, so the merged tree's code key is new and the land runs `clm affected`'s selection of what the branch can break, under the land and check locks. In this repository that selection is often everything (`[affected] all` names the CLI's dispatch and the check script), and the land compiles the merged tree, so each move of `main` costs nearly a full check per waiting land, in series. A spec or docs land on checked code waits behind it too.

`land` also knows one toolchain: it sets `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL` and ties its shared build place's lock to cargo, so a project in another language gets no warm place, and CLM prescribes tooling it should leave to the project.

## Approaches

- **Re-check only the overlap (chosen).** A test outside the branch's selection behaves on the merged tree as on `main`'s head, which landed checked; a test outside the selection of what `main` gained behaves as in the build's full check. Only tests in both selections can change, so the land runs those, or nothing. Sound exactly as far as the graph's facts are, which `graph-test-selection` already trusts; a gap shows after the land, in `[verify]`. The approach Uber's SubmitQueue takes with build-graph conflict analysis.
- **Check queued lands as a batch** (merge-queue style: one check for `main` plus every waiting branch, split on failure). Helps where the changes do overlap; designed next, as `queued-lands-each-run-a-full-check`.
- **Speculative parallel checks** (Zuul: A, A+B, A+B+C at once). Rejected: N builds at once, and six parallel builds already filled the disk (`builds-run-out-of-disk-mid-check`).
- **No tests at land at all, only `[verify]` after it.** Rejected for now: a broken `main` fails every parallel build's check for a reason not its own, and CLM's cost grows with the number of builds. The overlap rule already runs nothing when the sides are independent; each land records how it re-checked, so the ledger shows how often the overlap runs and what it costs, and dropping it can be decided from that.
- **Keep prebuilt test binaries per code key** (`cargo nextest archive`, the item's first direction). Rejected: tool-specific, and a land that edits the guide, which `clm guide` compiles in, still needs a rebuild, so it can't hold for every land.
- **Finer test selection** (per-test file-access tracing, coverage maps). Deferred to its own change after batching; it makes an overlap that can't be skipped smaller.

## Decisions

- The human chose the order: overlap first, then batching, then finer selection; and to keep the overlap check, logged, rather than land with no tests (2026-11-08).
- A failure after the land is reported with its evidence (whether the culprit's land skipped or narrowed its re-check, and on what basis) without presuming its cause: the graph, the code or a flaky test (the human, 2026-11-08).
- Going over `check_target_seconds` stays a report, not a refusal: refusing a slow land would stall `main`, the problem this change removes.
- The check lock stays as it is for lands. Dropping it for lands with a slot was considered, but lands that run a check still take turns on the land tree (`fixed-land-tree`), so it would buy nothing; the overlap rule is what takes checks out of the queue.
- `CLM_LAND_DIR` is added beside the cargo variables, which `clm land` still sets, so nothing breaks and nothing is marked for removal yet. Removing them, with the starter script's change and the migration that carries it to projects (`upgrades-carry-migrations`), is a later change of its own (`backlog/land-still-sets-cargo-variables`), proposed with its contract step once projects have the note.
- A composed verdict is a second land verdict that doesn't rest on a cleaned tree, beside a build's recorded check: it rests on that check and on `main`'s. `fixed-land-tree` and `worktree-check-counts` are amended to accept it.
- A composed `checked` record names only the two keys it rests on; what the land ran stays on the `landed` record (`graph-test-selection`, `land-times-on-the-landed-record`).

## Outcome

- `clm affected --overlap <a> <b> [--base <ref>]` prints what both `<base>..<a>` and `<base>..<b>` can break (`check.affected/overlap-selection`).
- A land whose merged tree's code key is new, whose head was checked whole and whose fetched `main` is checked, runs `[land] check` with the overlap as `CLM_SELECTION`, or no check at all when the overlap is empty, and records a composed `checked` record and how it re-checked (`land/overlap-recheck`, `ledger/composed-check`, `ledger/landed-recheck`).
- `clm verify` names how a culprit's land re-checked (`verify/culprit-recheck`).
- Every land check in the land tree gets `CLM_LAND_DIR` (`land/land-dir-env`); `ci-check.sh` points cargo at it, and an advised `clm upgrade` note tells projects to (`ci/land-dir`, `upgrade/migration-land-dir`).
- This repository runs its full check after lands with `[verify]` (`config/verify-here`).

## Constraints

- No change to a land whose code key is already checked, or to one with no full check of its head: both run as today.
- The overlap rule never runs fewer tests than the facts allow: an `all` on one side makes the overlap the other side's whole selection, and anything it can't compute falls back to today's selected check.
- `ARCHITECTURE.md` gains `land-checks-stay-fast`, a carried, replaceable constraint, so the branch needs review.

## Open questions

- None yet.
