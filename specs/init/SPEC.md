---
title: Init
status: active
owners: ["@clunny"]
depends_on: [model, config, vcs]
paths: ["crates/clm-init/**"]
successor: ""
serves: [capability/adopt-clm]
---

## Contract
Bringing a repository under CLM (`init-cli-and-skill`). The deterministic half; the conversation and the drafting are `plugin`'s `/clm:init` skill.

- `clm init [--lang <language>]... [--no-detectors] [--host none|git|github]` sets up CLM's tooling in the CLM repository found from the current directory, or at the top of the git repository when there is none (`init-root`), and prints the steps only a human can do. Run again, even after upgrading `clm`, it regenerates nothing: every file it writes belongs to the project once written, since no workflow is generated (`land-locally`). The one file it edits rather than writes whole is `.gitignore`, to which it adds each of `clm.local.toml`, `.clm/` and `.claude/worktrees/` that git doesn't already ignore there, or all three when it has no `.gitignore` (`local-settings`, `init-ignores-session-files`). The root component it writes owns the files `clm init` writes, and sets `outside`, which turns on `clm check`'s path ownership, to the repository's existing code, so the bootstrap commit passes it before the first spec graph lands (`incremental-adoption`).
- `clm survey` prints a compact map of the CLM repository found from the current directory, or of the whole git repository when there is none (`root-from-cwd`), for drafting its spec graph (`brownfield-survey`). `clm survey --components <file>` checks a proposed component map against the code and exits 1 with what's wrong. The map may leave part of the code for later, as `outside` globs, as the root's `outside` does (`incremental-adoption`); it checks coverage over the same code files `clm check`'s path ownership reads, so a map it accepts passes that rule once written as the spec graph.
- `import_edges(root)`, a library call, returns the file-to-file import edges `clm survey` finds in the repository at `root`, for `check`'s `undeclared-imports` rule, which `clm check` runs on the spec graph's own components (`undeclared-imports`).
- `clm guide [<page>]` prints CLM's guide, the pages of `docs/guide/` and the spec format, `docs/spec-format.md`, compiled into the binary, and the page `constraints`, generated from CLM's carried constraints: with no page, the index of page names and titles; with one, that page's Markdown (`method-in-the-plugin`, `guide-in-the-binary`, `spec-format-in-the-guide`). It reads no file at run time, so it works offline, in any repository or none, and prints the guide of the version that runs. No link in the guide leads outside it, so every link works in an adopter's project and in `clm guide`'s output.

Files `clm init` writes, each only when missing, and the one it adds lines to:

| File | Owner |
|---|---|
| `clm.toml` | project |
| `scripts/clm-check.sh` | project |
| `ARCHITECTURE.md`, `CAPABILITIES.md`, `specs/SPEC.md`, `CLAUDE.md` | project |
| `.gitignore`: the lines `clm.local.toml`, `.clm/` and `.claude/worktrees/`, each added when missing | project |

## Design
Crate `clm-init`. Templates are compiled into the binary (`include_str!`), so a given `clm` version always writes the same bytes for the same inputs, and tests compare output to fixtures.

**`clm init`.** Fails outside a git repository. It writes in the CLM repository found from the current directory as `root-from-cwd` finds it, and at the top of the git repository when there is none, so rerunning it from a subdirectory, or in a CLM repository that is a subdirectory such as `tool/`, writes that repository's files instead of starting a second one. `clm-init` finds the repository itself, by the same rule as `clm-hooks`' `clm_root`, because it doesn't depend on `clm-hooks`. Then:
- *`clm.toml`*, when missing: `host`, `approval = "any"` with a comment on `other`, `[land] check = "scripts/clm-check.sh"`, and one `[[detectors]]` table per `--lang` from a built-in table (at least `rust`, `typescript` and `python`; each with its `hint` and a `requires` naming the language's manifest, `Cargo.toml`, `package.json` or `pyproject.toml`, so a repository without it skips the detector until it lands, and `typescript`'s tool run from `node_modules/.bin/`, so one without its dependencies installed is told to install them: `detector-requires-manifest`), or none with `--no-detectors`, which `/clm:init` passes for an existing codebase so it passes `clm check` before its dead code is cleaned up. `host` is `--host` when given; otherwise `github` when `origin`'s URL names `github.com`, `git` for any other remote, and `none` with no remote. It reads the URL from `git remote get-url origin` and calls no host.
- *`scripts/clm-check.sh`*, when missing: an executable starter that builds and tests for each `--lang` (`cargo test`, `npm ci && npm test`, `pytest`), each line run only once the language's manifest exists, so the bootstrap of a repository with no code yet passes it, for the project to edit. It is what `clm land` runs before `clm check`.
- *`ARCHITECTURE.md`*, *`CAPABILITIES.md`*, *`specs/SPEC.md`* and *`CLAUDE.md`*, when missing: an `ARCHITECTURE.md` of a short preamble, a pointer to `clm guide constraints` for CLM's carried constraints, which reach the project from the `clm` it runs rather than as a copy in the file (`carried-constraints`), with how a project replaces or drops one by a `#### Replaces` block, and an empty `## Constraints` section for the project's own entries (`model/human-layers`), so a new repository starts with no entry its own tests would have to prove; a `CAPABILITIES.md` of a short preamble and an empty `## Capabilities` section, for the project's capabilities; a root component titled after the repository with `status: unbuilt` and no requirements; and a `CLAUDE.md` with CLM's working rules for sessions and a pointer to the guide as `clm guide`, never as a URL, so an adopter's sessions read the guide of the `clm` they run, offline (`method-in-the-plugin`). The rules that must hold live in `clm` and the plugin's skills and hooks, so an older `CLAUDE.md` never weakens them. `clm check` passes on the result.
- *`.gitignore`*, in the same directory as `clm.toml`: three lines CLM needs ignored, in this order. `clm.local.toml` is `config`'s settings file for one clone, never committed (`local-settings`); `.clm/` holds a session's scratch files, such as a pull request body, and `clm run`'s and `clm draft`'s plans and prompts; and `.claude/worktrees/` holds the worktrees `plugin`'s skills cut their session branches in and Claude Code puts subagents' worktrees in (`init-ignores-session-files`). Left unignored, `git status` lists them, `git worktree remove` refuses a worktree holding a pull request body, and `git add -A` commits a scratch file or records a worktree as an embedded repository. An entry is *listed* when git already ignores it there: `git check-ignore -q --no-index <path>`, run through `clm-git` in that directory, exits 0 for the path `clm.local.toml`, `.clm/pr-body.md` or `.claude/worktrees/x/f` respectively (`reuse-first`), one call per entry in that order, since `-q` with several paths can't tell them apart. `--no-index` has git read the patterns whatever the index holds: without it git reports a tracked file as not ignored, so a repository that commits `clm.local.toml` would get the line again on every run (`init-rerun`). `clm-git`'s `git` reports exit 1 as a failure carrying its status, which `clm-init` reads as not listed, as `clm-git`'s own `is_ancestor` does, so `vcs` needs nothing new. So git's own rules decide, not a matcher of CLM's (an ignore file elsewhere, such as a parent directory's `.gitignore`, counts too), and the line `clm init` appends goes last, after any negation, so it takes effect. A git failure fails `clm init` before `.gitignore` is touched; files written before it stay written. That includes git refusing a probe path, as when `.clm` is a symbolic link: it fails closed. Because git reads the machine's own configuration, the tests run `clm init` with `GIT_CONFIG_GLOBAL` and `XDG_CONFIG_HOME` pointing into an empty temporary directory and `GIT_CONFIG_NOSYSTEM=1`, so a developer's global ignore can't change the outcome. Git also reads this clone's own excludes (`.git/info/exclude`, `core.excludesFile`), so an entry only they ignore counts as listed and isn't added for other clones: accepted, since that clone's sessions are the ones `clm init` runs for, and adding the line by hand fixes it. The `.gitignore` edits are those of `init-ignores-local-settings`; the added endings are `\r\n` when the file already holds one, so a Windows file stays consistent. In a CLM repository that is a subdirectory such as `tool/`, the lines go in `tool/.gitignore`, where they cover `tool/.clm/` and `tool/.claude/worktrees/`, the directories sessions started in `tool/` use; a session started at the top of the git repository is that repository's own to ignore.

It writes no workflow: checks run and branches land through `clm land` on whoever's machine runs it, and approvals live in the ledger. It then prints, one per line under `Next steps (yours):`, what's left for the human: commit and push the result to `main` themselves, and, when the host protects `main`, let the identity that runs `clm land` push it (a bypass or an allowed pusher). It can't tell whether `main` is protected without calling the host, so it prints the second step as a conditional. A step that is already done isn't printed.

**Upgrade steps** (`init-next-steps`). A repository set up by an earlier `clm init` keeps its files on a rerun (`init-rerun`), so the steps for bringing them up to date are the human's. `clm init` reads a kept `clm.toml` as plain TOML, not through `config`'s loader, which rejects a `merge` key, so it can say what to delete. In the `[land]` check, any value, an empty string included, counts as set; a malformed one is `config`'s to report. The workflow's first line is read after any byte-order mark, and a workflow with any other first line is the project's: no step names it. The workflow and `clm-build.sh` are looked for under the CLM repository and, when that is a subdirectory such as `tool/`, also at the top of the git repository, where the old `clm init` may have written them for the host to read; each step names the file by its path from the git repository's top (`tool/.github/workflows/clm.yml`) and the exact text to add or delete. A `clm.toml` that isn't valid TOML gets none of the `clm.toml` steps: `clm config` and every command that loads the settings report its error. `clm init` itself, which loads the settings only to pick the host its conditional step names, runs even with a `merge` key.

Re-running lists each file it skipped as `kept <path>` (`init-rerun`).

**`clm survey`.** Its root is `--root` when given, else the CLM repository found from the current directory, else the top of the git repository, so `/clm:init` maps the whole repository from any directory (`root-from-cwd`). Reads the tracked files from `clm-git`'s `tracked_files`, so a file the index holds at several merge stages is read once, skipping `specs/`, `changes/`, `backlog/`, `.github/` and binary files. Prints, as plain text that fits one context for a large repository:
- each directory holding files, with file and line counts per language (by extension);
- manifests (`Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod` and the like) and workspace members;
- directories that hold tests, by convention (`tests/`, `__tests__/`, `*_test.*`, `*.test.*`, `#[cfg(test)]` files);
- Markdown docs and ADRs (`docs/adr/`, `docs/decisions/`, `adr/`);
- import edges between directories, with a count: for Rust, workspace path dependencies and `mod`/`use crate::` paths; for TypeScript, relative imports and workspace packages; for Python, imports that resolve to files in the repository. Extraction uses tree-sitter grammars and one query file per language (`tree-sitter-analysis`). Other languages' files are counted but have no edges.

Directories are listed by path, depth-first, and edges sorted, so the output is the same on every run.

**`clm survey --components <file>`.** The file is YAML: `components:` a list of `{ id, paths, depends_on }`, `paths` being globs as in a component's frontmatter. The files a component's `paths` match are `clm-git`'s `tracked_matching` of them, the same list `clm size` counts a component's code from, so `clm survey --components` and `clm size` agree on which files a component owns (`tracked-files-in-git-layer`). `clm-init` keeps its file names as lossy `String`s, since the survey is text for an agent to read: a name that isn't UTF-8 can't be read back from its lossy form and is left out of the counts. A listing failure is reported with `clm-git`'s error, which names the whole `ls-files` command. It checks coverage over the code files `check/unowned-files` reads, not the surveyed files, and the map may set `outside` globs as the root does (`incremental-adoption`). It reports, one per line in `check`'s format with the file as path: a code file no component's `paths` and no `outside` glob match (`survey/uncovered`), a code file two components match (`survey/overlap`), an `outside` glob that matches no code file outside every component (`survey/stale-outside`), an import edge from a file of component `a` to a file of component `b` where `b` isn't `a` or in `a`'s `depends_on` (`survey/undeclared-edge`), and a `depends_on` cycle (`survey/cycle`). It exits 0 with no output when there are none. Until `20261102-rename-node-to-component-contract`, the flag's and the key's old names are read too (`survey-components`, `component-words`).

**`clm guide`** (`guide-in-the-binary`, `spec-format-in-the-guide`, `guide-table-at-build-time`). `docs/guide/` is the one source of the guide's own pages, and `docs/spec-format.md` joins them as the page `spec-format`, since an adopter's sessions write deltas, requirements and markers in that format. The table of `(name, markdown)` pages is generated, not written: `crates/clm-init/build.rs` lists the `.md` files of `docs/guide/` and writes `$OUT_DIR/guide_pages.rs`, a `const PAGES` holding one `include_str!` per page, by the file's absolute path written with `{:?}`, which escapes any character a string literal can't hold as it is, which `guide.rs` pulls in with `include!`. The binary holds every page's bytes as of the build and reads no file at run time, and a page added to `docs/guide/` on a spec branch is in the next build with no edit under `crates/`: adding a page is one spec change, reviewed as `guide-from-docs` wants (`guide-table-from-files`). `name` is the file name without `.md`, ordered as `guide-table-from-files` says; `spec-format` is the one page the index list doesn't name (`reference` links it). The ordering is one pure function over the directory's file names and `README.md`'s text, with a reader that lists a directory and calls it, in a module the build script pulls in with `#[path]`, as `crates/clm/build.rs` does `version.rs`; the crate holds it as `#[cfg(test)] mod`, since nothing at run time calls it and clippy denies dead code, and its unit tests run the reader on a temporary directory without a build. The module uses `std` alone, since it compiles in the build script too. The script prints `cargo:rerun-if-changed` for `docs/guide/` (a directory, which Cargo scans whole, so a page added or removed reruns it), for `docs/spec-format.md`, for itself and for the module it pulls in, since a script that prints the directive at all is rerun for those paths alone. `clm guide` prints one line per page, `<name>  <title>` (two spaces between), the title being the page's first `# ` heading. `clm guide <page>` prints that page's Markdown unchanged; an unknown name exits 1 and prints the index to stderr. Links between guide pages stay relative (`shaping-the-graph.md`, and `../spec-format.md` from a page in `docs/guide/`), which a session reads as `clm guide shaping-the-graph` or `clm guide spec-format`. A test reads `docs/guide/` and fails when a `.md` page there is missing from the table, as proof that the generated table is complete; the tests that read each page's file from `docs/guide/<name>.md` read `docs/spec-format.md` for `spec-format`, and the one that checks the order allows pages the list doesn't name between those it does and `spec-format`. No `include_str!` under `crates/clm-init/src/` names a `docs/guide/` path, and a test that scans the source files fails on one that does, so the table can't quietly go back to being written by hand; the tests and the link fixtures name the directory as a path to read, and the scanning test builds its needle with `concat!` so it doesn't match itself.

**Links stay inside the guide** (`guide-links-inside`). A guide page's links must work wherever the page is read: in `clm guide`'s output in an adopter's project and on GitHub. The page's directory is `docs/guide/`, or `docs/` for `spec-format`. CLM's own repository files, its decisions and its `ARCHITECTURE.md`, are named in prose as CLM's repository, never linked, since an adopter's project has neither. The rule is a function over a page's file path and Markdown, with the set of page files, so its own tests feed it failing and passing cases without touching `docs/`. A link whose target is wrapped in `<...>` or followed by a title (`[t](x.md "title")`) is judged by its path alone. Reference-style definitions (`[label]: target`) and autolinks (`<https://...>`) aren't inline links and aren't read. The scanner is hand-written, with no Markdown dependency: it skips fenced blocks (```` ``` ```` or `~~~` fences) and code spans of any backtick count, and doesn't exempt indented blocks, which the guide doesn't use; link text may nest brackets and a target may hold balanced parentheses, as in CommonMark. A scheme is `[A-Za-z][A-Za-z0-9+.-]*:`. A relative path is normalised lexically (`./x.md` is `x.md`, `a/../x.md` is `x.md`), and one that climbs above the repository root, or keeps a `?` query, fails. Two pages with one name, such as a later `docs/guide/spec-format.md`, fail the tests, as they should. The scenarios that run the binary are proved in `crates/clm/tests/guide.rs`, where `init/guide-command`'s binary tests are.

**The constraints page** (`guide-constraints-page`). `clm guide constraints` is the one guide page not read from a file: `clm-init` builds it at run time from `model`'s compiled copy of CLM's carried entries (`model/carried-constraints`), so it is the set the running `clm` enforces. It has no file under `docs/guide/`, so `guide-table-from-files` and `guide-links-inside` don't read it.

## Requirements

### Requirement: init-files
`clm init` SHALL write `clm.toml`, `scripts/clm-check.sh` (executable), `ARCHITECTURE.md`, `CAPABILITIES.md`, `specs/SPEC.md` and `CLAUDE.md`, each only when it is missing, and SHALL write no file under `.github/`. `clm.toml` SHALL set `host` (from `--host`, else from `origin`'s URL: `github` for a GitHub URL, `git` for any other, or `none` with no remote), `approval = "any"` and `[land] check = "scripts/clm-check.sh"`. Neither `ARCHITECTURE.md` nor `CAPABILITIES.md` SHALL hold an entry; `ARCHITECTURE.md` SHALL name, outside its `## Constraints` section, `clm guide constraints`, where CLM's carried constraints are. On the result, `clm check` SHALL pass. Outside a git repository it SHALL exit 1 and write nothing. It SHALL call no `gh`.

#### Scenario: empty repository
- GIVEN an empty git repository whose `origin` is `https://github.com/o/r.git`
- WHEN `clm init --lang rust` runs
- THEN all six files exist, `clm-check.sh` is executable and holds `cargo test`, `clm.toml` holds the Rust detector, `host = "github"`, `approval = "any"` and `check = "scripts/clm-check.sh"`, `.github/` doesn't exist, and `clm check` exits 0

#### Scenario: human layers
- GIVEN an empty git repository, and another holding a `CAPABILITIES.md` of its own
- WHEN `clm init` runs in each
- THEN in the first `clm hashes architecture` and `clm hashes capability` list nothing, `clm pending` lists nothing, and `ARCHITECTURE.md` names `clm guide constraints`, and in the second `CAPABILITIES.md` is unchanged and the output says `kept CAPABILITIES.md`

#### Scenario: no remote
- GIVEN an empty git repository with no remote
- WHEN `clm init` runs
- THEN `clm.toml` holds `host = "none"`

#### Scenario: host given
- GIVEN the GitHub repository above
- WHEN `clm init --host git` runs
- THEN `clm.toml` holds `host = "git"`

#### Scenario: project files kept
- GIVEN a repository with its own `CLAUDE.md` and `clm.toml`
- WHEN `clm init` runs
- THEN both are unchanged, and the output lists `kept CLAUDE.md` and `kept clm.toml`

#### Scenario: not a repository
- GIVEN a directory that isn't in a git repository
- WHEN `clm init` runs
- THEN it exits 1 and the directory is unchanged

#### Needs
- model/human-layers

### Requirement: init-rerun
Running `clm init` again SHALL change no existing file, except that it SHALL add to `.gitignore` the lines `init-ignores-local-settings` names that it lacks. For the same version and inputs it SHALL write the same bytes.

#### Scenario: upgrade
- GIVEN a repository initialised by an earlier `clm`, whose `clm-check.sh` the project has edited and whose `.gitignore` lists `clm.local.toml`, `.clm/` and `.claude/worktrees/`
- WHEN `clm init` of a later version runs
- THEN `git status` shows no change

#### Scenario: upgrade adds CAPABILITIES.md
- GIVEN a repository initialised by a `clm` that wrote no `CAPABILITIES.md`
- WHEN `clm init` of a later version runs
- THEN `git status` shows only `CAPABILITIES.md` added

#### Scenario: upgrade adds the ignore lines
- GIVEN a repository initialised by an earlier `clm`, whose `.gitignore` holds only `/target/`
- WHEN `clm init` of a later version runs
- THEN `git status` shows only `.gitignore` changed, and its lines are `/target/`, `clm.local.toml`, `.clm/` then `.claude/worktrees/`

#### Scenario: upgrade from the one-line ignore file
- GIVEN a repository initialised by an earlier `clm`, whose `.gitignore` is `clm.local.toml`
- WHEN `clm init` of a later version runs
- THEN `git status` shows only `.gitignore` changed, and its lines are `clm.local.toml`, `.clm/` then `.claude/worktrees/`

#### Scenario: idempotent
- GIVEN a repository just initialised
- WHEN `clm init` runs again with the same arguments
- THEN `git status` shows no change

### Requirement: init-detectors
`clm.toml` written by `clm init` SHALL hold one detector for each `--lang` given, each with a `hint`, and none with `--no-detectors`.

#### Scenario: two languages
- GIVEN an empty repository
- WHEN `clm init --lang rust --lang python` runs
- THEN `clm config get detectors` lists one Rust and one Python detector

#### Scenario: existing codebase
- GIVEN a repository with code
- WHEN `clm init --lang rust --no-detectors` runs
- THEN `clm config get detectors` prints nothing

### Requirement: init-next-steps
`clm init` SHALL end by printing, under `Next steps (yours):`, the human's remaining steps: pushing the result to `main` themselves, and, if the host protects `main`, letting the identity that runs `clm land` push it. Before the push step it SHALL print one step for each upgrade a repository set up before `20261025-forge-adapters` still needs, reading a kept `clm.toml` as TOML without `config`'s validation, in this order: adding `host = "<host>"` when `clm.toml` sets no `host`, `<host>` being the host it would have written; deleting `merge` when `clm.toml` sets it; adding `[land]` with `check = "scripts/clm-check.sh"` when `clm.toml` sets no `land.check`, naming `.github/scripts/clm-build.sh` as the edits to move when that file exists; and deleting `.github/workflows/clm.yml` when that file's first line starts with `# Generated by clm init`, looking for it, and for `clm-build.sh`, under the CLM repository and at the top of the git repository. A `clm.toml` that isn't valid TOML SHALL get none of the `clm.toml` steps, and the workflow step SHALL still be printed. When it prints any of these it SHALL also print the push step. It SHALL change no existing file for them, and SHALL NOT name `CLM_RELEASES_TOKEN`, Actions, auto-merge or a status check.

#### Scenario: next steps
- GIVEN a fresh repository
- WHEN `clm init` runs
- THEN the output's next steps name pushing to `main` and `clm land`, and don't name `CLM_RELEASES_TOKEN` or `auto-merge`

#### Scenario: repository set up before forge adapters
- GIVEN a repository whose `origin` is `https://github.com/o/r.git`, with a committed `clm.toml` holding only `approval = "any"` and `merge = "auto"`, a `.github/workflows/clm.yml` whose first line is `# Generated by clm init (clm 0.5.0). Do not edit: run clm init again to regenerate.`, and a `.github/scripts/clm-build.sh`
- WHEN `clm init` runs
- THEN it exits 0; `clm.toml`, `clm.yml` and `clm-build.sh` are unchanged; and its next steps name, in this order and before the push step, `host = "github"`, deleting `merge`, `check = "scripts/clm-check.sh"` together with `clm-build.sh`, and deleting `.github/workflows/clm.yml`

#### Scenario: no remote
- GIVEN the repository set up before forge adapters, with no remote
- WHEN `clm init` runs
- THEN the `host` step names `host = "none"`

#### Scenario: settings that aren't TOML
- GIVEN the repository set up before forge adapters, with `clm.toml` holding `host = `
- WHEN `clm init` runs
- THEN it exits 0, no next step names `host =`, `merge` or `[land]`, and one names deleting `.github/workflows/clm.yml`

#### Scenario: old workflow at the top of a subdirectory CLM repository
- GIVEN a git repository whose CLM repository is its subdirectory `tool/`, initialised by today's `clm init` and committed, with a committed `.github/workflows/clm.yml` at the top whose first line is `# Generated by clm init (clm 0.5.0). Do not edit: run clm init again to regenerate.`, and the current directory `tool/`
- WHEN `clm init` runs
- THEN `git status` shows no change, and a next step names deleting `.github/workflows/clm.yml`

#### Scenario: current settings
- GIVEN a repository just initialised by `clm init`
- WHEN `clm init` runs again
- THEN no next step names `clm.toml`, `merge` or `clm.yml`

#### Scenario: the project's own workflow
- GIVEN a repository just initialised by `clm init`, with a `.github/workflows/clm.yml` whose first line is `name: CLM`
- WHEN `clm init` runs again
- THEN no next step names `clm.yml`

### Requirement: survey-map
`clm survey` SHALL print, for the tracked files outside `specs/`, `changes/`, `backlog/` and `.github/`, each directory with file and line counts per language, the manifests, test directories, docs and ADRs, and the import edges between directories for Rust, TypeScript and Python, and SHALL print the same bytes on every run over the same tree.

#### Scenario: two crates
- GIVEN a Cargo workspace with crates `a` and `b`, where `a` has a path dependency on `b`
- WHEN `clm survey` runs
- THEN the output lists both crates' directories with their Rust file and line counts, both `Cargo.toml` manifests, and an edge from `crates/a` to `crates/b`

#### Scenario: TypeScript import
- GIVEN `web/src/app.ts` importing `./lib/util`
- WHEN `clm survey` runs
- THEN the output has an edge from `web/src` to `web/src/lib`

#### Scenario: stable
- GIVEN any repository
- WHEN `clm survey` runs twice
- THEN both outputs are identical

### Requirement: init-root
`clm init` SHALL write in the first of the current directory and its parents that holds `specs/SPEC.md`, searching no further up than the first directory that holds `.git`, and at the top of the git repository when there is none.

#### Scenario: rerun from a subdirectory
- GIVEN a repository initialised by `clm init` and the current directory `src/` inside it
- WHEN `clm init` runs again
- THEN `git status` shows no change, and nothing is written under `src/`

#### Scenario: CLM repository in a subdirectory
- GIVEN a git repository whose CLM repository is its subdirectory `tool/`, initialised by an earlier `clm`, with `tool/.gitignore` listing `clm.local.toml`, `.clm/` and `.claude/worktrees/`, and the current directory `tool/changes/`
- WHEN `clm init` of a later version runs
- THEN `git status` shows no change, and nothing is written outside `tool/`

#### Scenario: no CLM repository
- GIVEN a git repository with no `specs/SPEC.md` and the current directory `src/` inside it
- WHEN `clm init` runs
- THEN it writes its files at the repository's top level, and nothing under `src/`

### Requirement: git-through-layer
`clm-init` SHALL list `clm-git` among its dependencies and SHALL run no `git` or `gh` process itself: every call goes through `clm-git`'s `git`, `gh`, `run` or repo queries (`git-plumbing-layer`), and its own wrapper is gone (`no-dead-code`).

#### Scenario: covered by the scan
- GIVEN the workspace
- WHEN a test in `clm-init` reads `crates/clm-init/Cargo.toml` and every file under `crates/clm-init/src`
- THEN `clm-git` is a dependency and no file holds `Command::new("git")` or `Command::new("gh")`

#### Serves
- architecture/reuse-first

### Requirement: init-ignores-local-settings
`clm init` SHALL make the `.gitignore` in the directory where it writes `clm.toml` list `clm.local.toml`, `.clm/` and `.claude/worktrees/`, where an entry is listed when `git check-ignore -q --no-index <path>`, run through `clm-git` in that directory, exits 0 for the path `clm.local.toml`, `.clm/pr-body.md` or `.claude/worktrees/x/f` respectively. When the file is missing it SHALL write it with the three lines, in that order, and list `wrote .gitignore`. When some entries are not listed it SHALL append each of them at the end of the file, on a line of its own and in that order, first adding a line ending when the file is non-empty and doesn't end with one, listing `added <entry> to .gitignore` for each, in that order; the added line endings SHALL be `\r\n` when the file holds a `\r\n`, else `\n`. Otherwise it SHALL leave the file unchanged and list `kept .gitignore`. When `git check-ignore` exits with anything but 0 or 1, or can't be run, `clm init` SHALL fail with git's message and leave `.gitignore` unchanged. It SHALL leave every other byte of an existing `.gitignore` as it was, whether or not the file is UTF-8 (`init-ignores-session-files`).

#### Scenario: empty ignore file
- GIVEN a repository whose `.gitignore` is empty
- WHEN `clm init` runs
- THEN `.gitignore` is `clm.local.toml\n.clm/\n.claude/worktrees/\n`, with no blank first line, and the output lists `added clm.local.toml to .gitignore`, `added .clm/ to .gitignore` then `added .claude/worktrees/ to .gitignore`

#### Scenario: Windows line endings
- GIVEN a repository whose `.gitignore` is `target/\r\n/clm.local.toml\r\n`
- WHEN `clm init` runs
- THEN `.gitignore` is `target/\r\n/clm.local.toml\r\n.clm/\r\n.claude/worktrees/\r\n`

#### Scenario: new repository
- GIVEN an empty git repository
- WHEN `clm init` runs
- THEN `.gitignore`'s lines are `clm.local.toml`, `.clm/` then `.claude/worktrees/`, and `git check-ignore clm.local.toml .clm/pr-body.md .claude/worktrees/x/f` exits 0 printing all three paths

#### Scenario: existing ignore file without a final newline
- GIVEN a repository whose `.gitignore` is `node_modules/` with no final newline
- WHEN `clm init` runs
- THEN `.gitignore`'s lines are `node_modules/`, `clm.local.toml`, `.clm/` then `.claude/worktrees/`

#### Scenario: some already listed
- GIVEN a repository whose `.gitignore` lines are `/clm.local.toml` and `/.clm`
- WHEN `clm init` runs
- THEN `.gitignore`'s lines are `/clm.local.toml`, `/.clm` then `.claude/worktrees/`

#### Scenario: all already listed
- GIVEN a repository whose `.gitignore` lines are `/clm.local.toml`, `.clm/` and `/.claude/worktrees`
- WHEN `clm init` runs
- THEN `.gitignore` is unchanged and the output lists `kept .gitignore`

#### Scenario: a broader pattern is a listing
- GIVEN a repository whose `.gitignore` lines are `clm.local.toml`, `.clm/` and `.claude/`
- WHEN `clm init` runs
- THEN `.gitignore` is unchanged and the output lists `kept .gitignore`

#### Scenario: a line git doesn't match
- GIVEN a repository whose `.gitignore` lines are `  clm.local.toml` (two leading spaces), `.clm/` and `.claude/worktrees/`
- WHEN `clm init` runs
- THEN `.gitignore`'s lines are `  clm.local.toml`, `.clm/`, `.claude/worktrees/` then `clm.local.toml`, and the output lists `added clm.local.toml to .gitignore`

#### Scenario: a later negation
- GIVEN a repository whose `.gitignore` lines are `clm.local.toml`, `.clm/`, `.claude/worktrees/` then `!clm.local.toml`
- WHEN `clm init` runs
- THEN `.gitignore`'s last line is `clm.local.toml`, the output lists `added clm.local.toml to .gitignore`, and `git check-ignore -q clm.local.toml` exits 0

#### Scenario: tracked local settings
- GIVEN a repository whose `.gitignore` lines are `clm.local.toml`, `.clm/` and `.claude/worktrees/`, and that commits `clm.local.toml` all the same
- WHEN `clm init` runs twice
- THEN `.gitignore` is unchanged after each run and each run's output lists `kept .gitignore`

#### Scenario: CLM repository in a subdirectory
- GIVEN a git repository whose CLM repository is its subdirectory `tool/`, with no `tool/.gitignore`, and the current directory `tool/`
- WHEN `clm init` runs
- THEN `tool/.gitignore`'s lines are `clm.local.toml`, `.clm/` then `.claude/worktrees/`, and the top level has no new `.gitignore`

### Requirement: import-edges
`import_edges(root)` SHALL return, as `Ok`, each import edge `clm survey` finds between two tracked files of the repository at `root`, as `(importing file, imported file)` with repository-relative paths, each pair once, sorted, and the same edges whose directories `clm survey` prints and `clm survey --components` checks. A file whose text can't be parsed SHALL contribute no edges rather than fail the call, and a `root` outside a git work tree SHALL give `Ok` with no edges; a `git` failure inside one SHALL be the `Err`.

#### Scenario: not a repository
- GIVEN a directory that is not in a git work tree, holding the two-crate workspace's files
- WHEN `import_edges` is called on it
- THEN it returns `Ok` with no edges

#### Scenario: path dependency
- GIVEN the two-crate workspace, where `crates/a/Cargo.toml` has a path dependency on `b`
- WHEN `import_edges` is called on it
- THEN it returns the pair (`crates/a/Cargo.toml`, `crates/b/Cargo.toml`), the edge `clm survey --components` names in its `survey/undeclared-edge` line

#### Scenario: TypeScript file edge
- GIVEN `web/src/app.ts` importing `./lib/util`, and `web/src/lib/util.ts`
- WHEN `import_edges` is called
- THEN it returns the pair (`web/src/app.ts`, `web/src/lib/util.ts`)

### Requirement: init-root-ownership
The `specs/SPEC.md` that `clm init` writes SHALL set `paths: [".gitignore", "clm.toml", "scripts/clm-check.sh"]`. It SHALL set `outside` to the top-level entries of the repository's other code files: of the files the bootstrap commit will hold, which are the files the index holds under the CLM repository and the untracked files under it that the `.gitignore` doesn't ignore as `clm init` leaves it (`clm-git`'s `files_to_commit`, read after `clm init` has written its `.gitignore` lines, so `clm init` writes those lines before it writes `specs/SPEC.md`), other than those under `specs/`, `changes/`, `backlog/` or `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and the three files in `paths`, the first component of each path, written `<component>/**` for a file in a directory and as the file's name for a file at the top, each once, sorted by bytes, each written as a YAML double-quoted string with `\` and `"` escaped; and `outside: []` when there is none, so path ownership is always on in a repository `clm init` sets up. So the committed result of `clm init` in a repository with code passes `clm check`'s path ownership (`check/unowned-files`), even when it commits a file that was untracked when `clm init` ran, such as the `.claude/settings.json` Claude Code writes when the plugins are enabled for the project, and the first spec graph narrows `outside` as its components take the code (`incremental-adoption`). `clm survey` and `clm survey --components` still read the index alone. Its tests assert the frontmatter written; the check itself is `check`'s.

#### Scenario: repository with code
- GIVEN a git repository with no `specs/` tracking `src/main.rs`, `src/lib/x.rs`, `scripts/build.sh`, `README.md` and `docs/a.md`
- WHEN `clm init` runs
- THEN `specs/SPEC.md`'s frontmatter has `paths: [".gitignore", "clm.toml", "scripts/clm-check.sh"]` and `outside: ["README.md", "scripts/**", "src/**"]`

#### Scenario: empty repository
- GIVEN an empty git repository
- WHEN `clm init` runs
- THEN `specs/SPEC.md`'s frontmatter has `paths: [".gitignore", "clm.toml", "scripts/clm-check.sh"]` and `outside: []`

#### Scenario: untracked files the bootstrap commits
- GIVEN a git repository with no `specs/`, no global or system git excludes, tracking `src/main.rs` and a `.gitignore` holding `build/`, with the untracked files `.claude/settings.json`, `build/out.o` and `.clm/pr-body.md`
- WHEN `clm init` runs
- THEN `specs/SPEC.md`'s frontmatter has `outside: [".claude/**", "src/**"]`, naming neither `build/**`, which the `.gitignore` ignored before, nor `.clm/**`, which the line `clm init` adds ignores

#### Serves
- capability/adopt-clm
- architecture/incremental-adoption

#### Needs
- model/outside-globs
- vcs/files-to-commit

### Requirement: guide-command
`clm guide` SHALL print one line per page of the guide, as compiled into the binary, each `<name>  <title>`. The pages SHALL be every `.md` file in `docs/guide/`, `docs/spec-format.md` and the generated page `constraints` (`guide-constraints-page`), whose title is `CLM's constraints`; `<name>` is the file name without `.md` (`index` for `README.md`) and `<title>` the page's first `# ` heading; `index` SHALL come first, then the pages `README.md`'s numbered list names, in its order, then `spec-format`, then `constraints`. For a page read from a file, every page but `constraints`, `clm guide <name>` SHALL print its Markdown exactly as its file is at build time, and exit 0; `clm guide constraints` prints the page `guide-constraints-page` states. For a name that isn't a page it SHALL exit 1, printing `no guide page <name>` and the index to stderr. It SHALL read no file at run time.

#### Scenario: index
- GIVEN the binary built from this repository
- WHEN `clm guide` runs in a directory that isn't in a repository
- THEN it exits 0, its first line names `index`, one line is `shaping-the-graph  Shaping the graph`, its second-to-last line is `spec-format  Spec format (v0, provisional)` and its last line is `constraints  CLM's constraints`

#### Scenario: one page
- GIVEN the binary built from this repository
- WHEN `clm guide shaping-the-graph` runs
- THEN it prints `docs/guide/shaping-the-graph.md` byte for byte and exits 0

#### Scenario: the spec format
- GIVEN the binary built from this repository
- WHEN `clm guide spec-format` runs
- THEN it prints `docs/spec-format.md` byte for byte and exits 0

#### Scenario: unknown page
- GIVEN the binary
- WHEN `clm guide no-such-page` runs
- THEN it exits 1 and stderr holds `no guide page no-such-page` and the index

#### Scenario: page left out
- GIVEN a `.md` file in `docs/guide/` that the compiled table doesn't hold
- WHEN the tests run
- THEN a test fails naming the file

#### Serves
- architecture/method-in-the-plugin
- capability/adopt-clm

### Requirement: claude-md-points-at-guide
The `CLAUDE.md` that `clm init` writes SHALL point at the guide by naming `clm guide`, and SHALL hold no URL (`http://` or `https://`).

#### Scenario: pointer
- GIVEN an empty repository
- WHEN `clm init` runs
- THEN its `CLAUDE.md` holds `clm guide` and no `https://`

#### Serves
- architecture/method-in-the-plugin

### Requirement: survey-tracked-through-git
`clm survey` SHALL list the tracked files it maps through `clm-git`'s `tracked_files`, and `clm survey --components` SHALL list the files a component's `paths` match through `clm-git`'s `tracked_matching` (`tracked-files-in-git-layer`). No file under `crates/clm-init/src` SHALL run `git ls-files` itself (`no-dead-code`).

#### Scenario: no copy of its own
- GIVEN the workspace
- WHEN a test under `crates/clm-init/tests` reads every file under `crates/clm-init/src`, comments included, looking for the text `ls-files` built from parts so the test doesn't match itself
- THEN no file holds it

#### Serves
- architecture/reuse-first

#### Needs
- vcs/tracked-files

### Requirement: shared-plumbing
`clm-init` SHALL take the remote's name from `clm-git`'s `REMOTE` wherever it names the remote to git (`remote get-url`, the `diff` against the remote's `main`), building each such argument from `REMOTE` rather than spelling it: no line of a file under `crates/clm-init/src`, apart from those whose first non-blank characters are `//`, SHALL hold `"origin"`, `"origin/` or `remotes/origin` (`remote-plumbing-scan`). `clm init` SHALL keep its output, the files it writes and its exit codes.

#### Scenario: no copy of the remote's name
- GIVEN the workspace
- WHEN a test in `clm-init` reads every file under `crates/clm-init/src`, skipping each line whose first non-blank characters are `//`
- THEN no line holds `"origin"`, `"origin/` or `remotes/origin`

#### Serves
- architecture/reuse-first

### Requirement: guide-links-inside
Every inline link or image target in a guide page (`[text](target)`, `![alt](target)`), outside code spans and fenced code blocks, SHALL be an absolute URL (a scheme followed by `:`), an anchor (starting `#`), or a relative path that, with any `#` anchor removed and resolved against the directory of the page's file, names the file of a guide page. A test SHALL check every page `clm guide` holds and fail naming each page and target that breaks this.

#### Scenario: this repository's guide
- GIVEN the guide's pages as compiled into the binary, `reference` linking `../spec-format.md` and `using-clm.md`
- WHEN the tests run
- THEN the links test passes

#### Scenario: a link out of the guide
- GIVEN a page `docs/guide/reference.md` holding `[Decisions](../decisions/)`, `[ARCHITECTURE.md](../../ARCHITECTURE.md)` and `[gone](missing.md)`, with the pages `docs/guide/reference.md` and `docs/spec-format.md`
- WHEN the rule runs on it
- THEN it reports `reference: ../decisions/`, `reference: ../../ARCHITECTURE.md` and `reference: missing.md`

#### Scenario: links that stay
- GIVEN a page `docs/guide/using-clm.md` holding `[Spec format](../spec-format.md)`, `[Reference](reference.md#skills)`, `[top](#top)`, `[Claude Code](https://claude.com/claude-code)` and, in a code span, `` `[x](../decisions/)` ``, with the pages `docs/guide/using-clm.md`, `docs/guide/reference.md` and `docs/spec-format.md`
- WHEN the rule runs on it
- THEN it reports nothing

#### Serves
- architecture/method-in-the-plugin

### Requirement: guide-table-from-files
The table of pages `clm guide` prints SHALL be generated when `clm-init` builds, from the `.md` files in `docs/guide/` and from `docs/spec-format.md`, so that no `include_str!` under `crates/clm-init/src/` names a `docs/guide/` path: a page added to or removed from `docs/guide/` SHALL be in or out of the next build with no change under `crates/`. The table's order SHALL be `index`, then the pages `docs/guide/README.md` links, each in the order of its first `](<name>.md)` link, then any page it doesn't link, by name, then `spec-format`. A `docs/guide/spec-format.md`, a missing `README.md` or a page that isn't UTF-8 SHALL fail the build, naming the file.

#### Scenario: order from the index page
- GIVEN a directory holding `README.md`, which links `b.md` and then `a.md`, and the pages `a.md`, `b.md` and `z.md`
- WHEN the table's order is computed from it
- THEN it is `index`, `b`, `a`, `z`, `spec-format`

#### Scenario: no page named in source
- GIVEN the source files under `crates/clm-init/src/`
- WHEN a test reads them
- THEN no `include_str!` in them names `docs/guide/`, and the table the build generated holds every `.md` file of `docs/guide/`

#### Scenario: a page named spec-format
- GIVEN a directory holding `README.md` and `spec-format.md`
- WHEN the table's order is computed from it
- THEN it fails, naming `spec-format.md`

#### Scenario: this repository
- GIVEN this repository's `docs/guide/` and `docs/spec-format.md`
- WHEN `clm guide` runs from the binary built from it
- THEN its index lists `index`, then the pages `README.md`'s numbered list names in that order, then `spec-format`, each with its title

#### Serves
- architecture/method-in-the-plugin

### Requirement: architecture-template-proof
The `ARCHITECTURE.md` that `clm init` writes SHALL say, in its preamble, that each of the project's entries names the rules that enforce it under `#### Enforced by`, or says why no program can check it under `#### Untested`, or both, and that a test never cites an entry (`human-layer-proof`); it SHALL NOT contain `req:architecture/`. It SHALL hold no `## Enforced by clm` section and no statement of a carried constraint: it SHALL name `clm guide constraints` and `#### Replaces`, saying that CLM's carried constraints reach the project from the `clm` it runs and that a project replaces a `replaceable` one or drops a `default` one with a `#### Replaces` block, which a human reviews (`carried-constraints`).

#### Scenario: fresh repository
- GIVEN an empty git repository
- WHEN `clm init` runs
- THEN its `ARCHITECTURE.md` names `#### Enforced by` and `#### Untested`, holds no `req:architecture/` and no `## Enforced by clm`, names `clm guide constraints` and `#### Replaces`, and `clm check` exits 0

#### Serves
- capability/adopt-clm
- architecture/method-in-the-plugin
- architecture/carried-constraints

### Requirement: template-says-pull-request
No file under `crates/clm-init/templates/` SHALL call a host's pull request a page: none SHALL hold `page's` (only the guide, which `clm guide` prints, has pages).

#### Scenario: the written architecture
- GIVEN an empty repository
- WHEN `clm init` runs
- THEN its `ARCHITECTURE.md` holds no `page's URL`, and no line of a file under `crates/clm-init/templates/` holds `page's`

#### Serves
- architecture/plain-terms

### Requirement: survey-components
`clm survey --components <file>` SHALL read the map's list of components from its top-level key `components` and an optional top-level `outside`, a list of globs like the root's `outside` (`model/outside-globs`). Its *code files* SHALL be those `check/unowned-files` reads: the files the index holds under the repository, binary files and `.github/` included, other than those under `specs/`, `changes/`, `backlog/` or `docs/` and other than `ARCHITECTURE.md`, `CAPABILITIES.md` and `CLAUDE.md`. Over them it SHALL report each code file no component covers and no `outside` glob matches (`survey/uncovered`), each code file two components cover (`survey/overlap`), and each `outside` glob that matches no code file outside every component (`survey/stale-outside`, with the message `outside glob <glob> matches no file outside every component`); a file a component covers and an `outside` glob matches is the component's. It SHALL report each import edge from one component's file to another component's file that its `depends_on` doesn't allow (`survey/undeclared-edge`), and each `depends_on` cycle (`survey/cycle`); an edge with an end in a file no component covers SHALL NOT be reported. A glob matches a file as the git pathspec `:(glob)<glob>` does. A map whose `outside` isn't a list of non-empty strings SHALL be refused as a map that doesn't parse is. The code files are listed apart from the files `clm survey`'s map reads for import edges, which still skip `.github/` and binary files (`survey-map`). It SHALL exit 1 when it reports any and 0 with no output otherwise. A map that sets `nodes`, the key `20261102-rename-node-to-component` kept beside `components`, SHALL fail with exit 1, naming the file, `nodes` and `components`, and the old flag SHALL fail as clap fails for an unknown argument (`clm/node-names-retired`). No `clm:remove-by 20261102-rename-node-to-component-contract` marker SHALL be left.

#### Scenario: uncovered file
- GIVEN a component map whose components cover `crates/a/**` only, in the two-crate workspace
- WHEN `clm survey --components` runs
- THEN it exits 1 with a `survey/uncovered` line naming a file under `crates/b`

#### Scenario: undeclared edge
- GIVEN components `a` (`crates/a/**`, `depends_on: []`) and `b` (`crates/b/**`) in the two-crate workspace
- WHEN `clm survey --components` runs
- THEN it exits 1 with a `survey/undeclared-edge` line naming `a` and `b`

#### Scenario: good map
- GIVEN components `a` (`crates/a/**`, `depends_on: [b]`) and `b` (`crates/b/**`), covering every code file
- WHEN `clm survey --components` runs
- THEN it exits 0 with no output

#### Scenario: part left for later
- GIVEN a component map with component `a` (`crates/a/**`, `depends_on: []`), a component covering the workspace's top-level files, and `outside: ["crates/b/**"]`, in the two-crate workspace, where a file under `crates/a` imports one under `crates/b`
- WHEN `clm survey --components` runs
- THEN it exits 0 with no output

#### Scenario: outside glob that exempts nothing
- GIVEN components `a` (`crates/a/**`, `depends_on: [b]`), `b` (`crates/b/**`) and one covering the workspace's top-level files, together covering every code file, and `outside: ["crates/c/**"]`
- WHEN `clm survey --components` runs
- THEN it exits 1 with a `survey/stale-outside` line naming `crates/c/**`

#### Scenario: the code files check reads
- GIVEN the two-crate workspace with tracked files `docs/notes.md` and `.github/workflows/ci.yml` besides its own, and components `a` (`crates/a/**`, `depends_on: [b]`), `b` (`crates/b/**`) and one covering the workspace's top-level files
- WHEN `clm survey --components` runs
- THEN it exits 1 with a `survey/uncovered` line naming `.github/workflows/ci.yml`, and no line naming `docs/notes.md`

#### Scenario: the old key
- GIVEN a map that lists its components under `nodes`
- WHEN `clm survey --components` runs on it
- THEN it exits 1 naming the file, `nodes` and `components`

#### Needs
- clm/component-flags

#### Serves
- capability/adopt-clm
- architecture/incremental-adoption
- architecture/plain-terms

### Requirement: template-says-architectural-constraint
The files `clm init` writes SHALL call an entry of `ARCHITECTURE.md` an architectural constraint: the `ARCHITECTURE.md` it writes SHALL start its preamble with `The architectural constraints:` and hold its project's entries in an empty `## Constraints` section, with no `## Requirements` section, and the `CLAUDE.md` it writes SHALL say that `ARCHITECTURE.md`'s architectural constraints apply to all work. No file under `crates/clm-init/templates/` SHALL hold the old term (the word `architecture` followed by the word `requirement`, singular or plural, in any case).

#### Scenario: the written layers
- GIVEN an empty git repository
- WHEN `clm init` runs
- THEN its `ARCHITECTURE.md` holds `The architectural constraints:` and `## Constraints` and no `## Requirements`, its `CLAUDE.md` holds `architectural constraints`, neither holds the old term, and `clm check` exits 0

#### Serves
- architecture/plain-terms

#### Needs
- model/human-layers

### Requirement: templates-say-new-words
The root `SPEC.md` `clm init` writes SHALL have `status: unbuilt`; no file under `crates/clm-init/templates/` SHALL hold a word `plain-terms-renames` replaced for a component status, a backlog kind or a `change.md` key.

#### Scenario: a new repository
- GIVEN an empty repository
- WHEN `clm init` runs
- THEN its `specs/SPEC.md` has `status: unbuilt`, no file under `crates/clm-init/templates/` holds a word `plain-terms-renames` replaced, and the model loads the tree with no violation

#### Serves
- architecture/plain-terms

#### Needs
- model/valid-frontmatter
- model/parse-changes

### Requirement: says-component
The text `clm-init` prints and writes SHALL say component, components, ancestors and placeholder component, never the old words, as `component-words` defines them: the messages of `clm survey --components` (`component `<id>` appears twice`, `depends on unknown component`, `<path> is in no component's paths`, `<from> (component `<a>`) imports <to> (component `<b>`)`, `matches no file outside every component`), and every file under `crates/clm-init/templates/` (`<component-id>/<slug>` in the `CLAUDE.md` it writes, `How <name> is split into components.` in the root `SPEC.md`, and the preamble of the `ARCHITECTURE.md`). A name CLM hasn't renamed keeps its spelling, and so does a decision a file cites. A message that names the map's old key to say it is old keeps it. A test reads the string literals in `crates/clm-init/src/` and every file under `crates/clm-init/templates/`, and fails naming the file, the line and the word for each old word `component-words` doesn't keep.

#### Scenario: a new repository
- GIVEN an empty repository
- WHEN `clm init` runs
- THEN its `CLAUDE.md` holds `<component-id>/<slug>`, its `specs/SPEC.md`'s Design reads `How <name> is split into components.`, and none of the files it writes holds an old word but in a constraint's or a decision's name

#### Scenario: a duplicate in the map
- GIVEN a component map listing component `a` twice
- WHEN `clm survey --components` runs on it
- THEN it exits 1 with a line naming component `a`, and no line holds an old word (`component-words`)

#### Serves
- architecture/plain-terms

### Requirement: claude-md-names-the-skills
The `CLAUDE.md` that `clm init` writes SHALL name `/clm:propose`, `/clm:build` and `/clm:help`, and SHALL NOT name `/clm:implement`.

#### Scenario: a new repository
- GIVEN a repository with no `CLAUDE.md`
- WHEN `clm init` runs
- THEN its `CLAUDE.md` holds `/clm:propose`, `/clm:build` and `/clm:help`, and not `/clm:implement`

#### Needs
- plugin/skill-named-build
- plugin/help-skill

### Requirement: doc-readers-marked
Each test file under `crates/clm-init/tests/` that reads a file of this repository's own tree (not a fixture's) at a path `ledger`'s `doc_only` names, such as `ARCHITECTURE.md`, SHALL hold a `clm:reads` marker (`check.affected/affected-readers`) whose glob matches every such path it reads, so a land that changes one of them runs it even when its code was already checked (`land/selected-tests`).

#### Scenario: CLM's own ARCHITECTURE.md
- GIVEN the `clm:reads` markers of the files `check`'s coverage scan reads in this repository's tree
- WHEN `clm-affected`'s `readers` runs on them with `ARCHITECTURE.md` as the only differing path
- THEN it selects `crates/clm-init/tests/says_component.rs`

#### Needs
- check.affected/affected-readers

### Requirement: init-without-manifest
For each `--lang`, the `scripts/clm-check.sh` that `clm init` writes SHALL first change to the CLM root (the parent of the script's own directory), wherever it is run from, and then run each language's line only when its manifest exists there: `cargo test` when `Cargo.toml` does, `npm ci && npm test` when `package.json` does, and `pytest` when `pyproject.toml` or `setup.py` does; with none of them it SHALL exit 0. Each detector SHALL have a `requires` naming its language's manifest: `Cargo.toml` for `rust`, `package.json` for `typescript` and `pyproject.toml` for `python`. The `typescript` detector SHALL have the command `["node_modules/.bin/knip"]`, `requires = "package.json"` and the hint `npm ci (or npm install --save-dev knip when package.json doesn't list it)`, so a repository without a `package.json` skips it and one whose dependencies aren't installed reports `could not run` with that hint rather than `failed`. In a repository with no code, the result of `clm init --lang rust --lang typescript --lang python` SHALL pass both `clm check` and `scripts/clm-check.sh`.

#### Scenario: empty repository, every language
- GIVEN an empty git repository, and on `PATH` a `cargo`, `npm` and `pytest` that each exit 1
- WHEN `clm init --lang rust --lang typescript --lang python` runs, then `clm check` and `scripts/clm-check.sh`
- THEN both exit 0, none of the three stubs ran, and `clm check`'s stderr names each detector as skipped

#### Scenario: manifest added later
- GIVEN that repository once it holds a `package.json`, and on `PATH` an `npm` that records its arguments and exits 0
- WHEN `scripts/clm-check.sh` runs
- THEN the stub recorded `ci` and then `test`

#### Scenario: dependencies not installed
- GIVEN a repository holding a `package.json` and no `node_modules/`
- WHEN `clm init --lang typescript` runs and then `clm check`
- THEN `clm check` exits 1 with a line starting `clm.toml: check/detectors: typescript-dead-code could not run:` and ending `; install with: npm ci (or npm install --save-dev knip when package.json doesn't list it)`

#### Serves
- capability/adopt-clm

#### Needs
- clm/detector-requires-skip
- check/detector-relative-command

### Requirement: guide-entry-pages
The guide's entry pages SHALL stay written for a newcomer (`guide-entry-pages-for-newcomers`). `docs/guide/quickstart.md` SHALL hold `/plugin marketplace add clunny/clm` and `/plugin install clm@clm`, SHALL name `/clm:init`, `/clm:build`, `/clm:propose`, `/clm:triage` and `/clm:health`, and SHALL link `using-clm.md` and `how-it-works.md`. `docs/guide/using-clm.md` SHALL name `/clm:propose`, `/clm:build`, `/clm:triage` and `/clm:health`, and SHALL link `how-it-works.md`. Neither page SHALL hold, outside fenced code blocks, a code span whose text, as written, starts with `clm ` (a command of the `clm` CLI); a skill such as `/clm:build`, and `clm` alone, are not one. A test SHALL read both pages from `docs/guide/` and fail naming the page and what it lacks or holds.

#### Scenario: this repository's guide
- GIVEN the pages `docs/guide/quickstart.md` and `docs/guide/using-clm.md` of this repository
- WHEN the tests run
- THEN the entry-pages test passes

#### Scenario: a command on an entry page
- GIVEN a `using-clm` page that otherwise meets the rule and holds `` run `clm land <branch>` ``
- WHEN the rule runs on it
- THEN it reports `using-clm: names clm land <branch>`

#### Scenario: a workflow missing
- GIVEN a `quickstart` page that otherwise meets the rule but never names `/clm:triage`
- WHEN the rule runs on it
- THEN it reports `quickstart: lacks /clm:triage`

#### Scenario: a command in a fenced block
- GIVEN a `quickstart` page that meets the rule and holds a fenced block with `clm status` in it
- WHEN the rule runs on it
- THEN it reports nothing

### Requirement: guide-constraints-page
`clm guide constraints` SHALL print a page titled `# CLM's constraints`, built from `model`'s compiled copy of CLM's carried entries (`model/carried-constraints`), never from the repository it runs in: a short preamble saying these are the architectural constraints every project built with CLM follows, from the `clm` it runs, what each tier means (`method`: CLM relies on it and a project can't replace it; `replaceable`: a project may replace it with its own constraint that keeps the stated guarantee; `default`: a project may drop it), and that a project replaces or drops one with a `#### Replaces` block in an entry of its own `ARCHITECTURE.md`, which a human reviews; Names in backticks that the page doesn't list are CLM's own decisions and constraints, kept in CLM's repository: they say why, and the rule is the text the page gives; then, for each carried entry in CLM's order, `## <slug>`, a line `Tier: <tier>`, its statement and, for a `replaceable` entry, a line `Guarantee: <text>`. It SHALL exit 0 and read no file at run time. Its tests hold a `clm:reads` marker naming `ARCHITECTURE.md` (`doc-readers-marked`).

#### Scenario: the page
- GIVEN the binary built from this repository
- WHEN `clm guide constraints` runs in a directory that isn't in a repository
- THEN it exits 0, its first line is `# CLM's constraints`, it holds `## proven-by-tests` followed by `Tier: method`, `## expand-contract` with `Tier: replaceable` and a `Guarantee:` line, and `## yagni` with `Tier: default`, and it holds no `## web-optional`

#### Needs
- model/carried-constraints

#### Serves
- architecture/carried-constraints
- architecture/method-in-the-plugin
