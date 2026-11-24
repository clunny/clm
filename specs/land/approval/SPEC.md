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
Whether a branch's diff is approved, and whether a reviewer's request holds it, for `clm land` (`land`) and `clm approve`, and the signing of approvals and lands (`signed-ledger-records`). An approval names the branch's *fingerprint* at its head (`gate.fingerprint/fingerprint`, against `origin/main`) and counts only while the branch still has it (`approve-pins-the-review`); it also names the requests for changes it clears (`ledger/request-cleared`). Who may approve is `config`'s `approval`, read from the base the land lands on (`approval-policy`, `land-policy-from-base`). A reviewer's `changes_requested` record for the current fingerprint, made in the web app (`web-review-feedback-as-ledger-records`), holds the branch until its fingerprint changes or the requester approves it with the request named. In a repository whose `main` holds `docs/allowed_signers`, approvals and lands are signed with the person's SSH key, and an approval counts only when its signature proves its writer.

Crate `clm-approval` offers `clm-land`:
- `Repo::new(root, env, settings)`: the repository at a CLM root, with the environment as a lookup (`Env`) and the loaded settings. On it: `branch_head(branch, for_land)`, a branch's head and `origin`'s tip of it when `origin` holds it, as `BranchHead { head, origin }`, by one rule for `clm land` and `clm approve` (`branch-head`); `branch_name(branch, verb)`, the branch named or the current one, refusing `main`; `rev(rev)`, the commit a revision names, or none; `fingerprint(base, head)`, and `listing(rev)`, a revision's tree entries as the fingerprint reads them; `changes(head)`, the change IDs a head adds or modifies (`branch-head`); `approval(fold, fingerprint, head, signed, other)`, whether a fingerprint's approvals count, as `Approvals { accepted, refused }`, each refused one with why (`approvals-counted`); `policy(base)`, the policy at a base and where it came from, as `Policy { other, source }` (`policy-at-base`); `signed(base)`, `require_key()` and `sign(body, namespace)` (`key-and-signing`); and `held(fold, branch, fingerprint, signed)`, the refusal a request for changes makes, or none, `signed` being the base commit when the repository is signed there (`request-refusal`).
- `approve(root, branch, env)`: what `clm approve` does, returning `Output { lines, warnings }`, the lines it prints on standard output and the warnings it writes on standard error (`approve`).
- `APPROVE` and `LAND`: `clm-approve` and `clm-land`, the namespaces an approval and a land are signed under.
- `Error`: `Refused`, nothing done for the reason given, which the commands exit 1 on, or `Git`, git, the ledger or the settings failed, which they exit 2 on. `clm-land`'s error converts from it with the same exits and messages.

`clm-approval` offers `clm approve` as a call, `approve_command(args, root, cwd, env, out, err)` (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the current directory, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status: 0 when it approved, 1 when it refused, 2 for a usage error or a failure of git, the ledger or the settings. Its arguments are `ApproveArgs`, which `crates/clm`'s parser reads, and whose `root()` is the `--root` the command line gives. `crates/clm` routes the command to it and adds nothing to what it writes.

## Design
Crate `clm-approval` (`land-approval-split`) can't depend on `clm-land`, which calls it (`undeclared-imports`, and a cycle), so it takes the CLM root, the environment lookup, the loaded settings and the pinned base commit as arguments, runs git through `clm-git` with the remote's name from `REMOTE` (`git-plumbing-layer`), and has an error type of its own. What `clm land` does with these answers, its exits and messages included, is `land`'s to state and to prove by landing a branch (`land/land`, `land/approval-policy`, `land/approval-from-base`, `land/changes-requested-holds`, `land/signed-land`), since this crate can land nothing (`commands-live-in-their-crates`); this component states what its calls answer.

**The branch** (`branch-head`). A branch's head is read from two refs, `refs/heads/<branch>` and `refs/remotes/origin/<branch>` as the caller's fetch left them, with `git rev-parse --verify --quiet`, and `clm-git`'s `is_ancestor` says which one is an ancestor of the other. The only name read is `main`, which `branch_name` refuses, since it is the base and never a branch to approve or land. A land takes the clone's head, and only when that head has everything `origin` holds of the branch: `origin`'s tip is the head or an ancestor of it. So a claim pushed early whose work is still local lands from the clone, as a `propose/*` branch has, and `clm land` then deletes the branch from `origin` leased on `origin`'s own tip (`land/landed-branch-cleanup`), which the landed head contains; a branch `origin` doesn't hold lands from the clone and nothing is deleted. A local branch behind `origin`'s, or diverged from it, would leave out what someone pushed; each gets the advice that fits it. An approve takes `origin`'s tip whenever the local branch is absent or an ancestor of it, so a reviewer approves what `origin` shows. Since `clm land` deletes every branch it lands from `origin`, other clones would often keep a tracking ref of a branch `origin` no longer holds; `clm approve`'s fetch prunes them (`git fetch --prune`), as `clm land`'s must (`land`), so a stale ref neither turns a land away nor gives an approve a tip `origin` no longer has. The fingerprint is `clm-gate`'s `fingerprint_of` over the `git ls-tree -r -z` listings of the head's merge base with the base commit and of the head, so it needs no worktree. The change IDs are `ledger`'s `change_ids` over the paths `git diff --name-only --no-renames --no-relative -z --diff-filter=AMT` lists between the merge base and the head under `changes/`, split on NUL bytes: with renames off a renamed change directory counts as added, with `-z` a non-ASCII ID isn't quoted away, with `--no-relative` a user's `diff.relative` doesn't empty the list, and a type change counts as a change, so they are the paths a tree listing shows differing, as `web.worker`'s approve reads them. The merge base for the change IDs and for the authors (below) is taken with the `origin/main` ref rather than the pinned commit; the two differ only when `main` moved after the pin, and then the land's push, leased on the pinned commit, fails and the land starts again from a new fetch.

**Approved** (`approvals-counted`, `approval-policy`). Under `other`, the authors are the `git log --format=%an` names of the commits between the merge base and the head, slugged as `ledger/person-slug` slugs, so the two can't drift; the lander, this clone's `writer` person, is read only in a signed repository under `other`. Every approval that doesn't count gets its reason, in the fold's order, so that `land` can tell a human why an approval they see doesn't count.

**The policy's source** (`policy-at-base`, `land-policy-from-base`). The root `clm land` runs in may be the branch's own checkout, whose `clm.toml` is the one the branch proposes. So the policy is the top-level `approval` string of the `clm.toml` at the top of the pinned base commit's tree (`git ls-tree --full-tree --name-only <sha> -- clm.toml` says whether it exists, `git cat-file blob <sha>:clm.toml` gives its bytes, parsed with the `toml` crate), its source named `origin/main:clm.toml` since that commit is the `origin/main` fetched. A CLM root in a subdirectory, whose own file is `<dir>/clm.toml`, gets `any` from the base, as `ledger`'s `verify` reads `<first parent>:clm.toml` for the same commit, so the two can't disagree about a land. Where `verify` reads a value that isn't a string as `any`, `policy-at-base` takes the conservative side. When the loaded `approval` setting's layer is the clone's `clm.local.toml` (`config`'s `local-settings`), its value is the policy and the base isn't read, since the clone landing owns it. Each land attempt asks afresh with its base, since `main` may move between attempts; `clm approve` reads no policy.

**Held** (`request-refusal`). A push that changes the diff orphans a request as it orphans an approval. The requester clears their own by approving the fingerprint with the request named: `clm approve` names every request of theirs it finds after its fetch (`approve`), and the web app's approve names those in their web ledger (`web.worker.review/approve`). Whether a request is cleared is `ledger`'s rule, the one the web app shows too, and it compares no clocks: each record's `at` is its own machine's. In a repository signed at the base, `Repo::held` asks `ledger`'s `cleared_at`, which counts only an approval whose signature checks, since an unsigned line can carry the requester's name. A request counts whether or not the repository or the record is signed: it withholds a land rather than granting one. Persons are matched by name: a web record's person is the GitHub login, so a requester whose CLI `writer` (`config`) isn't their login clears their request with `clm approve` only after setting `writer` to it, or leaves the author to push.

**`clm approve [<branch>]`** (`approve`). It is what the guard asks the human to confirm (`approval-policy`); the command doesn't ask again. It reads the agent harness from its lookup first (`approve-refuses-unconfirming-agent-harness`), then the branch, fetches, takes the head and the fingerprint against the commit of `origin/main` fetched, folds the ledger (`ledger`'s `fold`) for the requests it names, and appends the record, signed when that commit is signed. An approval that `origin` hasn't taken counts on no other clone until it is pushed.

**Signed repositories** (`signed-ledger-records`). Whether the repository is signed is read at the pinned base commit, by its SHA, so a concurrent fetch can't change which signers file applies. An approval is signed under `clm-approve` and a land under `clm-land`, through `ledger`'s `sign`. Counting checks each `approved` record's line as the fold read it (`Fold::lines`) with `ledger`'s `signed_by`, so a field a newer `clm` added still verifies, and under `other` also leaves out the lander's own approval: an author's name is free text, so without this an author could name someone else as the author and approve their own branch. The key check reads `gpg.format` and `user.signingkey` (`git config --get --type=path`, so `~/` expands) with the lookup's `HOME`, `XDG_CONFIG_HOME` and `GIT_CONFIG_GLOBAL`, as `ledger`'s `sign` reads them, so the check and the signing agree on the key; it opens a path's file to see that it can be read and signs nothing, so a touch key is touched once per merge commit. Without `docs/allowed_signers` at the base, nothing is signed or checked. `clm ledger verify` audits the same rules after the fact on every clone (`ledger`), which is what holds a lander who skips them.

## Requirements

### Requirement: approve
`clm approve [<branch>]` SHALL fetch, pruning the tracking refs of branches `origin` no longer holds, take the branch's head and its fingerprint against `origin/main` (`branch-head`), append an `approved` record with that fingerprint, the branch, the change ID the branch adds or modifies (or null; the first in sorted order when it adds or modifies several) and the requests it clears, and print `approved <fingerprint> <branch>`. The requests it clears SHALL be every `changes_requested` record the fold holds after its fetch for that branch at that fingerprint whose writer's person is the approver's (`ledger/request-cleared`), whether or not an earlier approval cleared it, and none, the record still naming an empty list, when there is none. It SHALL exit 1, appending nothing, for `main`, for a branch with no diff against `origin/main`, when no branch is named and `HEAD` is detached, when neither a local branch nor `origin/<branch>` has the name, and when `origin` has no `main`. When the record is appended but `origin` doesn't take it, it SHALL still print `approved <fingerprint> <branch>` and exit 0, and SHALL write on standard error a line starting `clm approve: warning: ` that says the approval isn't on `origin` yet and names `git push origin clm/ledger/<writer>`, `<writer>` the approver's.

#### Scenario: approve a spec branch
- GIVEN `spec/x` adding `changes/x/change.md`
- WHEN `clm approve spec/x` runs
- THEN the writer's ledger holds `approved` with the branch's fingerprint, `branch` `spec/x` and `change` `x`, and it printed `approved <fingerprint> spec/x`

#### Scenario: nothing to approve
- GIVEN a branch at `origin/main`'s tip
- WHEN `clm approve` runs on it
- THEN it exits 1 and appends nothing

#### Scenario: a branch that can't be approved
- GIVEN in turn: `main` named; `main` checked out and no branch named; `HEAD` detached and no branch named; a name that no local branch and no `origin/<branch>` has; and an `origin` with no `main`, approving a branch with a diff
- WHEN `clm approve` runs in each
- THEN each exits 1 with `clm approve: ` and why on standard error, nothing on standard output and the ledger unchanged, the first two naming `` `main` ``, the third saying to name the branch, the fourth naming the name and the last naming `origin/main`

#### Scenario: a branch origin deleted
- GIVEN a local `spec/x` at `c1` with a diff, and a tracking ref `origin/spec/x` at `c2`, a descendant of `c1`, for a branch `origin` has since deleted
- WHEN `clm approve spec/x` runs
- THEN the `approved` record's fingerprint is `c1`'s, and the clone has no `origin/spec/x`

#### Scenario: a branch with two changes
- GIVEN `spec/x` adding `changes/b/change.md` and `changes/a/change.md`
- WHEN `clm approve spec/x` runs
- THEN the `approved` record's `change` is `a`

#### Scenario: approve over a request
- GIVEN `spec/x` at fingerprint `f`, an approver whose writer is `christopher-lunny/1a2b3c4d`, and on `origin` `changes_requested` records `r1` for `spec/x` at `f` by `christopher-lunny/web`, `r2` for `spec/x` at `f` by `dana/web`, `r3` for `spec/x` at another fingerprint by `christopher-lunny/web`, and `r4` for `spec/y` at `f` by `christopher-lunny/web`, none of them in the clone before the fetch
- WHEN `clm approve spec/x` runs, and again for a `spec/z` that no request names
- THEN the first `approved` record's `clears` holds `r1` alone, and the second's is an empty list

#### Scenario: an approval not yet on origin
- GIVEN `spec/x` with a diff, and an `origin` whose `pre-receive` hook refuses every push to `clm/ledger/*`
- WHEN `clm approve spec/x` runs
- THEN it prints `approved <fingerprint> spec/x` and exits 0, the local ledger branch holds the record, and standard error holds a line starting `clm approve: warning: ` naming `git push origin clm/ledger/` and the approver's writer

#### Serves
- capability/approve-intent
- architecture/humans-review-intent

#### Needs
- ledger/request-cleared

### Requirement: signed-approve
When `origin/main`'s tree holds `docs/allowed_signers`, `clm approve` SHALL append its `approved` record signed as `ledger/record-signing` signs, under the namespace `clm-approve`, and SHALL exit 1 naming `user.signingkey` and append nothing when it can't sign. Otherwise it SHALL append the record unsigned.

#### Scenario: signed approval
- GIVEN `origin/main` holding `docs/allowed_signers`, a clone whose `user.signingkey` is an ed25519 key with `gpg.format = ssh`, and a branch `spec/x` with a diff
- WHEN `clm approve spec/x` runs
- THEN the writer's ledger holds an `approved` record for the branch's fingerprint whose `sig` `ledger/signature-check` accepts under `clm-approve` against that file when it lists the key for the writer's person

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

### Requirement: approve-retries-fetch
`clm approve` SHALL fetch `origin` as `vcs/fetch-retries` states, trying again after a failure. When `origin` can't be reached it SHALL exit 1 with a message that says it is `offline` and that the approver runs `clm approve` again once `origin` can be reached, saying nothing of pushing or landing; when the fetch keeps failing with `origin` reachable it SHALL exit 1 with git's message from the last fetch, which doesn't say `offline`. Either way it SHALL append nothing.

#### Scenario: an approve whose fetch loses a ref race
- GIVEN `spec/x` adding `changes/x/change.md`, and a script named `git` first on `PATH` that, for the first `fetch` naming `origin`, prints `error: cannot lock ref 'refs/remotes/origin/main'` on stderr and exits 1, and runs the real `git` for every other call
- WHEN `clm approve spec/x` runs
- THEN it prints `approved <fingerprint> spec/x`, the writer's ledger holds the `approved` record, and its stderr doesn't contain `offline`

#### Scenario: an approve whose fetch keeps failing with origin reachable
- GIVEN the same, with a `git` that fails every `fetch` naming `origin` that way
- WHEN `clm approve spec/x` runs
- THEN it exits 1, its message contains `cannot lock ref` and not `offline`, and the ledger has no new record

#### Scenario: an approve with origin unreachable
- GIVEN `spec/x` adding `changes/x/change.md`, and `origin`'s URL a path that doesn't exist
- WHEN `clm approve spec/x` runs
- THEN it exits 1, its message contains `offline` and `clm approve` and neither `push` nor `land`, and the ledger has no new record

#### Needs
- vcs/fetch-retries

#### Serves
- capability/approve-intent
- capability/work-concurrently

### Requirement: approve-refuses-unconfirming-agent-harness
`clm approve` SHALL, before it fetches or appends anything, read from its environment lookup which agent harness it runs under (`agent-harness/detect`), and when that is a recognised agent harness whose `confirm` ability is false (`agent-harness/abilities`) SHALL exit 1, appending nothing, with a message naming the agent harness and `approval-policy` and saying the human approves by running the same `clm approve` command in their own terminal, outside any agent. Under an agent harness whose `confirm` is true, or none, it SHALL go on as `approve` says. It SHALL read the markers from the environment lookup `approve` is given, not from the process's environment.

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
- THEN it appends the `approved` record

#### Needs
- agent-harness/detect
- agent-harness/abilities

#### Serves
- capability/approve-intent
- capability/work-from-any-agent-harness
- architecture/approval-policy
- architecture/agent-harness-adapters

### Requirement: commands-in-crate
`clm-approval` SHALL offer `clm approve` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, the current directory, the environment as a lookup, and a stream each for standard output and standard error; the command's `--root` overrides the root given, a relative one taken from the current directory given. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself: 0 when it approved, 1 when the command refuses and 2 when git, the ledger or the settings fail, writing each refusal and failure as `clm approve: <why>` on the error stream and nothing on the output stream. For arguments it can't parse it SHALL write the usage error to the error stream and return 2. Given `--help` it SHALL write on the output stream the command's help, its first line `Record your approval of a branch's current diff in the ledger`, then an empty line, then `Usage: clm approve [OPTIONS] [BRANCH]`, write nothing on the error stream, and return 0.

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `land.approval`'s own files
- WHEN it calls `clm-approval`'s call with the arguments of `clm approve` and two buffers
- THEN the buffers hold what `clm approve` prints to standard output and to standard error there, and the call returns the status `clm approve` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm approve` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Scenario: a refusal and a failure
- GIVEN the call asked to approve `main`, and the call given a `--root` that doesn't exist
- WHEN each runs
- THEN the first returns 1 and the second 2, each with nothing in the output buffer and an error buffer that starts `clm approve: `

#### Scenario: help
- GIVEN the arguments `--help`
- WHEN the call runs with them
- THEN the output buffer's first line is `Record your approval of a branch's current diff in the ledger`, an empty line and `Usage: clm approve [OPTIONS] [BRANCH]` follow, the error buffer is empty, and it returns 0

#### Scenario: a relative root
- GIVEN a root given that doesn't exist, a current directory `d`, and a repository with a branch to approve at `d/work`
- WHEN the call runs with `--root work`
- THEN it approves in `d/work`, returning 0 with `approved ` on the output buffer

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

### Requirement: approvals-counted
`clm-approval` SHALL tell its caller whether a fingerprint's approvals count under a policy, and, when none counts, for each `approved` record its writer and time and why it doesn't. Under `any` it SHALL count any `approved` record for the fingerprint. Under `other` it SHALL count a record only when its writer's person differs from the author name, slugged as `ledger/person-slug` slugs it, of every commit between the head's merge base with `origin/main` and the head, saying of one that doesn't that its writer is one of the branch's authors and naming `approval = "other"`. In a repository signed at the base it SHALL count a record only when `ledger/signature-check` accepts its line, as the fold read it, under `clm-approve` against that base's `docs/allowed_signers`, saying of one that doesn't that it isn't signed by its writer and naming `clm approve`, and, under `other`, only when its writer's person also differs from the writer person of the clone it runs in, the lander's, saying of one that doesn't that its writer is the lander. A fingerprint no `approved` record names SHALL NOT count, with no reason.

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
`clm-approval` SHALL give its caller the `approval` policy for a land onto a base commit, with where it came from: the value the clone's `clm.local.toml` sets, from `clm.local.toml`, when the loaded setting's layer is that file; otherwise the top-level `approval` string of the `clm.toml` committed at the top of the base commit's tree, from `origin/main:clm.toml`, `any` when that file or the key is missing. It SHALL never read the `clm.toml` of the root it runs in. When the base's `clm.toml` doesn't parse, or sets a value other than `any` and `other`, it SHALL return the error `Git`, which `land/approval-from-base` exits 2 on, whose message starts `approval from origin/main:clm.toml:`.

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
`clm-approval` SHALL give its caller the refusal a request for changes makes for a branch at a fingerprint, and none when nothing holds the branch. A `changes_requested` record in the fold for the branch holds it when it names that fingerprint and `ledger/request-cleared` doesn't count it as cleared, whether or not the record is signed; given a base commit at which the repository is signed, it SHALL count as clearing only an approval whose signature checks there, as `ledger/request-cleared`'s `cleared_at` does. The refusal SHALL be a heading line, then one `<branch>: land/changes-requested: <writer> at <at>: <text>` line per holding request, then one `<branch>: land/comment: <writer> on <target> at <at>: <text>` line per `commented` record for the branch at that fingerprint, `<target>` the requirement ID or the words `the change`, then a line saying the author pushes a change to the branch or the requester runs `clm approve <branch>`; each text trimmed, with every run of whitespace that holds a line break replaced by one space.

#### Scenario: held
- GIVEN a fold holding a `changes_requested` record for `spec/x` at `f` by `alice/web` with the text "Name the rule", and a `commented` record on `spec/x` at `f` with target `n/r`
- WHEN the refusal for `spec/x` at `f` is asked
- THEN it holds `spec/x: land/changes-requested: alice/web at` with `Name the rule`, then a `land/comment` line naming `n/r`, and names `clm approve spec/x`

#### Scenario: another fingerprint
- GIVEN the same fold
- WHEN the refusal for `spec/x` at `g` is asked
- THEN there is none

#### Scenario: cleared by the requester
- GIVEN the same fold with an `approved` record for `f` by `alice/1a2b3c4d` that names the request as one it clears
- WHEN the refusal for `spec/x` at `f` is asked
- THEN there is none

#### Scenario: another person's approval doesn't clear it
- GIVEN the fold of `held` with an `approved` record for `f` by `dana/9f9f9f9f` that names the request as one it clears
- WHEN the refusal for `spec/x` at `f` is asked
- THEN it holds the `land/changes-requested` line naming `alice/web`

#### Scenario: an unsigned approval in a signed repository
- GIVEN the fold of `held` with an unsigned `approved` record for `f` by `alice/1a2b3c4d` that names the request, a base whose `docs/allowed_signers` lists a key for `alice`, and the same fold with that approval signed by that key
- WHEN the refusal for `spec/x` at `f` is asked with that signed base, for each fold
- THEN the first holds the `land/changes-requested` line naming `alice/web` and the second gives none

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
`clm-approval` SHALL tell its caller whether the repository is signed at a base commit, which is whether that commit's tree holds `docs/allowed_signers`. It SHALL offer a key check that signs nothing and refuses, naming `user.signingkey`, unless git's `gpg.format` is `ssh` and `user.signingkey` is set, either to a public key given inline, starting `key::`, which passes with no file read, or to a path of a file that can be read, taken from the top of the repository when relative. It SHALL read git's configuration for the check with `HOME`, `XDG_CONFIG_HOME` and `GIT_CONFIG_GLOBAL` taken from the environment lookup where it gives them, as `ledger/record-signing` reads it. It SHALL sign a record's body under a namespace as `ledger/record-signing` signs, `clm-approve` for an approval and `clm-land` for a land, and refuse, naming `user.signingkey`, when the key can't sign.

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
- THEN `ledger/signature-check` accepts the signed record under `clm-land`, and not under `clm-approve`

#### Scenario: a key given inline
- GIVEN a clone whose `gpg.format` is `ssh` and whose `user.signingkey` is `key::` followed by an ed25519 public key, no file of that name existing
- WHEN the key check runs
- THEN it passes

#### Scenario: the key in a global config
- GIVEN a clone whose own config sets neither `gpg.format` nor `user.signingkey`, a file that sets `gpg.format = ssh` and `user.signingkey` to a readable key, named by the lookup's `GIT_CONFIG_GLOBAL`, and the process's own `GIT_CONFIG_GLOBAL` naming an empty file
- WHEN the key check runs
- THEN it passes

#### Serves
- capability/approve-intent
- architecture/signed-ledger-records

#### Needs
- ledger/record-signing
- ledger/signature-check

### Requirement: branch-head
`clm-approval` SHALL give `clm land` and `clm approve` a branch's head by one rule, whatever the branch's name, `origin` holding the branch exactly when `refs/remotes/origin/<branch>` exists as the caller's fetch left it, and with the head SHALL give `origin`'s tip of the branch when `origin` holds it. When `origin` doesn't hold the branch the head SHALL be the local branch, and when neither has it, a refusal naming the branch. When `origin` holds it and there is no local branch the head SHALL be `origin`'s tip. Asked for an approve, it SHALL give `origin`'s tip when the local branch is that tip or an ancestor of it, and the local branch otherwise. Asked for a land, it SHALL give the local branch when `origin`'s tip is the local branch or an ancestor of it, and otherwise SHALL refuse, naming the branch: when the local branch is an ancestor of `origin`'s tip, saying it is behind `origin/<branch>` and to bring it up to `origin/<branch>`, then land it; when neither is an ancestor of the other, saying it has diverged from `origin/<branch>` and to rebase it onto `origin/<branch>` or merge `origin/<branch>` into it, then land it. The fingerprint SHALL be `gate.fingerprint/fingerprint`'s over the trees of the head's merge base with the base commit given and of the head. The change IDs SHALL be what `ledger/change-ids` gives for the paths the head adds, modifies or changes the type of relative to its merge base with `origin/main`, a renamed path counting as added, so a deleted `change.md`, one under `changes/archive/` and one deeper than `changes/<id>/` SHALL NOT name one.

#### Scenario: which head
- GIVEN in turn: a local branch at `origin/<branch>`'s tip; a local branch ahead of it, once named `impl/a` and once `fix-x`; a local branch behind it; a local branch diverged from it; no local branch and `origin/<branch>`; a local branch `origin` doesn't hold, once named `spec/y` and once `fix-y`; a local branch whose tracking ref a pruning fetch removed after `origin` deleted the branch; and a name neither has
- WHEN its head is asked for an approve and for a land
- THEN at the tip, both give that tip with `origin`'s tip; ahead, either name, both give the local tip with `origin`'s tip; behind, an approve gives `origin`'s tip and a land is refused with a message naming the branch and saying it is behind; diverged, an approve gives the local tip and a land is refused with a message naming the branch and saying it has diverged; no local branch gives `origin`'s tip for both; a branch `origin` doesn't hold, either name, and the pruned one give the local tip for both, with no `origin` tip; and the last is refused for both, naming the branch

#### Scenario: a branch's change IDs
- GIVEN a head that, relative to its merge base with `origin/main`, adds `changes/b/change.md`, modifies `changes/a/change.md`, deletes `changes/c/change.md`, adds `changes/archive/d/change.md`, adds `changes/e/notes/change.md`, adds `changes/a-b/change.md`, and renames `changes/old/` to `changes/new/`
- WHEN its change IDs are asked
- THEN they are `a`, `a-b`, `b` and `new`, in that order

#### Needs
- ledger/change-ids
