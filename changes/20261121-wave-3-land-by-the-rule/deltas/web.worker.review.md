---
component: web.worker.review
base: { design: "8450111da729199b" }
---

## MODIFIED Design
**Boundaries** (`nested-splits`). `feedback::comment` takes the snapshot's requirement IDs through an argument, `graph: impl Future<Output = Option<BTreeSet<String>>>`: the IDs, or `None` when the snapshot can't be read (a 502). The router builds it from `clm-web`'s `snapshot::get`, and the route awaits it only for a comment with a target, after the branch checks. So this crate imports nothing of `clm-web`'s, and `web.worker` depends on it, not the reverse (`check/undeclared-imports`). The ledger writer (`web_writer`, `record`, `append`), `tip` and `permission`, which `feedback::request_changes` calls, are the platform's (`clm_web_platform::ledger` and `::access`).

The Worker caches each spec branch's review in the Cache API under the default branch's tip and the branch's head (`compare7:<owner>/<repo>@<tip>:<head>`, `review-cached`). A push to the default branch builds every review again, which keeps the lookup free of the compare call keying by merge base would need.

**Spec branches** (`spec-branches`). `GET /api/repos/{o}/{r}/branches` answers the list the app's spec-branch views start from: every branch from `GET /repos/{o}/{r}/git/matching-refs/heads/spec/` that `review::is_spec_branch` accepts, so every listed branch's review can open, sorted by name, each `{branch, head_sha, page}`. `page` is `{number, url, title}` from the open pull request on the default branch whose head is the branch in this repository, not a fork (`isCrossRepository` false), in the live state's one pull-request query, or `null` when there is none; that query's limit of 50 open pull requests is accepted. The list is kept with the live state. It lists the branches as refs, so a spec branch whose page was never opened, or was closed, still shows; it carries no labels and computes no fingerprint or approval per branch, which only the review does.

**Spec branch review.** For a `spec/*` branch touching spec paths, the Worker fetches the spec files at the merge base with `main` and at the head, loads both with `clm_model::load_files`, and compares components element by element with their content hashes and asset hashes. The result is, per component, the added, modified and removed requirements, the Contract, Design and frontmatter changes, and asset changes, each with its text before and after. It adds `change.md` (intent, `from`, `resolves`), whether the branch needs review (`review`, below), decisions added or changed, backlog edits, the branch's fingerprint (`clm_gate::fingerprint_of` over the Trees API listings of the compare's `merge_base_commit` and of the head, the same as `clm fingerprint`, see below), whether the fold holds an `approved` record for it and by whom, and whether a `checked` record names the head's code key. **Needs review** (`pr-review`, `gate/needs-review-of`). The review's top-level `review` is `clm_gate::needs_review_of`, the function `evaluate_at` itself decides by, so the Worker doesn't copy the gate's rule: its `paths` are those whose entries differ between the two Trees API listings the fingerprint already compares (below), its `approval_changed` is the setting comparison the review already makes (below), and its `changes` hold, for each change directory under `changes/` with a file among those paths that the head holds (a live change, or a new archived one), the head's `change.md` frontmatter parsed with `clm_model::parse_change_frontmatter`, `None` when the text is missing or doesn't parse. It is also true when `clm_gate::widened_exemptions` returns anything for the two graphs the review already loads, or `clm_gate::cuts_or_unloaded` does for two graphs loaded for it from only the files the gate loads (`specs/`, the two human layers and the live `change.md` files), since the review's own graphs hold the backlog and the decisions too, whose violations the gate never sees, the call also taking each change's `deleted-tests.txt` at both sides, so the Worker gives the gate's answer for an exemption and for a cut, and the review carries the cut lines as `cuts`. Whether a list's line names a test that exists is the gate's alone to check, since the Worker fetches no code. When either listing is truncated, `review` is true, as the gate fails closed on what it can't read. So a branch that edits `ARCHITECTURE.md`, `CAPABILITIES.md` or `docs/allowed_signers`, or changes the approval setting, shows the chip with no `change.md`. Two edge cases are accepted: a clean archive move of a change with burdens counts here though the gate counts no archive move, and a branch that also changes code is judged as if it were a spec branch; both err towards asking, and `clm land` still runs the gate. Because spec changes are applied on their branch (`apply-in-spec-pr`), the head's `specs/` already shows the result. The browser computes word-level marks from the before and after text. Changed HTML assets are previewed in a sandboxed `iframe` with no script origin access.

**The approval setting** (`approval-setting-branch`, `approval-setting-is-spec`). `clm.toml` is no spec path, but a change to the value of its top-level `approval` key is a spec change that needs review (`gate/approval-setting`), and a branch that makes only that change is a spec branch with no `change.md`. The Worker doesn't copy the gate's rule: whether a branch *changes the approval setting* is `clm_gate::approval_changed` (`gate/approval-changed`), the function the gate itself decides by, over the bytes of `clm.toml` at the merge base and at the head, `None` on a side where the listing has no such entry; an entry the listing has but whose text GitHub withholds (`text` null: a binary blob, or one that isn't UTF-8) counts as changed, as the gate fails closed on a file it can't read. `files::spec_files` already fetches `clm.toml` with the spec files (`snapshot-size`), so the comparison has both texts in hand and asks the function with no further request; `documents` lists the changed files among the model's paths (`files::is_spec_path`), and `clm.toml` only when the function says the setting changed, with both texts, so the review shows what the branch changes and **Open in full** opens it. A changed `.gitattributes`, fetched for sizes alone, is no spec change and isn't listed. The approve route, which lists trees but fetches no spec files, counts the branch as touching spec paths when a changed path satisfies `clm_gate::is_spec_path`, or when the two listings differ on `clm.toml` and the function says the setting changed over the two texts it fetches in one GraphQL query of two expressions (`files::objects`): one subrequest, made only for a branch that changes the file, inside the budget (`web-on-free-plan`). The other settings in `clm.toml`, its comments and its layout stay code to the Worker as to the gate. The review's `review` is true for such a branch, by `needs_review_of` (above); the approval it records is the same `approved` record `clm approve` writes, and `clm land` still runs the gate. The review itself tests no spec path; only the approve route does.

**Fingerprint** (`tree-diff-fingerprint`). The Worker hashes no diff: GitHub's compare diff is made under GitHub's own settings (renames detected, abbreviated blob IDs) and isn't byte for byte what any clone computes. It lists the merge base's tree as it already lists the head's (`files::listing`, `?recursive=1`, each a `clm_web_platform::keys::Listing`, which also gives the head's `code_key()`), and `clm_web_platform::keys::fingerprint` passes both listings' entries to `clm_gate::fingerprint_of`, the function `clm fingerprint` uses, so the two can't drift. They hash the same entries because the Trees API lists the whole repository and `clm fingerprint` lists the CLM root's tree (`fingerprint-covers-the-clm-root`), which are one tree in every repository the Worker lists, since it lists only those whose default branch holds `specs/SPEC.md` at its top (`web.worker`). A repository it doesn't list can still be reviewed by its URL, since the access check doesn't filter by the list; in one whose CLM root is a subdirectory they differ, and the Worker, which never learns that root, strips nothing: an approval its approve route writes there names a fingerprint `clm land` never computes, so it counts for nothing and the land fails closed. That costs one more Trees API request per uncached comparison, inside the budget (`web-on-free-plan`). When either listing is `truncated`, the review has no fingerprint, shows no approval and the branch can't be approved from the web; `clm approve` still works.

**Approve** (`approve-pins-the-review`). `POST /api/repos/{o}/{r}/branches/{branch}/approve` takes a JSON body `{"fingerprint": "<f>"}`, the `fingerprint` of the review the person was shown. A fingerprint differs from the one computed now when the branch was pushed (or `main` merged into it) after the review was drawn. Neither a bad body nor a stale fingerprint writes anything, so an approval only ever names a diff the person saw. The append is the user's token's Git Data API sequence: create the blob (the previous `records.jsonl` plus one line), the tree, the commit (parent: the branch's tip, or none on first use) and update or create the ref. That is four writes to the repository and nothing else; the record carries the branch's fingerprint, branch and change ID, and the requests it clears (`ledger/request-cleared`), and its `writer` is `<login>/web`, so the person is named by their GitHub login. The requests are read, after the fingerprint comparison, from the requester's own web ledger at its tip (`clm_web_platform::ledger::own`), not the minute-old fold: only the web app writes a `changes_requested` record, each to its writer's own web ledger, so that ledger holds every request of the person's, and one made a moment before the approve is among them. That read is the ref and one GraphQL query, two more subrequests for an approve. `clm land` then finds the record as it finds any other.

**Signed repositories** (`signed-repository`, `web-approve-off-when-signed`). A repository whose default branch's tip holds `docs/allowed_signers` is signed (`signed-ledger-records`): `clm land` counts only an `approved` record whose `sig` verifies against that file, and the Worker, which holds no one's SSH key, can't write one. Neither the review nor the approve route lists the tip's tree (the comparison lists the merge base and the head), so whether the tip is signed is one GraphQL query, `Signers`: `object(expression: "<tip>:docs/allowed_signers") { __typename }`, signed exactly when the object isn't null, whatever its type, as `clm_ledger::signed` counts any tree entry at the path. A commit never changes, so the answer is kept in the Cache API by tip for a day (`signed:<owner>/<repo>@<tip>`), and the review and the approve route share it; the tip itself is read fresh on every request. It sits outside the comparison. A failed query, or a null `repository`, answers 502, never `signed` false. An approval's `signed` is whether the record's line in the fold (`Fold::lines`) has a `sig` that is a non-empty string. The Worker doesn't verify signatures, so a signed approval is one `clm land` will check, not one known to count; an unsigned one in a signed repository never counts. The approve route checks, in order: the body (before any request to GitHub), the branch's name, that it exists and touches spec paths, each an empty 422; then whether the tip is signed, a 422 with the JSON body `{"error": "signed", "command": "clm approve <branch>"}`; then a truncated listing's missing fingerprint, an empty 422; then the fingerprint comparison, 409. It writes nothing on any of them. The activity feed lists every `approved` record. Verifying signatures in wasm is declined (`web-approve-off-when-signed`).

**Comments and change requests** (`web-review-feedback-as-ledger-records`, `review-feedback`). `held` is `current` and not cleared, as `ledger`'s `Fold::cleared` decides: the one rule `clm land` holds by (`ledger/request-cleared`). A comment's `target` is null, for the change as a whole, or a requirement ID `<component>/<slug>` that the branch's review lists (added, modified or removed, so a removed requirement's card can be commented on) or that the snapshot's graph holds, both of which are cached, so no spec file is loaded for a comment. `text` is stored trimmed. The body is checked first, before any request to GitHub, as Approve's is, and a 422 `{"error": <field>}` names `fingerprint`, `target`, `text` or `branch`. Both routes append with the four writes Approve makes and answer 200 with `{"id"}`. A comment doesn't compare its fingerprint with the branch's: a comment on an earlier version still reads, and the record says which version it was on. `POST .../branches/{branch}/request-changes` takes `{"fingerprint", "text"}`, `text` checked and stored as a comment's, and is otherwise checked as Approve is, the signed repository excepted, and by two more refusals, neither of which writes: 403 with `{"error": "permission"}` unless GitHub answers `write` or `admin` for the requester's `permission` on the repo, asked as the webhook asks for push recipients (`GET /repos/{o}/{r}/collaborators/{login}/permission` with the installation's token, kept 5 minutes per repo and login), since a hold on a land is for those who could land; and 409 with `{"error": "moved"}` when the fingerprint isn't the branch's now, so a request names only a diff the person saw. A signed repository doesn't refuse it: a request withholds a land rather than granting one, so `clm land` counts it unsigned (`land`). Comments and proposals need read access only, the access check every API route makes, which the request route makes too. The activity feed lists neither kind.

## MODIFIED Requirements

### Requirement: approve <!-- base: 4584ce5332f46c69 -->
The Worker SHALL approve a `spec/*` branch that touches spec paths, or that changes the approval setting (`approval-setting-branch`), by appending an `approved` record with the branch's fingerprint to `clm/ledger/<login>/web` through the Git Data API with the requester's own token, making no other write to the repository, and only when the request, on the branch route, names what the person reviewed: a `fingerprint` equal to the branch's fingerprint now. It SHALL answer 409 and write nothing when the named fingerprint no longer matches, and SHALL answer 422 and write nothing for a request whose body doesn't name a `fingerprint` (checked before any request to GitHub), and for a branch that doesn't exist, isn't `spec/*`, neither touches a spec path nor changes the approval setting, or has no fingerprint because a tree listing was truncated, and for a repository whose default branch's tip holds `docs/allowed_signers` (`signed-repository`). The 422 cases are checked before the 409 one. The record SHALL name, as the requests it clears (`ledger/request-cleared`), every `changes_requested` record for that branch at that fingerprint whose writer's person is the requester's login that the requester's own web ledger holds when read at its tip after the fingerprint comparison, and none, the record still naming an empty list, when it holds none; when that ledger can't be read the Worker SHALL answer 502 and write nothing.

#### Scenario: approve
- GIVEN a spec branch `spec/x` whose fingerprint is `f`
- WHEN the user approves it with `{"fingerprint": "f"}`
- THEN the writes made are exactly a blob, a tree, a commit and a ref update on `clm/ledger/<login>/web`, with the user's token, and the new `records.jsonl` ends with an `approved` line for `f` with writer `<login>/web`

#### Scenario: pushed after the review
- GIVEN the review of `spec/x` was drawn when its fingerprint was `f`, and the author then pushed a commit that changes a spec file, so its fingerprint is now `g`
- WHEN the user approves it with `{"fingerprint": "f"}`
- THEN the Worker answers 409 and makes no write

#### Scenario: nothing named
- GIVEN a spec branch `spec/x`
- WHEN the user approves it with no body
- THEN the Worker answers 422 and makes no write

#### Scenario: code branch
- GIVEN a branch `impl/a` that touches only `crates/`
- WHEN the user tries to approve it
- THEN the Worker answers 422 and makes no write

#### Scenario: signed and pushed after the review
- GIVEN a default branch whose tip holds `docs/allowed_signers`, and the review of `spec/x` drawn when its fingerprint was `f`, which is now `g`
- WHEN the user approves it with `{"fingerprint": "f"}`
- THEN the Worker answers 422, not 409, and makes no write

#### Scenario: approve over a request
- GIVEN `spec/x` at fingerprint `f`, and the user `alice`'s web ledger holding `changes_requested` records `r1` for `spec/x` at `f`, `r3` for `spec/x` at `g` and `r4` for `spec/y` at `f`, and `dana`'s web ledger holding `r2` for `spec/x` at `f`
- WHEN `alice` approves `spec/x` with `{"fingerprint": "f"}`; then `bob`, who has no web ledger, does the same; then `alice` again while GitHub fails the read of her web ledger
- THEN `alice`'s new `approved` line's `clears` holds `r1` alone, `bob`'s is an empty list, and the last answers 502 and makes no write

#### Serves
- capability/approve-intent
- architecture/approval-policy
- architecture/humans-review-intent
- architecture/git-is-truth

#### Needs
- web.worker.platform/shared-layer
- gate/approval-changed
- ledger/request-cleared

### Requirement: review-feedback <!-- base: 3d004f4e354a6113 -->
A spec branch's review SHALL carry `comments`, every `commented` record in the fold for the branch, oldest first, each with `id`, `writer`, `at`, `fingerprint`, `target`, `text` and `current`, whether its fingerprint is the review's, and `change_requests`, every `changes_requested` record for the branch the same way, each with `held` as well: `current`, and `ledger/request-cleared` not counting it as cleared, deciding it, and taking a writer's person apart anywhere, through `ledger`'s `cleared` and `person`, with no copy of either in `clm-web`. `POST /api/repos/{o}/{r}/branches/{branch}/comments` with `{"fingerprint", "target", "text"}` SHALL append a `commented` record with those three to `clm/ledger/<login>/web` through the Git Data API with the requester's own token, making no other write, without comparing the fingerprint with the branch's, and SHALL answer 422 with `{"error": <field>}`, writing nothing, for a `target` that is neither null nor a requirement ID the branch's review lists or the snapshot's graph holds, a text that is empty or over 4,000 characters once trimmed, a missing or empty string `fingerprint`, or a branch that isn't `spec/*`, doesn't exist or touches no spec path. `POST .../branches/{branch}/request-changes` with `{"fingerprint", "text"}` SHALL append a `changes_requested` record the same way only when `fingerprint` is the branch's fingerprint now and GitHub answers `write` or `admin` for the requester's permission on the repo, answering 409 and writing nothing when the fingerprint differs, 403 and writing nothing for any other permission, and 422 for a text a comment would be refused for and for the cases the approve route answers 422, except a signed repository, which it SHALL accept. When the review has no fingerprint, every entry's `current` and `held` SHALL be false.

#### Scenario: comment on a requirement
- GIVEN `spec/x`, whose head holds requirement `n/r`, with fingerprint `f`
- WHEN the user posts `{"fingerprint": "f", "target": "n/r", "text": "Name the rule"}` to its comments, and the review is requested
- THEN the writes were a blob, a tree, a commit and a ref update on `clm/ledger/<login>/web`, the new `records.jsonl` ends with a `commented` line for `spec/x` at `f` with target `n/r`, and the review's `comments` lists it with `current` true

#### Scenario: unknown target
- GIVEN the same branch
- WHEN the user posts a comment with target `n/missing`, then one with target `null` and an empty text, then one with target `m/old`, a requirement the review lists as removed, and the text "Keep it"
- THEN the first two answer 422 and nothing is written for them, and the third appends a `commented` line with target `m/old`

#### Scenario: changes requested
- GIVEN `spec/x` with fingerprint `f` in a repository whose default branch's tip holds `docs/allowed_signers`, and GitHub answering `write` for the user's permission
- WHEN the user posts `{"fingerprint": "f", "text": "Split it"}` to its request-changes, and the review is requested
- THEN the new `records.jsonl` ends with a `changes_requested` line for `spec/x` at `f`, and the review's `change_requests` lists it with `current` true and `held` true

#### Scenario: a reader can't hold a land
- GIVEN the same branch, and GitHub answering `read` for the user's permission
- WHEN the user posts the same request
- THEN the Worker answers 403 and makes no write

#### Scenario: moved
- GIVEN the review of `spec/x` was drawn when its fingerprint was `f`, which is now `g`
- WHEN the user posts `{"fingerprint": "f", "text": "Split it"}` to its request-changes
- THEN the Worker answers 409 and makes no write

#### Scenario: cleared
- GIVEN a `changes_requested` record for `spec/x` at `f` by `alice/web`, and an `approved` record for `f` by `alice/1a2b3c4d` that names the request as one it clears
- WHEN `spec/x`'s review is requested while its fingerprint is `f`
- THEN its `change_requests` lists the request with `current` true and `held` false

#### Serves
- capability/approve-intent
- capability/follow-from-browser

#### Needs
- web.worker.platform/shared-layer
- ledger/feedback-records
- ledger/request-cleared
