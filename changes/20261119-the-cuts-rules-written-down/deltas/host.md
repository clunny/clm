---
component: host
base: { design: "f2db0b3497d99948" }
---

## MODIFIED Design
Crate `clm-host`. One trait, `Host`, with these operations, and an implementation per setting. `github` runs `gh pr create --base main --head <branch> --title --body-file`, `gh pr view --json number,state,headRefName` and `gh api -X PATCH repos/{owner}/{repo}/pulls/<n> -F body=@<file>` (the REST call, since `gh pr edit` fails on `gh` versions that query the retired Projects (classic) API), and `gh issue create`. `branch delete` is `git push --force-with-lease=refs/heads/<branch>:<sha> origin :refs/heads/<branch>` for every host, `<sha>` being `--lease` or, without it, the tip of `refs/remotes/origin/<branch>`.

**Azure DevOps** (`azure-pull-requests`, `azure-through-az-cli`). `azure` runs the Azure CLI through `clm-git`'s `run`, with `--detect true` so `az` takes the organisation, project and repository from the clone's remote, and `--output json`; `az` holds the sign-in (`az login`, or `AZURE_DEVOPS_EXT_PAT`), so CLM reads no token. `pr open` first runs `az repos pr list --source-branch <branch> --target-branch main --status active` and, when it lists a pull request, prints the first one's URL; otherwise it runs `az repos pr create --source-branch <branch> --target-branch main --title <title> --description @<file>`, the file's path made absolute against the caller's directory, since `az` reads `@<file>` relative to the directory it runs in, the repository's root. Azure's `--source-branch` takes the bare branch name. The URL printed is the pull request's web page, `<repository.webUrl>/pullrequest/<pullRequestId>`, from the JSON `az` returns, since the `url` field there is the REST API's. `pr body` finds the current branch's active pull request the same way and runs `az repos pr update --id <pullRequestId> --description @<file>`. Every other operation is the trait's default, as for `git`: `issue` prints the report and says there is nowhere to file it, `parent` returns nothing, `list` and `alerts` return nothing, and the rest report that there is no such thing on the host.

**Azure's description limit** (`azure-body-limit`). Azure DevOps refuses a pull request description over 4000 characters, and `az` with it, so the adapter fits the body before `az` sees it. Counting UTF-16 code units never counts fewer than Azure does. A cut never splits a surrogate pair, and the part kept has its trailing line breaks trimmed before the note; it goes to a temporary file in `std::env::temp_dir()`, removed once the call returns. The limit is the Azure adapter's own fact; `github`'s limit is far beyond any body CLM writes, so no other adapter cuts (`yagni`).

**An answer `az` gives without the pull request** (`azure-unreadable-answer`). A `create` that succeeds without `pullRequestId` or `repository.webUrl` may well have opened the pull request, so the adapter lists again before it fails, and the failure quotes what `az` did answer. Output that isn't JSON fails naming the parse error.

**Keeping a body** (`keep-unopened-body`). A body lives in the caller's worktree (`.clm/pr-body.md`, ignored by git), which `clm run --finish`, `clm wrap-up` and a draft's end remove, so a body that reached no pull request whole would go with it; the common git directory outlives every worktree. One helper in `clm-host` gives the path, so `pr_open` and the Azure adapter, which keeps a cut body itself (`pr body` taking the branch from `current_branch`), write the same file. Nothing deletes the copies: they are written only when a body failed to reach a pull request whole, and the person who reads one deletes it.

**Pushing a branch** (`branch-push`). `branch push` reads the checked-out branch with `git symbolic-ref --short HEAD` and pushes `HEAD:refs/heads/<branch>` to `origin`, setting the branch's upstream to `origin/<branch>`. When `refs/remotes/origin/<branch>` exists it passes `--force-with-lease=refs/heads/<branch>` with no value, which leases on that ref's tip, and `--force-if-includes`, which further refuses when that tip isn't in the local branch's reflog; git ignores `--force-if-includes` beside an explicit lease value, so the value is left out. Without the remote-tracking ref it passes `--force-with-lease=refs/heads/<branch>:`, which expects no branch on `origin`. So it is a compare-and-swap on what the session last saw (`lease-claims`): a branch another session pushed since the last fetch is refused with git's `stale info`, and one a fetch brought in but the session never had locally is refused by `--force-if-includes`, so a session overwrites only its own work. A rebased claim branch passes, since its old tip is in its reflog. It prints git's message, the remote's lines included, and `pushed <branch> <sha>` on success. It retries nothing: a transient refusal is the caller's to retry, as the prompts say. When the push moved `origin`'s branch, it sets the local ref `refs/clm/pushed/<branch>` to the commit, its record that this clone pushed that tip, which `branch delete` reads. It tells from `git push --porcelain`, whose line for the ref is flagged `=` when `origin` already held the commit: such a push moved nothing, and the tip may be another clone's that this branch was made from, so it leaves the record as it was, as a refused push does.

**Refused branches.** `branch push` and `branch delete` refuse `main`, `clm/` branches and, for `branch push`, a detached `HEAD` before any git call that writes. The refusal must be the command's own: the guard sees only `clm branch ...`, never the `git push` it runs (`plugin.hooks.guard/push-guard`).

**Deleting only what this clone pushed** (`branch-delete`). The project's allow rules let an unattended session run `clm branch delete` with no classifier judging it (`unattended-steps-run-clm`), so the command itself must not release another session's claim or delete its spec branch, which would bypass the claim's expiry and takeover (`lease-claims`). So before the delete it goes on only when one of two records names the lease as a tip this clone pushed. The first is `branch push`'s own, `refs/clm/pushed/<branch>`, at the lease. The second is the newest entry of `refs/remotes/origin/<branch>`'s reflog (`git reflog show -n 1 --format='%H %gs' refs/remotes/origin/<branch>`), when it is git's `update by push` at the lease. Git writes that entry when a push from this clone to `origin` moves the branch, whether `clm claim`, `clm branch push`, `clm autosync` or a plain `git push` made it, and a fetch that brings another clone's tip writes a different entry, so a tip another session pushed is refused even after this clone fetched it. A fetch that finds the branch unchanged writes no entry, so a session's own branch stays deletable after `clm rebase` fetches. No record and no reflog, as when `core.logAllRefUpdates` is off, refuses: it fails closed.

**Why the record**. The reflog alone races in a clone that many sessions share. Every worktree of a clone shares its remote-tracking refs, and the sessions fetch all the time. When another session's fetch lists `origin` after a push has landed there but before the push has moved this clone's remote-tracking ref, the fetch stores the tip first, with `fetch origin: storing head`; the push's own update is then to the value the ref already holds, which git writes no reflog entry for. So the newest entry names a fetch and the session's own branch was refused (`backlog/clm-branch-delete-refuses-a-pushed-spec-branch-after-the-draft`, a draft's stub `spec/<id>` left on `origin`, holding its items). A fetch never writes under `refs/clm/`, so the record holds whatever the fetches do. It is a ref rather than a file because it names a commit, and like the checkpoint's review mark it is the clone's own, shared by its worktrees and never pushed: nothing lands, approves or passes a check on it, and losing it only falls back to the reflog. A tip another clone pushed later differs from the record, so the record never makes another session's tip deletable. Only `branch push` writes it: the other pushes of a session branch (`clm claim`'s first push, `clm autosync`) keep the reflog alone, and a session's later `branch push` of a claimed branch writes the record. `branch delete` deletes `refs/clm/pushed/<branch>` once `origin` has no such branch, whether it deleted the branch or found it gone. `clm land`'s delete of a landed branch goes through the library operation and leaves the branch's record: one ref per landed branch, naming a commit `main` holds, which costs nothing kept and which a later `branch delete` of that name, or `git update-ref -d`, removes. A test makes the race's state by storing the tip in `refs/remotes/origin/<branch>` with `git update-ref` before `clm branch push` pushes it, as the winning fetch does. Before that check it asks `origin` with `git ls-remote origin refs/heads/<branch>`, and a branch already gone, or a clone with no `origin`, is nothing to do and exits 0, so a draft deleting an `autosync/spec/<id>` that was never pushed doesn't fail. The check is the subcommand's, not the `Host` trait's: `clm land`'s delete of the branch it just landed calls the library operation, which keeps its plain lease.

**Defaults** (`pr-open-defaults`). Each argument given overrides its default alone, so the drafts and folds that write their own body keep passing it. `pr_open` fills in what is missing and then calls the `Host` trait's `page_open`, which keeps taking all three; `crates/clm` stays dispatch only. The title and body are drawn through `clm-git`'s `git` like every other git call (`git-through-layer`). The change is read with `git show <branch>:changes/<id>/change.md`, a failing `git show` meaning no change; its `## Intent` section runs to the next `## ` heading, trimmed, and the title comes from the section's first paragraph with its line breaks joined by spaces, the whole paragraph when no mark ends a sentence in it. The subjects come from `git log --reverse --format=%s origin/main..<branch>`; when that fails, as without an `origin/main`, the command exits 1 with git's message before any host call. A derived body goes to a temporary file in `std::env::temp_dir()`, outside the worktree, removed once the call returns, so git sees nothing. The parsing is text only: `clm-host` names nothing of `model` itself, and a change's `## Intent` heading is fixed by `spec-format`.

**Contribution operations.** On `github` the fork's parent and the head's details come from `gh repo view --json isFork,parent` and `gh pr view --json`, a fork's `main` is brought up with `gh repo sync`, a pull request on the parent opens with `gh pr create --repo <parent> --base main --head <owner>:<branch>`, a head is fetched with `git fetch origin pull/<number>/head:refs/heads/<branch>`, and comments, closing and labels use `gh issue` or `gh pr`. What they read from the host is a stranger's text, so a caller prints it as data and acts on none of it.

**The cuts in a body** (`pr-open`, `pr-body`). `clm-host` depends on `clm-gate` for `evaluate_at` and nothing else of it. The section goes first so that a host's limit, which cuts a body at its end (`azure-body-limit`), never cuts the list. It is added to whatever body is sent, given or drawn, since the skills pass a body of their own, and a leading `## Cuts` section such a body already holds is taken off first: the section is the gate's to write, so a kept copy or a body read back from the pull request can be sent again. A body kept because it didn't reach the pull request whole (`keep-unopened-body`) is kept with the section.

## MODIFIED Requirements

### Requirement: pr-open <!-- base: bdaae01c39f6c334 -->
`clm pr open <branch> --title <title> --body-file <file>` SHALL, on `host = "github"`, open a pull request from the branch to `main` with that title and body through `gh`, or print the URL of the open one that already exists, and print the pull request's URL; on `git` and `none` it SHALL print `none` and call nothing. On a host that has pull requests, when `gate`'s `evaluate_at` for the branch against `origin/main` gives kind `spec` and any cut (`gate/cuts`), the body it sends SHALL start with the line `## Cuts`, then one `- <line>` per cut as `clm gate` prints it, then a blank line, before the body given or drawn from the branch; for any other branch the body is sent as it is. A body given or drawn that itself starts with the line `## Cuts` SHALL first lose that section, whatever the branch cuts: the line and every line after it up to and including the first line that is empty or holds only whitespace, or to the body's end when there is none, so a body sent again never holds the section twice and a stale one goes. The body file is passed as given only when there is no cut and no such section. It SHALL evaluate the gate only once every check it made before has passed, directly before the call that sends the body, and SHALL pass the body file as given when there is no cut. When the gate can't be evaluated it SHALL exit 1 naming why, without that call.

#### Scenario: github
- GIVEN `host = "github"` and a fake `gh` that records its calls and prints a URL
- WHEN `clm pr open spec/x --title "x" --body-file body.md` runs
- THEN `gh` got `pr create` with `--base main`, `--head spec/x` and `--body-file body.md`, and the URL is printed

#### Scenario: a branch with cuts
- GIVEN `host = "github"`, a fake `gh`, and a branch `spec/x` that removes requirement `a/x` from `specs/a/SPEC.md`
- WHEN `clm pr open spec/x --title "x" --body-file body.md` runs
- THEN the body file `gh` got starts `## Cuts`, then `- specs/a/SPEC.md: gate/cut: requirement a/x is gone`, then a blank line and the contents of `body.md`

#### Scenario: a body that already holds the section
- GIVEN the branch of *a branch with cuts*, and a `body.md` that starts `## Cuts`, `- an older line`, a blank line and then `## Intent`
- WHEN `clm pr open spec/x --title "x" --body-file body.md` runs
- THEN the body `gh` got holds `## Cuts` once, with the line for `a/x` and without `- an older line`, followed by `## Intent`

#### Scenario: a branch without cuts
- GIVEN the same host and a branch `spec/y` that only adds a requirement
- WHEN `clm pr open spec/y --title "y" --body-file body.md` runs
- THEN the body `gh` got is the contents of `body.md`

#### Scenario: already open
- GIVEN the same, with a fake `gh` whose `pr view` reports an open PR for `spec/x`
- WHEN `clm pr open spec/x ...` runs
- THEN `gh pr create` isn't called and that PR's URL is printed

#### Scenario: no host
- GIVEN `host = "none"`
- WHEN `clm pr open spec/x --title "x" --body-file body.md` runs
- THEN it prints `none`, exits 0, and runs no `gh`

#### Needs
- gate/cuts

### Requirement: pr-body <!-- base: a7b89523384d6115 -->
`clm pr body <file>` SHALL set the body of the current branch's open pull request to the file's contents through the host, changing nothing else of the pull request, and print `updated #<number>`. For a branch with cuts the body SHALL start with the `## Cuts` section as `pr-open` gives it, so a corrected body keeps the list, evaluated and failing as `pr-open` says, and a leading `## Cuts` section of the file SHALL be replaced as `pr-open` replaces it. It SHALL exit 1 without a host call when the file is missing or blank, or when the branch has no open pull request; on `git` and `none` it SHALL print nothing and exit 0.

#### Scenario: body updated
- GIVEN `host = "github"`, the current branch `spec/x` with an open PR 7 according to a fake `gh`, and a file `body.md`
- WHEN `clm pr body body.md` runs
- THEN `gh` got `api -X PATCH repos/{owner}/{repo}/pulls/7` with `-F body=@body.md` and nothing else, and it printed `updated #7`

#### Scenario: a branch with cuts
- GIVEN the same, `spec/x` removing requirement `a/x` from `specs/a/SPEC.md`
- WHEN `clm pr body body.md` runs
- THEN the body `gh` got starts `## Cuts` and `- specs/a/SPEC.md: gate/cut: requirement a/x is gone`, followed by the contents of `body.md`

#### Scenario: a body read back from the pull request
- GIVEN the branch of *a branch with cuts*, and a `body.md` that starts `## Cuts`, `- an older line`, a blank line and then `## Intent`
- WHEN `clm pr body body.md` runs
- THEN the body `gh` got holds `## Cuts` once, with the line for `a/x` and without `- an older line`, followed by `## Intent`

#### Scenario: blank file
- GIVEN an empty `body.md`
- WHEN `clm pr body body.md` runs
- THEN it exits 1 and `gh` wasn't called

#### Needs
- gate/cuts

### Requirement: keep-unopened-body <!-- base: a8937350ea6c7e8b -->
When the host's `pr open` fails after `clm pr open`'s own checks have passed, on `github` or `azure`, `clm-host`'s `pr_open` SHALL first write the body it was opening with (the `--body-file`'s contents, or the body drawn from the branch, in either case with the `## Cuts` section `pr-open` puts first for a branch that cuts anything) to `<common git dir>/clm/pr-bodies/<branch>.md`, `<common git dir>` being `clm-git`'s `common_dir` and each `/` of the branch's name written as `-`, creating the directories and replacing an earlier copy, and its error SHALL end `; the body is kept at <path>` with that file's absolute path, so `clm pr open` exits 1 naming it. The `Azure` adapter SHALL write the whole body to the same path when it cuts one (`azure-body-limit`). When the gate can't be evaluated (`pr-open`) no body SHALL be kept, since none was made. Nothing else SHALL be written there: a pull request that opens with its whole body leaves no copy.

#### Scenario: the host refuses
- GIVEN `host = "azure"`, a fake `az` that lists no active pull request for `impl/a` and exits 1 on `pr create` printing `TF401398`, and `.clm/pr-body.md` holding `## Review`
- WHEN `clm pr open impl/a --title x --body-file .clm/pr-body.md` runs
- THEN it exits 1, its output holds `TF401398` and the path of `<common git dir>/clm/pr-bodies/impl-a.md`, and that file holds `## Review`

#### Scenario: a kept body holds its cuts
- GIVEN `host = "github"`, a fake `gh` whose `pr create` exits 1, and a branch `spec/x` that removes requirement `a/x`
- WHEN `clm pr open spec/x --title x --body-file body.md` runs
- THEN the kept file starts `## Cuts` and `- specs/a/SPEC.md: gate/cut: requirement a/x is gone`, followed by the contents of `body.md`

#### Scenario: from a linked worktree
- GIVEN the same, run in a linked worktree of the clone
- WHEN `clm pr open impl/a --title x --body-file .clm/pr-body.md` runs and the worktree is then removed
- THEN `<common git dir>/clm/pr-bodies/impl-a.md`, under the main repository's git directory, still holds `## Review`

#### Scenario: opened
- GIVEN `host = "github"` and a fake `gh` that prints a URL
- WHEN `clm pr open impl/a --title x --body-file body.md` runs
- THEN it prints the URL and no `clm/pr-bodies/` file exists

#### Serves
- capability/git-host
- capability/build-from-spec
