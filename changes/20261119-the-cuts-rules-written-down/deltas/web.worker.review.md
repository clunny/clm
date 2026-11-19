---
component: web.worker.review
base: { design: "50f111d773c81388" }
---

## MODIFIED Design
**Boundaries** (`nested-splits`). `feedback::comment` takes the snapshot's requirement IDs through an argument, `graph: impl Future<Output = Option<BTreeSet<String>>>`: the IDs, or `None` when the snapshot can't be read (a 502). The router builds it from `clm-web`'s `snapshot::get`, and the route awaits it only for a comment with a target, after the branch checks. So this crate imports nothing of `clm-web`'s, and `web.worker` depends on it, not the reverse (`check/undeclared-imports`). The ledger writer (`web_writer`, `record`, `append`), `tip` and `permission`, which `feedback::request_changes` calls, are the platform's (`clm_web_platform::ledger` and `::access`).

The Worker caches each spec branch's review in the Cache API under the default branch's tip and the branch's head (`compare6:<owner>/<repo>@<tip>:<head>`, `review-cached`). A push to the default branch builds every review again, which keeps the lookup free of the compare call keying by merge base would need.

**Spec branches** (`spec-branches`). `GET /api/repos/{o}/{r}/branches` answers the list the app's spec-branch views start from: every branch from `GET /repos/{o}/{r}/git/matching-refs/heads/spec/` that `review::is_spec_branch` accepts, so every listed branch's review can open, sorted by name, each `{branch, head_sha, page}`. `page` is `{number, url, title}` from the open pull request on the default branch whose head is the branch in this repository, not a fork (`isCrossRepository` false), in the live state's one pull-request query, or `null` when there is none; that query's limit of 50 open pull requests is accepted. The list is kept with the live state. It lists the branches as refs, so a spec branch whose page was never opened, or was closed, still shows; it carries no labels and computes no fingerprint or approval per branch, which only the review does.

**Spec branch review.** For a `spec/*` branch touching spec paths, the Worker fetches the spec files at the merge base with `main` and at the head, loads both with `clm_model::load_files`, and compares components element by element with their content hashes and asset hashes. The result is, per component, the added, modified and removed requirements, the Contract, Design and frontmatter changes, and asset changes, each with its text before and after. It adds `change.md` (intent, `from`, `resolves`), whether the branch needs review (`review`, below), decisions added or changed, backlog edits, the branch's fingerprint (`clm_gate::fingerprint_of` over the Trees API listings of the compare's `merge_base_commit` and of the head, the same as `clm fingerprint`, see below), whether the fold holds an `approved` record for it and by whom, and whether a `checked` record names the head's code key. **Needs review** (`pr-review`, `gate/needs-review-of`). The review's top-level `review` is `clm_gate::needs_review_of`, the function `evaluate_at` itself decides by, so the Worker doesn't copy the gate's rule: its `paths` are those whose entries differ between the two Trees API listings the fingerprint already compares (below), its `approval_changed` is the setting comparison the review already makes (below), and its `changes` hold, for each change directory under `changes/` with a file among those paths that the head holds (a live change, or a new archived one), the head's `change.md` frontmatter parsed with `clm_model::parse_change_frontmatter`, `None` when the text is missing or doesn't parse. It is also true when `clm_gate::widened_exemptions` returns anything for the two graphs the review already loads, or `clm_gate::cuts_or_unloaded` does for two graphs loaded for it from only the files the gate loads (`specs/`, the two human layers and the live `change.md` files), since the review's own graphs hold the backlog and the decisions too, whose violations the gate never sees, the call also taking each change's `deleted-tests.txt` at both sides, so the Worker gives the gate's answer for an exemption and for a cut, and the review carries the cut lines as `cuts`. Whether a list's line names a test that exists is the gate's alone to check, since the Worker fetches no code. When either listing is truncated, `review` is true, as the gate fails closed on what it can't read. So a branch that edits `ARCHITECTURE.md`, `CAPABILITIES.md` or `docs/allowed_signers`, or changes the approval setting, shows the chip with no `change.md`. Two edge cases are accepted: a clean archive move of a change with burdens counts here though the gate counts no archive move, and a branch that also changes code is judged as if it were a spec branch; both err towards asking, and `clm land` still runs the gate. Because spec changes are applied on their branch (`apply-in-spec-pr`), the head's `specs/` already shows the result. The browser computes word-level marks from the before and after text. Changed HTML assets are previewed in a sandboxed `iframe` with no script origin access.

**The approval setting** (`approval-setting-branch`, `approval-setting-is-spec`). `clm.toml` is no spec path, but a change to the value of its top-level `approval` key is a spec change that needs review (`gate/approval-setting`), and a branch that makes only that change is a spec branch with no `change.md`. The Worker doesn't copy the gate's rule: whether a branch *changes the approval setting* is `clm_gate::approval_changed` (`gate/approval-changed`), the function the gate itself decides by, over the bytes of `clm.toml` at the merge base and at the head, `None` on a side where the listing has no such entry; an entry the listing has but whose text GitHub withholds (`text` null: a binary blob, or one that isn't UTF-8) counts as changed, as the gate fails closed on a file it can't read. `files::spec_files` already fetches `clm.toml` with the spec files (`snapshot-size`), so the comparison has both texts in hand and asks the function with no further request; `documents` lists the changed files among the model's paths (`files::is_spec_path`), and `clm.toml` only when the function says the setting changed, with both texts, so the review shows what the branch changes and **Open in full** opens it. A changed `.gitattributes`, fetched for sizes alone, is no spec change and isn't listed. The approve route, which lists trees but fetches no spec files, counts the branch as touching spec paths when a changed path satisfies `clm_gate::is_spec_path`, or when the two listings differ on `clm.toml` and the function says the setting changed over the two texts it fetches in one GraphQL query of two expressions (`files::objects`): one subrequest, made only for a branch that changes the file, inside the budget (`web-on-free-plan`). The other settings in `clm.toml`, its comments and its layout stay code to the Worker as to the gate. The review's `review` is true for such a branch, by `needs_review_of` (above); the approval it records is the same `approved` record `clm approve` writes, and `clm land` still runs the gate. The review itself tests no spec path; only the approve route does.

**Fingerprint** (`tree-diff-fingerprint`). The Worker hashes no diff: GitHub's compare diff is made under GitHub's own settings (renames detected, abbreviated blob IDs) and isn't byte for byte what any clone computes. It lists the merge base's tree as it already lists the head's (`files::listing`, `?recursive=1`, each a `keys::Listing`, which also gives the head's `code_key()`), and `keys::fingerprint` passes both listings' entries to `clm_gate::fingerprint_of`, the function `clm fingerprint` uses, so the two can't drift. That costs one more Trees API request per uncached comparison, inside the budget (`web-on-free-plan`). When either listing is `truncated`, the review has no fingerprint, shows no approval and the branch can't be approved from the web; `clm approve` still works.

**Approve** (`approve-pins-the-review`). `POST /api/repos/{o}/{r}/branches/{branch}/approve` takes a JSON body `{"fingerprint": "<f>"}`, the `fingerprint` of the review the person was shown. A fingerprint differs from the one computed now when the branch was pushed (or `main` merged into it) after the review was drawn. Neither a bad body nor a stale fingerprint writes anything, so an approval only ever names a diff the person saw. The append is the user's token's Git Data API sequence: create the blob (the previous `records.jsonl` plus one line), the tree, the commit (parent: the branch's tip, or none on first use) and update or create the ref. That is four writes to the repository and nothing else; the record carries the branch's fingerprint, branch and change ID, and its `writer` is `<login>/web`, so the person is named by their GitHub login. `clm land` then finds the record as it finds any other.

**Signed repositories** (`signed-repository`, `web-approve-off-when-signed`). A repository whose default branch's tip holds `docs/allowed_signers` is signed (`signed-ledger-records`): `clm land` counts only an `approved` record whose `sig` verifies against that file, and the Worker, which holds no one's SSH key, can't write one. Neither the review nor the approve route lists the tip's tree (the comparison lists the merge base and the head), so whether the tip is signed is one GraphQL query, `Signers`: `object(expression: "<tip>:docs/allowed_signers") { __typename }`, signed exactly when the object isn't null, whatever its type, as `clm_ledger::signed` counts any tree entry at the path. A commit never changes, so the answer is kept in the Cache API by tip for a day (`signed:<owner>/<repo>@<tip>`), and the review and the approve route share it; the tip itself is read fresh on every request. It sits outside the comparison. A failed query, or a null `repository`, answers 502, never `signed` false. An approval's `signed` is whether the record's line in the fold (`Fold::lines`) has a `sig` that is a non-empty string. The Worker doesn't verify signatures, so a signed approval is one `clm land` will check, not one known to count; an unsigned one in a signed repository never counts. The approve route checks, in order: the body (before any request to GitHub), the branch's name, that it exists and touches spec paths, each an empty 422; then whether the tip is signed, a 422 with the JSON body `{"error": "signed", "command": "clm approve <branch>"}`; then a truncated listing's missing fingerprint, an empty 422; then the fingerprint comparison, 409. It writes nothing on any of them. The activity feed lists every `approved` record. Verifying signatures in wasm is declined (`web-approve-off-when-signed`).

**Comments and change requests** (`web-review-feedback-as-ledger-records`, `review-feedback`). `held` is `current` and not cleared, as `ledger`'s `Fold::cleared` decides: the one rule `clm land` holds by (`ledger/request-cleared`). A comment's `target` is null, for the change as a whole, or a requirement ID `<component>/<slug>` that the branch's review lists (added, modified or removed, so a removed requirement's card can be commented on) or that the snapshot's graph holds, both of which are cached, so no spec file is loaded for a comment. `text` is stored trimmed. The body is checked first, before any request to GitHub, as Approve's is, and a 422 `{"error": <field>}` names `fingerprint`, `target`, `text` or `branch`. Both routes append with the four writes Approve makes and answer 200 with `{"id"}`. A comment doesn't compare its fingerprint with the branch's: a comment on an earlier version still reads, and the record says which version it was on. `POST .../branches/{branch}/request-changes` takes `{"fingerprint", "text"}`, `text` checked and stored as a comment's, and is otherwise checked as Approve is, the signed repository excepted, and by two more refusals, neither of which writes: 403 with `{"error": "permission"}` unless GitHub answers `write` or `admin` for the requester's `permission` on the repo, asked as the webhook asks for push recipients (`GET /repos/{o}/{r}/collaborators/{login}/permission` with the installation's token, kept 5 minutes per repo and login), since a hold on a land is for those who could land; and 409 with `{"error": "moved"}` when the fingerprint isn't the branch's now, so a request names only a diff the person saw. A signed repository doesn't refuse it: a request withholds a land rather than granting one, so `clm land` counts it unsigned (`land`). Comments and proposals need read access only, the access check every API route makes, which the request route makes too. The activity feed lists neither kind.

## MODIFIED Requirements

### Requirement: pr-review <!-- base: 574ed19f1e28b258 -->
For a `spec/*` branch, the Worker SHALL return, per touched component, every added, modified and removed requirement, every changed Contract, Design and frontmatter, and every added, modified and removed asset, each with its content before and after, together with the change's intent, its `## Intent` section as a separate field, decisions and backlog edits, the branch's fingerprint, the `approved` records the ledger holds for it with their writers, whether a `checked` record names the head's code key, the pull request's URL when the host has one, and a top-level `review`: whether the branch needs a human's review, by `clm_gate::needs_review_of` (`gate/needs-review-of`) over the paths whose entries differ between the Trees API listings of the merge base and of the head, whether the approval setting changed (`approval-setting-branch`), and the head's `change.md` frontmatter of each change one of whose files is among those paths, `None` for one that is missing or doesn't parse; `review` SHALL also be true when `clm_gate::widened_exemptions` or `clm_gate::cuts_or_unloaded` returns anything, the first for the two graphs the review compares and the second, called only when a changed path is under `specs/` or is a `deleted-tests.txt`, for graphs it loads for that call from the files `gate/cuts` names and no other, each with the violations that loading gave and its side's commit ID, and the texts of each change's `deleted-tests.txt` among the files it fetches (`gate/needs-review-of`, `gate/cuts`), and the review SHALL carry a top-level `cuts`, the lines `clm gate` prints for those cuts, in its order, empty when there is none; so a side whose specs don't load gives the gate's one line for it and `review` true. `review` SHALL be true when a path that differs between the two listings is a `deleted-tests.txt` whose text GitHub withholds at the head, and when either listing is truncated `cuts` SHALL be empty, whatever else holds, since they would be read from part of the files. `review` SHALL be true when either listing is truncated. The change SHALL carry no `review` of its own. A comparison cached before this requirement SHALL NOT be served: the cache key is named anew.

#### Scenario: modified requirement
- GIVEN a spec branch that modifies requirement `n/r`, adds asset `assets/m.html` to `n` and adds decision `d`
- WHEN its review is requested
- THEN the response lists `n/r` as modified with both texts, `assets/m.html` as added, and `d` as added

#### Scenario: approval state
- GIVEN a spec branch and a ledger with an `approved` record for its fingerprint by writer `clunny/web`
- WHEN its review is requested
- THEN the response says it is approved by `clunny/web`

#### Scenario: a removed requirement
- GIVEN a spec branch that removes requirement `n/r` and changes nothing else
- WHEN its review is requested
- THEN the response's `review` is true and its `cuts` hold `specs/n/SPEC.md: gate/cut: requirement n/r is gone`

#### Scenario: specs that don't load
- GIVEN a spec branch whose head's `specs/n/SPEC.md` holds a requirement with no scenario
- WHEN its review is requested
- THEN the response's `review` is true and its `cuts` hold only `specs: gate/cut: the specs at <head> don't load, so what the branch cuts isn't listed`, `<head>` the head's commit ID

#### Scenario: a broken backlog item
- GIVEN a spec branch that adds a requirement to `specs/n/SPEC.md` and whose head holds a `backlog/` item the model rejects
- WHEN its review is requested
- THEN its `cuts` are empty, as `clm gate` lists none for it

#### Scenario: a list GitHub withholds
- GIVEN a spec branch that adds `changes/c/deleted-tests.txt`, whose text GitHub gives as null
- WHEN its review is requested
- THEN the response's `review` is true

#### Scenario: a truncated listing
- GIVEN a spec branch that removes requirement `n/r`, and a head listing GitHub truncates
- WHEN its review is requested
- THEN the response's `review` is true and its `cuts` are empty

#### Scenario: a widened exemption
- GIVEN a spec branch whose only change adds an exemption to `specs/n/SPEC.md`'s frontmatter
- WHEN its review is requested
- THEN the response's `review` is true and its `cuts` are empty

#### Scenario: a human layer without a change
- GIVEN a spec branch that only edits `ARCHITECTURE.md`
- WHEN its review is requested
- THEN the response's `review` is true and it has no `change`

#### Scenario: a change with burdens
- GIVEN a spec branch that adds `changes/x/change.md` with `human_burdens: [b]`, and one that adds `changes/y/change.md` with no `human_burdens` and edits `docs/decisions/d.md`
- WHEN each review is requested
- THEN the first's `review` is true, the second's is false, and neither `change` has a `review` field

#### Scenario: truncated listing
- GIVEN a spec branch whose head listing GitHub truncates
- WHEN its review is requested
- THEN its `review` is true and it carries no fingerprint

#### Serves
- capability/approve-intent
- architecture/humans-review-intent

#### Needs
- gate/cuts
- web.worker.platform/shared-layer
- gate/needs-review-of
