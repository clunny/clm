---
component: land.approval
base: { design: "27680bb8af128b91" }
---

## MODIFIED Design
Crate `clm-approval` (`land-approval-split`) can't depend on `clm-land`, which calls it (`undeclared-imports`, and a cycle), so it takes the CLM root, the environment lookup, the loaded settings and the pinned base commit as arguments, runs git through `clm-git` with the remote's name from `REMOTE` (`git-plumbing-layer`), and has an error type of its own, which `clm-land`'s `Error` converts from with the same exit codes and messages. Tests that land a branch to prove these rules live in `crates/clm-land/tests` and cite this component's requirements; the others, `clm approve`'s among them, live in `crates/clm-approval/tests`.

**The branch.** The head of `<branch>` is `origin/<branch>` when it exists and the local branch isn't ahead of it, else the local branch; for a land, a local `spec/*` or `impl/*` branch with commits `origin` lacks is refused, since those are pushed deliberately. The fingerprint is `clm-gate`'s `fingerprint_of` over the `git ls-tree -r -z` listings of the head's merge base with the base and of the head, so it needs no worktree. The change IDs are those of the live `changes/<id>/change.md` the head adds or modifies relative to that merge base, sorted.

**Approved** (`approval-policy`). A fingerprint is approved when an `approved` record names it and the policy accepts it. Under `other`, the authors are the `git log --format=%an` names of the commits between the merge base and the head, slugged with `ledger`'s `slug`, so the two can't drift (`ledger/person-slug`). When approvals of the fingerprint exist but none counts, the `gate/approval` message names each and why: not signed by its writer (naming `clm approve` with a signing key; an approval made in `web` is one, since `web` can't sign), by an author, or by the lander (naming `approval = "other"`), so the human isn't left wondering why an approval they see doesn't count. With `other`, the message also says once, on its own line, `approval = "other"` from `origin/main:clm.toml` (or `clm.local.toml`), whether approvals were refused or there were none, so a human whose checkout says `any` sees why the land disagrees.

**The policy's source** (`approval-from-base`, `land-policy-from-base`). The root `clm land` runs in may be the branch's own checkout, whose `clm.toml` is the one the branch proposes. So once an attempt has pinned its base commit, the policy is the top-level `approval` string of the `clm.toml` at the top of that commit's tree (`git ls-tree --full-tree --name-only <sha> -- clm.toml` says whether it exists, `git cat-file blob <sha>:clm.toml` gives its bytes, parsed with the `toml` crate). A CLM root in a subdirectory, whose own file is `<dir>/clm.toml`, gets `any` from the base, as `ledger`'s `verify` reads `<first parent>:clm.toml` for the same commit, so the two can't disagree about a land. An `approval` that isn't the string `any` or `other` is a settings error (`verify` reads a non-string as `any`; refusing is the conservative side), exit 2, before any check runs or anything is pushed or appended, its message starting `approval from <base>:clm.toml:` followed by the parse error or the value. When the loaded `approval` setting's layer is the clone's `clm.local.toml` (`config`'s `local-settings`), its value is the policy and the base isn't read, since the clone landing owns it. Each attempt reads the policy afresh with its base, since `main` may move between attempts. `clm approve` and `clm land --check-only` read no policy.

**Held** (`changes-requested-holds`). A push that changes the diff orphans a request as it orphans an approval, and the requester clears their own by approving; `Fold::cleared` compares `at` as strings, and an equal one isn't later. `clm land` checks it after each attempt's fetch, before the gate (so before `gate/approval` could print), the lock, the merge and the checks; in a signed repository the key check comes first. The refusal prints on stdout, in `check`'s format under a heading line, one `<branch>: land/changes-requested: <writer> at <at>: <text>` per holding request, then one `<branch>: land/comment: <writer> on <target> at <at>: <text>` per `commented` record for the branch at that fingerprint, `<target>` the requirement ID or the words `the change`, each text trimmed with every run of whitespace holding a line break replaced by one space. It holds whatever the branch's name and with `host = "none"`, which reads the local ledgers (the fold's by-branch index, `ledger/feedback-records`). A request counts whether or not the repository or the record is signed: it withholds a land rather than granting one, and `clm land` names its writer. Persons are matched by name: a web record's person is the GitHub login, so a requester whose CLI `writer` (`config`) isn't their login clears their request with `clm approve` only after setting `writer` to it, or leaves the author to push.

**`clm approve [<branch>]`** (`approve`) takes the branch's head and fingerprint as above. It is what the guard asks the human to confirm (`approve-in-session`); the command doesn't ask again.

**Signed repositories** (`signed-ledger-records`). When `ledger`'s `signed(root, <base>)` is true, `<base>` being the fetched base's commit, pinned by its SHA so a concurrent fetch can't change which signers file applies:
- *Approving* (`signed-approve`). The signing uses the namespace `clm-approve`.
- *Counting.* An `approved` record counts only when `signed_by(root, <base>, record, "clm-approve")` is true, and under `other` only when its writer's person also differs from the lander's (this clone's `writer` person), besides every author: an author's name is free text, so without this an author could name someone else as the author and approve their own branch.
- *The key check.* After each attempt's fetch, before the gate and before it takes the lock or makes a worktree, `clm land` checks that a key is configured: `user.signingkey` set with `gpg.format = ssh`, and, for a path, the file readable. When not, it exits 1 naming `user.signingkey`. It signs nothing yet, so a touch key is touched once per merge commit.
- *The land's record.* Once the gate on the merge commit has given the kind, `clm land` builds the full `landed` record and signs it under `clm-land`; a failure exits 1 and pushes nothing. Retried pushes of the same merge commit reuse it; an attempt that starts again makes a new merge commit and signs a new record; the append writes it as it is.

Without `docs/allowed_signers` at the base, nothing is signed or checked. `clm ledger verify` audits the same rules after the fact on every clone (`ledger`), which is what holds a lander who skips them.

## MODIFIED Requirements

### Requirement: approval-from-base <!-- base: 522ea46597a3432a -->
`clm land` SHALL count approvals under the `approval` policy that the top-level `clm.toml` committed at the base it lands on sets (the `origin/main` it fetched, on every host), `any` when that file or its `approval` key is missing, unless the clone's `clm.local.toml` sets `approval`, whose value SHALL be the policy for that clone's lands; it SHALL never take the policy from the `clm.toml` of the root it runs in, of the head or of the merged tree. When the base's `clm.toml` doesn't parse, or sets a value other than `any` and `other`, it SHALL exit 2 naming `<base>:clm.toml` and `approval`, before running any check or pushing or appending anything.

#### Scenario: a branch that weakens the policy is judged by the base
- GIVEN `origin/main` whose `clm.toml` sets `approval = "other"`, a `spec/x` that needs review authored by `Christopher Lunny` whose head sets `approval = "any"`, an `approved` record by writer `christopher-lunny/1a2b3c4d`, and no `clm.local.toml`
- WHEN `clm land spec/x` runs from a checkout of `spec/x`
- THEN it exits 1 naming `approval = "other"` from `origin/main:clm.toml`, and `main` is unchanged

#### Scenario: a branch that tightens the policy is judged by the base
- GIVEN `origin/main` whose `clm.toml` sets no `approval`, the same `spec/x` whose head sets `approval = "other"`, and the same `approved` record
- WHEN `clm land spec/x` runs from a checkout of `spec/x`
- THEN it lands

#### Scenario: the clone's local file wins
- GIVEN `origin/main` whose `clm.toml` sets `approval = "other"`, the clone's `clm.local.toml` setting `approval = "any"`, and the self-approval of the first scenario
- WHEN `clm land spec/x` runs
- THEN it lands

#### Scenario: unreadable policy at the base
- GIVEN `origin/main` whose `clm.toml` sets `approval = "everyone"`, and a `spec/x` whose `clm.toml` sets `approval = "any"`, with an `approved` record
- WHEN `clm land spec/x` runs from a checkout of `spec/x`
- THEN it exits 2 naming `origin/main:clm.toml` and `approval`, runs no check, and `main` and the ledger are unchanged

#### Needs
- gate/files-outside-spec-paths

#### Serves
- capability/approve-intent
- architecture/land-policy-from-base

### Requirement: signed-land <!-- base: b1b71ffcee963880 -->
When `origin/main`'s tree holds `docs/allowed_signers`, `clm land` SHALL count an `approved` record only when `ledger`'s `signed_by` accepts it under `clm-approve` against that file, and, with `approval = "other"`, only when its writer's person also differs from the lander's writer person; it SHALL exit 1, before running any check or pushing, naming `user.signingkey` when no signing key is configured; and it SHALL append a `landed` record signed under `clm-land`, signed before the push, pushing nothing when signing fails. Without that file it SHALL count, sign and append as it did before.

#### Scenario: unsigned approval refused
- GIVEN a signed repository, a `spec/x` that needs review, a lander who can sign, and only an `approved` record for its fingerprint with no `sig`
- WHEN `clm land spec/x` runs
- THEN it exits 1 with `gate/approval` saying the approval isn't signed, and `main` is unchanged

#### Scenario: forged name refused
- GIVEN a signed repository whose `docs/allowed_signers` lists keys for `christopher-lunny` and `dana`, `approval = "other"`, a `spec/x` that needs review authored by `Christopher Lunny`, a lander `christopher-lunny` who can sign, and an `approved` record by writer `dana/1a2b3c4d` signed with `christopher-lunny`'s key
- WHEN `clm land spec/x` runs
- THEN it exits 1 and `main` is unchanged

#### Scenario: lander approved it themselves
- GIVEN the same file and policy, a `spec/x` that needs review whose commits are authored as `Zed`, a lander whose writer person is `christopher-lunny`, and an `approved` record signed by `christopher-lunny`
- WHEN `clm land spec/x` runs
- THEN it exits 1 naming `approval = "other"` and `main` is unchanged

#### Scenario: signed land
- GIVEN the `spec/x` that needs review of *forged name refused*, with an `approved` record by `dana` signed with `dana`'s key, landed by `christopher-lunny` who can sign
- WHEN `clm land spec/x` runs
- THEN it lands, and its `landed` record's `sig` is accepted by `signed_by` under `clm-land`

#### Scenario: lander can't sign
- GIVEN a signed repository, a code branch `impl/a`, and a lander with no `user.signingkey`
- WHEN `clm land impl/a` runs
- THEN it exits 1 naming `user.signingkey` before running the check command, and `main` and the ledger are unchanged

#### Serves
- capability/approve-intent
- architecture/signed-ledger-records

#### Needs
- ledger/record-signing
- ledger/signature-check
