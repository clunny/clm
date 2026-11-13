---
component: host
---

## MODIFIED Requirements
### Requirement: azure-pull-requests <!-- base: 03919edc54a95b11 -->
`clm-host` SHALL offer the `Azure` adapter, a public implementation of `Host` exported as `clm_host::Azure` that a caller can build and call directly, so its tests call it directly. Its `pr open` for a branch, a title and a body file SHALL give the web URL (`<repository web URL>/pullrequest/<id>`) of the branch's active pull request when `az repos pr list --source-branch <branch> --target-branch main --status active` lists one, and otherwise SHALL open one from the branch to `main` with that title and the file's contents as its description through `az repos pr create --source-branch <branch> --target-branch main --title <title> --description @<file>`, and give its web URL. Its `pr body` for a file SHALL set the description of the current branch's active pull request to the file's contents through `az repos pr update --id <id> --description @<file>`, changing nothing else, and report `updated #<id>`; it SHALL fail without a call to `az` when the file is missing or blank, and with no `az repos pr update` when the branch has no active pull request. Every `az` call SHALL pass `--detect true` and `--output json` and go through `clm-git`'s `run`. When `az` fails, as it does when it isn't installed, lacks the `azure-devops` extension or isn't signed in, the operation SHALL fail with `az`'s message.

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

### Requirement: azure-selected <!-- base: ef11e15de5c30a27 -->
With `host = "azure"`, `clm-host`'s `adapter` SHALL give the `Azure` adapter (`azure-pull-requests`, `azure-other-operations`), so `clm pr open` and `clm pr body` run `az` as those requirements say, `clm issue` and the contribution operations behave as on `git`, and a failure of `az` makes the command exit 1 printing `az`'s message. `adapter`'s arm for the `azure` value gives it, and this requirement's tests prove it through the commands. `capability/git-host`'s Azure DevOps scenario, which drives `clm pr open` with a stubbed `az`, sees the adapter once this requirement holds.

#### Scenario: pull request through the setting
- GIVEN a repository `clm.toml` with `host = "azure"` and a fake `az` that records its calls, lists no active pull request for `spec/x`, and answers `pr create` with `pullRequestId` 12 and `repository.webUrl` `https://dev.azure.com/o/p/_git/r`
- WHEN `clm pr open spec/x --title "x" --body-file body.md` runs
- THEN `az` got `repos pr create`, and it prints `https://dev.azure.com/o/p/_git/r/pullrequest/12`

#### Scenario: az not signed in
- GIVEN `host = "azure"` and a fake `az` that exits 1 printing `Please run 'az login'`
- WHEN `clm pr open spec/x --title "x" --body-file body.md` runs
- THEN it exits 1 and its output holds `az login`

#### Scenario: issue through the setting
- GIVEN `host = "azure"` and fake `az` and `gh` that record their calls
- WHEN `clm issue --title "x" --body-file r.md --repo clunny/clm` runs
- THEN it prints the report, says there is nowhere to file it, exits 1, and neither fake was called

#### Serves
- capability/git-host
- architecture/host-adapters

#### Needs
- config/host-setting
