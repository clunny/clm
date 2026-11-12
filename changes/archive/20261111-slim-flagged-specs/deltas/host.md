---
component: host
base: { design: "5f21e5f9879923f0" }
---

## MODIFIED Design
Crate `clm-host`. One trait, `Host`, with these operations, and an implementation per setting. `github` runs `gh pr create --base main --head <branch> --title --body-file`, `gh pr view --json number,state,headRefName` and `gh api -X PATCH repos/{owner}/{repo}/pulls/<n> -F body=@<file>` (the REST call, since `gh pr edit` fails on `gh` versions that query the retired Projects (classic) API), and `gh issue create`. `branch delete` is `git push --force-with-lease=refs/heads/<branch>:<sha> origin :refs/heads/<branch>` for every host, `<sha>` being `--lease` or, without it, the tip of `refs/remotes/origin/<branch>`.

**Azure DevOps** (`azure-pull-requests`, `azure-through-az-cli`). `azure` runs the Azure CLI through `clm-git`'s `run`, with `--detect true` so `az` takes the organisation, project and repository from the clone's remote, and `--output json`; `az` holds the sign-in (`az login`, or `AZURE_DEVOPS_EXT_PAT`), so CLM reads no token. `pr open` first runs `az repos pr list --source-branch <branch> --target-branch main --status active` and, when it lists a pull request, prints the first one's URL; otherwise it runs `az repos pr create --source-branch <branch> --target-branch main --title <title> --description @<file>`, the file's path made absolute against the caller's directory, since `az` reads `@<file>` relative to the directory it runs in, the repository's root. Azure's `--source-branch` takes the bare branch name. The URL printed is the pull request's web page, `<repository.webUrl>/pullrequest/<pullRequestId>`, from the JSON `az` returns, since the `url` field there is the REST API's. `pr body` finds the current branch's active pull request the same way and runs `az repos pr update --id <pullRequestId> --description @<file>`. Every other operation is the trait's default, as for `git`: `issue` prints the report and says there is nowhere to file it, `parent` returns nothing, `list` and `alerts` return nothing, and the rest report that there is no such thing on the host.

**Azure's description limit** (`azure-body-limit`). Azure DevOps refuses a pull request description over 4000 characters, and `az` with it, so the adapter fits the body before `az` sees it. Counting UTF-16 code units never counts fewer than Azure does. A cut never splits a surrogate pair, and the part kept has its trailing line breaks trimmed before the note; it goes to a temporary file in `std::env::temp_dir()`, removed once the call returns. The limit is the Azure adapter's own fact; `github`'s limit is far beyond any body CLM writes, so no other adapter cuts (`yagni`).

**An answer `az` gives without the pull request** (`azure-unreadable-answer`). A `create` that succeeds without `pullRequestId` or `repository.webUrl` may well have opened the pull request, so the adapter lists again before it fails, and the failure quotes what `az` did answer. Output that isn't JSON fails naming the parse error.

**Keeping a body** (`keep-unopened-body`). A body lives in the caller's worktree (`.clm/pr-body.md`, ignored by git), which `clm run --finish`, `clm wrap-up` and a draft's end remove, so a body that reached no pull request whole would go with it; the common git directory outlives every worktree. One helper in `clm-host` gives the path, so `pr_open` and the Azure adapter, which keeps a cut body itself (`pr body` taking the branch from `current_branch`), write the same file. Nothing deletes the copies: they are written only when a body failed to reach a pull request whole, and the person who reads one deletes it.

**Pushing a branch** (`branch-push`). `branch push` reads the checked-out branch with `git symbolic-ref --short HEAD` and pushes `HEAD:refs/heads/<branch>` to `origin`, setting the branch's upstream to `origin/<branch>`. When `refs/remotes/origin/<branch>` exists it passes `--force-with-lease=refs/heads/<branch>` with no value, which leases on that ref's tip, and `--force-if-includes`, which further refuses when that tip isn't in the local branch's reflog; git ignores `--force-if-includes` beside an explicit lease value, so the value is left out. Without the remote-tracking ref it passes `--force-with-lease=refs/heads/<branch>:`, which expects no branch on `origin`. So it is a compare-and-swap on what the session last saw (`lease-claims`): a branch another session pushed since the last fetch is refused with git's `stale info`, and one a fetch brought in but the session never had locally is refused by `--force-if-includes`, so a session overwrites only its own work. A rebased claim branch passes, since its old tip is in its reflog. It prints git's message, the remote's lines included, and `pushed <branch> <sha>` on success. It retries nothing: a transient refusal is the caller's to retry, as the prompts say (`orchestrator.run/prompt-commands`).

**Refused branches.** `branch push` and `branch delete` refuse `main`, `clm/` branches and, for `branch push`, a detached `HEAD` before any git call that writes. The refusal must be the command's own: the guard sees only `clm branch ...`, never the `git push` it runs (`plugin.hooks.guard/push-guard`).

**Deleting only what this clone pushed** (`branch-delete`). The project's allow rules let an unattended session run `clm branch delete` with no classifier judging it (`unattended-steps-run-clm`), so the command itself must not release another session's claim or delete its spec branch, which would bypass the claim's expiry and takeover (`lease-claims`). So before the delete it reads the newest entry of `refs/remotes/origin/<branch>`'s reflog (`git reflog show -n 1 --format='%H %gs' refs/remotes/origin/<branch>`), and goes on only when that entry is git's `update by push` and its commit is the lease. Git writes that entry when a push from this clone to `origin` moves the branch, whether `clm claim`, `clm branch push`, `clm autosync` or a plain `git push` made it, and a fetch that brings another clone's tip writes a different entry, so a tip another session pushed is refused even after this clone fetched it. A fetch that finds the branch unchanged writes no entry, so a session's own branch stays deletable after `clm rebase` fetches. No reflog, as when `core.logAllRefUpdates` is off, refuses: it fails closed. Before that check it asks `origin` with `git ls-remote origin refs/heads/<branch>`, and a branch already gone, or a clone with no `origin`, is nothing to do and exits 0, so a draft deleting an `autosync/spec/<id>` that was never pushed doesn't fail. The check is the subcommand's, not the `Host` trait's: `clm land`'s delete of the branch it just landed calls the library operation, which keeps its plain lease.

**Defaults** (`pr-open-defaults`). Each argument given overrides its default alone, so the drafts and folds that write their own body keep passing it. `pr_open` fills in what is missing and then calls the `Host` trait's `page_open`, which keeps taking all three; `crates/clm` stays dispatch only. The title and body are drawn through `clm-git`'s `git` like every other git call (`git-through-layer`). The change is read with `git show <branch>:changes/<id>/change.md`, a failing `git show` meaning no change; its `## Intent` section runs to the next `## ` heading, trimmed, and the title comes from the section's first paragraph with its line breaks joined by spaces, the whole paragraph when no mark ends a sentence in it. The subjects come from `git log --reverse --format=%s origin/main..<branch>`; when that fails, as without an `origin/main`, the command exits 1 with git's message before any host call. A derived body goes to a temporary file in `std::env::temp_dir()`, outside the worktree, removed once the call returns, so git sees nothing. The parsing is text only: `clm-host` doesn't depend on `model`, and a change's `## Intent` heading is fixed by `spec-format`.

**Contribution operations.** On `github` the fork's parent and the head's details come from `gh repo view --json isFork,parent` and `gh pr view --json`, a fork's `main` is brought up with `gh repo sync`, a pull request on the parent opens with `gh pr create --repo <parent> --base main --head <owner>:<branch>`, a head is fetched with `git fetch origin pull/<number>/head:refs/heads/<branch>`, and comments, closing and labels use `gh issue` or `gh pr`. What they read from the host is a stranger's text, so a caller prints it as data and acts on none of it.

## MODIFIED Requirements
### Requirement: shared-plumbing <!-- base: a65259c702291aaf -->
`clm-host`'s `branch delete` SHALL take the remote's name from `clm-git`'s `REMOTE` for the check that the remote exists, the leased push that deletes the branch and the `ls-remote` that confirms it, rather than spelling it: no line of a file under `crates/clm-host/src`, apart from those whose first non-blank characters are `//`, SHALL hold `"origin"`, `"origin/` or `remotes/origin` (`remote-plumbing-scan`). A message for a person may still name `origin`, as `` `<branch>` on origin is no longer at <lease>; left it `` does.

#### Scenario: no copy of the remote's name
- GIVEN the workspace
- WHEN a test in `clm-host` reads every file under `crates/clm-host/src`, skipping each line whose first non-blank characters are `//`
- THEN no line holds `"origin"`, `"origin/` or `remotes/origin`

#### Serves
- architecture/reuse-first

### Requirement: pr-open-defaults <!-- base: aa76a93181f759a4 -->
`clm pr open` SHALL take its branch, `--title` and `--body-file` as optional, each one given overriding only its own default. On `git` and `none` it SHALL print `none` and call nothing, whatever arguments it gets and whatever `HEAD` is, checking the setting before anything else. Otherwise, without a branch it SHALL use the checked-out branch, and exit 1 without a host call when `HEAD` is detached. Without `--title` or `--body-file`, on a branch `spec/<id>` whose tree holds `changes/<id>/change.md` with an `## Intent` section holding more than whitespace, the body SHALL be that section's text and the title its first sentence, ending at the first `.`, `?` or `!` followed by a space or the paragraph's end and keeping that mark; on any other branch the body SHALL be one `- <subject>` line per commit in `origin/main..<branch>`, oldest first, and the title the oldest commit's subject. A branch with no such change and no commit beyond `origin/main` SHALL exit 1 without a host call. The defaults SHALL be filled by one `clm-host` library call, `crates/clm` only passing the optional arguments through.

#### Scenario: a spec branch's change
- GIVEN `host = "github"`, a fake `gh` that records its calls, and the checked-out branch `spec/x` holding `changes/x/change.md` whose `## Intent` reads `Skills fail. They name a bare command.` followed by a `## Approaches` section
- WHEN `clm pr open` runs with no arguments
- THEN `gh` got `pr create` with `--head spec/x` and `--title "Skills fail."`, and the body file it was given held `Skills fail. They name a bare command.` and no `## Approaches`

#### Scenario: a branch without a change
- GIVEN `host = "github"`, a fake `gh`, and the checked-out branch `fix/y` with commits `first` and then `second` beyond `origin/main`
- WHEN `clm pr open` runs with no arguments
- THEN `gh` got `pr create` with `--head fix/y` and `--title first`, and the body file held `- first` then `- second`

#### Scenario: given arguments override
- GIVEN the branch of *a spec branch's change*, checked out, and a file `body.md`
- WHEN `clm pr open --title "mine" --body-file body.md` runs
- THEN `gh` got `pr create` with `--head spec/x`, `--title mine` and `--body-file body.md`

#### Scenario: nothing to describe
- GIVEN `host = "github"`, a fake `gh`, and the checked-out branch `fix/z` at `origin/main`'s tip
- WHEN `clm pr open` runs with no arguments
- THEN it exits 1 and `gh` wasn't called

#### Scenario: detached head
- GIVEN `host = "github"`, a fake `gh`, and a detached `HEAD`
- WHEN `clm pr open` runs with no arguments
- THEN it exits 1 and `gh` wasn't called

#### Scenario: no host
- GIVEN `host = "none"` and a detached `HEAD`
- WHEN `clm pr open` runs with no arguments
- THEN it prints `none`, exits 0, and runs no `gh`
