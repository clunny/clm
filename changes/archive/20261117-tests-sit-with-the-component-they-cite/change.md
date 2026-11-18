---
status: done
resolves: [web-tests-prove-one-web-component-from-another-s-files, a-build-is-told-to-re-cite-a-marker-in-a-file-it-may-not-change, land-approval-s-rules-for-clm-land-are-proven-only-by-landing]
---

## Intent

Phase 4 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`, step 5 of "Phase 4, what is next": the three designs that block `gate/own-files`. On `main` at `73ea838f`, 66 test markers cite a component other than the one that owns their file, which only the clause "a file that cites the component" lets a build change. This change says where each goes, so every test cites its file's owner, and has `clm check` keep it so once the root names its `wiring`, so the clause can go.

The human agreed each part on 2026-11-17, with a yes to deleting two duplicate tests. No behaviour is cut: every clause is moved or split in the same words, or stated for the first time where a test already proved it.

## Problem

- **The web app** (40 markers). `web/e2e/app.spec.ts` and `layout.spec.ts`, `web`'s browser tests, cite `web.views` 16 times and `web.client-live` twice. `web/src/views/api.test.ts` and `pr.test.ts`, `web.views`', cite `web` eight times and `web.client-live` once. The Worker's `deploy.rs`, `npmrc.rs` and `sign_in.rs` cite `web` 13 times; five of those are Rust tests that read `web`'s own `wrangler.toml` and `.npmrc`.
- **`land.approval`** (25 markers). Four of its requirements state what `clm land` does and are proven by landing a branch from `land`'s crate, which `clm-approval` can't depend on.
- **The instruction** (`orchestrator.run/pending-prompt`). The build prompt tells a build to re-cite a stale marker "wherever its file sits, another component's `paths` included", which the land will refuse once the clause is gone, and nothing asks the file's owner to do it.
- One more marker, in `crates/clm-check/tests/state_names.rs`, cites `check.coverage`; `backlog/clm-check-still-re-exports-the-coverage-library` already has `check` delete that test.

## Approaches

**The browser tests.** (A, taken) They become `web`'s proof: `web` states what the built app shows in a browser and the tests are re-cited where they stand. (B) Give them to `web.views` with a `tests` glob: no spec text, but they run `web`'s entry file and stylesheet, which a component's own tests may not (`components-own-their-tests`). (C) Leave them uncited: the clauses only a browser shows would have no proof.

**The Rust tests of `web`'s files.** (A, taken) Port them to `web`'s own test files, deleting the two whose every assertion is already made there. (B) A `tests` glob giving the two Rust files to `web`: no test changes, but `web`'s tests stay in the Worker's package.

**The stale marker.** (A) The prompt names it as the owner's to re-cite: a dead end, since the owner has nothing pending and is never queued. (B, taken) `clm check` refuses a marker that cites a component other than its file's owner, in every owned file, once the root names its `wiring`; the prompt follows the land.

**`land.approval`.** (A, taken) The four requirements move to `land`, and `land.approval` states what its own calls answer, with tests in its crate. (B) Move only: `clm-approval`'s deciding code would be proven only from another package.

## Outcome

- **`web`** gains `cost-views-in-the-browser`, `review-in-the-browser`, `graph-in-the-browser`, `backlog-in-the-browser` and `push-in-the-browser`, each scenario written from a browser test that exists. `invited-users` keeps what the build and `wrangler.toml` do; `approve-by-branch` and `notice-refresh` each lose one scenario that `web.views` states; `site-served-directly` and `no-cron-trigger` lose a sentence telling the build to edit the Worker's `deploy.rs`; `no-install-scripts` states the reading its test already makes.
- **`web.views`** gains `worker-requests`, what its request module does, which eight tests proved under `web`'s and `web.client-live`'s names. `approve-button` says the approval it shows is read from the review, which its test under `web`'s name proved, and no longer points back at `web/approve-by-branch`.
- **`web.worker`** gains `invited-sign-in`, the Worker's half of `web/invited-users`, with its two scenarios word for word.
- **`land`** gains `approval-policy`, `approval-from-base`, `signed-land` and `changes-requested-holds`, word for word but one sentence of the last, the scan for a copy of `ledger`'s rules, which stays with `land.approval`.
- **`land.approval`** loses those four and gains `approvals-counted`, `policy-at-base`, `request-refusal` and `key-and-signing`.
- **`check/tests-cite-their-owner`** judges every file a component owns once the root sets `wiring`. `check` takes one exemption, for `state_names.rs`.
- **`orchestrator.run/pending-prompt`** says "re-cite wherever it sits" only while the root names no `wiring`.
- **`gate/own-files`** needs the last two, so the clause goes only after them.
- `ARCHITECTURE.md`'s `approval-policy`, `signed-ledger-records` and `invite-only-web` name the requirements that enforce them from now on. No entry's statement changes.

`moved-tests.txt` lists 67 test-and-requirement lines, the 65 above and two in `clm-approval`'s own files, with the file each ends in and what it cites there.

## What is removed

Nothing that states behaviour. Listed so the human sees every removal:

- Two tests are deleted, with the human's yes: `deploys_the_clm_worker_on_the_apps_routes` and `sends_only_the_api_and_its_paths_to_the_worker` in `crates/clm-web/tests/deploy.rs`. `web/src/wrangler.test.ts` makes each of their assertions but two, the Worker's name and the assets directory, which join its tests and `web/site-served-directly`'s scenario.
- `crates/clm-web/tests/npmrc.rs` is deleted once its two tests are ported to `web/src/npmrc.test.ts`.
- `web/approve-by-branch`'s scenario "no fingerprint" folds into `web.views/approve-button`'s "disabled", which states it. `web/notice-refresh`'s scenario "offline copy" moves to `web.views/worker-requests`, word for word.
- Two sentences that told `web`'s build to edit files it may not go. `site-served-directly`'s named the Worker's `deploy.rs` and `capabilities/web/harness.ts`, which holds no marker, so the land refuses that edit today whatever the clause; `no-cron-trigger`'s was carried out long ago. Neither states behaviour.

## Constraints

- **Order.** `land.approval`'s removed requirements wait for `land`'s (`moved-markers-wait`). `web`'s modified requirements need `web.views/worker-requests` and `web.worker/invited-sign-in`, so those two builds re-cite their own tests first and `web`'s ports the Worker crate's three, deletes them there and deletes the backlog item. `check/tests-cite-their-owner` needs `land`'s four and `land.approval/request-refusal`. The web components are optional, so it can't need theirs: `backlog/web-tests-prove-one-web-component-from-another-s-files` is kept, rewritten, to block it until the last of the three web builds deletes it. `gate/own-files` needs `check/tests-cite-their-owner` and `orchestrator.run/pending-prompt`.
- **Sizes.** `web`'s spec goes over its budget of 12,000 tokens and `land`, far over already, takes four more requirements; `land.approval` shrinks. The requirements this adds are candidates for phase 5's regrouping with the others phase 4 added.
- **No migration.** Neither `wiring` nor the widened rule has been in a release.
- **The Designs name `moved-tests.txt`** in `land`, `land.approval`, `web`, `web.views` and `web.worker`, as the root's does for the first move; phase 4's closing branches take those sentences out.

## Open questions

From the architecture audit, left as they are:

- **`slim-components`.** `web`'s spec goes over its budget and `land`'s, at 40,000 tokens of 12,000 already, grows by four requirements. The change neither splits them nor raises a budget: the split of `land` is what moved these requirements out in the first place, and the plan's phase 5 regroups and cuts what phase 4 added before any budget is judged. Nothing fails for it today.
- **A required component waits on optional builds.** `check/tests-cite-their-owner`, and `gate/own-files` behind it, wait on a backlog item that `web`'s build deletes. `own-files-at-the-gate` says why that is taken here.
- **The guide runs ahead of the code** by the two sentences on `clm check`'s widened rule, until `check`'s build; a code branch can't carry a guide edit.

From the builder check, for later work and not this change's: `clm-approval` slugs author names with a copy of `ledger`'s `slug`; `web/e2e/live.spec.ts` holds a test of `web.client-live`'s socket under a `web/notice-refresh` marker; the unreleased upgrade note on `wiring` is `backlog/the-proof-is-real-upgrade-note-doesn-t-say-wiring-switches-on-a-check-too`.
