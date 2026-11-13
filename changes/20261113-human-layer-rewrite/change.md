---
status: applied
---

## Intent

Phase 1 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`: rewrite the human layer by hand, with the human in the conversation. `ARCHITECTURE.md` states each constraint in one or two sentences, labelled checked, measured or principle, with its mechanics in the decision it names; `CAPABILITIES.md` folds the runner into `build-from-spec`, moves `pin-working-method` to the constraints, adds the capability for measuring by evals and rewords scenarios the product's boundary can't observe. The contradictions the plan lists under Problem are removed, and `docs/the-clm-project.md` and the opening of `AGENTS.md` are brought up to date with the settled layer.

The plan item stays in the backlog: this change names it here only, never in `from` or `resolves`.

## Settled with the human (2026-11-13)

Pass 1, the constraints: 56 become 43. No enforcing requirement is deleted; each is pointed at the surviving entry or a capability.

- Merged: `regenerate-dont-merge` into `hash-based-staleness`; `trunk-named-main`, `no-local-land-with-remote` and `guard-blocks-push-to-main` into `land-locally`; `land-policy-from-base` and `approve-pins-the-review` into `approval-policy`; `web-approve-off-when-signed` into `signed-ledger-records`; `needs-alone-order` into `component-reconciliation`; `settings-before-use` into `checks-relax-before-use`.
- Moved down to their decisions: `per-writer-ledgers` (rules serve `git-is-truth`), `lease-claims` and `autosync-policy` (rules serve `work-concurrently`), `health-advises` (`know-health`), `say-guard-is-off` (`guard-sessions`), `views-name-their-commit` (`see-project-state`).
- Added: `pin-working-method` (from `CAPABILITIES.md`, not carried, checked) and `removal-needs-a-yes` (an entry of its own, carried as `method`, principle until the gate can tell a removal from a rewording).
- Labels: no entry is measured until phase 6. `path-ownership` is principle until phase 3's land check; `tests-run-concurrently`, `tests-run-in-their-own-tree`, `land-checks-stay-fast` and `expand-contract` are principle with their checked parts named. `#### Enforced by` lists programs only: the prompt-scan rules leave it.
- The label is a word in each entry's statement; `#### Enforced by` and `#### Untested` stay as the parser reads them, and a block for measured waits for phase 6.
- Mechanics an adopter is held to go to a guide page, since `clm guide constraints` prints only an entry's statement and adopters can't read this repository's decisions (`method-in-the-plugin`); CLM-internal mechanics go to decisions.

Work in progress: pass 2, the capabilities table, is with the human.
