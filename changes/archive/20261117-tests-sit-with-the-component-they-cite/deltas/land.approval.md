---
component: land.approval
base: { design: "dfaf509cda9cb4f5" }
---

## MODIFIED Design
Crate `clm-approval` (`land-approval-split`) can't depend on `clm-land`, which calls it (`undeclared-imports`, and a cycle), so it takes the CLM root, the environment lookup, the loaded settings and the pinned base commit as arguments, runs git through `clm-git` with the remote's name from `REMOTE` (`git-plumbing-layer`), and has an error type of its own, which `clm-land`'s `Error` converts from with the same exit codes and messages. What `clm land` does with these answers is `land`'s to state and to prove by landing a branch (`land/approval-policy`, `land/approval-from-base`, `land/changes-requested-holds`, `land/signed-land`), since this crate can land nothing (`commands-live-in-their-crates`). This component states what its calls answer (`approvals-counted`, `policy-at-base`, `request-refusal`, `key-and-signing`), and its tests call `clm-approval` and live in `crates/clm-approval/tests`. Until those four are built, `changes/20261117-tests-sit-with-the-component-they-cite/moved-tests.txt` names the tests that cite this component from `land`'s files and from this crate's, with the requirement each cites from then on. The two in this crate that cite `changes-requested-holds`, in `src/hold.rs` and `tests/hold_rules.rs`, are re-cited to `request-refusal` by the build that builds it: they are never removed as an obsolete requirement's.

**The branch.** The head of `<branch>` is `origin/<branch>` when it exists and the local branch isn't ahead of it, else the local branch; for a land, a local `spec/*` or `impl/*` branch with commits `origin` lacks is refused, since those are pushed deliberately. The fingerprint is `clm-gate`'s `fingerprint_of` over the `git ls-tree -r -z` listings of the head's merge base with the base and of the head, so it needs no worktree. The change IDs are those of the live `changes/<id>/change.md` the head adds or modifies relative to that merge base, sorted.

**Approved** (`approval-policy`). A fingerprint is approved when an `approved` record names it and the policy accepts it. Under `other`, the authors are the `git log --format=%an` names of the commits between the merge base and the head, slugged with `ledger`'s `slug`, so the two can't drift (`ledger/person-slug`). When approvals of the fingerprint exist but none counts, the `gate/approval` message names each and why: not signed by its writer (naming `clm approve` with a signing key; an approval made in `web` is one, since `web` can't sign), by an author, or by the lander (naming `approval = "other"`), so the human isn't left wondering why an approval they see doesn't count. With `other`, the message also says once, on its own line, `approval = "other"` from `origin/main:clm.toml` (or `clm.local.toml`), whether approvals were refused or there were none, so a human whose checkout says `any` sees why the land disagrees.

**The policy's source** (`approval-from-base`, `land-policy-from-base`). The root `clm land` runs in may be the branch's own checkout, whose `clm.toml` is the one the branch proposes. So once an attempt has pinned its base commit, the policy is the top-level `approval` string of the `clm.toml` at the top of that commit's tree (`git ls-tree --full-tree --name-only <sha> -- clm.toml` says whether it exists, `git cat-file blob <sha>:clm.toml` gives its bytes, parsed with the `toml` crate). A CLM root in a subdirectory, whose own file is `<dir>/clm.toml`, gets `any` from the base, as `ledger`'s `verify` reads `<first parent>:clm.toml` for the same commit, so the two can't disagree about a land. An `approval` that isn't the string `any` or `other` is a settings error (`verify` reads a non-string as `any`; refusing is the conservative side), exit 2, before any check runs or anything is pushed or appended, its message starting `approval from <base>:clm.toml:` followed by the parse error or the value. When the loaded `approval` setting's layer is the clone's `clm.local.toml` (`config`'s `local-settings`), its value is the policy and the base isn't read, since the clone landing owns it. Each attempt reads the policy afresh with its base, since `main` may move between attempts. `clm approve` and `clm land --check-only` read no policy.

**Held** (`changes-requested-holds`). A push that changes the diff orphans a request as it orphans an approval, and the requester clears their own by approving; `Fold::cleared` compares `at` as strings, and an equal one isn't later. `clm land` checks it after each attempt's fetch, before the gate (so before `gate/approval` could print), the lock, the merge and the checks; in a signed repository the key check comes first. The refusal prints on stdout, in `check`'s format under a heading line, one `<branch>: land/changes-requested: <writer> at <at>: <text>` per holding request, then one `<branch>: land/comment: <writer> on <target> at <at>: <text>` per `commented` record for the branch at that fingerprint, `<target>` the requirement ID or the words `the change`, each text trimmed with every run of whitespace holding a line break replaced by one space. It holds whatever the branch's name (the fold's by-branch index, `ledger/feedback-records`). A request counts whether or not the repository or the record is signed: it withholds a land rather than granting one, and `clm land` names its writer. Persons are matched by name: a web record's person is the GitHub login, so a requester whose CLI `writer` (`config`) isn't their login clears their request with `clm approve` only after setting `writer` to it, or leaves the author to push.

**`clm approve [<branch>]`** (`approve`) takes the branch's head and fingerprint as above. It is what the guard asks the human to confirm (`approval-policy`); the command doesn't ask again.

**Signed repositories** (`signed-ledger-records`). When `ledger`'s `signed(root, <base>)` is true, `<base>` being the fetched base's commit, pinned by its SHA so a concurrent fetch can't change which signers file applies:
- *Approving* (`signed-approve`). The signing uses the namespace `clm-approve`.
- *Counting.* An `approved` record counts only when `signed_by(root, <base>, record, "clm-approve")` is true, and under `other` only when its writer's person also differs from the lander's (this clone's `writer` person), besides every author: an author's name is free text, so without this an author could name someone else as the author and approve their own branch.
- *The key check.* After each attempt's fetch, before the gate and before it takes the lock or makes a worktree, `clm land` checks that a key is configured: `user.signingkey` set with `gpg.format = ssh`, and, for a path, the file readable. When not, it exits 1 naming `user.signingkey`. It signs nothing yet, so a touch key is touched once per merge commit.
- *The land's record.* Once the gate on the merge commit has given the kind, `clm land` builds the full `landed` record and signs it under `clm-land`; a failure exits 1 and pushes nothing. Retried pushes of the same merge commit reuse it; an attempt that starts again makes a new merge commit and signs a new record; the append writes it as it is.

Without `docs/allowed_signers` at the base, nothing is signed or checked. `clm ledger verify` audits the same rules after the fact on every clone (`ledger`), which is what holds a lander who skips them.

## REMOVED Requirements

### Requirement: approval-policy <!-- base: 4438df262352b6f2 -->

### Requirement: approval-from-base <!-- base: 094b6c3dd57cdb4b -->

### Requirement: signed-land <!-- base: a7c79d9c083374f0 -->

### Requirement: changes-requested-holds <!-- base: 27e6c16ba059604f -->

## ADDED Requirements

### Requirement: approvals-counted
`clm-approval` SHALL tell its caller whether a fingerprint's approvals count under a policy, and, when none counts, for each `approved` record its writer and time and why it doesn't. Under `any` it SHALL count any `approved` record for the fingerprint. Under `other` it SHALL count a record only when its writer's person differs from the slugged author name of every commit between the merge base and the head, saying of one that doesn't that its writer is one of the branch's authors and naming `approval = "other"`. In a repository signed at the base it SHALL count a record only when `ledger`'s `signed_by` accepts it under `clm-approve` against that base's `docs/allowed_signers`, saying of one that doesn't that it isn't signed by its writer and naming `clm approve`, and, under `other`, only when its writer's person also differs from the writer person of the clone it runs in, the lander's, saying of one that doesn't that its writer is the lander. A fingerprint no `approved` record names SHALL NOT count, with no reason.

#### Scenario: an author's own approval
- GIVEN a branch whose commits are authored by `Christopher Lunny`, and an `approved` record for its fingerprint by writer `christopher-lunny/1a2b3c4d`
- WHEN the approvals are counted under `other`, and again under `any`
- THEN the first doesn't count and gives one reason naming `christopher-lunny/1a2b3c4d` and `approval = "other"`, and the second counts

#### Scenario: another person
- GIVEN the same branch with an `approved` record by writer `dana/9f9f9f9f`
- WHEN the approvals are counted under `other`
- THEN they count, with no reason

#### Scenario: unsigned in a signed repository
- GIVEN a base whose tree holds `docs/allowed_signers`, and an `approved` record for the fingerprint with no `sig`
- WHEN the approvals are counted for that base
- THEN they don't count, and the reason says the record isn't signed by its writer and names `clm approve`

#### Scenario: a forged name
- GIVEN a base whose `docs/allowed_signers` lists keys for `christopher-lunny` and `dana`, and an `approved` record by writer `dana/1a2b3c4d` signed with `christopher-lunny`'s key
- WHEN the approvals are counted for that base
- THEN they don't count

#### Scenario: the lander's own approval
- GIVEN the same base, a branch whose commits are authored as `Zed`, a clone whose writer person is `christopher-lunny`, and an `approved` record signed by `christopher-lunny`
- WHEN the approvals are counted under `other`
- THEN they don't count, and the reason says its writer is the lander

#### Scenario: no approval
- GIVEN a fingerprint no `approved` record names
- WHEN the approvals are counted
- THEN they don't count and no reason is given

#### Serves
- capability/approve-intent
- architecture/approval-policy
- architecture/signed-ledger-records

#### Needs
- ledger/person-slug
- ledger/signature-check

### Requirement: policy-at-base
`clm-approval` SHALL give its caller the `approval` policy for a land onto a base commit, with where it came from: the value the clone's `clm.local.toml` sets, from `clm.local.toml`, when the loaded setting's layer is that file; otherwise the top-level `approval` string of the `clm.toml` committed at the top of the base's tree, from `<base>:clm.toml`, `any` when that file or the key is missing. It SHALL never read the `clm.toml` of the root it runs in. When the base's `clm.toml` doesn't parse, or sets a value other than `any` and `other`, it SHALL return the error `clm land` exits 2 on (`land/approval-from-base`), whose message starts `approval from <base>:clm.toml:`.

#### Scenario: the base's policy, not the checkout's
- GIVEN a base commit whose `clm.toml` sets `approval = "other"`, a root whose own `clm.toml` sets `approval = "any"`, and no `clm.local.toml`
- WHEN the policy for that base is asked
- THEN it is `other`, from `origin/main:clm.toml`

#### Scenario: no key at the base
- GIVEN a base commit whose `clm.toml` sets no `approval`, and a root whose own `clm.toml` sets `approval = "other"`
- WHEN the policy for that base is asked
- THEN it is `any`

#### Scenario: the clone's local file wins
- GIVEN a base commit whose `clm.toml` sets `approval = "other"`, and settings loaded with `approval = "any"` from the clone's `clm.local.toml`
- WHEN the policy for that base is asked
- THEN it is `any`, from `clm.local.toml`

#### Scenario: unreadable policy at the base
- GIVEN a base commit whose `clm.toml` sets `approval = "everyone"`
- WHEN the policy for that base is asked
- THEN it is an error whose message starts `approval from origin/main:clm.toml:` and names the value

#### Serves
- capability/approve-intent
- architecture/approval-policy

### Requirement: request-refusal
`clm-approval` SHALL give its caller the refusal a request for changes makes for a branch at a fingerprint, and none when nothing holds the branch. A `changes_requested` record in the fold for the branch holds it when it names that fingerprint and `ledger`'s `Fold::cleared` is false for it, whether or not the repository is signed or the record is. The refusal SHALL be a heading line, then one `<branch>: land/changes-requested: <writer> at <at>: <text>` line per holding request, then one `<branch>: land/comment: <writer> on <target> at <at>: <text>` line per `commented` record for the branch at that fingerprint, `<target>` the requirement ID or the words `the change`, then a line saying the author pushes a change to the branch or the requester runs `clm approve <branch>`; each text trimmed, with every run of whitespace that holds a line break replaced by one space. It SHALL decide it, and match a writer's person anywhere, through `ledger`'s `cleared` and `person`, with no copy of either in `clm-approval` or `clm-land`.

#### Scenario: held
- GIVEN a fold holding a `changes_requested` record for `spec/x` at `f` by `alice/web` with the text "Name the rule", and a `commented` record on `spec/x` at `f` with target `n/r`
- WHEN the refusal for `spec/x` at `f` is asked
- THEN it holds `spec/x: land/changes-requested: alice/web at` with `Name the rule`, then a `land/comment` line naming `n/r`, and names `clm approve spec/x`

#### Scenario: another fingerprint
- GIVEN the same fold
- WHEN the refusal for `spec/x` at `g` is asked
- THEN there is none

#### Scenario: cleared by the requester
- GIVEN the same fold with an `approved` record for `f` by `alice/1a2b3c4d` with a later `at` than the request
- WHEN the refusal for `spec/x` at `f` is asked
- THEN there is none

#### Scenario: another person's approval doesn't clear it
- GIVEN the fold of `held` with a later `approved` record for `f` by `dana/9f9f9f9f`
- WHEN the refusal for `spec/x` at `f` is asked
- THEN it holds the `land/changes-requested` line naming `alice/web`

#### Scenario: a text over several lines
- GIVEN a request whose text is `a`, a line break, two spaces and `b`
- WHEN the refusal is asked
- THEN its line ends `a b`

#### Serves
- capability/follow-from-browser
- capability/approve-intent
- architecture/humans-review-intent

#### Needs
- ledger/feedback-records
- ledger/request-cleared

### Requirement: key-and-signing
`clm-approval` SHALL tell its caller whether the repository is signed at a base commit, which is whether that commit's tree holds `docs/allowed_signers`. It SHALL offer a key check that signs nothing and refuses, naming `user.signingkey`, unless git's `gpg.format` is `ssh` and `user.signingkey` is set, to a file that can be read when it is a path, taken from the top of the repository when relative. It SHALL sign a record's body under a namespace through `ledger`, `clm-approve` for an approval and `clm-land` for a land, and refuse, naming `user.signingkey`, when the key can't sign.

#### Scenario: signed and not
- GIVEN a base commit whose tree holds `docs/allowed_signers`, and another whose tree doesn't
- WHEN each is asked whether it is signed
- THEN the first is and the second isn't

#### Scenario: no key
- GIVEN a clone with no `user.signingkey`
- WHEN the key check runs
- THEN it refuses, naming `user.signingkey`, and nothing is signed or appended

#### Scenario: a key that can't be read
- GIVEN a clone whose `gpg.format` is `ssh` and whose `user.signingkey` names a file that doesn't exist
- WHEN the key check runs
- THEN it refuses, naming `user.signingkey` and the file

#### Scenario: a land's record
- GIVEN a clone whose signing key `docs/allowed_signers` lists for `christopher-lunny`
- WHEN a `landed` body is signed under `clm-land`
- THEN `ledger`'s `signed_by` accepts the signed record under `clm-land`, and not under `clm-approve`

#### Serves
- capability/approve-intent
- architecture/signed-ledger-records

#### Needs
- ledger/record-signing
- ledger/signature-check
