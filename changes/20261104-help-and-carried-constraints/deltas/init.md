---
component: init
base: { design: "beccaa2673d02188" }
---

## MODIFIED Design
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

## ADDED Requirements

### Requirement: guide-constraints-page
`clm guide constraints` SHALL print a page titled `# CLM's constraints`, built from `model`'s compiled copy of CLM's carried entries (`model/carried-constraints`), never from the repository it runs in: a short preamble saying these are the architectural constraints every project built with CLM follows, from the `clm` it runs, what each tier means (`method`: CLM relies on it and a project can't replace it; `replaceable`: a project may replace it with its own constraint that keeps the stated guarantee; `default`: a project may drop it), and that a project replaces or drops one with a `#### Replaces` block in an entry of its own `ARCHITECTURE.md`, which a human reviews; then, for each carried entry in CLM's order, `## <slug>`, a line `Tier: <tier>`, its statement and, for a `replaceable` entry, a line `Guarantee: <text>`. It SHALL exit 0 and read no file at run time.

#### Scenario: the page
- GIVEN the binary built from this repository
- WHEN `clm guide constraints` runs in a directory that isn't in a repository
- THEN it exits 0, its first line is `# CLM's constraints`, it holds `## proven-by-tests` followed by `Tier: method`, `## expand-contract` with `Tier: replaceable` and a `Guarantee:` line, and `## yagni` with `Tier: default`, and it holds no `## web-optional`

#### Serves
- architecture/carried-constraints
- architecture/method-in-the-plugin

## MODIFIED Requirements

### Requirement: init-files <!-- base: 8acd552f1c2e9e0c -->
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

### Requirement: architecture-template-proof <!-- base: 4b06a668cfdb30fd -->
The `ARCHITECTURE.md` that `clm init` writes SHALL say, in its preamble, that each of the project's entries names the rules that enforce it under `#### Enforced by`, or says why no program can check it under `#### Untested`, or both, and that a test never cites an entry (`human-layer-proof`); it SHALL NOT contain `req:architecture/`. It SHALL hold no `## Enforced by clm` section and no statement of a carried constraint: it SHALL name `clm guide constraints` and `#### Replaces`, saying that CLM's carried constraints reach the project from the `clm` it runs and that a project replaces a `replaceable` one or drops a `default` one with a `#### Replaces` block, which a human reviews (`carried-constraints`).

#### Scenario: fresh repository
- GIVEN an empty git repository
- WHEN `clm init` runs
- THEN its `ARCHITECTURE.md` names `#### Enforced by` and `#### Untested`, holds no `req:architecture/` and no `## Enforced by clm`, names `clm guide constraints` and `#### Replaces`, and `clm check` exits 0

#### Serves
- capability/adopt-clm
- architecture/method-in-the-plugin
- architecture/carried-constraints

### Requirement: says-component <!-- base: 9a4c66280a3fa420 -->
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

### Requirement: guide-command <!-- base: 32520cc645726c1f -->
`clm guide` SHALL print one line per page of the guide, as compiled into the binary, each `<name>  <title>`. The pages SHALL be every `.md` file in `docs/guide/`, `docs/spec-format.md` and the generated page `constraints` (`guide-constraints-page`), whose title is `CLM's constraints`; `<name>` is the file name without `.md` (`index` for `README.md`) and `<title>` the page's first `# ` heading; `index` SHALL come first, then the pages `README.md`'s numbered list names, in its order, then `spec-format`, then `constraints`. `clm guide <name>` SHALL print that page's Markdown exactly as its file is at build time, and exit 0. For a name that isn't a page it SHALL exit 1, printing `no guide page <name>` and the index to stderr. It SHALL read no file at run time.

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

### Requirement: claude-md-names-the-skills <!-- base: 78ac34498489cc22 -->
The `CLAUDE.md` that `clm init` writes SHALL name `/clm:propose`, `/clm:build` and `/clm:help`, and SHALL NOT name `/clm:implement`.

#### Scenario: a new repository
- GIVEN a repository with no `CLAUDE.md`
- WHEN `clm init` runs
- THEN its `CLAUDE.md` holds `/clm:propose`, `/clm:build` and `/clm:help`, and not `/clm:implement`

#### Needs
- plugin/skill-named-build
- plugin/help-skill
