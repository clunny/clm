---
component: init
base: { contract: "b00b81d3ec955725", design: "4a7b79c9c0215f02" }
---

## MODIFIED Contract
Bringing a repository under CLM (`init-cli-and-skill`). The deterministic half; the conversation and the drafting are `plugin`'s `/clm:init` skill.

- `clm init [--lang <language>]... [--no-detectors] [--host none|git|github]` sets up CLM's tooling in the CLM repository found from the current directory, or at the top of the git repository when there is none (`init-root`), and prints the steps only a human can do. Run again, even after upgrading `clm`, it regenerates nothing: every file it writes belongs to the project once written, since no workflow is generated (`land-locally`). The one file it edits rather than writes whole is `.gitignore`, to which it adds each of `clm.local.toml`, `.clm/` and `.claude/worktrees/` that git doesn't already ignore there, or all three when it has no `.gitignore` (`local-settings`, `init-ignores-session-files`). On its first run, before the repository has a `specs/SPEC.md`, it also adds the narrow allow rules for the `clm` commands unattended sessions run to `.claude/settings.json` (`init-seeds-permissions`, `unattended-steps-run-clm`). The root component it writes owns the files `clm init` writes, and sets `outside`, which turns on `clm check`'s path ownership, to the repository's existing code, so the bootstrap commit passes it before the first spec graph lands (`incremental-adoption`).
- `clm init --check-permissions` writes nothing: it prints each of those rules that the repository's Claude Code project settings lack and exits 1 when one is missing, so the `build` and `triage` skills can tell the human, before the first unattended session, what to add (`init-check-permissions`).
- `clm survey` prints a compact map of the CLM repository found from the current directory, or of the whole git repository when there is none (`root-from-cwd`), for drafting its spec graph (`brownfield-survey`). `clm survey --components <file>` checks a proposed component map against the code and exits 1 with what's wrong. The map may leave part of the code for later, as `outside` globs, as the root's `outside` does (`incremental-adoption`); it checks coverage over the same code files `clm check`'s path ownership reads, so a map it accepts passes that rule once written as the spec graph.
- `import_edges(root)`, a library call, returns the file-to-file import edges `clm survey` finds in the repository at `root`, for `check`'s `undeclared-imports` rule, which `clm check` runs on the spec graph's own components (`undeclared-imports`).
- `clm guide [<page>]` prints CLM's guide, the pages of `docs/guide/` and the spec format, `docs/spec-format.md`, compiled into the binary, and the page `constraints`, generated from CLM's carried constraints: with no page, the index of page names and titles; with one, that page's Markdown (`method-in-the-plugin`, `guide-in-the-binary`, `spec-format-in-the-guide`). It reads no file at run time, so it works offline, in any repository or none, and prints the guide of the version that runs. No link in the guide leads outside it, so every link works in an adopter's project and in `clm guide`'s output.

Files `clm init` writes, each only when missing, and the ones it adds to:

| File | Owner |
|---|---|
| `clm.toml` | project |
| `scripts/clm-check.sh` | project |
| `ARCHITECTURE.md`, `CAPABILITIES.md`, `specs/SPEC.md`, `CLAUDE.md` | project |
| `.gitignore`: the lines `clm.local.toml`, `.clm/` and `.claude/worktrees/`, each added when missing | project |
| `.claude/settings.json`: the allow rules for unattended `clm` commands, each added when missing, on the first run only | project |

## MODIFIED Design
Crate `clm-init`. Templates are compiled into the binary (`include_str!`), so a given `clm` version always writes the same bytes for the same inputs, and tests compare output to fixtures.

**`clm init`.** Fails outside a git repository. It writes in the CLM repository found from the current directory as `root-from-cwd` finds it, and at the top of the git repository when there is none, so rerunning it from a subdirectory, or in a CLM repository that is a subdirectory such as `tool/`, writes that repository's files instead of starting a second one. `clm-init` finds the repository itself, by the same rule as `clm-hooks`' `clm_root`, because it doesn't depend on `clm-hooks`. Then:
- *`clm.toml`*, when missing: `host`, `upgraded` (**Upgrades**), `approval = "any"` with a comment on `other`, `[land] check = "scripts/clm-check.sh"`, and one `[[detectors]]` table per `--lang` from a built-in table (at least `rust`, `typescript` and `python`; each with its `hint` and a `requires` naming the language's manifest, `Cargo.toml`, `package.json` or `pyproject.toml`, so a repository without it skips the detector until it lands, and `typescript`'s tool run from `node_modules/.bin/`, so one without its dependencies installed is told to install them: `detector-requires-manifest`), or none with `--no-detectors`, which `/clm:init` passes for an existing codebase so it passes `clm check` before its dead code is cleaned up. `host` is `--host` when given; otherwise `github` when `origin`'s URL names `github.com`, `git` for any other remote, and `none` with no remote. It reads the URL from `git remote get-url origin` and calls no host.
- *`scripts/clm-check.sh`*, when missing: an executable starter that builds and tests for each `--lang` (`cargo test`, `npm ci && npm test`, `pytest`), each line run only once the language's manifest exists, so the bootstrap of a repository with no code yet passes it, for the project to edit. It is what `clm land` runs before `clm check`.
- *`ARCHITECTURE.md`*, *`CAPABILITIES.md`*, *`specs/SPEC.md`* and *`CLAUDE.md`*, when missing: an `ARCHITECTURE.md` of a short preamble, a pointer to `clm guide constraints` for CLM's carried constraints, which reach the project from the `clm` it runs rather than as a copy in the file (`carried-constraints`), with how a project replaces or drops one by a `#### Replaces` block, and an empty `## Constraints` section for the project's own entries (`model/human-layers`), so a new repository starts with no entry its own tests would have to prove; a `CAPABILITIES.md` of a short preamble and an empty `## Capabilities` section, for the project's capabilities; a root component titled after the repository with `status: unbuilt` and no requirements; and a `CLAUDE.md` with CLM's working rules for sessions and a pointer to the guide as `clm guide`, never as a URL, so an adopter's sessions read the guide of the `clm` they run, offline (`method-in-the-plugin`). The rules that must hold live in `clm` and the plugin's skills and hooks, so an older `CLAUDE.md` never weakens them. `clm check` passes on the result.
- *`.gitignore`*, in the same directory as `clm.toml`: three lines CLM needs ignored, in this order. `clm.local.toml` is `config`'s settings file for one clone, never committed (`local-settings`); `.clm/` holds a session's scratch files, such as a pull request body, and `clm run`'s and `clm draft`'s plans and prompts; and `.claude/worktrees/` holds the worktrees `plugin`'s skills cut their session branches in and Claude Code puts subagents' worktrees in (`init-ignores-session-files`). Left unignored, `git status` lists them, `git worktree remove` refuses a worktree holding a pull request body, and `git add -A` commits a scratch file or records a worktree as an embedded repository. An entry is *listed* when git already ignores it there: `git check-ignore -q --no-index <path>`, run through `clm-git` in that directory, exits 0 for the path `clm.local.toml`, `.clm/pr-body.md` or `.claude/worktrees/x/f` respectively (`reuse-first`), one call per entry in that order, since `-q` with several paths can't tell them apart. `--no-index` has git read the patterns whatever the index holds: without it git reports a tracked file as not ignored, so a repository that commits `clm.local.toml` would get the line again on every run (`init-rerun`). `clm-git`'s `git` reports exit 1 as a failure carrying its status, which `clm-init` reads as not listed, as `clm-git`'s own `is_ancestor` does, so `vcs` needs nothing new. So git's own rules decide, not a matcher of CLM's (an ignore file elsewhere, such as a parent directory's `.gitignore`, counts too), and the line `clm init` appends goes last, after any negation, so it takes effect. A git failure fails `clm init` before `.gitignore` is touched; files written before it stay written. That includes git refusing a probe path, as when `.clm` is a symbolic link: it fails closed. Because git reads the machine's own configuration, the tests run `clm init` with `GIT_CONFIG_GLOBAL` and `XDG_CONFIG_HOME` pointing into an empty temporary directory and `GIT_CONFIG_NOSYSTEM=1`, so a developer's global ignore can't change the outcome. Git also reads this clone's own excludes (`.git/info/exclude`, `core.excludesFile`), so an entry only they ignore counts as listed and isn't added for other clones: accepted, since that clone's sessions are the ones `clm init` runs for, and adding the line by hand fixes it. The `.gitignore` edits are those of `init-ignores-local-settings`; the added endings are `\r\n` when the file already holds one, so a Windows file stays consistent. In a CLM repository that is a subdirectory such as `tool/`, the lines go in `tool/.gitignore`, where they cover `tool/.clm/` and `tool/.claude/worktrees/`, the directories sessions started in `tool/` use; a session started at the top of the git repository is that repository's own to ignore.

It writes no workflow: checks run and branches land through `clm land` on whoever's machine runs it, and approvals live in the ledger. It then prints, one per line under `Next steps (yours):`, what's left for the human: commit and push the result to `main` themselves, and, when the host protects `main`, let the identity that runs `clm land` push it (a bypass or an allowed pusher). It can't tell whether `main` is protected without calling the host, so it prints the second step as a conditional. A step that is already done isn't printed.

**Upgrades** (`upgrade-migrations`). A repository set up by an earlier `clm init` keeps its files on a rerun (`init-rerun`); bringing it up to date is `upgrade`'s, through its migrations, `clm upgrade` and `/clm:upgrade`. `clm init` prints no upgrade step. The one thing it does for upgrades is stamp a `clm.toml` it writes with `upgraded`, the newest migration the running binary holds (`upgrade`'s `latest`), so a new repository starts with nothing pending. A kept `clm.toml` is never stamped: a repository with no `upgraded` is one `upgrade` judges by each migration's `done`.

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

**Allow rules for unattended sessions** (`init-seeds-permissions`, `init-check-permissions`, `unattended-steps-run-clm`). Builds and drafts run as subagents under the human's permission mode (`builds-in-subagents`). In Claude Code's auto mode a classifier judges each command that no rule allows, and it denies a build's force-push of its rebased branch and a draft's `clm land` of its own change. A project's allow rule resolves before the classifier, and auto mode drops only broad rules such as `Bash(*)`, so `clm-init` holds one public constant list, `UNATTENDED_RULES`, which the root's test of this repository's settings reads too (`clm/session-commands`): for each of the `clm` subcommands `apply`, `branch`, `check`, `claim`, `defect`, `gate`, `land`, `pr`, `rebase` and `workaround`, in that order, the two rules `Bash(clm <subcommand>)` and `Bash(clm <subcommand> *)`, the bare form first, as this repository writes its own script rules (`clm/session-commands`). They are the `clm` commands the build and draft prompts run that write: to the branch, to `origin` or to the backlog. `clm approve` is never among them, and an allow rule never outranks the guard's `ask` for it (`approval-policy`). No `git push` rule is: `clm branch push` and `clm branch delete` are the pushes an unattended session makes (`host`).

*Seeding.* The settings file is `.claude/settings.json` in the directory holding `clm.toml`, the CLM repository's root. `clm init` adds the rules only when `specs/SPEC.md` was missing when it started, so a rerun leaves the file as it is (`init-rerun`), and the root component it writes names the file in its `paths`, so the bootstrap commit passes path ownership whether `clm init` created the file or found it (`init-root-ownership`). A missing file, or one holding only whitespace, is written as `{"permissions":{"allow":[...]}}`, pretty-printed with two-space indents and a final newline. An existing file is parsed as JSON (`clm-init` enables `serde_json`'s `preserve_order` feature, so its keys keep their order; the feature unifies across the workspace, which only changes the order other crates' maps print in, never what they parse), JSON with comments or trailing commas counting as not JSON; the rules it lacks from `permissions.allow` are appended in the list's order, creating `permissions` and `allow` when missing, and it is written back pretty-printed in the same way, only when a rule was added. A file that isn't a JSON object, or whose `permissions` isn't an object or whose `permissions.allow` isn't an array, is left as it is, and the next steps name the rules to add by hand. Rules elsewhere in the file, the project's own, are kept.

*Checking.* `clm init --check-permissions` reads `.claude/settings.json` and `.claude/settings.local.json` there, each when present, and takes the union of their `permissions.allow` arrays, since Claude Code merges both; a file it can't read as that shape counts as allowing nothing. It compares rules as exact strings, so a rule in the older `Bash(clm land:*)` form counts as missing: the check names exactly what to add. It prints each missing rule on its own line, in the list's order, after a line `missing allow rules in .claude/settings.json:`, and exits 1; with none missing it prints nothing and exits 0. It writes nothing and needs no `specs/`. It finds the CLM repository as `clm init` does (`init-root`), and outside a git repository it exits 1 saying so, as `clm init` does. The rules it names, and the first-run seeding, are one list, so a rule added to the list is checked for in every repository at once, including those set up before it existed.

*Next steps.* On a rerun, when the check would report a rule missing, the next steps name `.claude/settings.json` and each missing rule, as the last upgrade step, right before the push step (`init-next-steps`). On a first run they name them only when the file couldn't be edited.

## ADDED Requirements

### Requirement: init-seeds-permissions
When the CLM repository has no `specs/SPEC.md` as it starts, `clm init` SHALL add to `permissions.allow` in `.claude/settings.json` at the CLM repository's root each of its unattended allow rules the array lacks, creating the file, `permissions` and `allow` when missing: for each of the `clm` subcommands `apply`, `branch`, `check`, `claim`, `defect`, `gate`, `land`, `pr`, `rebase` and `workaround`, the rules `Bash(clm <subcommand>)` and `Bash(clm <subcommand> *)`. It SHALL keep every other key and rule of an existing file, SHALL add no rule naming `clm approve` or `git`. When the file isn't a JSON object, or its `permissions` or `permissions.allow` has another type, it SHALL leave the file unchanged and name `.claude/settings.json` and the rules in its next steps. When the CLM repository already has `specs/SPEC.md`, it SHALL leave `.claude/settings.json` unchanged, and when the file lacks one of the rules, its next steps SHALL name `.claude/settings.json` and each missing rule, as the last step before the push step.

#### Scenario: empty repository
- GIVEN an empty git repository
- WHEN `clm init` runs
- THEN `.claude/settings.json` holds `permissions.allow` with exactly the twenty rules, `Bash(clm land)` and `Bash(clm land *)` among them, none naming `approve` or `git`, and `clm check` exits 0

#### Scenario: plugins already enabled
- GIVEN a git repository with no `specs/`, holding an untracked `.claude/settings.json` that sets `enabledPlugins` and `permissions.allow` `["Bash(make *)", "Bash(clm land *)"]`
- WHEN `clm init` runs
- THEN `.claude/settings.json` still sets `enabledPlugins`, its `permissions.allow` starts with `Bash(make *)` and `Bash(clm land *)`, holds each rule once, and `clm check` exits 0

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

### Requirement: init-check-permissions
`clm init --check-permissions` SHALL read `permissions.allow` from `.claude/settings.json` and `.claude/settings.local.json` at the CLM repository's root, each when present, and, comparing rules as exact strings, print `missing allow rules in .claude/settings.json:` and then each of `init-seeds-permissions`' rules that neither file allows, one per line in that requirement's order, and exit 1 when one is missing; otherwise it SHALL print nothing and exit 0. A file that isn't JSON of that shape SHALL count as allowing nothing. It SHALL write nothing, and SHALL conflict with every other option of `clm init`.

#### Scenario: all allowed
- GIVEN a repository just initialised by `clm init`
- WHEN `clm init --check-permissions` runs
- THEN it prints nothing, exits 0, and `git status` shows no change

#### Scenario: split across the two files
- GIVEN a repository whose `.claude/settings.json` allows every rule but `Bash(clm land)` and `Bash(clm land *)`, and whose `.claude/settings.local.json` allows `Bash(clm land *)`
- WHEN `clm init --check-permissions` runs
- THEN it prints the heading line and `Bash(clm land)` alone, and exits 1

#### Scenario: no settings
- GIVEN a repository with `specs/SPEC.md` and no `.claude/`
- WHEN `clm init --check-permissions` runs
- THEN it prints the heading and all twenty rules, exits 1, and `.claude/` still doesn't exist

#### Scenario: the older rule form
- GIVEN a repository whose `.claude/settings.json` allows `Bash(clm land:*)` and every rule but the two `clm land` ones
- WHEN `clm init --check-permissions` runs
- THEN it prints `Bash(clm land)` and `Bash(clm land *)`, and exits 1

#### Serves
- capability/build-from-spec
- capability/triage-unattended

## MODIFIED Requirements

### Requirement: init-root-ownership <!-- base: 82afadf4ea12450f -->
The `specs/SPEC.md` that `clm init` writes SHALL set `paths: [".claude/settings.json", ".gitignore", "clm.toml", "scripts/clm-check.sh"]`. It SHALL set `outside` to the top-level entries of the repository's other code files: of the files the bootstrap commit will hold, which are the files the index holds under the CLM repository and the untracked files under it that the `.gitignore` doesn't ignore as `clm init` leaves it (`clm-git`'s `files_to_commit`, read after `clm init` has written its `.gitignore` lines, so `clm init` writes those lines before it writes `specs/SPEC.md`), other than those under `specs/`, `changes/`, `backlog/` or `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and the four files in `paths`, the first component of each path, written `<component>/**` for a file in a directory and as the file's name for a file at the top, each once, sorted by bytes, each written as a YAML double-quoted string with `\` and `"` escaped; and `outside: []` when there is none, so path ownership is always on in a repository `clm init` sets up. So the committed result of `clm init` in a repository with code passes `clm check`'s path ownership (`check/unowned-files`), even when it commits a file that was untracked when `clm init` ran, such as an `.mcp.json`; the `.claude/settings.json` that Claude Code writes when the plugins are enabled for the project, and that `clm init` adds its allow rules to (`init-seeds-permissions`), is among the files in `paths`, and the first spec graph narrows `outside` as its components take the code (`incremental-adoption`). `clm survey` and `clm survey --components` still read the index alone. Its tests assert the frontmatter written; the check itself is `check`'s.

#### Scenario: repository with code
- GIVEN a git repository with no `specs/` tracking `src/main.rs`, `src/lib/x.rs`, `scripts/build.sh`, `README.md` and `docs/a.md`
- WHEN `clm init` runs
- THEN `specs/SPEC.md`'s frontmatter has `paths: [".claude/settings.json", ".gitignore", "clm.toml", "scripts/clm-check.sh"]` and `outside: ["README.md", "scripts/**", "src/**"]`

#### Scenario: empty repository
- GIVEN an empty git repository
- WHEN `clm init` runs
- THEN `specs/SPEC.md`'s frontmatter has `paths: [".claude/settings.json", ".gitignore", "clm.toml", "scripts/clm-check.sh"]` and `outside: []`

#### Scenario: untracked files the bootstrap commits
- GIVEN a git repository with no `specs/`, no global or system git excludes, tracking `src/main.rs` and a `.gitignore` holding `build/`, with the untracked files `.claude/settings.json`, `.mcp.json`, `build/out.o` and `.clm/pr-body.md`
- WHEN `clm init` runs
- THEN `specs/SPEC.md`'s frontmatter has `outside: [".mcp.json", "src/**"]`, naming neither `.claude/**`, whose one file `paths` names, `build/**`, which the `.gitignore` ignored before, nor `.clm/**`, which the line `clm init` adds ignores

#### Serves
- capability/adopt-clm
- architecture/incremental-adoption

#### Needs
