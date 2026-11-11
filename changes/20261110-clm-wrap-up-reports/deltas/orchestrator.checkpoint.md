---
component: orchestrator.checkpoint
---

## MODIFIED Requirements

### Requirement: checkpoint-reports-unsafe-work <!-- base: 8fdf5e8ba4b07d5d -->
`clm wrap-up` SHALL report, one line each, work CLM's flow makes that isn't safe on `origin`: a checkout on a `spec/*` or `impl/*` branch with uncommitted changes or untracked files that aren't ignored is *dirty*; a local `spec/*` or `impl/*` branch that isn't landed (`checkpoint`) is *unpushed* when it has commits its `origin` branch lacks, or no `origin` branch; and this clone's ledger branch is *unpushed* when it holds a record `origin` lacks. It SHALL print `dirty <path>` and `unpushed <branch>` for the session's own work: the checkout it runs in, the branch checked out there and the clone's ledger. It SHALL print `elsewhere dirty <path>` and `elsewhere unpushed <branch>` for the rest, a build's worktree in a `.claude/worktrees/agent-*` directory and its branch always among them. It SHALL also print `open <branch>` for each local `spec/*` or `impl/*` branch that is pushed but not landed, `complete <id>` for each change the queue reports complete on `origin/main`'s tree, extracted, and the violations of every `clm check` rule on that tree, each line starting `origin/main: ` before the violation's `<path>: <rule>: <message>`. It SHALL judge that tree with `check`'s `run_listed` (`check/run-listed`), given the files of the commit it extracted as `git ls-tree -r --name-only` lists them, so the rules that read the tracked files see the files `clm check` sees in a checkout of that commit, those under directories whose names start with `.` among them. The `open`, `elsewhere`, `complete`, `freed`, `place` and `left` lines SHALL NOT make the wrap-up incomplete.

#### Scenario: unpushed work
- GIVEN the checkout `clm wrap-up` runs in on local branch `spec/z`, with a commit `origin/spec/z` lacks
- WHEN `clm wrap-up` runs
- THEN it prints `unpushed spec/z` and `wrap-up incomplete`, and exits 1

#### Scenario: unpushed ledger
- GIVEN this clone's ledger branch with a record `origin` lacks
- WHEN `clm wrap-up` runs
- THEN it prints `unpushed clm/ledger/<writer>` and `wrap-up incomplete`, and exits 1

#### Scenario: open branch only
- GIVEN local branch `spec/z` pushed and unlanded, with nothing else outstanding
- WHEN `clm wrap-up` runs
- THEN it prints `open spec/z` and `wrap-up ok`, and exits 0

#### Scenario: unrecorded land
- GIVEN an `origin/main` with a merge commit no `landed` record names
- WHEN `clm wrap-up` runs
- THEN it prints `ledger/unrecorded-land` and `wrap-up incomplete`

#### Scenario: complete change
- GIVEN an `applied` change `x` in `origin/main` whose added requirements are all cited at their current versions
- WHEN `clm wrap-up` runs
- THEN it prints `complete x` and `wrap-up ok`, and exits 0

#### Scenario: human's files on main
- GIVEN the main checkout on `main` with an untracked `notes.txt`
- WHEN `clm wrap-up` runs
- THEN it prints no `dirty` line and exits 0

#### Scenario: another session's work
- GIVEN the main checkout on `spec/z` with an uncommitted change and a commit `origin/spec/z` lacks, and `clm wrap-up` run in a linked worktree `w` on `main`
- WHEN `clm wrap-up` runs in `w`
- THEN it prints `elsewhere dirty <main checkout>` and `elsewhere unpushed spec/z`, then `wrap-up ok`, and exits 0

#### Scenario: build's worktree is elsewhere
- GIVEN `clm wrap-up` run in a linked worktree `w`, and `.claude/worktrees/agent-1` on `impl/a` with an uncommitted change
- WHEN `clm wrap-up` runs in `w`
- THEN it prints `elsewhere dirty` with that worktree's path, then `wrap-up ok`, and exits 0

#### Scenario: outside glob under a hidden directory
- GIVEN an `origin/main` whose root sets `outside: [".azure-pipelines/**"]`, tracking `.azure-pipelines/ci.yml` in no component's `paths`, on which `clm check` passes
- WHEN `clm wrap-up` runs
- THEN it prints no `check/unowned-files` line, prints `wrap-up ok` and exits 0

#### Scenario: violation on main
- GIVEN an `origin/main` whose root sets `outside: []`, tracking `tools/x.sh` in no component's `paths`
- WHEN `clm wrap-up` runs
- THEN it prints a line starting `origin/main: tools/x.sh: check/unowned-files: ` and `wrap-up incomplete`, and exits 1

#### Serves
- capability/wrap-up-session
- architecture/git-is-truth

#### Needs
- check/run-listed
