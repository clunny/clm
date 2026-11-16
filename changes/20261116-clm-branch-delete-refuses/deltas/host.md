---
component: host
base: { design: "bbcbf924c2b1d21d" }
---

## MODIFIED Design
Crate `clm-host`. One trait, `Host`, with these operations, and an implementation per setting. `github` runs `gh pr create --base main --head <branch> --title --body-file`, `gh pr view --json number,state,headRefName` and `gh api -X PATCH repos/{owner}/{repo}/pulls/<n> -F body=@<file>` (the REST call, since `gh pr edit` fails on `gh` versions that query the retired Projects (classic) API), and `gh issue create`. `branch delete` is `git push --force-with-lease=refs/heads/<branch>:<sha> origin :refs/heads/<branch>` for every host, `<sha>` being `--lease` or, without it, the tip of `refs/remotes/origin/<branch>`.

**Azure DevOps** (`azure-pull-requests`, `azure-through-az-cli`). `azure` runs the Azure CLI through `clm-git`'s `run`, with `--detect true` so `az` takes the organisation, project and repository from the clone's remote, and `--output json`; `az` holds the sign-in (`az login`, or `AZURE_DEVOPS_EXT_PAT`), so CLM reads no token. `pr open` first runs `az repos pr list --source-branch <branch> --target-branch main --status active` and, when it lists a pull request, prints the first one's URL; otherwise it runs `az repos pr create --source-branch <branch> --target-branch main --title <title> --description @<file>`, the file's path made absolute against the caller's directory, since `az` reads `@<file>` relative to the directory it runs in, the repository's root. Azure's `--source-branch` takes the bare branch name. The URL printed is the pull request's web page, `<repository.webUrl>/pullrequest/<pullRequestId>`, from the JSON `az` returns, since the `url` field there is the REST API's. `pr body` finds the current branch's active pull request the same way and runs `az repos pr update --id <pullRequestId> --description @<file>`. Every other operation is the trait's default, as for `git`: `issue` prints the report and says there is nowhere to file it, `parent` returns nothing, `list` and `alerts` return nothing, and the rest report that there is no such thing on the host.

**Azure's description limit** (`azure-body-limit`). Azure DevOps refuses a pull request description over 4000 characters, and `az` with it, so the adapter fits the body before `az` sees it. Counting UTF-16 code units never counts fewer than Azure does. A cut never splits a surrogate pair, and the part kept has its trailing line breaks trimmed before the note; it goes to a temporary file in `std::env::temp_dir()`, removed once the call returns. The limit is the Azure adapter's own fact; `github`'s limit is far beyond any body CLM writes, so no other adapter cuts (`yagni`).

**An answer `az` gives without the pull request** (`azure-unreadable-answer`). A `create` that succeeds without `pullRequestId` or `repository.webUrl` may well have opened the pull request, so the adapter lists again before it fails, and the failure quotes what `az` did answer. Output that isn't JSON fails naming the parse error.

**Keeping a body** (`keep-unopened-body`). A body lives in the caller's worktree (`.clm/pr-body.md`, ignored by git), which `clm run --finish`, `clm wrap-up` and a draft's end remove, so a body that reached no pull request whole would go with it; the common git directory outlives every worktree. One helper in `clm-host` gives the path, so `pr_open` and the Azure adapter, which keeps a cut body itself (`pr body` taking the branch from `current_branch`), write the same file. Nothing deletes the copies: they are written only when a body failed to reach a pull request whole, and the person who reads one deletes it.

**Pushing a branch** (`branch-push`). `branch push` reads the checked-out branch with `git symbolic-ref --short HEAD` and pushes `HEAD:refs/heads/<branch>` to `origin`, setting the branch's upstream to `origin/<branch>`. When `refs/remotes/origin/<branch>` exists it passes `--force-with-lease=refs/heads/<branch>` with no value, which leases on that ref's tip, and `--force-if-includes`, which further refuses when that tip isn't in the local branch's reflog; git ignores `--force-if-includes` beside an explicit lease value, so the value is left out. Without the remote-tracking ref it passes `--force-with-lease=refs/heads/<branch>:`, which expects no branch on `origin`. So it is a compare-and-swap on what the session last saw (`lease-claims`): a branch another session pushed since the last fetch is refused with git's `stale info`, and one a fetch brought in but the session never had locally is refused by `--force-if-includes`, so a session overwrites only its own work. A rebased claim branch passes, since its old tip is in its reflog. It prints git's message, the remote's lines included, and `pushed <branch> <sha>` on success. It retries nothing: a transient refusal is the caller's to retry, as the prompts say. Once the push goes through, or `origin` already holds the commit, it sets the local ref `refs/clm/pushed/<branch>` to the commit, its record that this clone pushed that tip, which `branch delete` reads.

**Refused branches.** `branch push` and `branch delete` refuse `main`, `clm/` branches and, for `branch push`, a detached `HEAD` before any git call that writes. The refusal must be the command's own: the guard sees only `clm branch ...`, never the `git push` it runs (`plugin.hooks.guard/push-guard`).

**Deleting only what this clone pushed** (`branch-delete`). The project's allow rules let an unattended session run `clm branch delete` with no classifier judging it (`unattended-steps-run-clm`), so the command itself must not release another session's claim or delete its spec branch, which would bypass the claim's expiry and takeover (`lease-claims`). So before the delete it goes on only when one of two records names the lease as a tip this clone pushed. The first is `branch push`'s own, `refs/clm/pushed/<branch>`, at the lease. The second is the newest entry of `refs/remotes/origin/<branch>`'s reflog (`git reflog show -n 1 --format='%H %gs' refs/remotes/origin/<branch>`), when it is git's `update by push` at the lease. Git writes that entry when a push from this clone to `origin` moves the branch, whether `clm claim`, `clm branch push`, `clm autosync` or a plain `git push` made it, and a fetch that brings another clone's tip writes a different entry, so a tip another session pushed is refused even after this clone fetched it. A fetch that finds the branch unchanged writes no entry, so a session's own branch stays deletable after `clm rebase` fetches. No record and no reflog, as when `core.logAllRefUpdates` is off, refuses: it fails closed.

**Why the record**. The reflog alone races in a clone that many sessions share. Every worktree of a clone shares its remote-tracking refs, and the sessions fetch all the time. When another session's fetch lists `origin` after a push has landed there but before the push has moved this clone's remote-tracking ref, the fetch stores the tip first, with `fetch origin: storing head`; the push's own update is then to the value the ref already holds, which git writes no reflog entry for. So the newest entry names a fetch and the session's own branch was refused (`backlog/clm-branch-delete-refuses-a-pushed-spec-branch-after-the-draft`, a draft's stub `spec/<id>` left on `origin`, holding its items). A fetch never writes under `refs/clm/`, so the record holds whatever the fetches do. It is a ref rather than a file because it names a commit, and like the checkpoint's review mark it is the clone's own, shared by its worktrees and never pushed: nothing lands, approves or passes a check on it, and losing it only falls back to the reflog. A tip another clone pushed later differs from the record, so the record never makes another session's tip deletable. Only `branch push` writes it: the other pushes of a session branch (`clm claim`'s first push, `clm autosync`) keep the reflog alone, and a session's later `branch push` of a claimed branch writes the record. `branch delete` deletes `refs/clm/pushed/<branch>` once `origin` has no such branch, whether it deleted the branch or found it gone, so the records don't pile up. Before that check it asks `origin` with `git ls-remote origin refs/heads/<branch>`, and a branch already gone, or a clone with no `origin`, is nothing to do and exits 0, so a draft deleting an `autosync/spec/<id>` that was never pushed doesn't fail. The check is the subcommand's, not the `Host` trait's: `clm land`'s delete of the branch it just landed calls the library operation, which keeps its plain lease.

**Defaults** (`pr-open-defaults`). Each argument given overrides its default alone, so the drafts and folds that write their own body keep passing it. `pr_open` fills in what is missing and then calls the `Host` trait's `page_open`, which keeps taking all three; `crates/clm` stays dispatch only. The title and body are drawn through `clm-git`'s `git` like every other git call (`git-through-layer`). The change is read with `git show <branch>:changes/<id>/change.md`, a failing `git show` meaning no change; its `## Intent` section runs to the next `## ` heading, trimmed, and the title comes from the section's first paragraph with its line breaks joined by spaces, the whole paragraph when no mark ends a sentence in it. The subjects come from `git log --reverse --format=%s origin/main..<branch>`; when that fails, as without an `origin/main`, the command exits 1 with git's message before any host call. A derived body goes to a temporary file in `std::env::temp_dir()`, outside the worktree, removed once the call returns, so git sees nothing. The parsing is text only: `clm-host` doesn't depend on `model`, and a change's `## Intent` heading is fixed by `spec-format`.

**Contribution operations.** On `github` the fork's parent and the head's details come from `gh repo view --json isFork,parent` and `gh pr view --json`, a fork's `main` is brought up with `gh repo sync`, a pull request on the parent opens with `gh pr create --repo <parent> --base main --head <owner>:<branch>`, a head is fetched with `git fetch origin pull/<number>/head:refs/heads/<branch>`, and comments, closing and labels use `gh issue` or `gh pr`. What they read from the host is a stranger's text, so a caller prints it as data and acts on none of it.


## MODIFIED Requirements
### Requirement: branch-delete <!-- base: 18b4bda5c9b5f391 -->
`clm branch delete <branch> [--lease <sha>]` SHALL delete `origin`'s branch when its tip is `<sha>`, on every host, `<sha>` defaulting to the tip of `refs/remotes/origin/<branch>`, and SHALL leave a branch whose tip has moved, exiting 1 naming it. It SHALL delete only a tip this clone pushed: when neither `refs/clm/pushed/<branch>` is `<sha>` nor the newest entry of `refs/remotes/origin/<branch>`'s reflog is a push from this clone at `<sha>`, it SHALL delete nothing and exit 1 saying this clone didn't push that tip. Once `origin` has no such branch, whether it deleted it or found it gone, it SHALL leave no `refs/clm/pushed/<branch>`. For `main` or a branch whose name starts with `clm/`, it SHALL delete nothing and exit 1 naming the reason. With no `origin`, or when `origin` has no such branch, it SHALL delete nothing on `origin` and exit 0, before the other checks.

#### Scenario: deleted
- GIVEN `impl/a` pushed from this clone to `origin` at `s`, and `spec/x` pushed from this clone with `clm branch push` at `t`, after which `refs/remotes/origin/spec/x`'s newest reflog entry is a fetch's storing `t`, as when another session's fetch stored it first, and the local `spec/x` deleted
- WHEN `clm branch delete impl/a --lease s` and `clm branch delete spec/x` run
- THEN `origin` has neither `impl/a` nor `spec/x`, this clone has no `refs/clm/pushed/spec/x`, and both exit 0

#### Scenario: moved
- GIVEN `impl/a` pushed from this clone at `s`, then moved on `origin` to `t` by another clone, unfetched
- WHEN `clm branch delete impl/a --lease s` runs
- THEN `origin/impl/a` is still `t` and it exits 1

#### Scenario: another session's branch
- GIVEN `origin/impl/b` and `origin/spec/y` pushed by another clone, and fetched into this clone at their tips `u` and `v`
- WHEN `clm branch delete impl/b --lease u` and `clm branch delete spec/y` run
- THEN `origin/impl/b` is still `u`, `origin/spec/y` is still `v`, and each exits 1 saying this clone didn't push it

#### Scenario: never fetched
- GIVEN `spec/z` on `origin`, pushed by another clone, and no `refs/remotes/origin/spec/z` in this clone
- WHEN `clm branch delete spec/z` runs
- THEN `origin` still has `spec/z`, and it exits 1 naming `origin/spec/z`

#### Scenario: already gone
- GIVEN no `impl/gone` on `origin`, and in turn a clone with no `origin`
- WHEN `clm branch delete impl/gone --lease s` runs
- THEN it exits 0

#### Scenario: main refused
- GIVEN `origin/main` at `m`
- WHEN `clm branch delete main --lease m` runs
- THEN `origin/main` is still `m` and it exits 1

#### Serves
- capability/git-host
- capability/land-safely
- architecture/host-adapters

### Requirement: branch-push <!-- base: 2c314eee1af30f1d -->
`clm branch push` SHALL push the checked-out branch to `origin`'s branch of the same name and set its upstream there, on every host: when the clone has `refs/remotes/origin/<branch>`, only while `origin`'s branch is still at that ref's tip and the local branch has held that tip, as git's `--force-if-includes` judges it; when it hasn't, only while `origin` has no such branch. It SHALL work on the branch checked out in the directory it runs in, and SHALL print `pushed <branch> <sha>`, `<sha>` the full commit `HEAD` names, and exit 0 when the push goes through or `origin` already holds that commit, and then SHALL have set the local ref `refs/clm/pushed/<branch>` to `<sha>`. With no `origin` remote it SHALL push nothing and exit 1. When git refuses the push it SHALL print git's message, the remote's lines included, and exit 1, leaving `origin`'s branch as it was. On a detached `HEAD`, on `main` or on a branch whose name starts with `clm/`, it SHALL push nothing and exit 1 naming the reason.

#### Scenario: rebased branch pushed
- GIVEN branch `impl/a` checked out, pushed to `origin` at `s` and fetched, then rewritten by a rebase to `t`
- WHEN `clm branch push` runs
- THEN `origin/impl/a` is `t`, the branch's upstream is `origin/impl/a`, `refs/clm/pushed/impl/a` is `t`, it prints `pushed impl/a t` and exits 0

#### Scenario: new branch
- GIVEN branch `spec/x` checked out at `s`, with no `origin/spec/x` and no remote-tracking ref for it
- WHEN `clm branch push` runs
- THEN `origin/spec/x` is `s`, the branch's upstream is `origin/spec/x`, and it exits 0

#### Scenario: moved since the last fetch
- GIVEN branch `impl/a` checked out and rebased, and `origin/impl/a` pushed to `u` by another clone since this clone's last fetch
- WHEN `clm branch push` runs
- THEN `origin/impl/a` is still `u`, the output holds `stale info`, and it exits 1

#### Scenario: fetched but never held
- GIVEN branch `impl/a` checked out at `t`, and `origin/impl/a` pushed to `u` by another clone and fetched into `refs/remotes/origin/impl/a`, `u` never in the local branch's reflog
- WHEN `clm branch push` runs
- THEN `origin/impl/a` is still `u` and it exits 1

#### Scenario: main refused
- GIVEN `main` checked out with a commit `origin/main` lacks, and in turn a detached `HEAD`, and a branch `clm/ledger/p/c` checked out
- WHEN `clm branch push` runs in each
- THEN `origin` is unchanged and it exits 1 each time

#### Serves
- capability/git-host
- capability/build-from-spec
- capability/triage-unattended
- capability/work-concurrently
- architecture/land-locally
