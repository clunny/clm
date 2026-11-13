---
title: Approval
status: active
owners: ["@clunny"]
depends_on: [ledger, gate, config, vcs, agent-harness]
paths: ["crates/clm-approval/**"]
successor: ""
serves: [capability/approve-intent, architecture/humans-review-intent]
---

## Contract
Whether a branch's diff is approved, and whether a reviewer's request holds it, for `clm land` (`land`) and `clm approve`, and the signing of approvals and lands (`signed-ledger-records`). An approval names the branch's *fingerprint*, `clm fingerprint --base origin/main` at its head, and counts only while the branch still has it (`approve-pins-the-review`). Who may approve is `config`'s `approval`, read from the base the land lands on (`approval-policy`, `land-policy-from-base`). A reviewer's `changes_requested` record for the current fingerprint, made in the web app (`web-review-feedback-as-ledger-records`), holds the branch until its fingerprint changes or the requester approves it. In a repository whose `main` holds `docs/allowed_signers`, approvals and lands are signed with the person's SSH key, and an approval counts only when its signature proves its writer. Crate `clm-approval` offers `clm-land`: a branch's head, fingerprint and change IDs, computed one way for `clm land` and `clm approve`; `approve`; the policy at a base; whether a fingerprint's approvals count, and why each doesn't; the refusal a request for changes makes; and whether a base is signed, the key check and signing a record. `clm-land` re-exports `approve`.

## Design
Crate `clm-approval` (`land-approval-split`) can't depend on `clm-land`, which calls it (`undeclared-imports`, and a cycle), so it takes the CLM root, the environment lookup, the loaded settings and the pinned base commit as arguments, runs git through `clm-git` with the remote's name from `REMOTE` (`git-plumbing-layer`), and has an error type of its own, which `clm-land`'s `Error` converts from with the same exit codes and messages. Tests that land a branch to prove these rules live in `crates/clm-land/tests` and cite this component's requirements; the others, `clm approve`'s among them, live in `crates/clm-approval/tests`.

**The branch.** The head of `<branch>` is `origin/<branch>` when it exists and the local branch isn't ahead of it, else the local branch; for a land, a local `spec/*` or `impl/*` branch with commits `origin` lacks is refused, since those are pushed deliberately. The fingerprint is `clm-gate`'s `fingerprint_of` over the `git ls-tree -r -z` listings of the head's merge base with the base and of the head, so it needs no worktree. The change IDs are those of the live `changes/<id>/change.md` the head adds or modifies relative to that merge base, sorted.

**Approved** (`approval-policy`). A fingerprint is approved when an `approved` record names it and the policy accepts it. Under `other`, the authors are the `git log --format=%an` names of the commits between the merge base and the head, slugged with `ledger`'s `slug`, so the two can't drift (`ledger/person-slug`). When approvals of the fingerprint exist but none counts, the `gate/approval` message names each and why: not signed by its writer (naming `clm approve` with a signing key; an approval made in `web` is one, since `web` can't sign), by an author, or by the lander (naming `approval = "other"`), so the human isn't left wondering why an approval they see doesn't count. With `other`, the message also says once, on its own line, `approval = "other"` from `origin/main:clm.toml` (or `clm.local.toml`), whether approvals were refused or there were none, so a human whose checkout says `any` sees why the land disagrees.

**The policy's source** (`approval-from-base`, `land-policy-from-base`). The root `clm land` runs in may be the branch's own checkout, whose `clm.toml` is the one the branch proposes. So once an attempt has pinned its base commit, the policy is the top-level `approval` string of the `clm.toml` at the top of that commit's tree (`git ls-tree --full-tree --name-only <sha> -- clm.toml` says whether it exists, `git cat-file blob <sha>:clm.toml` gives its bytes, parsed with the `toml` crate). A CLM root in a subdirectory, whose own file is `<dir>/clm.toml`, gets `any` from the base, as `ledger`'s `verify` reads `<first parent>:clm.toml` for the same commit, so the two can't disagree about a land. An `approval` that isn't the string `any` or `other` is a settings error (`verify` reads a non-string as `any`; refusing is the conservative side), exit 2, before any check runs or anything is pushed or appended, its message starting `approval from <base>:clm.toml:` followed by the parse error or the value. When the loaded `approval` setting's layer is the clone's `clm.local.toml` (`config`'s `local-settings`), its value is the policy and the base isn't read, since the clone landing owns it. Each attempt reads the policy afresh with its base, since `main` may move between attempts. `clm approve` and `clm land --check-only` read no policy.

**Held** (`changes-requested-holds`). A push that changes the diff orphans a request as it orphans an approval, and the requester clears their own by approving; `Fold::cleared` compares `at` as strings, and an equal one isn't later. `clm land` checks it after each attempt's fetch, before the gate (so before `gate/approval` could print), the lock, the merge and the checks; in a signed repository the key check comes first. The refusal prints on stdout, in `check`'s format under a heading line, one `<branch>: land/changes-requested: <writer> at <at>: <text>` per holding request, then one `<branch>: land/comment: <writer> on <target> at <at>: <text>` per `commented` record for the branch at that fingerprint, `<target>` the requirement ID or the words `the change`, each text trimmed with every run of whitespace holding a line break replaced by one space. It holds whatever the branch's name (the fold's by-branch index, `ledger/feedback-records`). A request counts whether or not the repository or the record is signed: it withholds a land rather than granting one, and `clm land` names its writer. Persons are matched by name: a web record's person is the GitHub login, so a requester whose CLI `writer` (`config`) isn't their login clears their request with `clm approve` only after setting `writer` to it, or leaves the author to push.

**`clm approve [<branch>]`** (`approve`) takes the branch's head and fingerprint as above. It is what the guard asks the human to confirm (`approve-in-session`); the command doesn't ask again.

**Signed repositories** (`signed-ledger-records`). When `ledger`'s `signed(root, <base>)` is true, `<base>` being the fetched base's commit, pinned by its SHA so a concurrent fetch can't change which signers file applies:
- *Approving* (`signed-approve`). The signing uses the namespace `clm-approve`.
- *Counting.* An `approved` record counts only when `signed_by(root, <base>, record, "clm-approve")` is true, and under `other` only when its writer's person also differs from the lander's (this clone's `writer` person), besides every author: an author's name is free text, so without this an author could name someone else as the author and approve their own branch.
- *The key check.* After each attempt's fetch, before the gate and before it takes the lock or makes a worktree, `clm land` checks that a key is configured: `user.signingkey` set with `gpg.format = ssh`, and, for a path, the file readable. When not, it exits 1 naming `user.signingkey`. It signs nothing yet, so a touch key is touched once per merge commit.
- *The land's record.* Once the gate on the merge commit has given the kind, `clm land` builds the full `landed` record and signs it under `clm-land`; a failure exits 1 and pushes nothing. Retried pushes of the same merge commit reuse it; an attempt that starts again makes a new merge commit and signs a new record; the append writes it as it is.

Without `docs/allowed_signers` at the base, nothing is signed or checked. `clm ledger verify` audits the same rules after the fact on every clone (`ledger`), which is what holds a lander who skips them.

## Requirements

### Requirement: approve
`clm approve [<branch>]` SHALL fetch, compute the branch's fingerprint against `origin/main`, append an `approved` record with that fingerprint, the branch and the change ID the branch adds or modifies (or null), and print `approved <fingerprint> <branch>`. It SHALL refuse `main` and a branch with no diff against `origin/main`.

#### Scenario: approve a spec branch
- GIVEN `spec/x` adding `changes/x/change.md`
- WHEN `clm approve spec/x` runs
- THEN the writer's ledger holds `approved` with the branch's fingerprint, `branch` `spec/x` and `change` `x`, and it printed `approved <fingerprint> spec/x`

#### Scenario: nothing to approve
- GIVEN a branch at `origin/main`'s tip
- WHEN `clm approve` runs on it
- THEN it exits 1 and appends nothing

#### Serves
- capability/approve-intent
- architecture/humans-review-intent

### Requirement: signed-approve
When `origin/main`'s tree holds `docs/allowed_signers`, `clm approve` SHALL append its `approved` record signed with `ledger`'s `sign` under the namespace `clm-approve`, and SHALL exit 1 naming `user.signingkey` and append nothing when it can't sign. Otherwise it SHALL append the record unsigned.

#### Scenario: signed approval
- GIVEN `origin/main` holding `docs/allowed_signers`, a clone whose `user.signingkey` is an ed25519 key with `gpg.format = ssh`, and a branch `spec/x` with a diff
- WHEN `clm approve spec/x` runs
- THEN the writer's ledger holds an `approved` record for the branch's fingerprint whose `sig` `signed_by` accepts under `clm-approve` against that file when it lists the key for the writer's person

#### Scenario: no key
- GIVEN the same, with no `user.signingkey`
- WHEN `clm approve spec/x` runs
- THEN it exits 1 naming `user.signingkey` and the ledger is unchanged

#### Scenario: unsigned repository
- GIVEN `origin/main` without `docs/allowed_signers` and no `user.signingkey`
- WHEN `clm approve spec/x` runs
- THEN it appends an `approved` record without `sig`

#### Serves
- capability/approve-intent
- architecture/signed-ledger-records

#### Needs
- ledger/record-signing

### Requirement: approval-policy
With `approval = "other"`, `clm land` SHALL count an `approved` record only when its writer's person differs from the slugged author name of every commit between the merge base and the head, and SHALL exit 1 naming the policy when the only approvals are the authors' own. With `approval = "any"` it SHALL count any `approved` record for the fingerprint.

#### Scenario: self-approval refused
- GIVEN `approval = "other"`, a branch that needs review whose commits are authored by `Christopher Lunny`, and an `approved` record by writer `christopher-lunny/1a2b3c4d`
- WHEN `clm land` runs
- THEN it exits 1 naming `approval = "other"` and `main` is unchanged

#### Scenario: another person approves
- GIVEN the same, with an `approved` record by writer `dana/9f9f9f9f`
- WHEN `clm land` runs
- THEN it lands

#### Scenario: any
- GIVEN `approval = "any"` and the self-approval of the first scenario
- WHEN `clm land` runs
- THEN it lands

#### Serves
- capability/approve-intent
- architecture/approval-policy

#### Needs
- ledger/person-slug

### Requirement: approval-from-base
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
- architecture/approval-policy

### Requirement: signed-land
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

### Requirement: changes-requested-holds
`clm land` SHALL exit 1, pushing nothing, appending nothing and running no gate or check, when a `changes_requested` record in the fold for the branch names the head's fingerprint and `ledger`'s `Fold::cleared` is false for it (no `approved` record for that fingerprint by the same writer's person has a later `at`, compared as strings, whether or not that approval is one the policy counts), printing one `land/changes-requested` line per such request with its writer, time and text, then the branch's `commented` records at that fingerprint as `land/comment` lines, and saying the author pushes a change or the requester runs `clm approve <branch>`. It SHALL count such a request whether or not the repository is signed or the record is. It SHALL decide it, and match a writer's person anywhere, through `ledger`'s `cleared` and `person`, with no copy of either in `clm-land`. It SHALL land as before when every request for the branch names another fingerprint, or its writer's person approved the fingerprint after it.

#### Scenario: held
- GIVEN a branch `spec/x` whose fingerprint is `f`, an `approved` record for `f` by `bob/web`, a `changes_requested` record for `spec/x` at `f` by `alice/web` with the text "Name the rule", and a `commented` record on `spec/x` at `f` with target `n/r`
- WHEN `clm land spec/x` runs
- THEN it exits 1 printing `land/changes-requested` naming `alice/web` and `Name the rule`, then `land/comment` naming `n/r`, and `clm approve spec/x`; no check ran, and `main` on `origin` is unchanged

#### Scenario: cleared by a push
- GIVEN the same, after the author pushed a commit that changes a spec file, so the fingerprint is `g`, and an `approved` record for `g`
- WHEN `clm land spec/x` runs
- THEN it lands

#### Scenario: cleared by the requester
- GIVEN the held branch, and an `approved` record for `f` by `alice/1a2b3c4d` with a later `at` than the request
- WHEN `clm land spec/x` runs
- THEN it lands

#### Scenario: another person's approval doesn't clear it
- GIVEN the held branch with `approval = "any"`, and a later `approved` record for `f` by `dana/9f9f9f9f`
- WHEN `clm land spec/x` runs
- THEN it exits 1 printing `land/changes-requested`

#### Scenario: unsigned request in a signed repository
- GIVEN a `main` whose tip holds `docs/allowed_signers`, a branch `spec/x` at fingerprint `f` with a signed `approved` record by `dana/9f9f9f9f` that counts, and an unsigned `changes_requested` record for `spec/x` at `f` by `alice/web`
- WHEN `clm land spec/x` runs
- THEN it exits 1 printing `land/changes-requested` naming `alice/web`

#### Serves
- capability/follow-from-browser
- capability/approve-intent
- architecture/humans-review-intent

#### Needs
- ledger/feedback-records
- ledger/request-cleared

### Requirement: git-through-layer
`clm-approval` SHALL list `clm-git` among its dependencies, SHALL run no `git` or `gh` process itself, and SHALL build every argument that names the remote to git from `clm-git`'s `REMOTE`: no line of a file under `crates/clm-approval/src`, apart from those whose first non-blank characters are `//`, SHALL hold `Command::new("git")`, `Command::new("gh")`, `"origin"`, `"origin/` or `remotes/origin`. `clm-land` SHALL keep no copy of what it calls here: no file under `crates/clm-land/src` SHALL define a function named `fingerprint`, `held`, `policy` or `slug`.

#### Scenario: covered by the scan
- GIVEN the workspace
- WHEN a test in `clm-approval` reads `crates/clm-approval/Cargo.toml`, every file under `crates/clm-approval/src` and every file under `crates/clm-land/src`
- THEN `clm-git` is a dependency, no line of `clm-approval`'s sources outside a `//` comment holds `Command::new("git")`, `Command::new("gh")`, `"origin"`, `"origin/` or `remotes/origin`, and no file of `clm-land`'s defines `fn fingerprint`, `fn held`, `fn policy` or `fn slug`

#### Serves
- architecture/reuse-first

### Requirement: approve-retries-fetch
`clm approve` SHALL fetch `origin` through `clm-git`'s `fetch_retrying` (`vcs/fetch-retries`). When that returns `FetchError::Offline` it SHALL exit 1 saying it is offline, as it does today; when it returns `FetchError::Git` it SHALL exit 1 with git's message from the last fetch and no `offline`. Either way it SHALL append nothing.

#### Scenario: an approve whose fetch loses a ref race
- GIVEN `spec/x` adding `changes/x/change.md`, and a script named `git` first on `PATH` that, for the first `fetch` naming `origin`, prints `error: cannot lock ref 'refs/remotes/origin/main'` on stderr and exits 1, and runs the real `git` for every other call
- WHEN `clm approve spec/x` runs
- THEN it prints `approved <fingerprint> spec/x`, the writer's ledger holds the `approved` record, and its stderr doesn't contain `offline`

#### Scenario: an approve whose fetch keeps failing with origin reachable
- GIVEN the same, with a `git` that fails every `fetch` naming `origin` that way
- WHEN `clm approve spec/x` runs
- THEN it exits 1, its message contains `cannot lock ref` and not `offline`, and the ledger has no new record

#### Needs
- vcs/fetch-retries

#### Serves
- capability/approve-intent
- capability/work-concurrently

### Requirement: approve-refuses-unconfirming-agent-harness
`clm approve` SHALL, before it fetches or appends anything, ask `agent-harness`'s `detect` which agent harness its environment names, and when that agent harness's `abilities` hold `confirm` false and the agent harness isn't `None`, SHALL exit 1, appending nothing, with a message naming the agent harness and `approval-policy` and saying the human approves by running the same `clm approve` command in their own terminal, outside any agent. Under an agent harness whose `confirm` is true, or `None`, it SHALL go on as `approve` says. It SHALL read the markers from the environment lookup `approve` is given, not from the process's environment. The tests, capability tests included, that run `clm approve` SHALL remove both agent harnesses' markers from its environment unless a scenario sets them, so the suite passes from inside any agent harness; this requirement's build makes that change to them.

#### Scenario: under Codex
- GIVEN Codex's marker set in `clm approve spec/x`'s environment and a branch with a diff
- WHEN it runs
- THEN it exits 1, the writer's ledger is unchanged, and the message names `codex`, `approval-policy` and `own terminal`

#### Scenario: in the human's terminal
- GIVEN neither agent harness's marker set
- WHEN `clm approve spec/x` runs on a branch with a diff
- THEN it appends the `approved` record as `approve` says

#### Scenario: under Claude Code
- GIVEN `CLAUDECODE=1` set
- WHEN `clm approve spec/x` runs on a branch with a diff
- THEN it appends the `approved` record, the guard having asked the human first

#### Needs
- agent-harness/detect
- agent-harness/abilities

#### Serves
- capability/approve-intent
- capability/work-from-any-agent-harness
- architecture/approval-policy
- architecture/agent-harness-adapters
