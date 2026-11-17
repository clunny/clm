---
component: init
base: { design: "6f330739f0dfb545" }
---

## MODIFIED Design

Crate `clm-init`. Templates are compiled into the binary (`include_str!`), so a given `clm` version always writes the same bytes for the same inputs, and tests compare output to fixtures.

**`clm init`.** Fails outside a git repository. It writes in the CLM repository found from the current directory as `root-from-cwd` finds it, and at the top of the git repository when there is none, so rerunning it from a subdirectory, or in a CLM repository that is a subdirectory such as `tool/`, writes that repository's files instead of starting a second one. `clm-init` finds the repository itself, by the same rule as `clm-hooks`' `clm_root`, because it doesn't depend on `clm-hooks`. Then:
- *`clm.toml`*, when missing: `host`, `upgraded` (**Upgrades**), `approval = "any"` with a comment on `other`, `[land] check = "scripts/clm-check.sh"`, `[build] setup = "test ! -f package.json || npm ci"` when `--lang typescript` is given and no `[build]` table otherwise, so a build's worktree installs its own packages before its tests run rather than finding the main checkout's (`init-build-setup`, `build-setup-step`: Rust needs no setup, and a Python environment is the project's to choose), and one `[[detectors]]` table per `--lang` from a built-in table (at least `rust`, `typescript` and `python`; each with its `hint` and a `requires` naming the language's manifest, `Cargo.toml`, `package.json` or `pyproject.toml`, so a repository without it skips the detector until it lands, and `typescript`'s tool run from `node_modules/.bin/`, so one without its dependencies installed is told to install them: `detector-requires-manifest`), or none with `--no-detectors`, which `/clm:init` passes for an existing codebase so it passes `clm check` before its dead code is cleaned up. `host` is `--host` when given; otherwise what `clm-host`'s `host_for_remote` gives for `origin`'s URL (`github`, `azure` or `git`: `host/host-for-remote`), and `none` with no remote, so `clm-init` holds no host's names (`host-adapters`). It reads the URL from `git remote get-url origin` and calls no host.
- *`scripts/clm-check.sh`*, when missing: an executable starter that builds and tests for each `--lang` (`cargo test`, `npm ci && npm test`, `pytest`), each line run only once the language's manifest exists, so the bootstrap of a repository with no code yet passes it, for the project to edit. It is what `clm land` runs before `clm check`.
- *`ARCHITECTURE.md`*, *`CAPABILITIES.md`*, *`specs/SPEC.md`*, *`AGENTS.md`* and *`CLAUDE.md`*, when missing: an `ARCHITECTURE.md` of a short preamble, a pointer to `clm guide constraints` for CLM's carried constraints, which reach the project from the `clm` it runs rather than as a copy in the file (`carried-constraints`), with how a project replaces or drops one by a `#### Replaces` block, and an empty `## Constraints` section for the project's own entries (`model/human-layers`), so a new repository starts with no entry its own tests would have to prove; a `CAPABILITIES.md` of a short preamble and an empty `## Capabilities` section, for the project's capabilities; a root component titled after the repository with `status: unbuilt` and no requirements; an `AGENTS.md` with CLM's working rules for sessions and a pointer to the guide as `clm guide`, never as a URL, so an adopter's sessions read the guide of the `clm` they run, offline (`method-in-the-plugin`), and a `CLAUDE.md` of the one line `@AGENTS.md`, so Claude Code reads the same text every other agent harness does (`platform`). The rules that must hold live in `clm` and the plugin's skills and hooks, so an older `AGENTS.md` never weakens them. `clm check` passes on the result.
- *`.gitignore`*, in the same directory as `clm.toml`: three lines CLM needs ignored, in this order. `clm.local.toml` is `config`'s settings file for one clone, never committed (`local-settings`); `.clm/` holds a session's scratch files, such as a pull request body, and `clm run`'s and `clm draft`'s plans and prompts; and `.claude/worktrees/` holds the worktrees `plugin`'s skills cut their session branches in and Claude Code puts subagents' worktrees in (`init-ignores-session-files`). Left unignored, `git status` lists them, `git worktree remove` refuses a worktree holding a pull request body, and `git add -A` commits a scratch file or records a worktree as an embedded repository. An entry is *listed* when git already ignores it there: `git check-ignore -q --no-index <path>`, run through `clm-git` in that directory, exits 0 for the path `clm.local.toml`, `.clm/pr-body.md` or `.claude/worktrees/x/f` respectively (`reuse-first`), one call per entry in that order, since `-q` with several paths can't tell them apart. `--no-index` has git read the patterns whatever the index holds: without it git reports a tracked file as not ignored, so a repository that commits `clm.local.toml` would get the line again on every run (`init-rerun`). `clm-git`'s `git` reports exit 1 as a failure carrying its status, which `clm-init` reads as not listed, as `clm-git`'s own `is_ancestor` does, so `vcs` needs nothing new. So git's own rules decide, not a matcher of CLM's (an ignore file elsewhere, such as a parent directory's `.gitignore`, counts too), and the line `clm init` appends goes last, after any negation, so it takes effect. A git failure fails `clm init` before `.gitignore` is touched; files written before it stay written. That includes git refusing a probe path, as when `.clm` is a symbolic link: it fails closed. Because git reads the machine's own configuration, the tests run `clm init` with `GIT_CONFIG_GLOBAL` and `XDG_CONFIG_HOME` pointing into an empty temporary directory and `GIT_CONFIG_NOSYSTEM=1`, so a developer's global ignore can't change the outcome. Git also reads this clone's own excludes (`.git/info/exclude`, `core.excludesFile`), so an entry only they ignore counts as listed and isn't added for other clones: accepted, since that clone's sessions are the ones `clm init` runs for, and adding the line by hand fixes it. The `.gitignore` edits are those of `init-ignores-local-settings`; the added endings are `\r\n` when the file already holds one, so a Windows file stays consistent. In a CLM repository that is a subdirectory such as `tool/`, the lines go in `tool/.gitignore`, where they cover `tool/.clm/` and `tool/.claude/worktrees/`, the directories sessions started in `tool/` use; a session started at the top of the git repository is that repository's own to ignore.

It writes no workflow: checks run and branches land through `clm land` on whoever's machine runs it, and approvals live in the ledger. It then prints, one per line under `Next steps (yours):`, what's left for the human: commit and push the result to `main` themselves, then the lines `clm-host`'s `setup_steps` gives for the host (`host/host-setup-steps`): what to install and sign in to, and, when the host protects `main`, how to let the identity that runs `clm land` push it, in that host's terms. It can't tell whether `main` is protected without calling the host, so those steps are conditional. A step that is already done isn't printed.

**Upgrades** (`upgrade-migrations`). A repository set up by an earlier `clm init` keeps its files on a rerun (`init-rerun`); bringing it up to date is `upgrade`'s, through its migrations, `clm upgrade` and `/clm:upgrade`. `clm init` prints no upgrade step. The one thing it does for upgrades is stamp a `clm.toml` it writes with `upgraded`, the newest migration the running binary holds (the last of `migrations()`, `all-migrations`), so a new repository starts with nothing pending. A kept `clm.toml` is never stamped: a repository with no `upgraded` is one `upgrade` judges by each migration's `done`.

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

**Links stay inside the guide** (`guide-links-inside`). A guide page's links must work wherever the page is read: in `clm guide`'s output in an adopter's project and on GitHub. The page's directory is `docs/guide/`, or `docs/` for `spec-format`. CLM's own repository files, its decisions and its `ARCHITECTURE.md`, are named in prose as CLM's repository, never linked, since an adopter's project has neither. The rule is a function over a page's file path and Markdown, with the set of page files, so its own tests feed it failing and passing cases without touching `docs/`. A link whose target is wrapped in `<...>` or followed by a title (`[t](x.md "title")`) is judged by its path alone. Reference-style definitions (`[label]: target`) and autolinks (`<https://...>`) aren't inline links and aren't read. The scanner is hand-written, with no Markdown dependency: it skips fenced blocks (```` ``` ```` or `~~~` fences) and code spans of any backtick count, and doesn't exempt indented blocks, which the guide doesn't use; link text may nest brackets and a target may hold balanced parentheses, as in CommonMark. A scheme is `[A-Za-z][A-Za-z0-9+.-]*:`. A relative path is normalised lexically (`./x.md` is `x.md`, `a/../x.md` is `x.md`), and one that climbs above the repository root, or keeps a `?` query, fails. Two pages with one name, such as a later `docs/guide/spec-format.md`, fail the tests, as they should. Its scenarios are proved in `crates/clm-init/tests/guide.rs`, by `clm-init`'s call of `clm guide`, where `init/guide-command`'s tests are (`commands-in-crate`).

**The constraints page** (`guide-constraints-page`). `clm guide constraints` is the one guide page not read from a file: `clm-init` builds it at run time from `model`'s compiled copy of CLM's carried entries (`model/carried-constraints`), so it is the set the running `clm` enforces. It has no file under `docs/guide/`, so `guide-table-from-files` and `guide-links-inside` don't read it.

**Allow rules for unattended sessions** (`init-seeds-permissions`, `init-check-permissions`, `unattended-steps-run-clm`). Builds and drafts run as subagents under the human's permission mode (`builds-in-subagents`). In Claude Code's auto mode a classifier judges each command that no rule allows, and it denies a build's force-push of its rebased branch and a draft's `clm land` of its own change. A project's allow rule resolves before the classifier, and auto mode drops only broad rules such as `Bash(*)`, so `clm-init` holds one public constant list, `UNATTENDED_RULES`, which the root's test of this repository's settings reads too (`clm/session-commands`): for each of the `clm` subcommands `apply`, `branch`, `check`, `claim`, `defect`, `gate`, `land`, `pr`, `rebase`, `setup`, `verify` and `workaround`, in that order, the two rules `Bash(clm <subcommand>)` and `Bash(clm <subcommand> *)`, the bare form first, as this repository writes its own script rules (`clm/session-commands`). They are the `clm` commands the build and draft prompts run that write: to the branch, to `origin` or to the backlog, and `clm setup`, which writes the worktree's dependencies by running the project's `[build] setup` (`tests-run-in-their-own-tree`). The classifier denied a draft's `clm setup` as unauthorized persistence, which left the draft's code-only item unfixed (`setup failed`). The command it runs is the project's own, committed in `clm.toml` as `[land] check` is, which the allowed `clm land` runs, so allowing `clm setup` trusts nothing a land doesn't already. `clm approve` is never among them, and an allow rule never outranks the guard's `ask` for it (`approval-policy`). No `git push` rule is: `clm branch push` and `clm branch delete` are the pushes an unattended session makes (`host`).

*Seeding.* The settings file is `.claude/settings.json` in the directory holding `clm.toml`, the CLM repository's root. `clm init` adds the rules only when `specs/SPEC.md` was missing when it started, so a rerun leaves the file as it is (`init-rerun`), and the root component it writes names the file in its `paths`, so the bootstrap commit passes path ownership whether `clm init` created the file or found it (`init-root-ownership`). A missing file, or one holding only whitespace, is written as `{"permissions":{"allow":[...]}}`, pretty-printed with two-space indents and a final newline. An existing file is parsed as JSON (`clm-init` enables `serde_json`'s `preserve_order` feature, so its keys keep their order; the feature unifies across the workspace, which only changes the order other crates' maps print in, never what they parse), JSON with comments or trailing commas counting as not JSON; the rules it lacks from `permissions.allow` are appended in the list's order, creating `permissions` and `allow` when missing, and it is written back pretty-printed in the same way, only when a rule was added. A file that isn't a JSON object, or whose `permissions` isn't an object or whose `permissions.allow` isn't an array, is left as it is, and the next steps name the rules to add by hand. Rules elsewhere in the file, the project's own, are kept.

*Checking.* `clm init --check-permissions` reads `.claude/settings.json` and `.claude/settings.local.json` there, each when present, and takes the union of their `permissions.allow` arrays, since Claude Code merges both; a file it can't read as that shape counts as allowing nothing. It compares rules as exact strings, so a rule in the older `Bash(clm land:*)` form counts as missing: the check names exactly what to add. It prints each missing rule on its own line, in the list's order, after a line `missing allow rules in .claude/settings.json:`, and exits 1; with none missing it prints nothing and exits 0. It writes nothing and needs no `specs/`. It finds the CLM repository as `clm init` does (`init-root`), and outside a git repository it exits 1 saying so, as `clm init` does. The rules it names, and the first-run seeding, are one list, so a rule added to the list is checked for in every repository at once, including those set up before it existed.


*Next steps.* On a rerun, when the check would report a rule missing, the next steps name `.claude/settings.json` and each missing rule, as the last upgrade step, right before the push step (`init-next-steps`). On a first run they name them only when the file couldn't be edited.

*Ignored settings* (`init-names-ignored-settings`). The rules reach other machines, and the unattended sessions run there, only once `.claude/settings.json` is committed, and a project's `.gitignore` often ignores all of `.claude/`. So on every run, first or rerun, after its `.gitignore` edits, `clm init` asks git whether it would leave the file out of a commit: `git check-ignore -q .claude/settings.json`, through `clm-git` in the CLM repository's root, as for the `.gitignore` probes but without `--no-index`, since a file the index already tracks is committed whatever the patterns say and isn't reported. Exit 0 means ignored, exit 1 not, and any other failure fails `clm init`, as the `.gitignore` probes do. When it is ignored, the next steps say that git ignores `.claude/settings.json`, so its allow rules stay on this machine, and name the fix: in place of a pattern that ignores the whole directory, such as `.claude/`, ignore `.claude/*` and add `!.claude/settings.json` after it (git can't re-include a file whose directory it excludes), or commit the file with `git add -f`. The step comes before the missing-rules step, which stays the last step before the push step (`init-seeds-permissions`), and right before the push step when that one isn't printed. `clm init` edits no pattern of the project's own: which of `.claude/`'s other files the project keeps out is its choice, and the line it would rewrite may sit in another ignore file. The answer is git's, so this clone's own excludes count too: a file only `.git/info/exclude` ignores is reported, which is right, since this clone's commits would leave it out.

**Migrations** (`all-migrations`, `own-migrations-frozen`, `migrations-live-with-init`). What a project set up earlier holds is `init`'s business twice over: `clm init` writes it today, and a migration brings an older project to it. So every migration written since `20261114-migrations-with-templates` lives here, whatever it carries: a changed text of a file `clm init` writes, a settings key, a format `clm` reads. Each is a directory `crates/clm-init/src/migrations/<id>/` of the shape `upgrade`'s Design gives, pulled in from `crates/clm-init/src/migrations.rs`, which lists them in the order they landed and holds this crate's supersedes pairs. A migration of a file `clm init` writes takes its `new` text from the template itself, with `include_str!` of the same file `clm init` writes from, until a later migration moves that text on; then it keeps a copy of the text as it was, in its own directory, since a shipped migration never changes. So the build of a requirement that changes such a file writes the template, the migration and the migration's tests in this component's files, in one branch, and the upgrade corpus test (`clm/upgrade-corpus`) passes on that land. `migrations()` copies `upgrade`'s thirteen and appends this crate's, so the order a project's `upgraded` rests on never changes: nothing is ever added before the last of `upgrade`'s. `crates/clm-init/migrations.lock` is this crate's lock, checked and appended to by `upgrade`'s `check_lock` and `append_lock`; it doesn't exist until a release ships a migration of this crate. A test of one migration gives `upgrade`'s engine a list holding that migration alone, or the list up to it, never the whole list, so adding a migration fails no earlier test.



## MODIFIED Requirements



### Requirement: init-files <!-- base: 2e91537be80f384d -->
`clm init` SHALL write `clm.toml`, `scripts/clm-check.sh` (executable), `ARCHITECTURE.md`, `CAPABILITIES.md`, `specs/SPEC.md`, `AGENTS.md` and `CLAUDE.md`, each only when it is missing, and SHALL write no file under `.github/`. `AGENTS.md` SHALL hold the project's instructions for every agent harness (`platform`), and the `CLAUDE.md` it writes SHALL be the one line `@AGENTS.md`, which Claude Code reads as an import, so every agent harness reads one text. When the repository already has a `CLAUDE.md` that doesn't hold the line `@AGENTS.md`, `clm init` SHALL keep it, still write a missing `AGENTS.md`, and print a line saying to add `@AGENTS.md` to `CLAUDE.md` so Claude Code reads the same instructions. `clm.toml` SHALL set `host` (from `--host`, else what `clm-host`'s `host_for_remote` gives for `origin`'s URL, or `none` with no remote), `approval = "any"` and `[land] check = "scripts/clm-check.sh"`. Neither `ARCHITECTURE.md` nor `CAPABILITIES.md` SHALL hold an entry; `ARCHITECTURE.md` SHALL name, outside its `## Constraints` section, `clm guide constraints`, where CLM's carried constraints are. That the other commands accept the result is something only a `clm` process shows, so the root states and proves it (`clm/init-result-accepted`). Outside a git repository it SHALL exit 1 and write nothing. It SHALL call no `gh` and no `az`.

#### Scenario: empty repository
- GIVEN an empty git repository whose `origin` is `https://github.com/o/r.git`
- WHEN `clm init --lang rust` runs
- THEN all seven files exist, `CLAUDE.md` is `@AGENTS.md`, `clm-check.sh` is executable and holds `cargo test`, `clm.toml` holds the Rust detector, `host = "github"`, `approval = "any"` and `check = "scripts/clm-check.sh"`, and `.github/` doesn't exist

#### Scenario: human layers
- GIVEN an empty git repository, and another holding a `CAPABILITIES.md` of its own
- WHEN `clm init` runs in each
- THEN in the first neither `ARCHITECTURE.md` nor `CAPABILITIES.md` holds an entry and `ARCHITECTURE.md` names `clm guide constraints`, and in the second `CAPABILITIES.md` is unchanged and the output says `kept CAPABILITIES.md`

#### Scenario: no remote
- GIVEN an empty git repository with no remote
- WHEN `clm init` runs
- THEN `clm.toml` holds `host = "none"`

#### Scenario: Azure DevOps remote
- GIVEN an empty git repository whose `origin` is `https://o@dev.azure.com/o/p/_git/r`
- WHEN `clm init` runs
- THEN `clm.toml` holds `host = "azure"`

#### Scenario: host given
- GIVEN the GitHub repository above
- WHEN `clm init --host git` runs
- THEN `clm.toml` holds `host = "git"`

#### Scenario: project files kept
- GIVEN a repository with its own `CLAUDE.md` and `clm.toml`
- WHEN `clm init` runs
- THEN both are unchanged, the output lists `kept CLAUDE.md` and `kept clm.toml`, `AGENTS.md` is written, and the output says to add `@AGENTS.md` to `CLAUDE.md`

#### Scenario: not a repository
- GIVEN a directory that isn't in a git repository
- WHEN `clm init` runs
- THEN it exits 1 and the directory is unchanged

#### Needs
- model/human-layers
- config/host-setting
- clm/init-host-values
- host/host-for-remote

### Requirement: template-says-architectural-constraint <!-- base: edbeb5d2b59457d3 -->
The files `clm init` writes SHALL call an entry of `ARCHITECTURE.md` an architectural constraint: the `ARCHITECTURE.md` it writes SHALL start its preamble with `The architectural constraints:` and hold its project's entries in an empty `## Constraints` section, with no `## Requirements` section, and the `AGENTS.md` it writes SHALL say that `ARCHITECTURE.md`'s architectural constraints apply to all work. No file under `crates/clm-init/templates/` SHALL hold the old term (the word `architecture` followed by the word `requirement`, singular or plural, in any case).

#### Scenario: the written layers
- GIVEN an empty git repository
- WHEN `clm init` runs
- THEN its `ARCHITECTURE.md` holds `The architectural constraints:` and `## Constraints` and no `## Requirements`, its `AGENTS.md` holds `architectural constraints`, and neither holds the old term

#### Serves
- architecture/plain-terms

#### Needs
- model/human-layers

### Requirement: templates-say-new-words <!-- base: 4e249586ae9796e4 -->
The root `SPEC.md` `clm init` writes SHALL have `status: unbuilt`; no file under `crates/clm-init/templates/` SHALL hold a word `plain-terms-renames` replaced for a component status, a backlog kind or a `change.md` key.

#### Scenario: a new repository
- GIVEN an empty repository
- WHEN `clm init` runs
- THEN its `specs/SPEC.md` has `status: unbuilt`, no file under `crates/clm-init/templates/` holds a word `plain-terms-renames` replaced, and `clm-model` loads the tree `clm init` wrote with no violation

#### Serves
- architecture/plain-terms

#### Needs
- model/valid-frontmatter
- model/parse-changes

### Requirement: architecture-template-proof <!-- base: 54ed99e1c2b4a9da -->
The `ARCHITECTURE.md` that `clm init` writes SHALL say, in its preamble, that each of the project's entries names the rules that enforce it under `#### Enforced by`, or says why no program can check it under `#### Untested`, or both, and that a test never cites an entry (`human-layer-proof`); it SHALL NOT contain `req:architecture/`. It SHALL hold no `## Enforced by clm` section and no statement of a carried constraint: it SHALL name `clm guide constraints` and `#### Replaces`, saying that CLM's carried constraints reach the project from the `clm` it runs and that a project replaces a `replaceable` one or drops a `default` one with a `#### Replaces` block, which a human reviews (`carried-constraints`).

#### Scenario: fresh repository
- GIVEN an empty git repository
- WHEN `clm init` runs
- THEN its `ARCHITECTURE.md` names `#### Enforced by` and `#### Untested`, holds no `req:architecture/` and no `## Enforced by clm`, and names `clm guide constraints` and `#### Replaces`

#### Serves
- capability/adopt-clm
- architecture/method-in-the-plugin
- architecture/carried-constraints

### Requirement: init-seeds-permissions <!-- base: bda454a5515a65da -->
When the CLM repository has no `specs/SPEC.md` as it starts, `clm init` SHALL add to `permissions.allow` in `.claude/settings.json` at the CLM repository's root each of its unattended allow rules the array lacks, creating the file, `permissions` and `allow` when missing: for each of the `clm` subcommands `apply`, `branch`, `build`, `check`, `claim`, `defect`, `gate`, `land`, `pr`, `rebase`, `setup`, `verify` and `workaround`, the rules `Bash(clm <subcommand>)` and `Bash(clm <subcommand> *)`. `setup` is among them because the build and draft prompts run `clm setup` before their first test run on a code branch (`tests-run-in-their-own-tree`), and auto mode's classifier denies it when no rule allows it, which leaves a draft's code-only item unfixed; this repository's own `.claude/settings.json` allows its two rules first (`clm/session-commands`), so the build that adds them to the list leaves that test passing, and that build writes the note migration that offers them to projects set up earlier, as `upgrade/migration-setup-allow-rules` states it (`template-change-ships-its-migration`, `clm/upgrade-covers-allow-rules`). `build` is among them because `/clm:build` runs every step of its loop through `clm build`, and the loop keeps running its steps while the human is away (`build-loop-in-clm`), and this repository's own `.claude/settings.json` allows its two rules first (`clm/session-commands`), so the build that adds them to the list leaves that test passing; that build writes the note migration that offers them to projects set up earlier, as `upgrade/migration-build-allow-rules` states it. `verify` is among them because `/clm:build`'s loop runs `clm verify` when the project's triggers say, `/clm:triage` runs it at its end, and a pipeline in `pipeline` mode writes the ledger through it (`verify-after-land`); the build that adds it also adds its two rules to this repository's own `.claude/settings.json`, which `clm/session-commands` keeps equal to the list. It SHALL keep every other key and rule of an existing file, SHALL add no rule naming `clm approve` or `git`. When the file isn't a JSON object, or its `permissions` or `permissions.allow` has another type, it SHALL leave the file unchanged and name `.claude/settings.json` and the rules in its next steps. When the CLM repository already has `specs/SPEC.md`, it SHALL leave `.claude/settings.json` unchanged, and when the file lacks one of the rules, its next steps SHALL name `.claude/settings.json` and each missing rule, as the last step before the push step.

#### Scenario: empty repository
- GIVEN an empty git repository
- WHEN `clm init` runs
- THEN `.claude/settings.json` holds `permissions.allow` with exactly the twenty-six rules, `Bash(clm build)`, `Bash(clm build *)`, `Bash(clm land)`, `Bash(clm land *)`, `Bash(clm setup)`, `Bash(clm setup *)` and `Bash(clm verify *)` among them, and none naming `approve` or `git`

#### Scenario: plugins already enabled
- GIVEN a git repository with no `specs/`, holding an untracked `.claude/settings.json` that sets `enabledPlugins` and `permissions.allow` `["Bash(make *)", "Bash(clm land *)"]`
- WHEN `clm init` runs
- THEN `.claude/settings.json` still sets `enabledPlugins`, its `permissions.allow` starts with `Bash(make *)` and `Bash(clm land *)`, and holds each rule once

#### Scenario: settings that aren't JSON
- GIVEN a git repository with no `specs/` and a `.claude/settings.json` holding `{`
- WHEN `clm init` runs
- THEN it exits 0, `.claude/settings.json` is unchanged, and a next step names `.claude/settings.json` and `Bash(clm land *)`

#### Scenario: rerun leaves the settings alone
- GIVEN a repository initialised by an earlier `clm` with no `.claude/settings.json`, committed
- WHEN `clm init` runs
- THEN `git status` shows no change, and a next step, before the push step, names `.claude/settings.json`, `Bash(clm land *)` and `Bash(clm branch *)`

#### Scenario: rerun with the rules
- GIVEN a repository just initialised by `clm init`, committed
- WHEN `clm init` runs again
- THEN `git status` shows no change and no next step names `.claude/settings.json`

#### Serves
- capability/adopt-clm
- capability/build-from-spec
- capability/triage-unattended

#### Needs
- clm/session-commands

### Requirement: init-without-manifest <!-- base: 30800928bee8f39b -->
For each `--lang`, the `scripts/clm-check.sh` that `clm init` writes SHALL first change to the CLM root (the parent of the script's own directory), wherever it is run from, and then run each language's line only when its manifest exists there: `cargo test` when `Cargo.toml` does, `npm ci && npm test` when `package.json` does, and `pytest` when `pyproject.toml` or `setup.py` does; with none of them it SHALL exit 0. Each detector SHALL have a `requires` naming its language's manifest: `Cargo.toml` for `rust`, `package.json` for `typescript` and `pyproject.toml` for `python`. The `typescript` detector SHALL have the command `["node_modules/.bin/knip"]`, `requires = "package.json"` and the hint `npm ci (or npm install --save-dev knip when package.json doesn't list it)`, so a repository without a `package.json` skips it and one whose dependencies aren't installed reports `could not run` with that hint rather than `failed`. In a repository with no code, the `scripts/clm-check.sh` that `clm init --lang rust --lang typescript --lang python` writes SHALL pass. That `clm check` passes there too, and what it reports when the dependencies aren't installed, is the root's to state and prove (`clm/init-result-accepted`).

#### Scenario: empty repository, every language
- GIVEN an empty git repository, and on `PATH` a `cargo`, `npm` and `pytest` that each exit 1
- WHEN `clm init --lang rust --lang typescript --lang python` runs, then `scripts/clm-check.sh`, from the root and from a subdirectory
- THEN the script exits 0 each time, none of the three stubs ran, each detector in `clm.toml` has the `requires` naming its manifest, and the `typescript` detector has the command `node_modules/.bin/knip`

#### Scenario: manifest added later
- GIVEN that repository once it holds a `package.json`, and on `PATH` an `npm` that records its arguments and exits 0
- WHEN `scripts/clm-check.sh` runs
- THEN the stub recorded `ci` and then `test`

#### Serves
- capability/adopt-clm

#### Needs
- clm/detector-requires-skip
- check/detector-relative-command

### Requirement: init-build-setup <!-- base: aeb0d060a35c7e99 -->
When `typescript` is among the `--lang` values given, the `clm.toml` that `clm init` writes SHALL set `[build] setup = "test ! -f package.json || npm ci"`, so a repository with no `package.json` yet passes the setup and one with it installs its packages into the tree the setup runs in; without `--lang typescript` it SHALL write no `[build]` table. That `clm setup` runs the setup it wrote, and `clm check` passes on the result, is the root's to state and prove (`clm/init-result-accepted`).

#### Scenario: a TypeScript project
- GIVEN a git repository with no code
- WHEN `clm init --lang typescript` runs
- THEN `clm.toml` holds a `[build]` table whose `setup` is `test ! -f package.json || npm ci`

#### Scenario: a Rust project
- GIVEN a git repository with no code
- WHEN `clm init --lang rust` runs
- THEN `clm.toml` has no `[build]` table

#### Needs
- config/build-setup-setting
- orchestrator.run/setup-command

#### Serves
- architecture/tests-run-in-their-own-tree

### Requirement: commands-in-crate <!-- base: 130bb26b0b36e695 -->
`clm-init` SHALL offer each of `clm init`, `clm survey` and `clm guide` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, which the command's `--root` overrides, the directory it runs in, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. What `crates/clm` passes it today, of the settings and of other components' results, it takes as further arguments, since a crate loads only the settings of a component it depends on. Every test of `init`'s requirements that sat in the top-level test files of `crates/clm/tests/` SHALL sit in `init`'s own files and call `clm-init` itself, this call where the requirement states what a command prints or how it exits: none runs the `clm` binary (`commands-live-in-their-crates`). The tests of `clm guide` among them SHALL sit in `crates/clm-init/tests/guide.rs`, which holds their `clm:reads` markers of `docs/guide/**` and `docs/spec-format.md` (`clm/doc-readers-marked`), and none of `init`'s tests SHALL be left in `crates/clm/tests/init.rs` or `crates/clm/tests/guide.rs`: a test there that also runs another component's command is the root's (`clm/init-result-accepted`), and `init` proves what `clm init` wrote in a test of its own.

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `init`'s own files
- WHEN it calls `clm-init`'s call with the arguments of `clm init` and two buffers
- THEN the buffers hold what `clm init` prints to standard output and to standard error there, and the call returns the status `clm init` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm init` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Serves
- architecture/path-ownership
- capability/prove-by-tests
