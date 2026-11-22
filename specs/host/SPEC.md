---
title: Host
status: active
owners: ["@clunny"]
depends_on: [config, vcs, gate]
paths: ["crates/clm-host/**"]
successor: ""
serves: [capability/git-host, architecture/host-adapters]
---

## Contract
The one place CLM talks to a git host beyond `git` itself (`host-adapters`). The `host` setting picks the adapter: `github` (`gh`), `azure` (the Azure CLI, `az`, with its `azure-devops` extension), and `git` and `none`, which have no host and answer every host operation with nothing to do, `none` keeping the clone's own remote besides (`keep_remote`).

**Commands.** Crate `clm-host` offers each of `clm pr`, `clm issue` and `clm branch` as a call, `pr_command`, `issue_command` and `branch_command` (`commands-in-crate`, `commands-live-in-their-crates`): each takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `PrArgs`, `IssueArgs` and `BranchArgs` are those arguments as the binary's parser reads them, each giving the `--root` it was given. `crates/clm` routes the command to the call and adds nothing to what it writes. Every command exits 2 when the repository's settings don't load.
- `pr open [<branch>] [--title <title>] [--body-file <file>]`: opens a pull request for the branch against `main` on the host and prints its URL, or `none`. The branch defaults to the checked-out one, and the title and body to ones drawn from the branch: the `## Intent` of the change the branch adds, whatever the branch's name, otherwise its commits' subjects (`pr-open-defaults`), so a bare `clm pr open` works wherever a skill or a person writes it. A relative body file is read against the repository root on every host. A body that doesn't reach the pull request whole, because the host refused the call or its limit cut the body, is kept outside every worktree and the command names where (`keep-unopened-body`). The body of a spec branch that cuts anything starts with a `## Cuts` section, the lines `clm gate` prints for it, so the human reads what the branch removes where they read the diff (`gate/cuts`, `removal-needs-a-yes`). Nothing in CLM waits on a pull request: it is where a human reads the diff and where the host records the merge.
- `pr body <file>`: sets the body of the current branch's pull request to the file's contents, after the `## Cuts` section when the branch cuts anything, and changes nothing else (`own-pr-body`), cut to the host's limit as `pr open` cuts it, Azure DevOps keeping 4000 characters (`azure-body-limit`).
- `issue --title <title> --body-file <file> --repo <repo>`: files an issue where the host keeps them and prints its URL; on `azure`, `git` and `none` it prints the report and says there is nowhere to file it.
- `branch push`: pushes the checked-out branch to `origin`, overwriting `origin`'s branch only when its tip is the one last fetched and the branch has held it, plain git on every host. It is how an unattended session pushes its rebased branch with a `clm` command alone (`unattended-steps-run-clm`).
- `branch delete <branch> [--lease <sha>]`: deletes the remote branch when its tip is still `<sha>`, by default the tip last fetched, and only a tip this clone pushed, so a session can release its own claim but never another's, plain git on every host.

On every host, `clm pr` and `clm issue` exit 2 when the host's tool, `gh` or `az`, fails or gives an answer the adapter can't read, and 1 when the command itself refuses, as its requirements state, such as for a blank body file or a branch with no pull request (`host-tool-failure`), so a caller reads a host's failure the same way on `github` and on `azure`. `clm branch push` and `clm branch delete` exit 1 when they refuse, when `git push` is refused, by its lease or by `origin`, and when `origin` can't be reached, which is CLM's offline case, as `clm land` reads it (`land/offline-land`); they exit 2 when git fails otherwise. So a caller reads one rule: a refusal, offline included, is 1, and a tool that fails is 2, `gh` and `az` reporting an unreachable host as their own failure.

**Library calls.**
- `adapter(host)`: the adapter the `host` setting names, a `Host`. Its operations: `page_open(root, branch, title, body file)` and `page_body(root, file)`, what `pr open` and `pr body` ask of the host once the defaults are filled; `issue(root, title, body file, repo)`; the contribution operations below, `parent`, `sync`, `open_on_parent`, `show`, `fetch`, `comment`, `label` and `list`; `alerts`; `runs(root, workflow)`; and `branch_delete(root, branch, lease)`, the leased delete `clm land` makes of every branch it lands (`branch-delete-call`). Each gives its answer or an `Error`: `Refused(why)`, the work isn't possible or was refused; `Nowhere(report)`, an issue with nowhere to file it, carrying the report; or `Git(why)`, `git` or the host's tool failed or answered what the adapter can't read. The commands exit 1 on `Refused` and `Nowhere` and 2 on `Git`. `Shown`, `Listed`, `Alert` and `Run` are the values `show`, `list`, `alerts` and `runs` give. `clm defect` files through the `github` adapter's `issue` whatever the `host` setting, since its reports go to CLM's own repository on GitHub, not the project's host (`defects-file-on-github`).
- `Azure`: the adapter `adapter` gives for `azure`, public so a caller can build it directly.
- `pr_open(host, root, branch, title, body file)`: `clm pr open` once the setting is loaded, each of the three optional: it fills the defaults, asks the adapter and keeps a body that didn't reach the pull request whole, giving the URL, nothing on `git` and `none`, or the `Error` the command reports.
- `keep_remote(root, host)`: on `none`, a clone with no `origin` gets one of its own, a bare repository in its git directory, so every command, claim and land runs through `origin` as on any other host (`none-keeps-its-own-remote`). On every other host it does nothing.
- the contribution operations, for a project taking contributions from people without write access (`outside-contributions-are-intent`): the fork's parent; bringing a fork's `main` up to its parent's; opening a pull request on the parent; listing the open issues and the open pull requests from forks (`list-contributions`), not those of branches pushed to the repository itself, which `pr open` opens for reading a diff; and, for a pull request or issue by number, reading it, fetching a pull request's head, commenting, closing and labelling. `azure`, `git` and `none` have no such thing.
- `host_for_remote(url)`, the host a remote's URL names, for `clm init` (`host-for-remote`), and `setup_steps(host)`, the steps a person takes to set a repository on that host up for CLM, which `clm init` prints (`host-setup-steps`), so `clm init` holds no host's names or permissions itself.
- `alerts`, the open dependency alerts, for `clm intake` (`list-alerts`): each alert's number, severity, ecosystem, package and manifest path, so an alert reaches the backlog as an issue does (`dependency-alerts-through-intake`). `azure`, `git` and `none` have none.
- `runs`, the runs of a named pipeline on `main`, for `clm verify` (`list-runs`): each run's ID, head commit, whether it has completed, its conclusion and its URL, so a project whose slow tests run in its host's pipeline has their results recorded (`verify-after-land`). Only `github` lists them; the others answer that the host lists no pipeline runs.

Neither `branch push` nor `branch delete`, nor `branch_delete`, ever touches `main` or a branch under `clm/`, the ledgers' (`guard-blocks-push-to-main`): `main` moves only through `clm land` (`land-locally`).

A caller never runs `gh` or `az` itself; everything host-specific is here, and a new host is a new adapter. One caller is exempt, by name (`git-through-layer`): `clm-hooks`' release guard, which looks up the GitHub Actions run that a session's own `gh run rerun` targets, on the repository and host that command names, whatever the `host` setting says (`release-guard-runs-gh`).

## Design
Crate `clm-host`. One trait, `Host`, with these operations, and an implementation per setting. `github` runs `gh pr create --base main --head <branch> --title --body-file`, `gh pr view --json number,url,state,headRefName` and `gh api -X PATCH repos/{owner}/{repo}/pulls/<n> -F body=@<file>` (the REST call, since `gh pr edit` fails on `gh` versions that query the retired Projects (classic) API), and `gh issue create`. `branch_delete` is `git push --force-with-lease=refs/heads/<branch>:<sha> origin :refs/heads/<branch>` for every host, `<sha>` being the lease; `clm branch delete` gives it `--lease` or, without it, the tip of `refs/remotes/origin/<branch>`. A lease that fails because `origin` no longer has the branch is told from one that fails because it moved by asking `origin` again with `git ls-remote`, keeping only the line whose name is exactly `refs/heads/<branch>`, since `ls-remote`'s pattern also matches refs that merely end in it.

**A relative body file.** Every read of a body file joins it to the repository root, as the binary runs at the root it found (`root-from-cwd`), and the path a tool is given names that file: `gh` runs in the root, and the Azure adapter makes the path absolute against the root.

**Azure DevOps** (`azure-pull-requests`, `azure-through-az-cli`). `azure` runs the Azure CLI through `clm-git`'s `run`, with `--detect true` so `az` takes the organisation, project and repository from the clone's remote, and `--output json`; `az` holds the sign-in (`az login`, or `AZURE_DEVOPS_EXT_PAT`), so CLM reads no token. `pr open` first runs `az repos pr list --source-branch <branch> --target-branch main --status active` and, when it lists a pull request, gives the first one's URL; otherwise it runs `az repos pr create --source-branch <branch> --target-branch main --title <title> --description @<file>`. Azure's `--source-branch` takes the bare branch name. The URL is the pull request's web page, `<repository.webUrl>/pullrequest/<pullRequestId>`, from the JSON `az` returns, since the `url` field there is the REST API's. `pr body` finds the current branch's active pull request the same way and runs `az repos pr update --id <pullRequestId> --description @<file>`. Every other operation is the trait's default, as for `git` (`azure-other-operations`).

**Azure's description limit** (`azure-body-limit`). Azure DevOps refuses a pull request description over 4000 characters, and `az` with it, so the adapter fits the body before `az` sees it. Counting UTF-16 code units never counts fewer than Azure does. A cut never splits a surrogate pair, and the part kept has its trailing line breaks trimmed before the note; it goes to a temporary file in `std::env::temp_dir()`, removed once the call returns. The line that says a body was cut reaches the command that called the adapter, which writes it on its own call's error stream, never the process's. The limit is the Azure adapter's own fact; `github`'s limit is far beyond any body CLM writes, so no other adapter cuts (`yagni`).

**An answer `az` gives without the pull request** (`azure-unreadable-answer`). A `create` that succeeds without `pullRequestId` or `repository.webUrl` may well have opened the pull request, so the adapter lists again before it gives up, and quotes what `az` did answer.

**Keeping a body** (`keep-unopened-body`). A body lives in the caller's worktree (`.clm/pr-body.md`, ignored by git), which `clm run --finish`, `clm wrap-up` and a draft's end remove, so a body that reached no pull request whole would go with it; the common git directory outlives every worktree. One helper in `clm-host` gives the path, so `pr_open` and the Azure adapter, which keeps a cut body itself (`pr body` taking the branch from `current_branch`), write the same file. The file's name is the branch's with `/` written as `-` and `-` as `~`: git never allows `~` in a branch name, so the map is one-to-one, though a filesystem that ignores letter case still gives `spec/X` and `spec/x` one file. The `.markdown` ending marks the form: copies an older `clm` kept end in `.md`, with only `/` written as `-`, so no name of the new form equals an old one, and an old copy is never written over or read as another branch's. Old copies stay where they are, read by nothing, until the person who reads one deletes it. Nothing deletes the copies: they are written only when a body failed to reach a pull request whole, and the person who reads one deletes it.

**Pushing a branch** (`branch-push`). `branch push` reads the checked-out branch with `git symbolic-ref --short HEAD` and pushes `HEAD:refs/heads/<branch>` to `origin`, setting the branch's upstream to `origin/<branch>`. When `refs/remotes/origin/<branch>` exists it passes `--force-with-lease=refs/heads/<branch>` with no value, which leases on that ref's tip, and `--force-if-includes`, which further fails the push when that tip isn't in the local branch's reflog; git ignores `--force-if-includes` beside an explicit lease value, so the value is left out. Without the remote-tracking ref it passes `--force-with-lease=refs/heads/<branch>:`, which expects no branch on `origin`. So it is a compare-and-swap on what the session last saw (`lease-claims`): a branch another session pushed since the last fetch fails git's lease check (`stale info`), and one a fetch brought in but the session never had locally fails `--force-if-includes`, so a session overwrites only its own work. A rebased claim branch passes, since its old tip is in its reflog. It retries nothing: a transient failure is the caller's to retry, as the prompts say. Before the push it asks `origin` which tip it holds, with `git ls-remote origin refs/heads/<branch>` read for that exact name. When that tip is already the commit, the push moves nothing, and the tip may be another clone's that this branch was made from, so it leaves the local ref `refs/clm/pushed/<branch>` as it was, as after a push that fails, and as when `origin` can't be asked. Otherwise, once the push goes through, it sets that ref to the commit, its record that this clone pushed that tip, which `branch delete` reads. This leaves a narrow window: another clone pushing this exact commit between the `ls-remote` and the push is recorded as this clone's push. `git push --porcelain` would tell from the push itself, but it writes git's reason for a failed ref, such as `stale info`, to standard output, and `clm-git` carries only a failed command's standard error; the human chose to keep the window rather than change `vcs` for this one caller.

**Protected branches.** `branch push`, `branch delete` and `branch_delete` check the name against `main` and `clm/` before any git call that writes, and `branch push` checks that `HEAD` names a branch. The check is `clm-host`'s own: the guard sees only `clm branch ...`, never the `git push` it runs (`plugin.hooks.guard/push-guard`), and `clm land` deletes every branch it lands through `branch_delete`.

**Deleting only what this clone pushed** (`branch-delete`). The project's allow rules let an unattended session run `clm branch delete` with no classifier judging it (`unattended-steps-run-clm`), so the command itself must not release another session's claim or delete its spec branch, which would bypass the claim's expiry and takeover (`lease-claims`). So before the delete it goes on only when one of two records names the lease as a tip this clone pushed. The first is `branch push`'s own, `refs/clm/pushed/<branch>`, at the lease. The second is the newest entry of `refs/remotes/origin/<branch>`'s reflog (`git reflog show -n 1 --format='%H %gs' refs/remotes/origin/<branch>`), when it is git's `update by push` at the lease. Git writes that entry when a push from this clone to `origin` moves the branch, whether `clm claim`, `clm branch push`, `clm autosync` or a plain `git push` made it, and a fetch that brings another clone's tip writes a different entry, so a tip another session pushed never counts even after this clone fetched it. A fetch that finds the branch unchanged writes no entry, so a session's own branch stays deletable after `clm rebase` fetches. No record and no reflog, as when `core.logAllRefUpdates` is off, counts as not pushed here: it fails closed. `origin` can't be reached when `git ls-remote origin refs/heads/<branch>` fails, whether `branch delete` runs it before the record check or `branch_delete` runs it again after a push that fails for any reason but a moved tip, as `clm land` decides it is offline.

**Why the record**. The reflog alone races in a clone that many sessions share. Every worktree of a clone shares its remote-tracking refs, and the sessions fetch all the time. When another session's fetch lists `origin` after a push has landed there but before the push has moved this clone's remote-tracking ref, the fetch stores the tip first, with `fetch origin: storing head`; the push's own update is then to the value the ref already holds, which git writes no reflog entry for. So the newest entry names a fetch, and the session's own branch was left on `origin` (a draft's stub `spec/<id>`, holding its items). A fetch never writes under `refs/clm/`, so the record holds whatever the fetches do. It is a ref rather than a file because it names a commit, and like the checkpoint's review mark it is the clone's own, shared by its worktrees and never pushed: nothing lands, approves or passes a check on it, and losing it only falls back to the reflog. A tip another clone pushed later differs from the record, so the record never makes another session's tip deletable. Only `branch push` writes it: the other pushes of a session branch (`clm claim`'s first push, `clm autosync`) keep the reflog alone, and a session's later `branch push` of a claimed branch writes the record. `branch delete` deletes `refs/clm/pushed/<branch>` once `origin` has no such branch, whether it deleted the branch or found it gone. `clm land`'s delete of a landed branch goes through `branch_delete` and leaves the branch's record: one ref per landed branch, naming a commit `main` holds, which costs nothing kept and which a later `branch delete` of that name, or `git update-ref -d`, removes. Before the record check `branch delete` asks `origin` with `git ls-remote origin refs/heads/<branch>`, read for that exact name, so a draft deleting an `autosync/spec/<id>` that was never pushed finds nothing to do. The record check is the subcommand's, not the `Host` trait's: `clm land`'s delete of the branch it just landed keeps its plain lease.

**Defaults** (`pr-open-defaults`). Each argument given overrides its default alone, so the drafts and folds that write their own body keep passing it. `pr_open` fills in what is missing and then calls the `Host` trait's `page_open`, which keeps taking all three; `crates/clm` stays dispatch only. The title and body are drawn through `clm-git`'s `git` like every other git call (`git-through-layer`). The change is found from the branch's content, never its name: the paths `git diff --name-only --no-renames --diff-filter=A <base>..<branch>` lists (`--no-renames`, so a change withdrawn and redrafted under a new ID still reads as added), `<base>` being `git merge-base origin/main <branch>`, that are a `changes/<id>/change.md` outside `changes/archive/`; one such path is the change, and none or several mean no change. It is read with `git show <branch>:<path>`; its `## Intent` section runs to the next `## ` heading, trimmed, and the title comes from the section's first paragraph, which ends at the first line that is empty or holds only whitespace, the same end the `## Cuts` section has. This reading of "the change a branch adds" is `host`'s own: `clm land` counts added changes its own way, archive copies included, and the two share no code. The subjects come from `git log --reverse --format=%s origin/main..<branch>`. A derived body goes to a temporary file in `std::env::temp_dir()`, outside the worktree, removed once the call returns, so git sees nothing. The parsing is text only: `clm-host` names nothing of `model` itself, and a change's `## Intent` heading is fixed by `spec-format`.

**Contribution operations.** On `github` the fork's parent and the head's details come from `gh repo view --json isFork,parent` and `gh pr view --json`, a fork's `main` is brought up with `gh repo sync`, a pull request on the parent opens with `gh pr create --repo <parent> --base main --head <owner>:<branch>`, a head is fetched with `git fetch origin pull/<number>/head:refs/heads/<branch>`, and comments, closing and labels use `gh issue` or `gh pr`. What they read from the host is a stranger's text, so a caller prints it as data and acts on none of it.

**The cuts in a body** (`pr-open`, `pr-body`). `clm-host` depends on `clm-gate` for `evaluate_at` and nothing else of it. The section goes first so that a host's limit, which cuts a body at its end (`azure-body-limit`), never cuts the list. It is added to whatever body is sent, given or drawn, since the skills pass a body of their own, and a leading `## Cuts` section such a body already holds is taken off first: the section is the gate's to write, so a kept copy or a body read back from the pull request can be sent again. A body kept because it didn't reach the pull request whole (`keep-unopened-body`) is kept with the section.

## Requirements

### Requirement: branch-delete
`clm branch delete <branch> [--lease <sha>]` SHALL delete `origin`'s branch when its tip is `<sha>`, on every host, `<sha>` defaulting to the tip of `refs/remotes/origin/<branch>`, and SHALL leave a branch whose tip has moved from a lease this clone pushed, exiting 1 with `` `<branch>` on origin is no longer at <sha>; left it ``, `<sha>` written as the full commit hash the lease names. It SHALL delete only a tip this clone pushed, checking that before whether the tip has moved: when neither `refs/clm/pushed/<branch>` is `<sha>` nor the newest entry of `refs/remotes/origin/<branch>`'s reflog is a push from this clone at `<sha>`, it SHALL delete nothing and exit 1 saying this clone didn't push that tip. Once `origin` has no such branch, whether it deleted it or found it gone, it SHALL leave no `refs/clm/pushed/<branch>`. For `main` or a branch whose name starts with `clm/`, it SHALL delete nothing and exit 1 naming the reason. With no `origin`, or when `origin` has no such branch, it SHALL delete nothing on `origin` and exit 0, before the other checks. When `git push` fails for any reason but a moved tip while `origin` still answers, as when its hook rejects the delete, it SHALL print git's message and exit 1, and when `origin` can't be reached it SHALL exit 1 printing `offline: origin can't be reached; ` followed by git's message, in either case leaving `origin`'s branch and `refs/clm/pushed/<branch>` as they were.

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

#### Scenario: the remote refuses the delete
- GIVEN `impl/a` pushed from this clone with `clm branch push` at `s`, and then a `pre-receive` hook on `origin` that exits 1 printing `remote says no`
- WHEN `clm branch delete impl/a` runs
- THEN it exits 1, its error output holds `remote says no`, `origin/impl/a` is still `s`, and `refs/clm/pushed/impl/a` is still `s`

#### Scenario: origin can't be reached
- GIVEN `impl/a` pushed from this clone with `clm branch push` at `s`, and then `origin`'s URL set to a directory that doesn't exist
- WHEN `clm branch delete impl/a` runs
- THEN it exits 1, its error output holds `offline: origin can't be reached`, and `refs/clm/pushed/impl/a` is still `s`

#### Serves
- capability/git-host
- capability/land-safely
- architecture/host-adapters

### Requirement: issue
`clm issue --title <title> --body-file <file> --repo <repo>` SHALL, on `github`, run `gh issue create --repo <repo>` with the title and body and print the URL; on `git` and `none` it SHALL print the body and say there is nowhere to file it, exiting 1. On `git` and `none`, a body file that can't be read SHALL make it exit 1 naming the file, printing no report and not saying there is nowhere to file it.

#### Scenario: filed
- GIVEN `host = "github"` and a fake `gh` that prints a URL
- WHEN `clm issue --title "x" --body-file r.md --repo clunny/clm` runs
- THEN `gh` got `issue create --repo clunny/clm` with the title and the body, and the URL is printed

#### Scenario: nowhere to file
- GIVEN `host = "git"`
- WHEN `clm issue --title "x" --body-file r.md --repo clunny/clm` runs
- THEN it prints the report, says there is nowhere to file it, and exits 1

#### Scenario: an unreadable report
- GIVEN `host = "git"`, and in turn `host = "none"`, and no `r.md`
- WHEN `clm issue --title "x" --body-file r.md --repo clunny/clm` runs
- THEN it exits 1, its output is empty, and its error output names `r.md` and doesn't say there is nowhere to file it

#### Serves
- capability/report-defects
- capability/git-host
- architecture/host-adapters

### Requirement: git-through-layer
No crate in the workspace but `clm-git`, `clm-host` and `clm-guard` SHALL run the `gh` CLI, whether through `clm_git::gh` or through `clm_git::run` or `clm_git::run_within` with `"gh"` as its program; `clm-guard`'s only `gh` call SHALL be the release guard's run lookup (`plugin.hooks.guard/release-guard`, `release-guard-runs-gh`): one `run` or `run_within` call with `"gh"` as its program, and no `clm_git::gh`.

The scan reads each file with its whitespace removed, so a call rustfmt splits across lines still matches. A *`run` call with `"gh"` as its program* is the text `run(` or `run_within(`, with or without a path before it, whose second top-level argument (after the first comma outside brackets and quotes) is the literal `"gh"`; other `"gh"` literals in `clm-guard`, such as its tokenizer's `t == "gh"`, aren't calls. Outside the exempt crates any `"gh"` literal counts, whatever surrounds it.

No crate but `clm-git` and `clm-host` SHALL run the Azure CLI: outside those two, no `src/` file SHALL hold the string literal `"az"`, whatever surrounds it, `clm-guard` included (`azure-through-az-cli`).

`clm-web`'s `gh` module is an HTTP client for GitHub's API that runs no process, so it isn't a call to the `gh` CLI and the rule doesn't reach it; `web` reaches GitHub through its GitHub App (`host-adapters`, `web-optional`).

#### Scenario: no gh elsewhere
- GIVEN the workspace
- WHEN a test reads every `src/` file of every crate but `clm-git`, `clm-host` and `clm-guard`
- THEN none calls `clm_git::gh`, by its path or by importing it, and none holds the string literal `"gh"`

#### Scenario: no az elsewhere
- GIVEN the workspace
- WHEN a test reads every `src/` file of every crate but `clm-git` and `clm-host`
- THEN none holds the string literal `"az"`

#### Scenario: the release guard's lookup only
- GIVEN the workspace
- WHEN a test reads every `src/` file of `clm-guard`
- THEN none calls `clm_git::gh`, and exactly one `run` or `run_within` call, across all of them, has the literal `"gh"` as its program

#### Scenario: a run of gh is caught
- GIVEN the source texts `clm_git::run(&dir, "gh", &args, options)` and `clm_git::run_within(&dir, "gh", &args, options, limit)`, and each split across lines as rustfmt splits it
- WHEN the matcher for other crates and the `clm-guard` call counter each read them
- THEN both report a `gh` call in each, and the counter reports none in `t == "gh"` or in `prerun_within(&dir, "gh")`

#### Needs
- clm/no-watch-crate
- plugin.hooks.guard/git-through-layer

#### Serves
- architecture/host-adapters

### Requirement: pr-open
`clm pr open <branch> --title <title> --body-file <file>` SHALL, on `host = "github"`, open a pull request from the branch to `main` with that title and body through `gh`, or print the URL of the open one that already exists, and print the pull request's URL. On a host that has pull requests, when `clm gate` would read the branch against `origin/main` as kind `spec` with any cut (`gate/cuts`), the body it sends SHALL start with the line `## Cuts`, then one `- <line>` per cut as `clm gate` prints it, then a blank line, before the body given or drawn from the branch; for any other branch the body is sent as it is. A body given or drawn that itself starts with the line `## Cuts` SHALL first lose that section, whatever the branch cuts: the line and every line after it up to and including the first line that is empty or holds only whitespace, or to the body's end when there is none, so a body sent again never holds the section twice and a stale one goes. The body file SHALL be passed as given only when there is no cut and no such section. It SHALL evaluate the gate only once every check it made before has passed, directly before the call that sends the body. When the gate can't be evaluated it SHALL exit 1 naming why, without that call. A relative body file SHALL be read against the repository root the command works in, on every host, the body sent being that file's contents and never a file relative to another directory. A `--body-file` that can't be read SHALL make it exit 1 naming the file, opening nothing and keeping no body.

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

#### Scenario: a relative body file
- GIVEN a repository whose root holds `b.md` reading `root body`, and another directory, where the command runs, holding a `b.md` reading `other body`; with `host = "github"` and a fake `gh`, and in turn `host = "azure"` and a fake `az` that lists no active pull request, answers `pr create` with pull request 12 and lists it for `pr body`
- WHEN `clm pr open spec/x --title x --body-file b.md --root <repository>` runs, and `clm pr body b.md --root <repository>` on the checked-out `spec/x`
- THEN on each host the body the tool got, for each command, is `root body`

#### Scenario: an unreadable body file
- GIVEN `host = "github"` and a fake `gh` whose `pr view` finds no pull request, and in turn `host = "azure"` and a fake `az` that lists no active pull request, and no `missing.md`
- WHEN `clm pr open spec/x --title x --body-file missing.md` runs
- THEN it exits 1, its error output names `missing.md`, no `pr create` ran, and no `clm/pr-bodies/` file exists

#### Needs
- gate/cuts

### Requirement: pr-body
`clm pr body <file>` SHALL set the body of the current branch's open pull request to the file's contents through the host, changing nothing else of the pull request, and print `updated #<number>`. For a branch with cuts the body SHALL start with the `## Cuts` section as `pr-open` gives it, so a corrected body keeps the list, evaluated and failing as `pr-open` says, and a leading `## Cuts` section of the file SHALL be replaced as `pr-open` replaces it. It SHALL exit 1 calling no host when the file is missing or blank, a file holding nothing but a leading `## Cuts` section counting as blank, or when `HEAD` is detached, and SHALL exit 1 changing no pull request when the branch has no open pull request; on `git` and `none` it SHALL print nothing and exit 0. A relative `<file>` SHALL be read against the repository root as `pr-open` says.

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
- GIVEN an empty `body.md`, and in turn one holding only spaces, tabs and line breaks, no `body.md` at all, and a `body.md` holding only the lines `## Cuts` and `- an older line`
- WHEN `clm pr body body.md` runs
- THEN it exits 1 and `gh` wasn't called

#### Scenario: detached head
- GIVEN `host = "github"`, a fake `gh` that reports an open PR 7, a `body.md` holding `## Intent`, and a detached `HEAD`
- WHEN `clm pr body body.md` runs
- THEN it exits 1 and `gh` got no `api` call

#### Needs
- gate/cuts

### Requirement: contribution-operations
`clm-host` SHALL offer, on `github`, through `gh` and `clm-git`'s layer: `parent`, the fork's parent as `<owner>/<name>`, or nothing when `origin` isn't a fork; `sync`, bringing `origin`'s `main` up to its parent's without force; `open_on_parent(branch, title, body file)`, opening a pull request from `<owner>:<branch>` to the parent's `main`, or returning the URL of the open one; `show(number)`, returning a pull request's author, its author's numeric account ID (from `gh api users/<login>`, which a `Co-authored-by` trailer's `<id>+<login>@users.noreply.github.com` address needs), state, title, body, head repository URL, head branch, whether maintainers may edit and changed files, or an issue's author, its author's numeric account ID, state, title and body; `fetch(number, branch)`, making a local branch at a pull request's head without checking it out, refusing a branch that exists; `comment(number, body file, close)`, adding a comment and closing when `close` is set; and `label(number, label)`. On `git` and `none`, `parent` SHALL return nothing and each other operation SHALL report that there is no host, running no `gh`. On `github`, `sync` and `open_on_parent` SHALL refuse when `origin` isn't a fork, running no `gh repo sync` and no `gh pr create`.

#### Scenario: a fork's parent
- GIVEN `host = "github"` and a fake `gh` whose `repo view` reports a fork of `clunny/clm`
- WHEN `parent` runs
- THEN it returns `clunny/clm`

#### Scenario: opening on the parent
- GIVEN that fake `gh`, with `origin` owned by `dana`
- WHEN `open_on_parent` runs for `spec/x`
- THEN `gh` got `pr create` with `--repo clunny/clm`, `--base main` and `--head dana:spec/x`

#### Scenario: a pull request's files
- GIVEN a fake `gh` reporting pull request 7 with changed files `backlog/x.md` and `crates/a/src/lib.rs`
- WHEN `show` runs for 7
- THEN it returns both files, the head repository's URL, whether maintainers may edit and the author's numeric account ID

#### Scenario: closing with a comment
- GIVEN a fake `gh`
- WHEN `comment` runs for 7 with `close` set
- THEN `gh` got the comment's body and closed 7

#### Scenario: no host
- GIVEN `host = "none"`
- WHEN `show` runs for 7
- THEN it reports that there is no host, and no `gh` ran

#### Scenario: not a fork
- GIVEN `host = "github"` and a fake `gh` whose `repo view` reports a repository that isn't a fork
- WHEN `sync` runs, and `open_on_parent` for `spec/x`
- THEN each returns `Refused`, and `gh` got neither `repo sync` nor `pr create`

#### Serves
- capability/contribute-from-outside

### Requirement: list-contributions
`clm-host` SHALL offer `list`, which on `github` returns the repository's open issues and the open pull requests whose head is in another repository, each with its number, whether it is an issue or a pull request, and its author's login, through `gh issue list` and `gh pr list` with `--state open` and `--json`, each with `--limit 1000` rather than `gh`'s default of 30, `gh pr list`'s `--json` also asking for `isCrossRepository`, and SHALL return a pull request only when `gh` reports `isCrossRepository` true, leaving out one whose head is a branch of the repository itself and one reported without the field, since every branch CLM pushes opens one for reading its diff (`pr open`) and none of those is a contribution (`outside-contributions-are-intent`); an author whose account is gone is `ghost`, as `gh` reports it. It SHALL return no title, body or other text the author wrote, so nothing a caller prints from it carries a stranger's words. On `git` and `none` it SHALL return an empty list and run no `gh`. An item without a number SHALL be left out, and an answer that isn't a JSON list SHALL make it return an error.

#### Scenario: issues and pull requests
- GIVEN `host = "github"` and a fake `gh` reporting open issue 73 by `clunny` and open pull request 80 by `dana` from a fork
- WHEN `list` runs
- THEN it returns issue 73 by `clunny` and pull request 80 by `dana`, and `gh` got `--state open` for both

#### Scenario: the repository's own branch
- GIVEN `host = "github"` and a fake `gh` reporting open pull request 80 by `dana` from a fork and open pull request 81 by `clunny` whose head is `impl/a` in the repository itself
- WHEN `list` runs
- THEN it returns pull request 80 by `dana` only

#### Scenario: no host
- GIVEN `host = "none"`
- WHEN `list` runs
- THEN it returns nothing, and no `gh` ran

#### Scenario: an answer it can't read
- GIVEN `host = "github"` and a fake `gh` whose `issue list` reports issue 73 by `clunny` and an issue with no number, and in turn one whose `issue list` prints `not json`, and one whose `issue list` prints `{"message": "x"}`
- WHEN `list` runs
- THEN the first gives issue 73 alone, and the other two return an error

#### Serves
- capability/contribute-from-outside

### Requirement: branch-push
`clm branch push` SHALL push the checked-out branch to `origin`'s branch of the same name and set its upstream there, on every host: when the clone has `refs/remotes/origin/<branch>`, only while `origin`'s branch is still at that ref's tip and the local branch has held that tip, as git's `--force-if-includes` judges it; when it hasn't, only while `origin` has no such branch. It SHALL work on the branch checked out in the directory it runs in, and SHALL print `pushed <branch> <sha>`, `<sha>` the full commit `HEAD` names, and exit 0 when the push goes through or `origin` already holds that commit. When the push moved `origin`'s branch it SHALL set the local ref `refs/clm/pushed/<branch>` to `<sha>`; when `origin` already held the commit, or the push is refused, it SHALL leave that ref as it was. With no `origin` remote it SHALL push nothing and exit 1. When git refuses the push it SHALL print git's message, the remote's lines included, and exit 1, leaving `origin`'s branch as it was. On a detached `HEAD`, on `main` or on a branch whose name starts with `clm/`, it SHALL push nothing and exit 1 naming the reason.

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

#### Scenario: another clone's tip pushed again
- GIVEN `origin/impl/b` pushed to `u` by another clone and fetched, and a local `impl/b` made from it and checked out at `u`
- WHEN `clm branch push` runs, then `clm branch delete impl/b`
- THEN the push exits 0, this clone has no `refs/clm/pushed/impl/b`, `origin/impl/b` is still `u`, and the delete exits 1

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

### Requirement: list-alerts
`clm-host` SHALL offer `alerts`, which on `github` returns the repository's open Dependabot alerts, each with its number, severity (`low`, `medium`, `high` or `critical`), ecosystem, package name and manifest path, through `gh api --paginate` on `repos/{owner}/{repo}/dependabot/alerts` with `state=open`, so every page is read, taking the number from `number`, the severity from `security_advisory.severity`, and the ecosystem, package and manifest path from `dependency.package.ecosystem`, `dependency.package.name` and `dependency.manifest_path`. A field holding whitespace or a control character SHALL come back with each such character replaced by `?`, a severity outside those four as `unknown`, and a missing or empty field as `unknown`, so a caller that prints a field prints one word. It SHALL return no advisory summary, description or other text of the advisory. When `gh` fails, as it does when alerts are off or the login may not read them, it SHALL return an error carrying `gh`'s message. On `git` and `none` it SHALL return an empty list and run no `gh`. An answer that isn't JSON, a page that isn't a list, or an alert without a number SHALL make it return an error.

#### Scenario: open alerts
- GIVEN `host = "github"` and a fake `gh` reporting open alert 4, `high`, ecosystem `npm`, package `vite`, manifest `web/package-lock.json`
- WHEN `alerts` runs
- THEN it returns alert 4 with those fields, and `gh` got `api`, `--paginate` and `state=open`

#### Scenario: a field with a space
- GIVEN a fake `gh` reporting open alert 5 whose package is `a b`
- WHEN `alerts` runs
- THEN it returns alert 5 with package `a?b`

#### Scenario: alerts unreadable
- GIVEN a fake `gh` that exits 1 on `api`, printing `HTTP 403`
- WHEN `alerts` runs
- THEN it returns an error whose message holds `HTTP 403`

#### Scenario: no host
- GIVEN `host = "none"`
- WHEN `alerts` runs
- THEN it returns nothing, and no `gh` ran

#### Scenario: alerts it can't read
- GIVEN a fake `gh` whose `api` prints `not json`, and in turn `{"message": "off"}`, and a page holding one alert with a severity and no number
- WHEN `alerts` runs on each
- THEN it returns an error each time

#### Serves
- capability/keep-a-backlog
- capability/git-host

### Requirement: azure-pull-requests
`clm-host` SHALL offer the `Azure` adapter, which `adapter` gives for `azure`. Its `pr open` for a branch, a title and a body file SHALL give the web URL (`<repository web URL>/pullrequest/<id>`) of the branch's active pull request when `az repos pr list --source-branch <branch> --target-branch main --status active` lists one, and otherwise SHALL open one from the branch to `main` with that title and the file's contents as its description through `az repos pr create --source-branch <branch> --target-branch main --title <title> --description @<file>`, and give its web URL. Its `pr body` for a file SHALL set the description of the current branch's active pull request to the file's contents through `az repos pr update --id <id> --description @<file>`, changing nothing else, and report `updated #<id>`; it SHALL fail without a call to `az` when the file is missing or blank, and with no `az repos pr update` when the branch has no active pull request. Every `az` call SHALL pass `--detect true` and `--output json`. When `az` fails, as it does when it isn't installed, lacks the `azure-devops` extension or isn't signed in, the operation SHALL fail with `az`'s message.

#### Scenario: opened
- GIVEN a fake `az` that records its calls, lists no active pull request for `spec/x`, and answers `pr create` with `pullRequestId` 12 and `repository.webUrl` `https://dev.azure.com/o/p/_git/r`
- WHEN the `Azure` adapter's `pr open` runs for `spec/x` with title `x` and body file `body.md`
- THEN `az` got `repos pr create` with `--source-branch spec/x`, `--target-branch main`, `--title x` and `--description @body.md`, and it gives `https://dev.azure.com/o/p/_git/r/pullrequest/12`

#### Scenario: already open
- GIVEN the same, with a fake `az` whose `pr list` reports active pull request 12 for `spec/x`
- WHEN the `Azure` adapter's `pr open` runs for `spec/x`
- THEN `az repos pr create` isn't called and it gives that pull request's web URL

#### Scenario: body updated
- GIVEN the current branch `spec/x` with active pull request 12 according to a fake `az`, and a file `body.md`
- WHEN the `Azure` adapter's `pr body` runs for `body.md`
- THEN `az` got `repos pr update` with `--id 12` and `--description @body.md` and nothing else that changes the pull request, and it reports `updated #12`

#### Scenario: az not signed in
- GIVEN a fake `az` that exits 1 printing `Please run 'az login'`
- WHEN the `Azure` adapter's `pr open` runs for `spec/x`
- THEN it fails with a message holding `az login`

#### Serves
- capability/git-host
- architecture/host-adapters

### Requirement: azure-other-operations
The `Azure` adapter's operations other than `pr open`, `pr body` and the branch deletion every host shares SHALL behave as on `git`, running no `az` and no `gh`: `issue` SHALL give the report and say there is nowhere to file it, as a failure; `parent` SHALL return nothing; `list` and `alerts` SHALL return an empty list; and `sync`, `open_on_parent`, `show`, `fetch`, `comment` and `label` SHALL report that there is no host.

#### Scenario: issue
- GIVEN fake `az` and `gh` that record their calls
- WHEN the `Azure` adapter's `issue` runs with title `x` and body file `r.md`
- THEN it fails giving the report and saying there is nowhere to file it, and neither fake was called

#### Scenario: contributions and alerts
- GIVEN the same
- WHEN the `Azure` adapter's `parent`, `list`, `alerts` and `show` for 7 run
- THEN `parent` returns nothing, `list` and `alerts` return nothing, `show` reports that there is no host, and neither fake was called

#### Serves
- capability/git-host
- architecture/host-adapters
- architecture/yagni

### Requirement: host-for-remote
`clm-host` SHALL offer `host_for_remote(url)`, giving the `host` setting a remote's URL names: `github` when the URL's host is `github.com` or `ssh.github.com`, `azure` when it is `dev.azure.com` or `ssh.dev.azure.com` or ends in `.visualstudio.com`, and `git` for any other URL. It SHALL parse the host out of the URL's forms `https://[user@]host[:port]/...`, `http://...`, `ssh://[user@]host[:port]/...` and `user@host:path`, and compare it whole and without regard to case, so a path or a longer host that merely contains one of those names gives `git`. It SHALL run no process.

#### Scenario: hosts named
- GIVEN the URLs `https://github.com/o/r.git`, `git@github.com:o/r.git`, `https://o@dev.azure.com/o/p/_git/r`, `git@ssh.dev.azure.com:v3/o/p/r`, `https://o.visualstudio.com/p/_git/r` and `ssh://git@example.com/r.git`
- WHEN `host_for_remote` reads each
- THEN it gives `github`, `github`, `azure`, `azure`, `azure` and `git`

#### Scenario: a name in the path
- GIVEN the URLs `https://example.com/dev.azure.com/r.git`, `https://o.visualstudio.com.example.org/r` and `https://example.com/github.com/r.git`
- WHEN `host_for_remote` reads each
- THEN it gives `git` for each

#### Serves
- capability/git-host
- capability/adopt-clm
- architecture/host-adapters

#### Needs
- config/host-setting

### Requirement: host-setup-steps
`clm-host` SHALL offer `setup_steps(host)`, the lines a person follows to set a repository on that host up for CLM, which `clm init` prints among its next steps: on `azure`, installing the Azure CLI with its `azure-devops` extension and signing in with `az login`, then, if `main` has branch policies, granting the identity that runs `clm land` Azure DevOps' `Bypass policies when pushing` permission on `main`; on `github`, if `main` is protected, adding the identity that runs `clm land` to the bypass list of the rule or ruleset that protects it; on `git`, if the server protects `main`, letting the identity that runs `clm land` push it; on `none`, nothing. Each line about a protected `main` SHALL name `clm land`. It SHALL run no process.

#### Scenario: azure
- GIVEN `host = "azure"`
- WHEN `setup_steps` runs
- THEN its lines name `azure-devops`, `az login`, `Bypass policies when pushing` and `clm land`

#### Scenario: github and none
- GIVEN `host = "github"`, and in turn `host = "none"`
- WHEN `setup_steps` runs
- THEN the first's lines name `clm land` and none of `az login`, `azure-devops` or `Bypass policies when pushing`, and the second gives no line

#### Serves
- capability/git-host
- capability/adopt-clm
- architecture/host-adapters

#### Needs
- config/host-setting

### Requirement: azure-selected
With `host = "azure"`, `clm-host`'s `adapter` SHALL give the `Azure` adapter (`azure-pull-requests`, `azure-other-operations`), so `clm pr open` and `clm pr body` run `az` as those requirements say, `clm issue` and the contribution operations behave as on `git`, and a failure of `az` makes the command exit 2 printing `az`'s message (`host-tool-failure`).

#### Scenario: pull request through the setting
- GIVEN a repository `clm.toml` with `host = "azure"` and a fake `az` that records its calls, lists no active pull request for `spec/x`, and answers `pr create` with `pullRequestId` 12 and `repository.webUrl` `https://dev.azure.com/o/p/_git/r`
- WHEN `clm pr open spec/x --title "x" --body-file body.md` runs
- THEN `az` got `repos pr create`, and it prints `https://dev.azure.com/o/p/_git/r/pullrequest/12`

#### Scenario: az not signed in
- GIVEN `host = "azure"` and a fake `az` that exits 1 printing `Please run 'az login'`
- WHEN `clm pr open spec/x --title "x" --body-file body.md` runs
- THEN it exits 2 and its output holds `az login`

#### Scenario: issue through the setting
- GIVEN `host = "azure"` and fake `az` and `gh` that record their calls
- WHEN `clm issue --title "x" --body-file r.md --repo clunny/clm` runs
- THEN it prints the report, says there is nowhere to file it, exits 1, and neither fake was called

#### Needs
- config/host-setting

#### Serves
- capability/git-host
- architecture/host-adapters

### Requirement: list-runs
`clm-host` SHALL offer `runs(workflow)`, which on `github` returns the runs of that workflow on branch `main`, newest first, each with its ID, head commit, whether it has completed, its conclusion and its URL, through `gh run list --workflow <workflow> --branch main --event push --limit 100 --json databaseId,headSha,status,conclusion,url`, taking completed from `status` being `completed` and the conclusion from `conclusion`. A head commit that isn't 40 hex digits SHALL drop the run, and a conclusion that is missing, empty or holds anything but lower-case letters and `_` SHALL come back as `unknown`, so a caller prints one word. It SHALL return no run's title, log or other text. When `gh` fails it SHALL return an error carrying `gh`'s message. On `azure`, `git` and `none` it SHALL return an error saying the host lists no pipeline runs, and run no `gh` or `az`. A run without a numeric ID SHALL be dropped, and an answer that isn't a JSON list SHALL make it return an error.

#### Scenario: runs listed
- GIVEN `host = "github"` and a fake `gh` reporting a completed run 11 of `slow` on commit `c` with conclusion `failure`, and a run 12 still `in_progress`
- WHEN `runs("slow")` runs
- THEN it returns both, run 11 completed with conclusion `failure` and its URL, run 12 not completed, and `gh` got `run list`, `--workflow slow`, `--branch main` and `--event push`

#### Scenario: odd conclusion
- GIVEN a fake `gh` reporting a completed run whose conclusion is `Fail ure`
- WHEN `runs("slow")` runs
- THEN the run comes back with conclusion `unknown`

#### Scenario: no runs on other hosts
- GIVEN `host = "azure"`
- WHEN `runs("slow")` runs
- THEN it returns an error saying the host lists no pipeline runs, and neither `gh` nor `az` was called

#### Scenario: a run it can't read
- GIVEN a fake `gh` reporting run 11 on commit `c` and a run on commit `c` with no `databaseId`, and in turn one whose `run list` prints `not json`, and one whose `run list` prints `{"message": "x"}`
- WHEN `runs("slow")` runs
- THEN the first gives run 11 alone, and the other two return an error

#### Serves
- capability/slow-tests-after-land

### Requirement: none-keeps-a-remote
`clm-host` SHALL offer `keep_remote(root, host)`. When `host` is `none` and the clone has no remote named `origin`, it SHALL make `clm/remote.git` under the clone's common git directory a bare repository, keeping one already there, add it as `origin` by its absolute path, and return that path; it SHALL push nothing and write nothing in any worktree. On any other host, or when the clone has `origin`, it SHALL change nothing and return nothing. Run from a linked worktree, it SHALL give every worktree of the clone the same `origin`. It SHALL make the bare repository even in a clone the environment names with `GIT_DIR` and `GIT_WORK_TREE` (a work tree whose git directory is elsewhere), while reading the remotes, finding the common git directory and adding `origin` still run on the clone those variables name.

#### Needs
- vcs/git-isolated

#### Scenario: no remote
- GIVEN `host = "none"`, a clone with a commit on `main` and no remote
- WHEN `keep_remote` runs
- THEN `<common git dir>/clm/remote.git` is a bare repository, `git remote get-url origin` prints its absolute path, it has no branch, and the working tree is unchanged

#### Scenario: from a linked worktree
- GIVEN the same clone with no remote, and a linked worktree of it
- WHEN `keep_remote` runs in the linked worktree
- THEN the bare repository is under the common git directory, not the worktree's, and `git remote get-url origin` prints the same path in both checkouts

#### Scenario: a clone the environment names
- GIVEN `host = "none"`, a work tree `w` with a commit on `main` and no remote, whose git directory `g` is elsewhere, and `GIT_DIR=g` and `GIT_WORK_TREE=w` set in the environment `keep_remote` runs in
- WHEN `keep_remote(w, none)` runs
- THEN it succeeds, `g/clm/remote.git` is a bare repository, and `git remote get-url origin`, run with those variables, prints its absolute path

#### Scenario: already kept
- GIVEN the clone after *no remote*, its `origin` then removed with `git remote remove origin`
- WHEN `keep_remote` runs
- THEN `origin` names the same bare repository again, and what it held is still there

#### Scenario: nothing to do
- GIVEN `host = "none"` with an `origin` naming another bare repository, and in turn `host = "git"` with no remote
- WHEN `keep_remote` runs in each
- THEN the remotes are unchanged, no `clm/remote.git` exists, and it returns nothing

### Requirement: pr-open-defaults
`clm pr open` SHALL take its branch, `--title` and `--body-file` as optional, each one given overriding only its own default. On `git` and `none` it SHALL print `none` and call nothing, whatever arguments it gets and whatever `HEAD` is, checking the setting before anything else. Otherwise, without a branch it SHALL use the checked-out branch, and exit 1 without a host call when `HEAD` is detached. Without `--title` or `--body-file`, when the branch adds, against its merge base with `origin/main`, exactly one `changes/<id>/change.md` outside `changes/archive/`, whatever the branch's name, and that file's `## Intent` section holds more than whitespace, the body SHALL be that section's text, up to the next `## ` heading and trimmed, and the title its first sentence: drawn from the section's first paragraph, which ends at the first line that is empty or holds only whitespace, with its lines trimmed and joined by single spaces, ending at the first `.`, `?` or `!` followed by a space or the paragraph's end and keeping that mark, or the whole paragraph when no mark ends a sentence in it; otherwise the body SHALL be one `- <subject>` line per commit in `origin/main..<branch>`, oldest first, and the title the oldest commit's subject. A branch with no such change and no commit beyond `origin/main` SHALL exit 1 without a host call. When the defaults are needed and the branch can't be read against `origin/main`, as in a clone with no `origin/main` or a branch sharing no history with it, it SHALL exit 1 with a message naming `origin/main` and holding git's message, if git printed one, before any host call. `pr_open` SHALL fill in the defaults as `clm pr open` does, given the host setting, the root and the same optional arguments, and return the URL, nothing on `git` and `none`, or the error the command reports.

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
- GIVEN `host = "none"` and a detached `HEAD`, and in turn `host = "git"`
- WHEN `clm pr open` runs with no arguments, and in turn as `clm pr open spec/x --title "x" --body-file body.md`
- THEN it prints `none`, exits 0, and runs no `gh`

#### Scenario: the change the branch adds
- GIVEN `host = "github"`, a fake `gh`, and in turn the checked-out branch `fix/y` adding `changes/y/change.md` whose `## Intent` reads `Fix it. Now.`; `spec/x` adding `changes/other/change.md` whose `## Intent` reads `Other. Words.`; `spec/x` whose `changes/x/change.md` `origin/main` already holds, with one commit `edit` beyond it; and `fix/w` adding two changes in one commit `both`
- WHEN `clm pr open` runs with no arguments on each
- THEN `gh` got `--title` `Fix it.`, `Other.`, `edit` and `both` in turn, with the bodies `Fix it. Now.`, `Other. Words.`, `- edit` and `- both`

#### Scenario: a title from the first paragraph
- GIVEN `host = "github"`, a fake `gh`, and in turn the checked-out branch `spec/x` whose change's `## Intent` reads `Is it so? Yes.`; `No stop here`, then on the next line `across lines`, then a blank line and `Second. Para.`; `Version 1.2 ships. More.`; and `First line`, then a line holding only spaces, then `Next. Para.`
- WHEN `clm pr open` runs with no arguments on each
- THEN `gh` got `--title` `Is it so?`, `No stop here across lines`, `Version 1.2 ships.` and `First line` in turn

#### Scenario: no origin/main
- GIVEN `host = "github"`, a fake `gh`, and the checked-out branch `fix/z` with a commit `z`, in a clone whose `refs/remotes/origin/main` is deleted
- WHEN `clm pr open` runs with no arguments
- THEN it exits 1, its error output holds git's message naming `origin/main`, and `gh` wasn't called

### Requirement: azure-body-limit
The `Azure` adapter's `pr open` and `pr body` SHALL give `az` a description of at most 4000 characters, counted in UTF-16 code units. A body file within that SHALL be passed as it is. For a longer one they SHALL pass, through a temporary file outside the worktree, the longest leading part of the body that ends at a line break and leaves room for the note, or, when none does, the longest that ends at a character, followed by a blank line and the note `_Cut to fit Azure DevOps's 4000-character limit; the whole body is kept at <path>._`, the whole within 4000; SHALL keep the whole body as `keep-unopened-body` says, `<path>` naming that copy; and SHALL leave the body file unchanged. Through `clm pr open` and `clm pr body`, the command SHALL write `cut the body to 4000 characters; the whole is kept at <path>` to the error stream its call is given, and its output stream SHALL hold what it holds for a body within the limit.

#### Scenario: a long body
- GIVEN a fake `az` that records the description file's contents, lists no active pull request for `spec/x`, and answers `pr create` with `pullRequestId` 12 and `repository.webUrl` `https://dev.azure.com/o/p/_git/r`, and a `body.md` of 300 lines of 20 characters each
- WHEN the `Azure` adapter's `pr open` runs for `spec/x` with `body.md`
- THEN the description `az` got is at most 4000 characters, starts with `body.md`'s first line, ends with the note naming `<common git dir>/clm/pr-bodies/spec-x.markdown`, that file holds all 300 lines, `body.md` is unchanged, and it gives `https://dev.azure.com/o/p/_git/r/pullrequest/12`

#### Scenario: a short body
- GIVEN the same fake `az`, and a `body.md` of 10 lines
- WHEN the `Azure` adapter's `pr open` runs for `spec/x` with `body.md`
- THEN the description `az` got is `body.md`'s contents, and no `pr-bodies/spec-x.markdown` exists

#### Scenario: an update
- GIVEN the current branch `spec/x` with active pull request 12 according to a fake `az`, and a `body.md` of 300 lines of 20 characters each
- WHEN the `Azure` adapter's `pr body` runs for `body.md`
- THEN the description `az repos pr update` got is at most 4000 characters and ends with the note, and it reports `updated #12`

#### Scenario: the cut said through the command
- GIVEN `host = "azure"`, the fake `az` of *a long body*, and that `body.md`
- WHEN `clm-host`'s call for `clm pr open` runs with `spec/x --title x --body-file body.md` and two buffers
- THEN it returns 0, the error buffer holds `cut the body to 4000 characters; the whole is kept at <common git dir>/clm/pr-bodies/spec-x.markdown`, and the output buffer holds only `https://dev.azure.com/o/p/_git/r/pullrequest/12`

#### Serves
- capability/git-host
- architecture/host-adapters

### Requirement: azure-unreadable-answer
When `az repos pr create` succeeds with JSON that lacks `pullRequestId` or `repository.webUrl`, the `Azure` adapter's `pr open` SHALL run `az repos pr list --source-branch <branch> --target-branch main --status active` again and give the web URL of the first pull request it lists. When that lists none, or lists one whose JSON lacks those fields too, it SHALL fail with a message that starts `az repos pr create: no pullRequestId or repository.webUrl in the answer: ` and goes on with the text `az` printed to `create`, its runs of whitespace joined into single spaces and cut to its first 500 characters. When the first `az repos pr list` lists an active pull request whose JSON lacks `pullRequestId` or `repository.webUrl`, `pr open` SHALL fail with a message holding `no pullRequestId or repository.webUrl in the answer`, opening nothing; when it lacks `pullRequestId`, `pr body` SHALL fail the same way, updating nothing. Output of `az` that isn't JSON SHALL make the operation fail with a message holding `bad JSON` and the parse error.

#### Scenario: opened but unread
- GIVEN a fake `az` whose first `pr list` for `spec/x` lists nothing, whose `pr create` answers `{"status": "active"}`, and whose next `pr list` lists active pull request 12 with `repository.webUrl` `https://dev.azure.com/o/p/_git/r`
- WHEN the `Azure` adapter's `pr open` runs for `spec/x`
- THEN it gives `https://dev.azure.com/o/p/_git/r/pullrequest/12`

#### Scenario: nothing to read
- GIVEN a fake `az` whose `pr list` always lists nothing and whose `pr create` answers `{"status": "active"}`
- WHEN the `Azure` adapter's `pr open` runs for `spec/x`
- THEN it fails with a message holding `no pullRequestId or repository.webUrl in the answer` and `"status": "active"`

#### Scenario: listed without fields
- GIVEN a fake `az` whose `pr list` for `spec/x` lists `{"status": "active"}`, the current branch `spec/x` and a `body.md` holding `## Intent`
- WHEN the `Azure` adapter's `pr open` runs for `spec/x`, and its `pr body` for `body.md`
- THEN each fails with a message holding `no pullRequestId or repository.webUrl in the answer`, and `az` got neither `pr create` nor `pr update`

#### Scenario: an answer that isn't JSON
- GIVEN a fake `az` whose `pr list` lists nothing and whose `pr create` prints `not json`
- WHEN the `Azure` adapter's `pr open` runs for `spec/x`
- THEN it fails with a message holding `bad JSON`

#### Serves
- capability/git-host
- architecture/host-adapters

### Requirement: keep-unopened-body
When the host's `pr open` fails after `clm pr open`'s own checks have passed, on `github` or `azure`, `clm pr open` SHALL first write the body it was opening with (the `--body-file`'s contents, or the body drawn from the branch, in either case with the `## Cuts` section `pr-open` puts first for a branch that cuts anything) to `<common git dir>/clm/pr-bodies/<name>.markdown`, `<common git dir>` being the clone's common git directory, the one its worktrees share, and `<name>` the branch's name with each `/` written as `-` and each `-` as `~`, so two branches whose names differ other than in letter case never share a copy, creating the directories and replacing an earlier copy of that branch's, and its error SHALL end `; the body is kept at <path>` with that file's absolute path, so `clm pr open` exits with the status `host-tool-failure` gives a failing host's tool, 2, naming it. The `Azure` adapter SHALL write the whole body to the same path when it cuts one (`azure-body-limit`). When the gate can't be evaluated (`pr-open`) no body SHALL be kept, since none was made. A body that, with its `## Cuts` section, is empty or holds only whitespace SHALL not be kept, and when the copy can't be written the error SHALL be the host's alone, with no `; the body is kept at`. Nothing else SHALL be written there: a pull request that opens with its whole body leaves no copy, and a copy an older `clm` kept there, whose name ends `.md`, SHALL be left as it is.

#### Scenario: the host refuses
- GIVEN `host = "azure"`, a fake `az` that lists no active pull request for `impl/a` and exits 1 on `pr create` printing `TF401398`, and `.clm/pr-body.md` holding `## Review`
- WHEN `clm pr open impl/a --title x --body-file .clm/pr-body.md` runs
- THEN it exits 2, its output holds `TF401398` and the path of `<common git dir>/clm/pr-bodies/impl-a.markdown`, and that file holds `## Review`

#### Scenario: a kept body holds its cuts
- GIVEN `host = "github"`, a fake `gh` whose `pr create` exits 1, and a branch `spec/x` that removes requirement `a/x`
- WHEN `clm pr open spec/x --title x --body-file body.md` runs
- THEN it exits 2, and the kept file starts `## Cuts` and `- specs/a/SPEC.md: gate/cut: requirement a/x is gone`, followed by the contents of `body.md`

#### Scenario: from a linked worktree
- GIVEN the same, run in a linked worktree of the clone
- WHEN `clm pr open impl/a --title x --body-file .clm/pr-body.md` runs and the worktree is then removed
- THEN `<common git dir>/clm/pr-bodies/impl-a.markdown`, under the main repository's git directory, still holds `## Review`

#### Scenario: opened
- GIVEN `host = "github"` and a fake `gh` that prints a URL
- WHEN `clm pr open impl/a --title x --body-file body.md` runs
- THEN it prints the URL and no `clm/pr-bodies/` file exists

#### Scenario: two branches, two copies
- GIVEN `host = "github"`, a fake `gh` whose `pr create` exits 1, branches `spec/a-b` and `spec-a/b`, each beyond `origin/main` and cutting nothing, with bodies `one` and `two`, and an older `clm`'s copy `<common git dir>/clm/pr-bodies/spec-a-b.md` holding `old`
- WHEN `clm pr open spec/a-b --title x --body-file one.md` and `clm pr open spec-a/b --title x --body-file two.md` run
- THEN `<common git dir>/clm/pr-bodies/spec-a~b.markdown` holds `one`, `<common git dir>/clm/pr-bodies/spec~a-b.markdown` holds `two`, and `spec-a-b.md` still holds `old`

#### Scenario: nothing worth keeping
- GIVEN `host = "github"` and a fake `gh` whose `pr create` exits 1 printing `create failed`, with a `body.md` holding only spaces and line breaks on a branch that cuts nothing, and in turn a good `body.md` and a file where `<common git dir>/clm/pr-bodies` would be
- WHEN `clm pr open impl/a --title x --body-file body.md` runs
- THEN it exits 2 each time, its error output holds `create failed` and not `the body is kept at`, and no `clm/pr-bodies/impl-a.markdown` exists

#### Serves
- capability/git-host
- capability/build-from-spec

### Requirement: commands-in-crate
`clm-host` SHALL offer each of `clm pr`, `clm issue` and `clm branch` as a call of its own that takes the command's arguments as the command line gives them, the repository root the `clm` binary found from the directory it runs in, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and nothing to the output stream and return 2. When the repository's settings can't be loaded, as for a `clm.toml` naming no host CLM knows or a user settings file that sets `host`, the call for each of `clm pr open`, `clm pr body`, `clm issue`, `clm branch push` and `clm branch delete` SHALL write `clm <command>: ` and the error, which names the file, to the error stream, call nothing and return 2, `<command>` being `pr open`, `pr body`, `issue`, `branch push` or `branch delete`. For `--help`, given after the command or after one of its subcommands, it SHALL write that command's help to the output stream, nothing to the error stream, and return 0. The help's first line SHALL be the command's description, then an empty line, then its `Usage:` line naming the command in full:
- `clm pr`: ``A branch's pull request on the host the `host` setting names``, then `Usage: clm pr <COMMAND>`;
- `clm pr open`: ``Open the branch's pull request against `main` on the host, or find the open one, and print its URL, or `none` without a host``, then a line starting `Usage: clm pr open `;
- `clm pr body`: ``Set the body of the current branch's open pull request to a file's contents``, then a line starting `Usage: clm pr body `;
- `clm issue`: ``File an issue where the host the `host` setting names keeps them and print its URL; without a host, print the report and exit 1``, then a line starting `Usage: clm issue `;
- `clm branch`: ``Remote branches, through the host adapter the `host` setting names``, then `Usage: clm branch <COMMAND>`;
- `clm branch push`: ``Push the checked-out branch to `origin`, overwriting only what this clone last saw``, then a line starting `Usage: clm branch push `;
- `clm branch delete`: ``Delete `origin`'s branch when its tip is still `--lease` (default: the tip last fetched) and this clone pushed it, on every host``, then a line starting `Usage: clm branch delete `.

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `host`'s own files
- WHEN it calls `clm-host`'s call with the arguments of `clm pr` and two buffers
- THEN the buffers hold what `clm pr` prints to standard output and to standard error there, and the call returns the status `clm pr` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm pr` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Scenario: settings that don't load
- GIVEN a repository whose `clm.toml` sets `host = "nowhere"`, and in turn a repository with no such fault and an environment lookup whose `XDG_CONFIG_HOME` names a directory holding `clm/config.toml` that sets `host`
- WHEN the call for `clm issue --title x --body-file r.md --repo o/r`, and in turn for `clm pr open`, runs there
- THEN it returns 2, the error buffer starts `clm issue: ` (or `clm pr open: `) and names `clm.toml` in the first case and `config.toml` in the second, and no host's tool ran

#### Scenario: help
- GIVEN each of the three calls
- WHEN it is given `--help` for each of `clm pr`, `clm pr open`, `clm pr body`, `clm issue`, `clm branch`, `clm branch push` and `clm branch delete`
- THEN it returns 0 with nothing in the error buffer, and its output buffer starts with that command's description, an empty line and its `Usage:` line, as stated above

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

### Requirement: host-tool-failure
When the host's tool, `gh` on `github` or `az` on `azure`, can't be run, exits non-zero, or gives an answer the adapter can't read, `clm pr open`, `clm pr body` and `clm issue` SHALL exit 2, with a message that holds what the tool printed on its error output, or why its answer can't be read, on every host that has the tool. A refusal that a requirement of `host` states as exit 1, such as `pr-body`'s for a blank body file or a branch with no pull request, `pr-open`'s for a gate that can't be evaluated, or `issue`'s on a host with nowhere to file it, SHALL stay exit 1, so a caller tells a host's failure from the command's refusal the same way whichever host the project uses.

#### Scenario: gh fails to open a pull request
- GIVEN `host = "github"` and a fake `gh` whose `pr view` finds no pull request and whose `pr create` exits 1 printing `create failed`
- WHEN `clm pr open spec/x --title x --body-file body.md` runs
- THEN it exits 2 and its error output holds `create failed`

#### Scenario: gh fails to file an issue
- GIVEN `host = "github"` and a fake `gh` whose `issue create` exits 1 printing `create failed`
- WHEN `clm issue --title x --body-file r.md --repo o/r` runs
- THEN it exits 2 and its error output holds `create failed`

#### Scenario: az fails to set a body
- GIVEN `host = "azure"`, the current branch `spec/x`, a file `body.md` holding `## Intent`, and a fake `az` that exits 1 printing `Please run 'az login'`
- WHEN `clm pr body body.md` runs
- THEN it exits 2 and its error output holds `az login`

#### Scenario: an answer that can't be read
- GIVEN `host = "github"`, the current branch `spec/x`, a file `body.md` holding `## Intent`, and a fake `gh` whose `pr view` prints `not json`
- WHEN `clm pr body body.md` runs
- THEN it exits 2 and its error output holds `bad JSON`

### Requirement: branch-delete-call
`adapter(host)`'s `branch_delete(root, branch, lease)` SHALL, on every host, delete `origin`'s `branch` when its tip is `lease`, reading no record of who pushed it, and return `Ok`. For `main` or a branch whose name starts with `clm/` it SHALL delete nothing and return `Refused` naming the reason, before any git call and whatever `origin` holds. Any other branch it SHALL leave, returning `Ok`, when the clone has no `origin` or `origin` has no branch of exactly that name. When `origin`'s branch has moved from `lease` it SHALL delete nothing and return `Refused` with `` `<branch>` on origin is no longer at <lease>; left it ``. When `git push` fails for any other reason while `origin` still answers, as when its hook rejects the delete, it SHALL delete nothing and return `Refused` with git's message, and when `origin` can't be reached it SHALL return `Refused` with `offline: origin can't be reached; ` followed by git's message.

#### Scenario: the call land makes
- GIVEN a clone whose `origin` holds `impl/a` at `s`, pushed by another clone and with no `refs/clm/pushed/impl/a` here; `impl/b`, moved on `origin` from `s` to `t`; no `impl/gone`, while `origin` holds the ref `refs/x/refs/heads/impl/gone`; `main` at `m`; `clm/ledger/p/c`; and `impl/c`, whose every update a `pre-receive` hook on `origin` rejects printing `remote says no`; in turn a clone with no `origin`; and in turn a clone whose `origin` names a directory that doesn't exist
- WHEN `branch_delete` runs for `impl/a` with lease `s`, `impl/b` with lease `s`, `impl/gone` with lease `s`, `main` with lease `m`, `clm/ledger/p/c` and `impl/c` each at its tip, in the clone with no `origin` for `impl/a` with lease `s` and for `main` with lease `m`, and in the clone whose `origin` can't be reached for `impl/a` with lease `s`
- THEN `origin` no longer has `impl/a` and that call returns `Ok`; `impl/b` is still `t` and the call returns `Refused` with `` `impl/b` on origin is no longer at s; left it ``; the call for `impl/gone` returns `Ok`; `main`, `clm/ledger/p/c` and `impl/c` are unchanged, the first two calls returning `Refused` and the third `Refused` holding `remote says no`; the clone with no `origin` returns `Ok` for `impl/a` and `Refused` for `main`; and the clone whose `origin` can't be reached returns `Refused` starting `offline: origin can't be reached`

#### Serves
- capability/land-safely
- architecture/host-adapters
