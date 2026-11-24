---
component: web.worker.review
base: { contract: "84b7970b7408818c", design: "bfb4bf3ab6bca144" }
---

## MODIFIED Contract
Crate `clm-web-review`: a spec branch's review and what is written on one, for `web.worker`'s router, which makes the session and access checks and passes the `Access` they give (`clm_web_platform::access`). Built for `wasm32-unknown-unknown` and natively for tests, with no `worker` dependency. It offers:
- `branches::list(p, access)`: the spec branches, each a `SpecBranch { branch, head_sha, page }`, `page` a `Page { number, url, title }` or none, and none in place of the list when GitHub doesn't answer (`spec-branches`); the router answers it on `GET .../branches`;
- `review::branch(p, access, branch)`: a spec branch's review (`GET .../branches/{branch}/review`), as a `clm_web_platform::http::Response`;
- `approve::branch(p, access, login, branch, pin)`: approve (`POST .../branches/{branch}/approve`), as a `Response`;
- `feedback::comment(p, access, login, branch, comment, graph)` and `feedback::request_changes(p, access, login, branch, request)` (`POST .../branches/{branch}/comments` and `.../request-changes`), each as a `Response`; `graph` is the snapshot's requirement IDs, a future of the IDs or of none when the snapshot can't be read;
- the body parsers the router runs before any request to GitHub: `approve::fingerprint(body)`, the fingerprint an approve's body names, or none; and `feedback::parse_comment(body)` and `feedback::parse_request(body)`, a checked `Comment` or `Request`, or the 422 `Response` that refuses the body (`review-feedback`, `request-changes`).

Nothing else of the crate is offered. The routes, the response shapes and the answer to an approve whose body names no fingerprint are `web.worker`'s contract. It imports nothing of `clm-web`.

## MODIFIED Design
**Boundaries** (`nested-splits`). `feedback::comment` takes the snapshot's requirement IDs as a future the router builds from `clm-web`'s snapshot view, awaited only for a comment with a target and after the branch checks, so this crate imports nothing of `clm-web`'s and `web.worker` depends on it, not the reverse (`check/undeclared-imports`). The ledger reads and the append (`ledger::fold`, `own`, `branch_tip`, `record`, `append`), the permission lookup and the installation token are the platform's (`clm_web_platform::ledger`, `::access`, `::install`). What a branch's paths mean is the gate's (`clm_gate::is_spec_path`, `approval_changed`, `review_from_files`), and what a record says is `ledger`'s (`cleared`, `cleared_by_sig`, `carries_sig`, `change_ids`, `person`): this crate holds no copy of their rules.

**The comparison is cached** (`review-cached`). The Worker keeps each spec branch's comparison in the Cache API under the default branch's tip and the branch's head (`compare8:<owner>/<repo>@<tip>:<head>`, a day, since a commit never changes). A push to the default branch builds every review again, which keeps the lookup free of the compare call keying by merge base would need. The key's number moves whenever the comparison changes what it holds or how it is decided, so no comparison an earlier build kept is served (`gate-answer`): `compare8` since `documents` hold every spec path and `review` and `cuts` come from the gate's one call. The ledger's approvals, comments and requests, `checked`, `signed` and the page are read live on each request.

**Spec branches** (`spec-branches`). The list starts from `GET /repos/{o}/{r}/git/matching-refs/heads/spec/`, keeps the names the review accepts, so every listed branch's review opens, and takes each `page` from the live state's pull-request listing (`live::pulls`, `web.worker.platform/open-pulls`), which reads at most the first 300 open pull requests GitHub lists, a limit accepted. It is kept with the live state. It lists the branches as refs, so a spec branch whose page was never opened, or was closed, still shows; it carries no labels and computes no fingerprint or approval per branch, which only the review does.

**Spec branch review** (`pr-review`). The Worker asks the compare API for the merge base of the default branch's tip and the branch's head, lists both commits' trees (`files::listing`, `?recursive=1`), and fetches each side's files the replica and the gate read (`files::spec_files`, `web.worker.platform/spec-files`). It loads both sides with `clm_model::load_files` and compares components element by element by content hash and asset hash. `documents` come from the two listings, not from the files fetched, so a changed spec path GitHub gives no text for (a binary file under `docs/`) is still listed, with an empty text, and the person sees every path an approve would accept. A changed `.gitattributes`, fetched for sizes alone, is no spec path and isn't listed. The page is found by its own REST query (`head=<owner>:<branch>`, so a fork's branch of the same name is left out), kept with the live state; the list's page and the review's are one rule done two ways, the review's costing one subrequest and finding the page whatever the number of open pull requests. Because spec changes are applied on their branch (`apply-in-spec-pr`), the head's `specs/` already shows the result. The browser computes word-level marks from the before and after text, and previews a changed HTML asset in a sandboxed `iframe` with no script origin access.

**What the gate would say** (`gate-answer`, `gate/review-from-files`). `review` and `cuts` are what `clm_gate::review_from_files` gives over the paths whose entries differ between the two listings, each with its status from the two entries as `git diff --no-renames --name-status` would give it, no archive moves (two listings can't tell one), whether the approval setting changed, and each side's commit, listed paths and fetched files, the call `evaluate_at` itself decides by, so the Worker reads no file set and no list text of its own. A path the listing holds whose text GitHub withholds (`text` null: a binary blob, or one that isn't UTF-8) is one the call can't read: a file the model reads is a load violation of that side, and any other (a binary file under `specs/` outside `assets/`) is ignored as `clm gate` ignores it, and a `deleted-tests.txt` is named back as withheld, which the review prints as its own `gate/cut` line. Whether a list's line names a test that exists is the gate's alone to check, since the Worker fetches no code. When either listing is truncated nothing is read: `review` is true and `cuts` empty, as the gate fails closed on what it can't read. The statuses come from the listings, not the compare API's `files`, which detects renames and lists at most 300 files. The Worker can't run git or replay a rename, and passes every changed path and no archive move, so where the gate leaves a path out by reading more than the trees (`rename/replay`, `archive-move`, a branch that also changes code, which the gate calls `mixed`) the review asks a human's review the gate wouldn't: erring towards asking is safe, and `clm land` still runs the gate.

**The approval setting** (`approval-setting-branch`, `approval-setting-is-spec`). `clm.toml` is no spec path, but a change to the value of its top-level `approval` key is a spec change that needs review (`gate/approval-setting`), and a branch that makes only that change is a spec branch with no `change.md`. Whether a branch changes the setting is `clm_gate::approval_changed` over the file's bytes at the merge base and at the head, `None` on a side whose listing has no such entry; an entry whose text GitHub withholds counts as changed, as the gate fails closed on a file it can't read. `files::spec_files` already fetches `clm.toml`, so the review has both texts in hand and asks the function with no further request. The approve route, which lists trees but fetches no spec files, fetches the two texts in one GraphQL query of two expressions (`files::objects`), and only for a branch whose listings differ on the file: one subrequest inside the budget (`web-on-free-plan`). The other settings in `clm.toml`, its comments and its layout stay code to the Worker as to the gate.

**Fingerprint** (`fingerprint-from-trees`, `tree-diff-fingerprint`). The Worker hashes no diff: GitHub's compare diff is made under GitHub's own settings (renames detected, abbreviated blob IDs) and isn't byte for byte what any clone computes. It lists the merge base's tree as it lists the head's, and `keys::fingerprint` gives the fingerprint of the two listings as `clm fingerprint` would (`web.worker.platform/fingerprint-from-listings`). They hash the same entries because the Trees API lists the whole repository and `clm fingerprint` lists the CLM root's tree (`fingerprint-covers-the-clm-root`), which are one tree in every repository the Worker lists, since it lists only those whose default branch holds `specs/SPEC.md` at its top (`web.worker`). A repository it doesn't list can still be reviewed by its URL; in one whose CLM root is a subdirectory the two differ, and an approval the Worker writes there names a fingerprint `clm land` never computes, so it counts for nothing and the land fails closed. The merge base's listing costs one Trees API request per uncached comparison, inside the budget. `clm approve` still works where a listing is truncated.

**Approve** (`approve`, `approve-pins-the-review`). The body names the `fingerprint` of the review the person was shown; a fingerprint differs from the one computed now when the branch was pushed (or `main` merged into it) after the review was drawn, so an approval only ever names a diff the person saw. The append is the platform's four Git Data API writes (`web.worker.platform/web-ledger-append`), with `writer` `<login>/web`, so the person is named by their GitHub login. The record's `change` is named by `ledger`'s `change_ids`, as `clm approve` names it, over the paths the head holds that the merge base lacks or differs on. The requests it clears are read, after the fingerprint comparison, from the requester's own web ledger at its tip (`ledger::own`), not the minute-old fold: only the web app writes a `changes_requested` record, each to its writer's own web ledger, so that ledger holds every request of the person's, and one made a moment before the approve is among them. `ledger::own` and then `ledger::append` read that ledger's ref and file twice, two subrequests more than one read would cost; the answer can't differ, since the append moves the ref with `force` false. `clm land` then finds the record as it finds any other.

**Signed repositories** (`signed-repository`, `web-approve-off-when-signed`). A repository whose default branch's tip holds `docs/allowed_signers` is signed (`signed-ledger-records`): `clm land` counts only an `approved` record whose `sig` verifies against that file, and the Worker, which holds no one's SSH key and can't run `ssh-keygen`, can neither write one nor check one. Neither the review nor the approve route lists the tip's tree, so whether the tip is signed is one GraphQL query, `Signers`: `object(expression: "<tip>:docs/allowed_signers") { __typename }`, signed exactly when the object isn't null, whatever its type, as `clm_ledger::signed` counts any tree entry at the path. A commit never changes, so the answer is kept in the Cache API by tip for a day (`signed:<owner>/<repo>@<tip>`), and the review and the approve route share it; the tip itself is read fresh on every request. Since the Worker verifies no signature, it counts a line that carries a non-empty `sig` (`Fold::carries_sig`) as one `clm land` will check: an approval's `signed`, and the approvals that clear a request in a signed repository (`Fold::cleared_by_sig`). An unsigned approval in a signed repository counts for nothing, here as at the land. Over the same ledgers and the same tip, a `sig` that doesn't verify is the one case the review shows otherwise than the land decides, and then only as not held where the land holds; the review's fold is up to a minute old and the land's is its clone's, so a newer approval, one not yet pushed, or a tip that gained `docs/allowed_signers` since can make them differ either way. Verifying signatures in wasm is declined (`web-approve-off-when-signed`).

**Comments and change requests** (`web-review-feedback-as-ledger-records`, `review-feedback`, `request-changes`). A comment's `target` is null, for the change as a whole, or a requirement ID that the branch's review lists (added, modified or removed, so a removed requirement's card can be commented on) or that the snapshot's graph holds; both are cached, so no spec file is loaded for a comment, and the graph is asked first. A comment doesn't compare its fingerprint with the branch's: a comment on an earlier version still reads, and the record says which version it was on. A change request asks the requester's permission as the webhook asks for push recipients (`GET /repos/{o}/{r}/collaborators/{login}/permission` with the installation's token, kept 5 minutes per repo and login, `web.worker.platform/permission`), since a hold on a land is for those who could land; it is asked before the fingerprint is compared, so a reader learns nothing of the branch's state. The installation's token is had before the permission, even when the permission is cached: one REST call per request that the platform's `permission` call, which takes the token, makes necessary. A signed repository doesn't refuse a request: it withholds a land rather than granting one, so `clm land` counts it unsigned (`land`). Comments need read access only, the access check every API route makes.

## ADDED Requirements

### Requirement: gate-answer
A spec branch's review SHALL carry a top-level `review`, whether `clm gate` would ask a human's review of the branch's spec changes, and `cuts`, the `gate/cut` lines `clm gate` prints for the branch, in its order, empty when there is none: both as `gate/review-from-files` gives them over the paths whose entries differ between the Trees API listings of the merge base and of the head, each with the status those listings give it (`A` for a path only the head holds, `D` for one only the merge base holds, `T` for one whose entry turns between a file, a symlink and a gitlink, `M` otherwise), with no archive moves, whether the approval setting changed (`approval-setting-branch`), and each side's commit, the paths its listing holds, and its files as `web.worker.platform/spec-files` gives them, a listed path whose text GitHub withholds being one the call can't read. After the gate's lines, `cuts` SHALL hold one line per list that call names as withheld, `<path>: gate/cut: GitHub withholds its text at <commit>, so the tests it lists aren't listed`, `<commit>` the commit it names, so the page never asks a yes to tests it doesn't show without saying so. When either listing is truncated, `review` SHALL be true and `cuts` empty, whatever else holds. Where `clm gate` reads more than two trees can show, the review SHALL ask: `review` SHALL be true for a branch whose only change moves a change with `human_burdens` to `changes/archive/`, for a `spec/*` branch that also changes a code file, and for a branch whose change `rename/replay` accepts, whenever the call says so for the paths that differ, though `clm gate` asks no review of them. The change SHALL carry no `review` of its own. A review that an earlier build of the Worker cached under an earlier rule SHALL NOT be served.

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
- THEN the response's `review` is true and its `cuts` hold `changes/c/deleted-tests.txt: gate/cut: GitHub withholds its text at <head>, so the tests it lists aren't listed`, `<head>` the head's commit ID

#### Scenario: a list GitHub withholds at the merge base
- GIVEN a spec branch that adds the line `DELETE tests/t.rs::two` to `changes/c/deleted-tests.txt`, whose text GitHub gives as null at the merge base and as `DELETE tests/t.rs::one` and that line at the head
- WHEN its review is requested
- THEN the response's `review` is true and its `cuts` hold `changes/c/deleted-tests.txt: gate/cut: GitHub withholds its text at <base>, so the tests it lists aren't listed`, `<base>` the merge base's commit ID, and no line naming `tests/t.rs::one` or `tests/t.rs::two`

#### Scenario: a truncated listing
- GIVEN a spec branch that removes requirement `n/r`, and in turn a head listing and a merge base listing GitHub truncates
- WHEN its review is requested each time
- THEN the response's `review` is true and its `cuts` are empty each time

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

#### Scenario: the review errs toward asking
- GIVEN a branch that only moves `changes/x/`, whose `change.md` has `human_burdens: [b]`, to `changes/archive/x/`; a `spec/*` branch that removes `n/r` and changes `crates/a/src/lib.rs`; and a branch whose change sets `renames` and renames `n/r` to `n/s`, with no term rename and no `human_burdens`
- WHEN each review is requested
- THEN each says `review` true, where `clm gate` prints, in turn, `spec`, `mixed` and `spec`

#### Serves
- capability/approve-intent
- architecture/humans-review-intent

#### Needs
- gate/review-from-files
- web.worker.platform/spec-files

### Requirement: request-changes
`POST /api/repos/{o}/{r}/branches/{branch}/request-changes` with `{"fingerprint", "text"}`, its body refused by `feedback::parse_request` as a comment's is by `feedback::parse_comment` (`review-feedback`), `target` aside, SHALL append a `changes_requested` record with the branch, its fingerprint now and the text trimmed to `clm/ledger/<login>/web` (`web.worker.platform/web-ledger-append`) with the requester's own token, making no other write, and answer 200 with `{"id"}`, the record's `id`, only when GitHub answers `write` or `admin` for the requester's permission on the repo, asked with the installation's token (`web.worker.platform/permission`), and the body's `fingerprint` is the branch's now. It SHALL write nothing and answer, checking in this order: 422 `{"error": "branch"}` for a branch a comment is refused for; 422 `{"error": "fingerprint"}` when the branch has no fingerprint because a tree listing was truncated; 403 `{"error": "permission"}` for any permission but `write` or `admin`; and 409 `{"error": "moved"}` when the fingerprint isn't the branch's now. A repository whose default branch's tip holds `docs/allowed_signers` SHALL NOT refuse it. It SHALL answer 502, writing nothing, when GitHub fails the read of the branch's ref (`web.worker.platform/branch-tip`), the default branch's tip or the listings, or gives no installation, token or permission, and 502 when the append fails, the ledger's ref then where it was.

#### Scenario: changes requested
- GIVEN `spec/x` with fingerprint `f` in a repository whose default branch's tip holds `docs/allowed_signers`, and GitHub answering `write` for the user's permission
- WHEN the user posts `{"fingerprint": "f", "text": "Split it"}` to its request-changes, and the review is requested
- THEN the answer is 200 with the `id` of the line appended, the new `records.jsonl` ends with a `changes_requested` line for `spec/x` at `f`, and the review's `change_requests` lists it with `current` true and `held` true

#### Scenario: a reader can't hold a land
- GIVEN the same branch, and GitHub answering `read` for the user's permission
- WHEN the user posts the same request, and the same with a stale fingerprint
- THEN the Worker answers 403 with `{"error": "permission"}` both times and makes no write

#### Scenario: moved
- GIVEN the review of `spec/x` was drawn when its fingerprint was `f`, which is now `g`
- WHEN the user posts `{"fingerprint": "f", "text": "Split it"}` to its request-changes
- THEN the Worker answers 409 with `{"error": "moved"}` and makes no write

#### Scenario: what a refused request says
- GIVEN a requester GitHub answers `write` for, and in turn `spec/nope`, which doesn't exist; `spec/x` with a truncated head listing; `spec/x` in a repository whose installation GitHub doesn't give; `spec/x` whose ref lookup GitHub fails with a 502; and `spec/x` with its fingerprint, while GitHub refuses to move `clm/ledger/<login>/web`'s ref
- WHEN a request is posted for each
- THEN they answer 422 `{"error": "branch"}`, 422 `{"error": "fingerprint"}`, 502, 502 and 502, none writing and the last leaving the ledger's ref where it was

#### Serves
- capability/approve-intent
- capability/follow-from-browser

#### Needs
- web.worker.platform/web-ledger-append
- web.worker.platform/branch-tip
- web.worker.platform/permission
- ledger/feedback-records

## MODIFIED Requirements

### Requirement: pr-review <!-- base: 751149f9b1521d2e -->
For a `spec/*` branch, the review SHALL answer 200 with:
- `branch`, `base` (the default branch), `base_sha` (its tip) and `head_sha` (the branch's);
- per touched component, every added, modified and removed requirement, every changed Contract, Design and frontmatter, and every added, modified and removed asset, each with its text before and after, a binary asset's text being empty on each side that holds it;
- `change`: the `id`, `status`, `from` and `resolves` of the live change the branch touches, its `intent`, the body of its `change.md` after the frontmatter, and its `intent_section`, the body of its `## Intent` section, null when that is missing or empty;
- `decisions` and `backlog`, the decisions and backlog items it adds, modifies or removes, each with its text before and after;
- `documents`: every path whose entries differ between the Trees API listings of the merge base and of the head that `gate/classify`'s `is_spec_path` accepts, and `clm.toml` when the approval setting changed (`approval-setting-branch`), each with its text before and after, null on a side whose listing doesn't hold it and empty on a side whose text GitHub withholds;
- `fingerprint` (`fingerprint-from-trees`); `approvals`, the `approved` records the ledger holds for that fingerprint, each with its `writer` and `at`; and `checked`, whether a `checked` record names the head's code key, false when the head's listing is truncated;
- `page`, the `number` and `url` of the open pull request on the default branch whose head is the branch in this repository, not a fork's, null when there is none or GitHub doesn't answer;
- `review` and `cuts` (`gate-answer`), `signed` (`signed-repository`), and `comments` and `change_requests` (`review-feedback`).

It SHALL answer 404 for a name `spec-branches` refuses and for a branch GitHub says doesn't exist (`web.worker.platform/branch-tip`), and 502 when GitHub fails the read of the branch's ref, of the default branch's tip, of the comparison or of the ledgers.

#### Scenario: modified requirement
- GIVEN a spec branch that modifies requirement `n/r`, adds asset `assets/m.html` to `n` and adds decision `d`
- WHEN its review is requested
- THEN the response lists `n/r` as modified with both texts, `assets/m.html` as added, and `d` as added

#### Scenario: approval state
- GIVEN a spec branch and a ledger with an `approved` record for its fingerprint by writer `clunny/web`
- WHEN its review is requested
- THEN the response says it is approved by `clunny/web`

#### Scenario: not a reviewable branch
- GIVEN `impl/n`, `spec/../main`, `spec/missing`, which doesn't exist, `spec/x`, whose ref lookup GitHub fails with a 502, and `spec/y`, whose compare GitHub can't answer
- WHEN each review is requested
- THEN the first three answer 404 and the last two 502

#### Scenario: what the branch changes, whole
- GIVEN a spec branch that modifies `specs/n/SPEC.md`, edits `CAPABILITIES.md` and `CLAUDE.md`, adds `docs/allowed_signers` and a binary `docs/img.png`, and changes `.gitattributes`
- WHEN its review is requested
- THEN its `documents` hold `specs/n/SPEC.md`, `CAPABILITIES.md` and `CLAUDE.md` with their texts before and after, `docs/allowed_signers` with `before` null and its text after, and `docs/img.png` with `before` null and `after` empty, and not `.gitattributes`

#### Scenario: a binary asset
- GIVEN a branch that adds `assets/logo.png`, whose bytes aren't UTF-8, to `n` and modifies a binary asset `assets/a.bin` of `n`
- WHEN its review is requested
- THEN `assets/logo.png` is added with `after` empty, and `assets/a.bin` modified with both texts empty

#### Scenario: the page
- GIVEN `spec/c2` with open pull request #5 on the default branch and a fork's pull request #6 from a branch of the same name, and then a GitHub that fails the pull-request query
- WHEN the review is requested each time
- THEN `page` is `{number: 5, url}` and then null, the review answering 200 both times

#### Serves
- capability/approve-intent
- architecture/humans-review-intent

#### Needs
- web.worker.platform/branch-tip
- web.worker.platform/spec-files
- web.worker.platform/fake-github
- gate/classify

### Requirement: review-cached <!-- base: 4c4b8baaed3825b4 -->
The Worker SHALL answer a spec branch's review from the Cache API, without fetching spec files, while the default branch's tip and the branch's head are the commits it was built for.

#### Scenario: unchanged branch
- GIVEN `spec/x`'s review was built for default-branch tip `111` and head `222`
- WHEN it is requested again and the tip and head are still `111` and `222`
- THEN the answer is the same and no spec files are fetched

#### Scenario: new push to the branch
- GIVEN `spec/x`'s review was built for head `222`
- WHEN it is requested after `spec/x` moved to `333`
- THEN the review is built again from `333`

### Requirement: approve <!-- base: 7dc5d5a3af14ea40 -->
The approve route SHALL approve a `spec/*` branch that touches a spec path (`gate/classify`'s `is_spec_path`) or changes the approval setting (`approval-setting-branch`), when the request names what the person reviewed, a `fingerprint` equal to the branch's fingerprint now, by appending an `approved` record to `clm/ledger/<login>/web` (`web.worker.platform/web-ledger-append`) with the requester's own token, making no other write to the repository, and answering 200 with an empty body. The record SHALL name that fingerprint, the `branch`, and as its `change` the first of the IDs `ledger/change-ids` gives for the paths the head's listing holds whose entries the merge base's lacks or differs on, null when there is none, as `clm approve` names it. It SHALL name, as the requests it clears (`ledger/request-cleared`), every `changes_requested` record for that branch at that fingerprint whose writer's person is the requester's login that the requester's own web ledger holds when read at its tip after the fingerprint comparison, and none, the record still naming an empty list, when it holds none. It SHALL write nothing and answer, checking in this order: an empty 422 for a branch that isn't `spec/*`, that GitHub says doesn't exist (`web.worker.platform/branch-tip`), or that neither touches a spec path nor changes the approval setting; the 422 `signed-repository` gives for a repository whose default branch's tip holds `docs/allowed_signers`; an empty 422 when the branch has no fingerprint because a tree listing was truncated; and 409 when the named fingerprint isn't the branch's now. It SHALL answer 502 and write nothing when GitHub fails the read of the branch's ref, of the default branch's tip, of the listings, of `clm.toml`'s texts or of the requester's web ledger, and 502 when the append fails, the ledger's ref then where it was.

#### Scenario: approve
- GIVEN a spec branch `spec/x` whose fingerprint is `f`
- WHEN the user approves it with `{"fingerprint": "f"}`
- THEN the Worker answers 200 with an empty body, the writes made are exactly a blob, a tree, a commit and a ref update on `clm/ledger/<login>/web`, with the user's token, and the new `records.jsonl` ends with an `approved` line for `f` with `branch` `spec/x` and writer `<login>/web`

#### Scenario: pushed after the review
- GIVEN the review of `spec/x` was drawn when its fingerprint was `f`, and the author then pushed a commit that changes a spec file, so its fingerprint is now `g`
- WHEN the user approves it with `{"fingerprint": "f"}`
- THEN the Worker answers 409 and makes no write

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

#### Scenario: the change it names
- GIVEN `spec/x` adding `changes/x/change.md`; `spec/arch` touching no change; `spec/two` adding `changes/x/change.md` and `changes/x-y/change.md`; and `spec/z`, which moves `changes/x/` to `changes/archive/x/` and adds `changes/z/change.md`
- WHEN each is approved with its fingerprint
- THEN the records name `change` `x`, null, `x` and `z`

#### Scenario: refusals in order
- GIVEN a default branch whose tip holds `docs/allowed_signers`, with `spec/missing`, which doesn't exist, `spec/code`, which touches only `crates/`, `impl/a`, and `spec/x`, whose merge base's listing GitHub truncates; and the same `spec/x` in a repository whose tip has no `docs/allowed_signers`
- WHEN each is approved with a fingerprint that isn't its own
- THEN the first three answer an empty 422, the signed `spec/x` 422 with `error` `signed`, and the unsigned one an empty 422, none writing

#### Scenario: GitHub fails
- GIVEN `spec/x` whose ref lookup GitHub fails with a 502; then a default branch whose ref lookup it fails; then GitHub refusing to move `clm/ledger/<login>/web`'s ref
- WHEN the user approves `spec/x` with its fingerprint each time
- THEN each answers 502, and no write is made and the ledger's ref is where it was

#### Serves
- capability/approve-intent
- architecture/approval-policy
- architecture/humans-review-intent
- architecture/git-is-truth

#### Needs
- web.worker.platform/web-ledger-append
- web.worker.platform/branch-tip
- gate/classify
- gate/approval-changed
- ledger/request-cleared
- ledger/change-ids

### Requirement: fingerprint-from-trees <!-- base: c44557f55a293833 -->
A spec branch's `fingerprint` SHALL be what `web.worker.platform/fingerprint-from-listings` gives for the Trees API listings of the compare's merge base and of the branch's head, never a hash of GitHub's diff, whatever GitHub's rename and abbreviation settings. The review SHALL match the ledger's `approved` records by it, and the approve route SHALL name it in the record it appends. When either listing is truncated, the review SHALL carry no fingerprint and the approve route SHALL refuse the branch.

#### Serves
- capability/approve-intent
- architecture/hash-based-staleness

#### Needs
- web.worker.platform/fingerprint-from-listings

#### Scenario: truncated listing
- GIVEN a spec branch whose merge base's Trees API listing is `truncated`
- WHEN its review is requested and then its approval
- THEN the review has no fingerprint and approve refuses, appending nothing

### Requirement: spec-branches <!-- base: a0bae05a67fb8564 -->
`branches::list` SHALL give every branch of the repo named `spec/` followed by one or more `/`-separated parts, each non-empty, not starting with `.`, not ending in `.lock` and made only of ASCII letters, digits, `-`, `_` and `.`, with no `..` anywhere in the name, sorted by name, each with `branch`, `head_sha` and `page`: the `number`, `url` and `title` of the open pull request on the default branch whose head is that branch of this repository, not a fork's, among the first 300 open pull requests GitHub lists (`web.worker.platform/open-pulls`), or none when there is none among them. It SHALL give no other branch, SHALL be answered from the live state's cache for 60 seconds, and SHALL give none in place of a list when GitHub fails the listing of the `spec/` refs or the pull-request query.

#### Scenario: with and without a page
- GIVEN branches `spec/a` at `111`, with open pull request #7 "Add a" on `main`, `spec/b` at `222`, with none, and `impl/c`, `spec/.hidden`, `spec/a..b`, `spec/x.lock` and `specs/d`
- WHEN the branch list is requested
- THEN it is `spec/a` with head `111` and page #7 "Add a" and its URL, then `spec/b` with head `222` and page `null`, and nothing else

#### Scenario: GitHub fails the list
- GIVEN a GitHub that fails `matching-refs/heads/spec/`, and then one that fails the pull-request query
- WHEN `branches::list` runs each time
- THEN it gives none each time

#### Needs
- web.worker.platform/open-pulls
- web.worker.platform/fake-github

### Requirement: signed-repository <!-- base: a987323453d11063 -->
A spec branch's review SHALL carry `signed`, true exactly when the default branch's tip holds an entry at `docs/allowed_signers`, and SHALL mark each of its `approvals` `signed` exactly when `ledger/cleared-by-a-signed-line`'s `carries_sig` says that record's line carries a non-empty `sig`. For a `spec/*` branch that exists and touches spec paths in a repository whose `signed` is true, the approve route SHALL answer 422 with the JSON body `{"error": "signed", "command": "clm approve <branch>"}`, `<branch>` the branch approved, and make no write to the repository. The Worker SHALL ask GitHub whether a tip is signed at most once a day for each tip, the review and the approve route sharing the answer. When the Worker can't learn whether the tip is signed, the review and the approve route SHALL answer 502 and write nothing.

#### Scenario: signed repository
- GIVEN a default branch whose tip holds `docs/allowed_signers`, a spec branch `spec/x` whose fingerprint is `f`, and a ledger holding an `approved` record for `f` with a `sig` by `dana/9f9f9f9f` and one without by `clunny/web`
- WHEN `spec/x`'s review is requested, and then the user approves it with `{"fingerprint": "f"}`
- THEN the review says `signed` true and lists `dana/9f9f9f9f` with `signed` true and `clunny/web` with `signed` false, and the approve answers 422 with `error` `signed` and `command` `clm approve spec/x` and makes no write

#### Scenario: unsigned repository
- GIVEN the same branch and ledger, and a default branch whose tip has no `docs/allowed_signers`
- WHEN `spec/x`'s review is requested
- THEN the review says `signed` false, and both approvals are listed with their `signed` as in *signed repository*

#### Scenario: behind main
- GIVEN a spec branch `spec/y` branched before `main` gained `docs/allowed_signers`, so its merge base has no such file and `main`'s tip does
- WHEN `spec/y`'s review is requested
- THEN the review says `signed` true

#### Scenario: lookup fails
- GIVEN a GitHub that fails the query asking whether the tip holds `docs/allowed_signers`
- WHEN a spec branch's review is requested, and then its approve with the branch's fingerprint
- THEN both answer 502 and no write is made

#### Scenario: asked once per tip
- GIVEN a default branch whose tip holds `docs/allowed_signers`
- WHEN `spec/x`'s review is requested twice and then the user approves it
- THEN GitHub was asked whether that tip is signed once

#### Serves
- capability/approve-intent
- architecture/approval-policy

#### Needs
- ledger/cleared-by-a-signed-line

### Requirement: approval-setting-branch <!-- base: 598840479fb3f3ae -->
For a `spec/*` branch whose merge base and head differ on `clm.toml`, whether the branch changes the approval setting SHALL be what `gate/approval-changed` gives for the file's bytes at the merge base and at the head, a side whose listing has no such file counting as missing; a side whose file exists but whose text GitHub withholds SHALL count as changed. When the setting changed, the review SHALL list `clm.toml` among its `documents` with its text before and after, and the branch SHALL count as touching a spec path for the approve route (`approve`); when it didn't, and when the file is the same on both sides, the review SHALL NOT list it and the file SHALL NOT make the branch approvable. The review SHALL make no request for the file beyond the spec files it fetches, and the approve route SHALL fetch its two texts only for a branch whose listings differ on it, in one request. A review that an earlier build of the Worker cached SHALL NOT be served for such a branch.

#### Scenario: only the policy changes
- GIVEN `main`'s `clm.toml` sets `approval = "other"`, and `spec/policy` changes it to `approval = "any"` and changes nothing else
- WHEN its review is requested and the user then approves it with the review's fingerprint
- THEN the review lists no component change and no `change`, and its `documents` hold `clm.toml` with the text before and after; and the approve answers 200, appending an `approved` record for that fingerprint to `clm/ledger/<login>/web`

#### Scenario: a comment in the policy file
- GIVEN `spec/comment` changes only a comment line in `clm.toml`
- WHEN its review is requested and the user approves it
- THEN the review's `documents` don't hold `clm.toml`, and the approve answers 422 and makes no write

#### Scenario: the file no longer parses
- GIVEN `spec/broken` changes `clm.toml` so that it no longer parses as TOML, and nothing else
- WHEN its review is requested
- THEN its `documents` hold `clm.toml`, as the gate counts the file a spec change

#### Serves
- capability/approve-intent
- architecture/humans-review-intent

#### Needs
- gate/approval-changed

### Requirement: review-feedback <!-- base: bb8c3fbe5e1d4fbf -->
A spec branch's review SHALL carry `comments`, every `commented` record in the fold for the branch, oldest first, each with `id`, `writer`, `at`, `fingerprint`, `target`, `text` and `current`, whether its fingerprint is the review's, and `change_requests`, every `changes_requested` record for the branch the same way, each with `held` as well: `current`, and not cleared as `ledger/request-cleared` decides, where, in a review whose `signed` is true (`signed-repository`), only an approval whose line carries a non-empty `sig` counts (`ledger/cleared-by-a-signed-line`'s `cleared_by_sig`). When the review has no fingerprint, every entry's `current` and `held` SHALL be false. `POST /api/repos/{o}/{r}/branches/{branch}/comments` with `{"fingerprint", "target", "text"}` SHALL append a `commented` record with those three, the text trimmed, to `clm/ledger/<login>/web` (`web.worker.platform/web-ledger-append`) with the requester's own token, making no other write, without comparing the fingerprint with the branch's, and answer 200 with `{"id"}`, the record's `id`. `feedback::parse_comment` SHALL refuse a body with 422 `{"error": <field>}`, naming the first of these the body fails: `fingerprint`, for a body that isn't JSON or whose `fingerprint` isn't a non-empty string; `target`, for a `target` that is present and neither null nor a string; and `text`, for a `text` that is missing, empty or over 4,000 characters once trimmed; a missing `target` SHALL read as null. The route SHALL write nothing and answer 422 `{"error": "branch"}` for a branch that isn't `spec/*`, that GitHub says doesn't exist (`web.worker.platform/branch-tip`), or that neither touches a spec path nor changes the approval setting, and 422 `{"error": "target"}` for a `target` that is neither null nor a requirement ID the branch's review lists as added, modified or removed or the snapshot's graph holds. It SHALL answer 502 and write nothing when GitHub fails the read of the branch's ref, of the default branch's tip or of the listings, when a comment with a target needs the snapshot or the branch's comparison and can't have it, and 502 when the append fails, the ledger's ref then where it was.

#### Scenario: comment on a requirement
- GIVEN `spec/x`, whose head holds requirement `n/r`, with fingerprint `f`
- WHEN the user posts `{"fingerprint": "f", "target": "n/r", "text": "Name the rule"}` to its comments, and the review is requested
- THEN the answer is 200 with the `id` of the line appended, the writes were a blob, a tree, a commit and a ref update on `clm/ledger/<login>/web`, the new `records.jsonl` ends with a `commented` line for `spec/x` at `f` with target `n/r`, and the review's `comments` lists it with `current` true

#### Scenario: unknown target
- GIVEN the same branch
- WHEN the user posts a comment with target `n/missing`, then one with target `null` and an empty text, then one with target `m/old`, a requirement the review lists as removed, and the text "Keep it"
- THEN the first two answer 422 and nothing is written for them, and the third appends a `commented` line with target `m/old`

#### Scenario: cleared
- GIVEN a `changes_requested` record for `spec/x` at `f` by `alice/web`, and an `approved` record for `f` by `alice/1a2b3c4d` that names the request as one it clears
- WHEN `spec/x`'s review is requested while its fingerprint is `f`
- THEN its `change_requests` lists the request with `current` true and `held` false

#### Scenario: held in a signed repository
- GIVEN a default branch whose tip holds `docs/allowed_signers`, a `changes_requested` record `r` for `spec/x` at `f` by `alice/web`, and in turn as the only approval one for `f` by `alice/1a2b3c4d` naming `r` with no `sig`, and the same with a non-empty `sig`
- WHEN `spec/x`'s review is requested while its fingerprint is `f`
- THEN `r` is listed with `held` true with the unsigned approval, and `held` false with the other

#### Scenario: the target can't be checked
- GIVEN `spec/x`, and in turn a snapshot that can't be read, and a snapshot that lacks `n/r` while GitHub fails the fetch of the branch's spec files
- WHEN a comment with target `n/r` is posted each time, and one with target null
- THEN the two with a target answer 502 and write nothing, and the null one answers 200

#### Scenario: an odd body
- GIVEN the bodies `not json`; `{"fingerprint": "f", "text": "x"}`; `{"target": 3}`; `{"fingerprint": "f", "target": 3, "text": ""}`; and one with `fingerprint` `f` and 4,000 `é` as its text
- WHEN `feedback::parse_comment` reads each
- THEN they give 422 `fingerprint`; a comment with target null; 422 `fingerprint`; 422 `target`; and a comment with that text

#### Scenario: GitHub fails
- GIVEN `spec/x` whose ref lookup GitHub fails with a 502, and then GitHub refusing to move `clm/ledger/<login>/web`'s ref
- WHEN the user posts a comment with target null each time
- THEN each answers 502, and no write is made and the ledger's ref is where it was

#### Serves
- capability/approve-intent
- capability/follow-from-browser

#### Needs
- web.worker.platform/web-ledger-append
- web.worker.platform/branch-tip
- ledger/feedback-records
- ledger/request-cleared
- ledger/cleared-by-a-signed-line
