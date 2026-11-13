---
component: web.worker.review
---

## MODIFIED Requirements

### Requirement: approve <!-- base: 4584ce53923a5ad4 -->
The Worker SHALL approve a `spec/*` branch that touches spec paths, or that changes the approval setting (`approval-setting-branch`), by appending an `approved` record with the branch's fingerprint to `clm/ledger/<login>/web` through the Git Data API with the requester's own token, making no other write to the repository, and only when the request, on the branch route, names what the person reviewed: a `fingerprint` equal to the branch's fingerprint now. It SHALL answer 409 and write nothing when the named fingerprint no longer matches, and SHALL answer 422 and write nothing for a request whose body doesn't name a `fingerprint` (checked before any request to GitHub), and for a branch that doesn't exist, isn't `spec/*`, neither touches a spec path nor changes the approval setting, or has no fingerprint because a tree listing was truncated, and for a repository whose default branch's tip holds `docs/allowed_signers` (`signed-repository`). The 422 cases are checked before the 409 one.

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

#### Serves
- capability/approve-intent
- architecture/approval-policy
- architecture/humans-review-intent
- architecture/git-is-truth

#### Needs
- web.worker.platform/shared-layer
- gate/approval-changed
