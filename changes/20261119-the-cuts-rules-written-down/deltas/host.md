---
component: host
base: {}
---

## MODIFIED Requirements

### Requirement: pr-open <!-- base: bdaae01c39f6c334 -->
`clm pr open <branch> --title <title> --body-file <file>` SHALL, on `host = "github"`, open a pull request from the branch to `main` with that title and body through `gh`, or print the URL of the open one that already exists, and print the pull request's URL; on `git` and `none` it SHALL print `none` and call nothing. On a host that has pull requests, when `gate`'s `evaluate_at` for the branch against `origin/main` gives kind `spec` and any cut (`gate/cuts`), the body it sends SHALL start with the line `## Cuts`, then one `- <line>` per cut as `clm gate` prints it, then a blank line, before the body given or drawn from the branch; for any other branch the body is sent as it is. A body given or drawn that itself starts with the line `## Cuts` SHALL first lose that section, the line and every line after it up to and including the first blank one, so a body sent again never holds the section twice. It SHALL evaluate the gate only once every check it made before has passed, directly before the call that sends the body, and SHALL pass the body file as given when there is no cut. When the gate can't be evaluated it SHALL exit 1 naming why, without that call.

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
