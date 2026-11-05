---
component: plugin.hooks.guard
---

## MODIFIED Requirements

### Requirement: release-guard <!-- base: c8e4bdbbeede14ff -->
`clm hook guard` SHALL exit 2, writing a reason that names `humans-cut-releases` to stderr, for a `Bash` command with a segment that:
- runs `gh workflow run` whose workflow argument (its first word after `workflow run` that isn't an option or the value of `-R`/`--repo`) names the release workflow or the site workflow: a file name `release.yml`, `release.yaml`, `site.yml` or `site.yaml`, a path ending in one of them after a `/`, or `release` or `site`, each in any case; or
- runs `gh release` with `create`, `edit`, `upload`, `delete` or `delete-asset`; or
- runs `gh api` with a token containing `/dispatches` or `/releases`, and with a `-f`, `-F`, `--field`, `--raw-field` or `--input` token, or with `-X` or `--method` set to anything but `GET`; or
- runs `gh run rerun` with a run ID, or with a job ID given by `-j` or `--job` (`-j <id>`, `-j<id>`, `--job <id>` or `--job=<id>`), when a run it re-runs is a release run or can't be looked up (`flaky-failures-rerun`). A site run counts as a release run: re-running it would deploy the site again. A segment with both is judged by each. Re-running a release run would publish, merge or deploy again. A `gh run rerun` with neither stays allowed: `gh` then asks for a run interactively, which a session's Bash can't answer.

The guard SHALL look each run up through the REST API, which every `gh` version serves: `gh api [--hostname <host>] repos/<owner>/<repo>/actions/runs/<run-id>`, and for a job ID first `gh api [--hostname <host>] repos/<owner>/<repo>/actions/jobs/<job-id>`, whose `run_id` names the run. The repository is `[<host>/]<owner>/<repo>`, and a host in it becomes `--hostname <host>`. When the segment has `-R`/`--repo`, which wins as it does for `gh`, its value is the repository; a value that isn't of that form counts as a failed lookup. Otherwise the guard SHALL first run `gh repo view --json url`, once per segment, which resolves the repository as `gh run rerun` does, and take the repository from the `url` it reports: the part after `://` split at `/` into exactly three non-empty parts, `<host>/<owner>/<repo>`, so `--hostname <host>` is always passed. A `repo view` that fails, or a `url` that isn't of that form, makes every lookup of the segment a failed lookup. Each `gh` call SHALL run in the checkout the segment runs in, found as `push-guard` finds a push's checkout (the event's `cwd`, moved by each earlier `cd <dir>` it can follow), with the segment's own `GH_REPO=` and `GH_HOST=` assignments in its environment, so `gh` resolves the repository and host as it would for the re-run, and the lookup asks that host. A run is a release run when its `path`, cut at any `@`, has the file name `release.yml`, `release.yaml`, `site.yml` or `site.yaml` in any case, or its `name` is `release` or `site` in any case. A lookup that fails, returns a job whose `run_id` isn't a positive integer, or returns a run without a string `path` and `name` counts as a release run. The guard SHALL stop looking up at the first release run, and its `gh` calls SHALL share a budget of 30 seconds, so the guard answers before the hook's own timeout, which Claude Code treats as allowing. Each call SHALL run through `clm-git`'s `run_within` with what remains of the budget as its limit (`bounded-runs`), so one still running when the budget runs out is killed, not left running after the hook exits, and counts as a failed lookup; a call not yet started when the budget is spent SHALL NOT be started, and counts as a failed lookup.

Reading releases and workflow runs, and re-running a run of any other workflow, SHALL stay allowed.

In the scenarios below, a fake `gh` for `o/r` answers `repo view --json url` with `https://github.com/o/r`, unless the scenario says otherwise.

#### Scenario: dispatching a release
- GIVEN the commands `gh workflow run release.yml -f version=0.2.0`, `gh workflow run Release.YAML` and `gh workflow run RELEASE`
- WHEN `clm hook guard` runs on each
- THEN each exits 2 naming `humans-cut-releases`

#### Scenario: dispatching another workflow
- GIVEN the command `gh workflow run ci.yml --ref release`
- WHEN `clm hook guard` runs on it
- THEN it exits 0

#### Scenario: creating a release directly
- GIVEN the command `gh release create v0.2.0 --generate-notes`
- WHEN `clm hook guard` runs on it
- THEN it exits 2

#### Scenario: dispatching through the API
- GIVEN the command `gh api repos/o/r/actions/workflows/release.yml/dispatches -f ref=main -f inputs[version]=0.2.0`
- WHEN `clm hook guard` runs on it
- THEN it exits 2

#### Scenario: reading releases
- GIVEN the commands `gh release list`, `gh release view v0.1.0`, `gh release download v0.1.0 -p 'clm-*'` and `gh run list --workflow release.yml`
- WHEN `clm hook guard` runs on each
- THEN each exits 0

#### Scenario: re-running a release run
- GIVEN a fake `gh` for `o/r` whose `api repos/o/r/actions/runs/77` reports `name` `Release` and `path` `.github/workflows/release.yml`
- WHEN `clm hook guard` runs on `gh run rerun 77 --failed`
- THEN it exits 2 naming `humans-cut-releases`

#### Scenario: a release workflow by another file or name
- GIVEN a fake `gh` for `o/r` whose `api repos/o/r/actions/runs/77` reports `name` `Publish` and `path` `.github/workflows/release.yaml`, and whose `actions/runs/78` reports `name` `release` and `path` `.github/workflows/publish.yml`, and whose `actions/runs/79` reports `name` `Publish` and `path` `.github/workflows/release.yml@refs/heads/main`
- WHEN `clm hook guard` runs on `gh run rerun 77`, `gh run rerun 78` and `gh run rerun 79`
- THEN each exits 2

#### Scenario: re-running a CI run
- GIVEN a fake `gh` for `o/r` that, like `gh` 2.45, fails on `run view --json` with the field `path`, and whose `api repos/o/r/actions/runs/42` reports `name` `CI` and `path` `.github/workflows/ci.yml`
- WHEN `clm hook guard` runs on `gh run rerun 42 --failed`
- THEN it exits 0

#### Scenario: re-running a release run's job
- GIVEN a fake `gh` for `o/r` whose `api repos/o/r/actions/jobs/900` reports `run_id` `77`, and whose `actions/runs/77` reports `name` `Release` and `path` `.github/workflows/release.yml`
- WHEN `clm hook guard` runs on `gh run rerun --job 900`, `gh run rerun -j 900`, `gh run rerun -j900`, `gh run rerun --job=900` and `gh run rerun 42 --job 900`, where run `42` is a CI run
- THEN each exits 2

#### Scenario: re-running a CI run's job
- GIVEN a fake `gh` for `o/r` whose `api repos/o/r/actions/jobs/901` reports `run_id` `42`, and whose `actions/runs/42` reports `name` `CI` and `path` `.github/workflows/ci.yml`
- WHEN `clm hook guard` runs on `gh run rerun --job 901`
- THEN it exits 0

#### Scenario: the re-run's repository and host
- GIVEN a fake `gh` that records the arguments, directory and `GH_REPO` and `GH_HOST` of each call, answers `repo view --json url` with `https://ghe.example.com/o/r` when its `GH_REPO` is `o/r` and with `https://github.com/p/q` otherwise, and reports a CI run for any run, and an event whose `cwd` has a subdirectory `other`
- WHEN `clm hook guard` runs on `cd other && gh run rerun 42`, `gh run rerun 42 -R o/r`, `gh run rerun 42 --repo ghe.example.com/o/r`, `GH_REPO=o/r gh run rerun 42 -R x/y` and `GH_HOST=ghe.example.com GH_REPO=o/r gh run rerun 42`
- THEN each exits 0, and the run lookups ask, with these arguments in any order before the path: `api --hostname github.com repos/p/q/actions/runs/42` in `other`, after a `repo view` in `other`; `api repos/o/r/actions/runs/42`; `api --hostname ghe.example.com repos/o/r/actions/runs/42`; `api repos/x/y/actions/runs/42`; and `api --hostname ghe.example.com repos/o/r/actions/runs/42`, after a `repo view` with `GH_HOST` `ghe.example.com` and `GH_REPO` `o/r`; all but the first in the event's `cwd`, and only the first and last after a `repo view`

#### Scenario: a repository that can't be read
- GIVEN a fake `gh` for `o/r` that reports a CI run for any lookup
- WHEN `clm hook guard` runs on `gh run rerun 42 -R https://github.com/o/r` and on `gh run rerun 42 -R r`
- THEN each exits 2

#### Scenario: a repository that can't be resolved
- GIVEN a fake `gh` that reports a CI run for any `api` lookup, and fails on `repo view`, or answers it with a `url` of `o/r`
- WHEN `clm hook guard` runs on `gh run rerun 42` with each
- THEN each exits 2

#### Scenario: run can't be looked up
- GIVEN a fake `gh` for `o/r` that fails on `api`, one whose `api` prints something that isn't a run or a job, one whose job reports `run_id` `"x"`, one that never answers `api`, and one that never answers `repo view`
- WHEN `clm hook guard` runs on `gh run rerun 42` and on `gh run rerun --job 900` with each
- THEN each exits 2, the last two within 35 seconds; and the last two, each of which writes its process ID to a file and then `exec`s its sleep, have no process left with that ID once the guard has exited

#### Scenario: dispatching the site
- GIVEN the commands `gh workflow run site.yml`, `gh workflow run .github/workflows/Site.YAML` and `gh workflow run SITE`
- WHEN `clm hook guard` runs on each
- THEN each exits 2 naming `humans-cut-releases`

#### Serves
- capability/guard-sessions

#### Needs
- vcs/bounded-runs
- host/git-through-layer
