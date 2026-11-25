---
status: done
resolves: [the-rule-for-which-files-the-web-replica-fetches-is-held-three-times, the-replica-s-snapshot-leaves-out-a-component-s-binary-assets]
---

## Intent

Wave 6 of phase 5 of `backlog/a-passing-check-proves-less-than-the-spec-claims.md`, its branch A of two: the Worker's components, `web.worker`, `web.worker.platform`, `web.worker.replica`, `web.worker.review` and `web.live` (crates `clm-web`, `clm-web-platform`, `clm-web-replica`, `clm-web-review` and `clm-web-live`), are each brought, in one pass and one delta, to the rule for a requirement (`requirements-state-what-the-contract-shows`), with small deltas of `replica`, `ledger`, `gate` and `land.approval`, one to four requirements each, where a fix needs a call only they can offer. Each requirement now states what its component's Contract shows: what each route answers and writes, what the objects hold and push, and what each platform call gives, refusals and failures included. Scans of source text are cut, rules held twice get one provider, the object's refusals move from the routes to the object that makes them, and the faults the pass found are mended by each component's build. Branch B, `20261124-wave-6-app-by-the-rule`, holds `web`, `web.views`, `web.client-live` and `web.site` with a small delta of `ci`, and is rebuilt on this one once it lands.

## What the pass found

Each component was read against the rule and the code by one reader, then drafted by one drafter who checked each of the reader's entries against the code, with a check and a correction round. The builds mend, worst first:

- **A branch is approvable from the web with files the review never showed** (`web.worker.review`, `web.worker.platform`): the approve route accepts any changed path the gate counts as a spec path (all of `docs/`, `CLAUDE.md`, `AGENTS.md`), but the review shows only the narrower copy of the rule's files among those it fetched. A branch adding only `docs/allowed_signers`, the repository's first trusted keys, can be approved unseen. The review's `documents` now come from the listings: every changed path the gate counts, each side's text, and the platform's `spec_files` fetches every such file.
- **The review shows a request not held where `clm land` holds the branch** (`web.worker.review`): in a signed repository, `held` ignores signing, so an unsigned approval clears a request on the web while the land still holds it. It now counts only an approval whose line carries a `sig`, through `ledger`'s new call (the one difference left is recorded in `docs/decisions/the-web-review-sees-a-signature-not-its-validity.md`).
- **An append to a person's web ledger writes when the ref can't be read** (`web.worker.platform`): a failed ref read reads as "no branch", and the append writes a parentless commit holding only the new line, then asks to create the ref; only GitHub's 422 stops it. It now writes nothing and fails.
- **The review of a commit with more than 20 binary assets never completes** (`web.worker.platform`): the 20-blob cap counts cache hits, so every read stops at the same asset.
- **A repo with more than 50 open pull requests sees them flap closed** (`web.worker.replica`): the resync reads one page of 50 and closes every held pull request it didn't list, about every 10 minutes while a page is open. It now reads pages of 100, at most 300, through one platform call (`live::pulls_direct`), and closes only on a complete listing.
- **The app's graph page throws against the real Worker** (`web.worker`): the snapshot route's answer carries no `repo`, which the app reads. It now carries `repo` as the access check names it, cached or not, and its top-level keys are stated.
- **Sign-in with an unusable `COOKIE_KEY` loops** (`web.worker`): the cookie seals to nothing, the app gets 401 and sends the person to sign in again. The callback now answers 500 with a short page, sets no session and clears the state cookie.
- **A push endpoint the push service reports gone is pushed to for good** (`web.worker.replica`): the object logs `/send`'s `gone` and keeps it. It now keeps the gone subscriptions' ids and skips them until no fold holds them, a disposable cache of the object that a rebuild re-learns at one push per gone endpoint.
- **A failed read answers as if nothing were there** (`web.worker.platform`, `web.worker.review`, `web.worker.replica`): the platform's ref read and write-access check fold a failure into "missing" or "refused", so the review answers 404, approve, comments and change requests 422, and the object's writes 404, where GitHub failed. New calls (`ledger::branch_tip`, `access::checked_direct`) tell the two apart, and each answers 502 for a failed read, a 401 from the object's access check included.
- **A first build puts every claim in the activity feed twice** (`web.worker.replica`): the resync records a creation with no actor for each claim, then GitHub's events give the same claims. During the first build the resync records none.
- **A symlink or submodule changed by a push is held as a regular file** (`web.worker.replica`): the compare path gives every file mode `100644`. Each file's mode is now read from its parent tree in the same query, GitHub's integer modes turned back into the mode text.
- **A listed ref whose commit time doesn't parse is read as deleted** (`web.worker.replica`): it is now kept, at the time it is listed.
- **A verified webhook body that isn't JSON is dropped silently** for `pull_request`, `installation` and `installation_repositories` (`web.worker`); it is 400 for all four repo-naming events now.
- **The approval names the wrong change** (`web.worker.review`): a branch adding `changes/x/` and `changes/x-y/` records `x-y` where `clm approve` records `x`. Both now go through `ledger`'s `change_ids`, and `land.approval`'s `branch-head` moves onto it.
- **The live socket's forwarding is untestable and loses an answer** (`web.worker`): the request it forwards, and a 101 with no socket, are built in wasm-only code, and a non-101 answer reaches the page with its status only. They move to host code, and the answer passes whole.
- **`web.live`'s binding decides what only the core should** (`web.live`): a `POST` whose body isn't UTF-8 gets 422 on any path where `hub` says 404; `ping`→`pong` is set only in the binding; the `Upgrade` header is matched only there. The core now does all three, and an endpoint with a port and no host (`https://:8443/s`) is refused.
- **A `+` in a branch name or slug reads as a space** (`web.worker.platform`): `percent_decode` turns `+` into a space in path segments; it now does so only in a query.
- **Rules held twice, given one provider**: the gate's reading of a spec branch's files, copied three times in the review (`gate/review-from-files`); which files the replica fetches (`web.worker.platform` adopts `replica`'s rule, resolving `the-rule-for-which-files-the-web-replica-fetches-is-held-three-times`); which refs the views read (`replica/refs-the-views-read`, which also stops `replica` accepting `impl/` with no name as a claim); the RFC 3339 parse and format (`ledger/record-times`); the send limit of 10 (`web.live`'s `MAX_MESSAGES`); the percent-encoder (both onto the `percent-encoding` crate); the NUL probe (`replica`'s alone); the open pull requests query (`live::pulls_direct`); and the Worker's copy of the proposal slug rule, which needs only whether a slug would be empty.
- **A component's binary assets are missing from the replica's snapshot** (`replica`, `web.worker.replica`; resolves `the-replica-s-snapshot-leaves-out-a-component-s-binary-assets`): the replica now keeps each asset's hash (not its bytes), and the object fetches binary assets whole. `replica`'s `File` constructors (`replica/files-a-source-gives`) landed before this change, by `20261124-replica-offers-file-constructors`, so a field `File` gains here breaks no source once the object builds its files through them (Build order).
- **Dead code**: KV in three components (the platform's trait, the Worker's adapter, `web`'s binding on branch B); `live`'s copies of tips, claims and events with the platform's `clm-orchestrator` dependency; the fake's unused helpers; the platform's unused file listers and binary probe; unused dev-dependencies of `clm-web`; and public items with no caller outside their crate.
- **Comments and Contracts that name what doesn't exist**: `shared-layer` and `fingerprint-from-trees` in platform files; `web.worker/spec-branches` and `review-cached` in the review's.
- **Needs against `depends_on`**: `web.worker`'s on `web.views` and `web` requirements, and `web.live`'s on `model/parse-backlog` and `web.worker/cache-is-disposable`, ordered nothing and go.
- **Found and filed, outside this change's requirements**: a marker above a `describe` block counts once for coverage while `uncited-test` counts every test inside it (`backlog/a-marker-above-a-describe-block-counts-once-for-coverage-while`, a CLM defect); `clm land`'s refusal of a deleted test reads as if a re-cited test must keep its citations (`backlog/clm-land-s-refusal-of-a-deleted-test-reads-as-if-a-test-keeping`, a CLM defect).

## Cuts

### Under an earlier yes, or moves (nothing lost)

- **`web.worker.platform/unix-time`** moves whole to `ledger/record-times`, with its two scenarios by name ("times GitHub gives", "text that isn't such a time"). The parse is `ledger`'s, which writes the `at` it reads; the platform stops holding a copy. Its three tests are rewritten by `ledger`'s build in `clm-ledger` and deleted here.
- **The object's refusals leave the routes** (`web.worker/decide`, `architecture-decide`, `proposals`, `answers`): the 422s, 409s and 404s the object makes, and their scenarios *reworded since*, *signed*, *unknown capability*, *not an entry*, *refused*, *taken by a change* and *withdrawn*, move by name to `web.worker.replica/write-refusals` and `writes-through-object`; decide's *GitHub fails* merges into *the tip can't be read* there. The routes Need them and say "any other refusal SHALL be the object's". The 14 Worker tests that drive these through the real object are re-cited to `web.worker/writes-to-object`, not deleted.
- **`web.worker/invited-sign-in`'s "A request for `/_clm/invited.json` SHALL answer 404"** and its scenario "the list is never served": `reserved-paths` states it and its scenario "the invited list" shows it with a session and without.
- **The fingerprint's equality with `clm fingerprint`** leaves `web.worker.review/fingerprint-from-trees` for `web.worker.platform/fingerprint-from-listings`, scenario "same as the CLI" by name, with the two keys tests moved.
- **The approve's missing-fingerprint 422** leaves `web.worker.review/approve` for `web.worker/branch-routes`, scenario *nothing named* by name, proven through the real router where the review's copy ran a stand-in router.
- **`web.worker.review/pr-review`'s scenario *truncated listing***: `gate-answer`'s *a truncated listing* and `fingerprint-from-trees`' *truncated listing* show both halves.
- **`web.worker.review`'s Contract bullet `review::signed` and `review::signed_refusal`**: nothing outside the crate calls either; the rule stays in `signed-repository`.
- **Dead code with no requirement** (`no-dead-code`, through `backlog/platform-fake-dead-helpers`, widened here): `live`'s copies, the file listers and binary probe, `install::rfc3339`, the fake's helpers. And **withdrawn calls, in steps** (`old-paths-removed`): `platform::Kv`, `access::check_direct`, `files::is_spec_path`, `install::unix_time`, each replaced by a stated call. `ledger::tip` stays, stated by `branch-tip` beside `branch_tip`, for callers such as the review's test helper that need only a tip.
- **Design sentences that restate a requirement, place tests, or record a finished build order** (no SHALL): `web.worker`'s "Moving the routes", "Tests of `web`'s files" and the Proposals, Decide and Notifications paragraphs' restatements; `web.live`'s restatements of `hub`, `push-send` and `web-push-request`, its sentences about other components' code, its test-placement sentence, and the `notices` build instruction (its file no longer exists); `web.worker.review`'s refusal orders and bodies, and its sentences on the activity feed (the platform's `live::events`, now deleted); `web.worker.platform`'s "The platform answers no path specially".

### Needing the human's yes now

`web.worker`:
- **`no-scheduled-event`, whole.** Its subject is `worker.rs`'s text; nothing can send a scheduled event while `web` sets no cron trigger, and its second scenario repeats the wasm build the export requirements run. Lost: a guard against someone re-adding a scheduled handler (`no-dead-code` would remove one). Saves a requirement, two scenarios and a third full wasm build, the only one without its own target directory. Its Need on `web/no-cron-trigger` goes with it.
- **"the class is exported" of `live-class-export` and `replica-class-export`**: they read `Cargo.toml` and `lib.rs` text. "The Worker builds" shows the module exports both classes. Lost: a check on how the export is written. Saves two scenarios and two tests.
- **`deploy.rs`'s tests of other files' text**, cited or not: `the_worker_reads_the_build_time_assets_through_its_binding` (cites `invited-sign-in` but asserts wrangler's binding and a literal of `worker.rs`; the list read through the binding is shown by `sign_in.rs::without_the_invited_list_nobody_gets_in`), `the_code_reads_the_designs_secrets_and_links_the_app`, `readme_records_how_to_redo_the_setup` (whole: `web/README.md` text and the KV id) and `binds_the_kv_namespace` (a dead binding). All but the first cite nothing, so `deleted-tests.txt` can't name them (`gate/deleted-tests` refuses a line naming a test with no marker): the session deletes them, with the two below and the slug unit test, after the human's yes (Build order).
- **`deploy.rs::no_secret_is_committed`** (uncited): that `web/wrangler.toml` holds no `[vars]` and `web/README.md` no private key. Lost: that guard. The alternative is to keep it as a repository-wide check, `ci`'s or a secret scanner's, not a per-crate text scan. Proposed: cut.
- **`deploy.rs::builds_with_worker_build_matching_the_pinned_worker_crate`** (uncited): it guards a deploy whose `worker-build` doesn't match the pinned `worker` crate, which builds a module whose glue doesn't fit. `web`'s drafter declined it. Lost: that guard. The alternatives are a `web` requirement on the pin, or `build:worker` reading the version from `Cargo.lock`, which makes the guard unneeded. Proposed: cut.
- **The Worker's copy of the slug rule** and its unit test `proposals.rs::slugs_as_clm_defect_does` (it cites `web.worker/proposals`, so `deleted-tests.txt` lists it, added by `20261124-wave-6-app-by-the-rule`; the build deletes it with the copy): the Worker needs only "would the slug be empty". `proposals`' scenario "the slug" still proves slugs end to end through a real object. Lost: a unit test of a copy.
- **The Design's GitHub App manifest** (the JSON to create the App again). Lost: a copy of the manifest in the spec; the App's settings on GitHub hold it. Saves about 150 tokens the budget needs.

`web.worker.platform`:
- **`shared-layer`**, requirement and scenario "one copy": that `clm-web` and `clm-web-review` hold no `src/` file named like a platform module. Its subject is source layout, and a copy under another name dodged it. Lost: that scan. Saves a requirement, a tree-reading test and eight Needs that held the review's builds behind any reword; its scenario "the append" moves to `web-ledger-append` by name.
- **`object-reads`' "where the Worker's form of the same function keeps its answer in the Cache API, it SHALL call that form rather than repeat it"**: it said how code is shared, and backwards (the cached form calls the uncached one). Lost: nothing observable; "answer as the cached form answers" keeps the behaviour. Its trait-bound clause is restated as the platform parts it runs on.
- **The uncited test `src/live.rs::nested_branches_are_left_out`**, with the dead `live::tips` it tests. It cites nothing, so no list line names it.
- **`old-paths-removed`, once built**, states only an absence, and is then a cut to propose, as `20261122-old-calls-gone-is-cut` cut `land.approval/old-calls-gone`. Not cut now: its Needs order the withdrawn calls' removal after their last user. Until then it is a named exception to `requirements-state-what-the-contract-shows`: its proof reads this crate's source, which that rule would refuse.

`web.worker.replica`:
- **`object-class`'s "no source file of the crate SHALL name `Kv` or `Cache`"** and its scenario *no cache*. Lost: an in-crate scan for the bare identifier `Kv`. `web.worker/cache-is-disposable`'s isolation test still scans every crate's sources for `KvStore`, and this crate's for `kv` and `Cache`.
- **`object-class`'s THEN "the crate's sources hold `#[durable_object]` on a type named `Replica`"**, and its sentence naming the platform's trait bounds. Lost: a text check of the attribute. The class's export is shown by `web.worker`'s wasm build; the Design keeps the platform's description.

`web.live`:
- **The test `wasm.rs::needs_no_getrandom`**: that the wasm dependency tree holds no `getrandom`, and no `worker` without `durable`. No requirement states either and no caller observes them; a `getrandom` with no wasm backend already fails the wasm build. Lost: a guard that a dependency bump doesn't pull in `getrandom` with a `js` backend, which would build and work. Saves two `cargo tree` runs per test pass. The alternative is a clause of `builds-for-wasm` ("its crypto pulls in no `getrandom`"), a dependency-tree rule rather than behaviour.
- **`hub`'s "without panicking"**: the panicking default handlers are `worker`'s, run only in the binding, which no test runs. What it guarded (a close or error closes the socket, a message is ignored) stays stated.

## What the requirements no longer state

`web.worker`:
- `no-scheduled-event`, whole: "The Worker SHALL handle no scheduled event: `crates/clm-web/src/worker.rs` SHALL hold no `#[event(scheduled)]` handler and import neither `ScheduledEvent` nor `ScheduleContext` … and the wasm build of `clm-web` SHALL still succeed."
- `live-class-export`: "`clm-web` SHALL depend on `clm-web-live` with its `durable` feature in its `wasm32` target dependencies and re-export `Live` (`pub use clm_web_live::Live`)"; now the module exports the class, however written. `replica-class-export`: the same for `clm-web-replica` and `Replica`.
- `invited-sign-in`: "A request for `/_clm/invited.json` SHALL answer 404, so the list is never served." (`reserved-paths` keeps it.)
- `decide`: the 422 cases for a tip whose specs don't load, an unknown slug and a signed repository, the 409 for a moved version, "The 422 cases are checked before the 409 one", and the 502 when the object can't read the tip or the append fails; `architecture-decide`: the same for `ARCHITECTURE.md`. Moved to `web.worker.replica/write-refusals` and `writes-through-object`.
- `proposals`: the 422 for a component the replica's graph doesn't hold, the 409 `taken` sentence, and the withdrawal's 404 for an `id` the requester's own ledger doesn't hold open. Moved to `write-refusals`.
- `answers`: "404 with `{"error": "unknown"}` for an item the replica refuses as `Unknown`" and "or one the replica refuses as `Invalid`". Moved to `writes-through-object`.
- `sign-in` and `webhook-refresh`: "never in KV" and "writing no KV" become "never in the Cache API or any other store" and "storing nothing". `cache-is-disposable`: "after KV, the Cache API …" loses KV, and "only `clm-web` SHALL use its KV and Cache API bindings" becomes "its Cache API binding, and none SHALL use a KV namespace".
- `not-invited-page`: the apposition "the one constant in the Worker that names it".
- Design: "The tests that prove `web`'s requirements for what `web/wrangler.toml` and `web/.npmrc` hold are `web`'s and sit under `web/src/`" and "`deploy.rs` holds tests that cite nothing …".

`web.worker.platform`:
- `shared-layer`, whole: "`clm-web-platform` SHALL hold the Worker's platform traits, … as the modules `platform`, `http`, …, and `clm-web` and `clm-web-review` SHALL use them from it, holding no module of those names of their own."
- `unix-time`, whole, restated by `ledger/record-times`.
- `object-reads`: "where the Worker's form of the same function keeps its answer in the Cache API, it SHALL call that form rather than repeat it", and the trait-bound wording of its first sentence.
- `asset-responses`: the method signatures and "provided method"; the behaviour is kept.
- `code-key-doc-only`: "by calling it"; the behaviour is kept and `ledger/code-key` is now Needed.

`web.worker.replica`:
- `object-class`: "`clm-web-replica` SHALL define, for `wasm32`, the Durable Object class `Replica` (`#[durable_object]`), which … runs the alarm through `Object` over … a platform of its own that implements only `GitHub`, `Clock`, `Secrets`, `Live` and `Log`, and no source file of the crate SHALL name `Kv` or `Cache`." It becomes: the crate SHALL build for wasm on its own, and its class SHALL answer the three routes and run the alarm over its own SQLite storage.
- The Contract's "Answers 202 once the event is applied": on an object that holds nothing the event is not applied, and `first-build` says so.

`web.worker.review`:
- `approve`: "SHALL answer 422 and write nothing for a request whose body doesn't name a `fingerprint` (checked before any request to GitHub)". Moved to `web.worker/branch-routes`.
- `fingerprint-from-trees`: "In a repository whose CLM root is the top of its git repository, … that fingerprint SHALL equal what `clm fingerprint` prints for the branch against the default branch (`gate.fingerprint/fingerprint`)". Moved to `web.worker.platform/fingerprint-from-listings`, which it Needs; "with `clm_gate::fingerprint_of`" goes.
- `pr-review`: "`review` SHALL also be true when `clm_gate::widened_exemptions` or `clm_gate::cuts_or_unloaded` returns anything, …" and the withheld-list text rule, now `gate/review-from-files`', which `gate-answer` Needs; "the cache key is named anew" goes (the clause "SHALL NOT be served" stays).
- `approval-setting-branch`: "never by a rule of its own" and "the comparison's cache key is named anew".
- `review-feedback`: "deciding it, and taking a writer's person apart anywhere, through `ledger`'s `cleared` and `person`, with no copy of either in `clm-web`" (where code sits, naming the wrong crate). The request route's sentence moves to `request-changes`, stated in full.
- `spec-branches`: "`GET /api/repos/{o}/{r}/branches` SHALL list" becomes what `branches::list` gives; the route and its 502 are `web.worker/branch-routes`'. "whose name the branch review accepts" becomes the rule itself.

`web.live`:
- `hub`: "with the hibernation API" becomes what it means (the socket stays open, and keeps receiving notices, while the object is evicted); "without panicking" goes.
- `notices`: "this build also moves the fixtures of other components that assert the old URL (`crates/clm-web/tests/push_notify.rs`) and names them in its pull request".
- `builds-for-wasm`: "natively …, where its tests run" becomes "for the host without it".

`replica`, `ledger`, `gate`, `land.approval`: nothing; their deltas add requirements, widen `replica/snapshot-matches-clm` to binary assets, and reword `land.approval/branch-head` onto `ledger/change-ids` (a renamed change directory now counts as added).

## Narrowings

- `web.worker/live-class-export` and `replica-class-export` say only that the module exports the class, not how.
- `web.worker/decide`, `architecture-decide`, `proposals` and `answers` say less of their own: the object's refusals are `web.worker.replica`'s, which they Need. Nothing is lost across the two components.
- `web.worker/repo-access` adds no memo, failure or casing clause: `web.worker.platform/access-check` states them for the call the routes use.
- `web.worker.platform/object-reads` no longer lists `access::check_direct`, which `checked_direct` replaces; during the steps it is kept, marked, and stated by no requirement. `access-check` states no fallback for a missing `default_branch` (GitHub always names one). `asset-responses`' Design no longer claims HEAD, `If-None-Match` and `Range` reach the binding through the Worker (`web.worker/asset-adapter` states the pass-through).
- `web.worker.platform`'s `live::pulls` reads up to 300 open pull requests where it read 50: a widening.
- `web.worker.replica/applies-events`: "a fast-forward of at most 300 files" becomes "lists fewer than 300 files", which the code does (a compare cut at 300 can't be told from one of exactly 300). `writes-through-object` replaces the variant names `Unknown`, `Invalid` and `Preparing` by what they mean. `push-waiting` and `push-review` name `web.live/notices` and `MAX_MESSAGES` instead of functions and the number 10. `fetches-what-the-replica-wants` describes `wanted` and `keeps_text` in words and says "regular files", as `replica/files-it-fetches` meant.
- `web.worker.review/signed-repository` and `review-feedback`: in a signed repository, the Worker counts an approval whose line carries a `sig`, not one whose `sig` verifies (`the-web-review-sees-a-signature-not-its-validity`). `pr-review`'s reading of the gate's files and `fingerprint-from-trees`' CLI equality say less here and Need the requirement that now says it. The gate's call counts an archived change only by a `change.md` the head adds, as the review's code already did.
- `web.live/hub`'s "without panicking" (above). `web-push-request`'s `exp` keeps "at most 12 hours", now "later than `now` by at most 12 hours".

## Build order

From the Needs, each build taking the requirements whose Needs are current:

1. `ledger` (`record-times`, `cleared-by-a-signed-line`, `change-ids`), `gate` (`review-from-files`), `replica` (`asset-hashes`, `refs-the-views-read`, `snapshot-matches-clm`) and `web.live`, in any order.
2. `web.worker.platform`'s first build: every requirement but `old-paths-removed`. It takes KV out of `Platform`'s parts (`platform-parts`) and keeps the withdrawn calls marked `clm:remove-by` this change.
3. `land.approval` (`branch-head`, after `ledger/change-ids`), `web.worker.review`, and `web.worker.replica` (`fetches-binary-assets` Needing `replica/asset-hashes`), in any order.
4. `web.worker` (its routes Need `web.worker.replica/write-refusals` and `web.worker.review`'s `review-feedback`, `request-changes` and `spec-branches`; `cache-is-disposable` Needs `platform-parts`). It drops the KV adapter.
5. `web.worker.platform`'s second build (`old-paths-removed`, Needing the last users' moves: `web.worker/cache-is-disposable`, `web.worker.replica/writes-through-object` and `times-as-instants`, and `web.worker.review/pr-review`).

One component builds twice: `web.worker.platform` (2 and 5). On branch B, `web`'s requirement that drops the KV binding Needs `web.worker/cache-is-disposable`.

**Binary assets.** A field on a record other components build by hand goes in steps the session runs before the wave. `replica` offers `File::text`, `File::binary` and `File::unfetched` (`replica/files-a-source-gives`), by `20261124-replica-offers-file-constructors`, landed before this change and built by the session. Then `replica` adds the asset's hash to `File` and widens `snapshot-matches-clm` (`asset-hashes`), and `web.worker.replica` fetches binary assets whole (`fetches-binary-assets`). No Need runs from `replica` onto an optional component, and no backlog item orders the steps. An object whose storage an older build wrote starts its first build again when it wakes, once.

**Steps no requirement states:**
- After the human's yes and before the build run, the session deletes, on small code branches, the uncited tests on the cut list that no `deleted-tests.txt` line can name: `deploy.rs`'s file-text tests (`binds_the_kv_namespace`, `the_code_reads_the_designs_secrets_and_links_the_app`, `no_secret_is_committed`), the pin test (`builds_with_worker_build_matching_the_pinned_worker_crate`) and the README test (`readme_records_how_to_redo_the_setup`). The build material says these deletions have landed.
- Before `replica`'s build, a small code branch of the session's moves `clm-web-replica`'s two `File` literals (`source.rs`'s `fetch`, `object.rs`'s `apply_compare`) onto the constructors, so the field `asset-hashes` adds breaks no build.
- The `evict_files` fix (`web.worker.review`'s test drops every key starting `files`, so `unchanged_branch` proves the cache answered under either key) landed before this change on a small code branch.
- `ledger::tip` stays (`web.worker.platform/branch-tip` states it), so the review's test helper keeps calling it and nothing moves it.
- `old-paths-removed` is kept as a named exception to `requirements-state-what-the-contract-shows`: it states an absence, proven by reading this crate's source, which that rule would refuse. It stays only to order the withdrawn calls' removal after their last users, and is a cut to propose once built, as `land.approval/old-calls-gone` was.
- The gone-endpoint list `web.worker.replica` keeps is a disposable cache of the object, consistent with `git-is-truth` and `web.worker/cache-is-disposable`: a rebuild from git forgets it, and it is re-learned at one push per gone endpoint. Its Design says so.
- The platform's dead code, `live`'s copies and the `clm-orchestrator` dependency: `backlog/platform-fake-dead-helpers`, widened here, resolved by `web.worker.platform`'s first build with a `Resolves:` trailer.
- The unused `clm-context` dev-dependency: `backlog/clm-web-lists-clm-context-as-a-dev-dependency-nothing-uses`, resolved by `web.worker`'s build with a `Resolves:` trailer.
- `crates/clm-web/tests/keys.rs::code_key_matches_ledger` stays as it is: it cites nothing and copies a case `web.worker.platform/code-key-doc-only` states, so it loses nothing by staying and is no cut; the platform's next pass can take it.
- After both builds land, dropping `context` from `web.worker`'s `depends_on` and `orchestrator` from `web.worker.platform`'s: `backlog/the-web-worker-crates-depends-on-keep-components-their-builds-stop-importing`, by a small spec change that lists it in `resolves` (`check/undeclared-imports` sees both imports until the builds remove them).

## Decisions

The human's (2026-11-24):
- **Two branches.** A, this one: `web.worker`, `web.worker.platform`, `web.worker.replica`, `web.worker.review` and `web.live`, with small deltas of components outside the wave where a clause moves or a provider's call is needed. B: `web`, `web.views`, `web.client-live` and `web.site`, with a small delta of `ci`. A lands first; B is rebuilt on A.
- **No release; wave 6's branches are approved in a session** with `clm approve`.

The session's, that this branch carries:
- The snapshot names its repo (`web.worker/snapshot-names-its-repo`), and its top-level keys are stated in that requirement and tested in Rust, not by reading `web`'s `types.ts`.
- KV is dead code in three components, removed in Needs order: the platform takes it out of `Platform`'s parts, `web.worker` drops its adapter, the platform removes the trait; `web` drops the binding on branch B.
- A verified webhook body that isn't JSON gets 400 for all four repo-naming events. Sign-in with an unusable `COOKIE_KEY` answers 500, not a loop.
- Binary assets are fixed in this wave. The `File` constructors came first, by a small spec change the session landed and built, then a small code branch moves the object's `File` literals onto them; this branch's `replica` delta holds only `asset-hashes`, `refs-the-views-read` and the widening of `snapshot-matches-clm`.
- Pull requests flapping closed, duplicate claims on a first build and symlinks held as files are `web.worker.replica`'s build's fixes; the dedupe is the object's, not `replica`'s (only the object knows a ref it didn't hold wasn't looked at yet).
- Held in a signed repository: the review counts an approval as `clm land` does as far as the Worker can know, through `ledger`'s one call; the Worker sees a `sig`'s presence only, and the difference is recorded in the decision `the-web-review-sees-a-signature-not-its-validity`, added here.
- Files approvable unseen: every file the approve route accepts is a file the review shows; the gate's file reading becomes one call `gate` offers for wasm (`gate/review-from-files`), under which `clm gate`'s output is unchanged for every branch. The gate's Design names `review_from_files` where the Worker reaches `needs_review_of` and `widened_exemptions`.
- A missing ref and a failed read are told apart by the platform (`branch-tip`, `access-check`), and every route and write answers 502 for a failed read.
- Small deltas of `ledger`, `land.approval`, `gate` and `replica` ride this branch only as far as the fixes need their calls.
- The percent-encoder is the `percent-encoding` crate in both `web.live` and `web.worker.platform`, with no dependency between them. The send limit is `web.live`'s `MAX_MESSAGES`, which `web.worker.replica` uses.
- `ping`→`pong` is stated by `web.live/hub` and proven through the fake Host; log lines are not stated.
- The RFC 3339 parse and format are `ledger`'s (`ledger/record-times`); the platform adopts them; `orchestrator`'s and `replica`'s copies stay, and `backlog/rfc-3339-times-are-parsed-by-three-copies-of-one-function` is narrowed to them. The proposal slug item is narrowed to `plugin.hooks` and `replica`, `web.worker`'s copy going here.
- The rule for which refs the views read has one provider, `replica` (`refs-the-views-read`), fixed here rather than deferred.
- The open pull requests are read by one paged platform call, `live::pulls_direct`, which the resync uses and `live::pulls` caches.
- `live`'s dead copies and the fake's dead helpers go with the platform's build (`compare_status` and `modes` stay); `orchestrator` and `context` leave `depends_on` by a later small spec change, through one deferred item.
- `shared-layer` is cut and its eight Needs in `web.worker.review` are retargeted to `web.worker.platform/web-ledger-append` or dropped. `old-paths-removed` is kept, Needing the last users, as a named exception to `requirements-state-what-the-contract-shows`, and is a cut to propose once built. `ledger::tip` stays.
- The gone-endpoint list is a disposable cache of the object, re-learned at one push per gone endpoint after a rebuild.
- The uncited tests on the cut list are deleted by the session on small code branches after the human's yes and before the build run, not by a backlog item the build loop never takes.
- `ledger::append` on a failed ref read writes nothing and fails; `percent_decode` keeps `+` in a path segment.
- A test inside a `describe` block whose marker cites a requirement is a CLM defect, filed by the session; inside the wave the builds of `web`, `web.views` and `web.client-live` (branch B) put a marker on each test that proves a requirement.

## Open questions

**Held in a signed repository** (`web.worker.review`), for the human to read: the Worker can see that an approval's line carries a `sig` but can't verify it in wasm, so the web review can show a request as not held where `clm land` holds it, in five cases, each a `sig` that is present but doesn't verify, and never the opposite over the same ledgers and tip. The cases, the differences where the two read different inputs, and why it is safe (the land still verifies) are in `docs/decisions/the-web-review-sees-a-signature-not-its-validity.md`, added by this branch. A verifier for wasm would be `ledger`'s call, in a later change if the difference matters.

Also for the human, each with the session's recommendation:
- **What no run here shows**: Cloudflare answering `ping` with `pong` without waking the object, and sockets surviving the object's eviction (`web.live/hub`; `web.client-live/live-updates` on branch B relies on the first); the replica class's wiring of its three routes and alarm (`web.worker.replica/object-class`); and in `web.worker/live-socket`, a stub or fetch that fails and the copy of a non-101 answer from the runtime's response. Each is stated, and proven through the fakes where one exists. Accepted.
- **Gone push endpoints are a disposable cache of the object, not in git**: no record `ledger` offers today lets the app end another person's subscription. An object rebuilt from git forgets the list and pushes each gone endpoint once more, which re-learns it; this is a cache `git-is-truth` allows. A ledger record for it would be a later `ledger` change. Accepted.
- **The one-time rebuild of each repo's object** after `fetches-binary-assets` is deployed: reads answer 503 for a few alarms, the first recompute sends no push (a question landing then is announced to nobody), and gone endpoints are pushed once more. Accepted.
- **Costs kept, not mended** (`web.worker.review`): an approve reads the person's web ledger twice; the installation token is had before a cached permission; the review's page query stays beside the branch list's, since the list reads at most 300 open pull requests and the page query finds any.
- **Sizes**: `web.worker`'s spec is about 15,960 tokens of its 16,000; `web.worker.review/gate-answer` (10 scenarios) and `web.worker/writes-to-object` (7) may show as `many-scenarios`, each one clause.
