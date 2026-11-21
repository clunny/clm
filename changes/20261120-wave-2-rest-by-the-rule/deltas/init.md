---
component: init
base: { contract: "f6357020eec67907", design: "e1f934a641f57baa" }
---

## MODIFIED Contract
Bringing a repository under CLM (`init-cli-and-skill`). The deterministic half; the conversation and the drafting are `plugin`'s `/clm:init` skill.

- `clm init [--lang <language>]... [--no-detectors] [--host none|git|github|azure]` sets up CLM's tooling in the CLM repository found from the current directory, or at the top of the git repository when there is none (`init-root`), and prints the steps only a human can do. Run again, even after upgrading `clm`, it regenerates nothing: every file it writes belongs to the project once written, since no workflow is generated (`land-locally`). The one file it edits rather than writes whole is `.gitignore`, to which it adds each of `clm.local.toml`, `.clm/` and `.claude/worktrees/` that git doesn't already ignore there, or all three when it has no `.gitignore` (`local-settings`, `init-ignores-session-files`). On its first run, before the repository has a `specs/SPEC.md`, it also adds the narrow allow rules for the `clm` commands unattended sessions run to `.claude/settings.json` (`init-seeds-permissions`, `unattended-steps-run-clm`). The root component it writes owns the files `clm init` writes, and sets `outside`, which turns on `clm check`'s path ownership, to the repository's existing code, so the bootstrap commit passes it before the first spec graph lands (`incremental-adoption`). It prints a line for each file it wrote, kept or added to, then the steps, and exits 0; when it fails, as outside a git repository or for a `--lang` it doesn't know, it prints `clm init: ` and the error on standard error and exits 1 (`init-files`, `init-detectors`).
- `clm init --check-permissions` writes nothing: it prints each of those rules that the repository's Claude Code project settings lack and exits 1 when one is missing, so the `build` and `triage` skills can tell the human, before the first unattended session, what to add (`init-check-permissions`). Outside a git repository it exits 1 with the error `clm init` gives.
- `clm survey [--root <dir>]` prints a compact map of the repository at `--root`, read relative to the current directory, else of the CLM repository found from the current directory, else of the whole git repository (`root-from-cwd`), for drafting its spec graph (`brownfield-survey`); it exits 2 with `clm survey: ` and the error on standard error when it can't list the repository's files (`survey-map`). `clm survey --components <file>` checks a proposed component map against the code and exits 1 with one line per finding on standard output; a map it can't use, or files it can't list, exit 2 with `clm survey: ` and the reason on standard error (`survey-components`). The map may leave part of the code for later, as `outside` globs, as the root's `outside` does (`incremental-adoption`); it checks coverage over the same code files `clm check`'s path ownership reads, so a map it accepts passes that rule once written as the spec graph.
- `import_edges(root)`, a library call, returns the file-to-file import edges `clm survey` finds in the repository at `root`, for `check`'s `undeclared-imports` rule, which `clm check` runs on the spec graph's own components (`undeclared-imports`).
- `clm guide [<page>]` prints CLM's guide, the pages of `docs/guide/` and the spec format, `docs/spec-format.md`, compiled into the binary, and the page `constraints`, generated from CLM's carried constraints: with no page, the index of page names and titles; with one, that page's Markdown (`method-in-the-plugin`, `guide-in-the-binary`, `spec-format-in-the-guide`). A name that isn't a page exits 1 with `no guide page <name>` and the index on standard error (`guide-command`). It reads no file at run time, so it works offline, in any repository or none, and prints the guide of the version that runs. No link in the guide leads outside it, so every link works in an adopter's project and in `clm guide`'s output.
- `UNATTENDED_RULES`, a public constant: the allow rules `clm init` seeds and `clm init --check-permissions` checks for, in their order (`init-seeds-permissions`), which the root's tests of this repository's own settings read (`clm/session-commands`).

Files `clm init` writes, each only when missing, and the ones it adds to:

| File | Owner |
|---|---|
| `clm.toml` | project |
| `scripts/clm-check.sh` | project |
| `ARCHITECTURE.md`, `CAPABILITIES.md`, `specs/SPEC.md`, `AGENTS.md`, `CLAUDE.md` | project |
| `.gitignore`: the lines `clm.local.toml`, `.clm/` and `.claude/worktrees/`, each added when missing | project |
| `.claude/settings.json`: the allow rules for unattended `clm` commands, each added when missing, on the first run only | project |

`clm-init` also holds the migrations written since `20261114-migrations-with-templates` and offers the whole list (`all-migrations`): `migrations()`, `upgrade`'s own followed by its own in the order they landed; `own_migrations()`, its own alone, which the root's lock step (`clm upgrade --lock`) checks and appends to `crates/clm-init/migrations.lock`; and `supersedes()`, the pairs saying which migration supersedes which note. Every caller of `upgrade`'s engine passes `migrations()` and `supersedes()`: the root for `clm upgrade` and the check before each command, `plugin.hooks` for the session-start reminder, and `clm init` itself for the `upgraded` it stamps.

`clm-init` offers each of `clm init`, `clm survey` and `clm guide` as a call, `init_command`, `survey_command` and `guide_command` (`commands-in-crate`, `commands-live-in-their-crates`): each takes the command's arguments as the command line gives them, the CLM repository its caller found from the current directory, if any (only `clm survey` reads it, and its `--root` overrides it), the directory it runs in, the environment as a lookup (`Env`) and a stream each for standard output and standard error, and returns the exit status: 2, with the usage error on the error stream, for arguments it can't parse, 2 for a stream it can't write, and 0 for `--help`, with the help on the output stream. The commands' arguments are `InitArgs`, `SurveyArgs` and `GuideArgs`, which `crates/clm`'s parser embeds, reading `SurveyArgs`' `root` too. `crates/clm` routes the command to the call and adds nothing to what it writes.

## MODIFIED Design
Crate `clm-init`. Templates are compiled into the binary (`include_str!`), so a given `clm` version always writes the same bytes for the same inputs, and tests compare output to fixtures. `init(cwd, options, env)`, its `Options` and `check_nodes(root, file, yaml)` are public only for the crate's own tests, which reach the commands' work through them as well as through the calls; no other crate calls them, and the Contract doesn't offer them. The three commands share one private parser struct, generic over the command's arguments; its doc comment is not the help, so each call hands the parser its command's description, set as clap's `about` beside the command's name, which clap shows as the help's first line, and `crates/clm` takes nothing of the crate's to show it. `tests/says_component.rs` keeps its `clm:reads ARCHITECTURE.md` marker, though none of its tests reads CLM's own `ARCHITECTURE.md` once the scan goes, since the root's `doc_readers.rs::other_crates_read_architecture_md` names the file as a reader of it.

**`clm init`.** It writes in the CLM repository found from the current directory as `root-from-cwd` finds it, and at the top of the git repository when there is none, so rerunning it from a subdirectory, or in a CLM repository that is a subdirectory such as `tool/`, writes that repository's files instead of starting a second one. `clm-init` finds the repository itself, by the same rule as `clm-hooks`' `clm_root`, because it doesn't depend on `clm-hooks`. Then:
- *`clm.toml`*, when missing: `host`, `upgraded` (**Upgrades**), `approval = "any"` with a comment on `other`, `[land] check = "scripts/clm-check.sh"`, `[build] setup = "test ! -f package.json || npm ci"` when `--lang typescript` is given and no `[build]` table otherwise, so a build's worktree installs its own packages before its tests run rather than finding the main checkout's (`init-build-setup`, `build-setup-step`: Rust needs no setup, and a Python environment is the project's to choose), and one `[[detectors]]` table per `--lang` from a built-in table (`rust`, `typescript` and `python`; each with its `hint` and a `requires` naming the language's manifest, `Cargo.toml`, `package.json` or `pyproject.toml`, so a repository without it skips the detector until it lands, and `typescript`'s tool run from `node_modules/.bin/`, so one without its dependencies installed is told to install them: `detector-requires-manifest`), or none with `--no-detectors`, which `/clm:init` passes for an existing codebase so it passes `clm check` before its dead code is cleaned up. `host` is `--host` when given; otherwise what `clm-host`'s `host_for_remote` gives for `origin`'s URL (`github`, `azure` or `git`: `host/host-for-remote`), and `none` with no remote, so `clm-init` holds no host's names (`host-adapters`). It reads the URL from `git remote get-url origin` and calls no host. The comment above `host` names the values `config` accepts, so it changes with `config/host-setting`; the comment's change carries no migration, by the human's waiver of 2026-11-21 recorded in `upgrades-carry-migrations` (a comment's text alone).
- *`scripts/clm-check.sh`*, when missing: an executable starter that builds and tests for each `--lang` (`cargo test`, `npm ci && npm test`, `pytest`), each line run only once the language's manifest exists, so the bootstrap of a repository with no code yet passes it, for the project to edit. It is what `clm land` runs before `clm check`.
- *`ARCHITECTURE.md`*, *`CAPABILITIES.md`*, *`specs/SPEC.md`*, *`AGENTS.md`* and *`CLAUDE.md`*, when missing: an `ARCHITECTURE.md` of a short preamble, a pointer to `clm guide constraints` for CLM's carried constraints, which reach the project from the `clm` it runs rather than as a copy in the file (`carried-constraints`), with how a project replaces or drops one by a `#### Replaces` block, and an empty `## Constraints` section for the project's own entries (`model/human-layers`), so a new repository starts with no entry its own tests would have to prove; a `CAPABILITIES.md` of a short preamble and an empty `## Capabilities` section, for the project's capabilities; a root component titled after the repository with `status: unbuilt` and no requirements; an `AGENTS.md` with CLM's working rules for sessions and a pointer to the guide as `clm guide`, never as a URL, so an adopter's sessions read the guide of the `clm` they run, offline (`method-in-the-plugin`), and a `CLAUDE.md` of the one line `@AGENTS.md`, so Claude Code reads the same text every other agent harness does (`platform`). The rules that must hold live in `clm` and the plugin's skills and hooks, so an older `AGENTS.md` never weakens them. `clm check` passes on the result.
- *`.gitignore`*, in the same directory as `clm.toml`: three lines CLM needs ignored, in this order. `clm.local.toml` is `config`'s settings file for one clone, never committed (`local-settings`); `.clm/` holds a session's scratch files, such as a pull request body, and `clm run`'s and `clm draft`'s plans and prompts; and `.claude/worktrees/` holds the worktrees `plugin`'s skills cut their session branches in and Claude Code puts subagents' worktrees in (`init-ignores-session-files`). Left unignored, `git status` lists them, `git worktree remove` refuses a worktree holding a pull request body, and `git add -A` commits a scratch file or records a worktree as an embedded repository. An entry is *listed* when git already ignores it there: `git check-ignore -q --no-index <path>`, run through `clm-git` in that directory, exits 0 for the path `clm.local.toml`, `.clm/pr-body.md` or `.claude/worktrees/x/f` respectively (`reuse-first`), one call per entry in that order, since `-q` with several paths can't tell them apart. `--no-index` has git read the patterns whatever the index holds: without it git reports a tracked file as not ignored, so a repository that commits `clm.local.toml` would get the line again on every run (`init-rerun`). `clm-git`'s `git` reports exit 1 as a failure carrying its status, which `clm-init` reads as not listed, as `clm-git`'s own `is_ancestor` does, so `vcs` needs nothing new. So git's own rules decide, not a matcher of CLM's (an ignore file elsewhere, such as a parent directory's `.gitignore`, counts too), and the line `clm init` appends goes last, after any negation, so it takes effect. The probes run after `clm.toml`, the check script, `ARCHITECTURE.md` and `CAPABILITIES.md` are written and before `.gitignore` is touched. Git also reads this clone's own excludes (`.git/info/exclude`, `core.excludesFile`), so an entry only they ignore counts as listed and isn't added for other clones: accepted, since that clone's sessions are the ones `clm init` runs for, and adding the line by hand fixes it. The `.gitignore` edits are those of `init-ignores-local-settings`; the added endings are `\r\n` when the file already holds one, so a Windows file stays consistent. In a CLM repository that is a subdirectory such as `tool/`, the lines go in `tool/.gitignore`, where they cover `tool/.clm/` and `tool/.claude/worktrees/`, the directories sessions started in `tool/` use; a session started at the top of the git repository is that repository's own to ignore.

It writes no workflow: checks run and branches land through `clm land` on whoever's machine runs it, and approvals live in the ledger. It then gives the human the steps left (`init-next-steps`): committing and pushing the result to `main` themselves, then the lines `clm-host`'s `setup_steps` gives for the host (`host/host-setup-steps`): what to install and sign in to, and, when the host protects `main`, how to let the identity that runs `clm land` push it, in that host's terms. It can't tell whether `main` is protected without calling the host, so those steps are conditional; whether the push is done it reads from `git status` and from `git diff --quiet` against `origin/main` as last fetched, before the remote `init-keeps-a-remote` adds.

**Upgrades** (`upgrade-migrations`). A repository set up by an earlier `clm init` keeps its files on a rerun (`init-rerun`); bringing it up to date is `upgrade`'s, through its migrations, `clm upgrade` and `/clm:upgrade`. `clm init` prints no upgrade step. The one thing it does for upgrades is stamp a `clm.toml` it writes with `upgraded`, the newest migration the running binary holds (the last of `migrations()`, `all-migrations`), so a new repository starts with nothing pending. A kept `clm.toml` is never stamped: a repository with no `upgraded` is one `upgrade` judges by each migration's `done`.

**`clm survey`.** Its root is `--root` when given, else the CLM repository found from the current directory, else the top of the git repository, so `/clm:init` maps the whole repository from any directory (`root-from-cwd`). Reads the tracked files from `clm-git`'s `tracked_files`, so a file the index holds at several merge stages is read once, skipping `specs/`, `changes/`, `backlog/`, `.github/` and binary files. Prints, as plain text that fits one context for a large repository:
- each directory holding files, with file and line counts per language (by extension);
- manifests (`Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod` and the like) and workspace members;
- directories that hold tests, by convention (`tests/`, `__tests__/`, `*_test.*`, `*.test.*`, `#[cfg(test)]` files);
- Markdown docs and ADRs (`docs/adr/`, `docs/decisions/`, `adr/`);
- import edges between directories, with a count: for Rust, workspace path dependencies and `mod`/`use crate::` paths; for TypeScript, relative imports and workspace packages; for Python, imports that resolve to files in the repository. Extraction uses tree-sitter grammars and one query file per language (`tree-sitter-analysis`). Other languages' files are counted but have no edges.

Directories are listed by path, depth-first, and edges sorted, so the output is the same on every run.

**`clm survey --components <file>`.** The file is YAML: `components:` a list of `{ id, paths, depends_on }`, `paths` being globs as in a component's frontmatter. The files a component's `paths` match are `clm-git`'s `tracked_matching` of them, the same list `clm size` counts a component's code from, so `clm survey --components` and `clm size` agree on which files a component owns (`tracked-files-in-git-layer`). `clm-init` keeps its file names as lossy `String`s, since the survey is text for an agent to read: a name that isn't UTF-8 can't be read back from its lossy form and is left out of the counts. It checks coverage over the code files `check/unowned-files` reads, not the surveyed files, and the map may set `outside` globs as the root does (`incremental-adoption`). Its findings are kept as `(rule, message)` pairs in a sorted set, so each is printed once and the output is the same on every run; each is printed as a `check` violation with the map's path as its path.

**`clm guide`** (`guide-in-the-binary`, `spec-format-in-the-guide`, `guide-table-at-build-time`). `docs/guide/` is the one source of the guide's own pages, and `docs/spec-format.md` joins them as the page `spec-format`, since an adopter's sessions write deltas, requirements and markers in that format. The table of `(name, markdown)` pages is generated, not written: `crates/clm-init/build.rs` lists the `.md` files of `docs/guide/` and writes `$OUT_DIR/guide_pages.rs`, a `const PAGES` holding one `include_str!` per page, by the file's absolute path written with `{:?}`, which escapes any character a string literal can't hold as it is, which `guide.rs` pulls in with `include!`. No `include_str!` under `crates/clm-init/src/` names a `docs/guide/` path. The binary holds every page's bytes as of the build and reads no file at run time, and a page added to `docs/guide/` on a spec branch is in the next build with no edit under `crates/`: adding a page is one spec change, reviewed as `guide-from-docs` wants (`guide-table-from-files`). `name` is the file name without `.md`, ordered as `guide-table-from-files` says, by `README.md`'s links as `guide-table-at-build-time` decided; `spec-format` is the one page the index doesn't link (`reference` links it). The ordering is one pure function over the directory's file names and `README.md`'s text, with a reader that lists a directory and calls it, in a module the build script pulls in with `#[path]`, as `crates/clm/build.rs` does `version.rs`; the crate holds it as `#[cfg(test)] mod`, since nothing at run time calls it and clippy denies dead code, and its unit tests run the reader on a temporary directory without a build. The module uses `std` alone, since it compiles in the build script too. The script prints `cargo:rerun-if-changed` for `docs/guide/` (a directory, which Cargo scans whole, so a page added or removed reruns it), for `docs/spec-format.md`, for itself and for the module it pulls in, since a script that prints the directive at all is rerun for those paths alone. A page's title is its first `# ` heading. Links between guide pages stay relative (`shaping-the-graph.md`, and `../spec-format.md` from a page in `docs/guide/`), which a session reads as `clm guide shaping-the-graph` or `clm guide spec-format`.

**Links stay inside the guide** (`guide-links-inside`). A guide page's links must work wherever the page is read: in `clm guide`'s output in an adopter's project and on GitHub. The page's directory is `docs/guide/`, or `docs/` for `spec-format`. CLM's own repository files, its decisions and its `ARCHITECTURE.md`, are named in prose as CLM's repository, never linked, since an adopter's project has neither. The rule is a function over a page's file path and Markdown, with the set of page files, so its own tests feed it failing and passing cases without touching `docs/`. A link whose target is wrapped in `<...>` or followed by a title (`[t](x.md "title")`) is judged by its path alone. Reference-style definitions (`[label]: target`) and autolinks (`<https://...>`) aren't inline links and aren't read. The scanner is hand-written, with no Markdown dependency: it skips fenced blocks (```` ``` ```` or `~~~` fences) and code spans of any backtick count, and doesn't exempt indented blocks, which the guide doesn't use; link text may nest brackets and a target may hold balanced parentheses, as in CommonMark. A scheme is `[A-Za-z][A-Za-z0-9+.-]*:`. A relative path is normalised lexically (`./x.md` is `x.md`, `a/../x.md` is `x.md`), and one that climbs above the repository root, or keeps a `?` query, fails.

**The constraints page** (`guide-constraints-page`). `clm guide constraints` is the one guide page not read from a file: `clm-init` builds it at run time from `model`'s compiled copy of CLM's carried entries (`model/carried-constraints`), so it is the set the running `clm` enforces. It has no file under `docs/guide/`, so `guide-table-from-files` and `guide-links-inside` don't read it.

**Allow rules for unattended sessions** (`init-seeds-permissions`, `init-check-permissions`, `unattended-steps-run-clm`). Builds and drafts run as subagents under the human's permission mode (`builds-in-subagents`). In Claude Code's auto mode a classifier judges each command that no rule allows, and it denies a build's force-push of its rebased branch and a draft's `clm land` of its own change. A project's allow rule resolves before the classifier, and auto mode drops only broad rules such as `Bash(*)`, so `clm-init` holds one public constant list, `UNATTENDED_RULES`, which the root's test of this repository's settings reads too (`clm/session-commands`): for each of the `clm` subcommands `apply`, `branch`, `build`, `check`, `claim`, `defect`, `gate`, `land`, `pr`, `rebase`, `setup`, `verify` and `workaround`, in that order, the two rules `Bash(clm <subcommand>)` and `Bash(clm <subcommand> *)`, the bare form first, as this repository writes its own script rules (`clm/session-commands`). They are the `clm` commands the build and draft prompts run that write: to the branch, to `origin` or to the backlog, and `clm setup`, which writes the worktree's dependencies by running the project's `[build] setup` (`tests-run-in-their-own-tree`). The classifier denied a draft's `clm setup` as unauthorized persistence, which left the draft's code-only item unfixed (`setup failed`). The command it runs is the project's own, committed in `clm.toml` as `[land] check` is, which the allowed `clm land` runs, so allowing `clm setup` trusts nothing a land doesn't already. `build` is among them because `/clm:build` runs every step of its loop through `clm build`, which keeps running while the human is away (`build-loop-in-clm`), and `verify` because `/clm:build`'s loop runs `clm verify` when the project's triggers say, `/clm:triage` runs it at its end, and a pipeline in `pipeline` mode writes the ledger through it (`verify-after-land`). Each rule added to the list ships a note migration offering it to projects set up earlier (`template-change-ships-its-migration`, `clm/upgrade-covers-allow-rules`), and this repository's own `.claude/settings.json` allows it first, which `clm/session-commands` keeps equal to the list. `clm approve` is never among them, and an allow rule never outranks the guard's `ask` for it (`approval-policy`). No `git push` rule is: `clm branch push` and `clm branch delete` are the pushes an unattended session makes (`host`).

*Seeding.* The settings file is `.claude/settings.json` in the directory holding `clm.toml`, the CLM repository's root. `clm init` adds the rules only when `specs/SPEC.md` was missing when it started, so a rerun leaves the file as it is (`init-rerun`), and the root component it writes names the file in its `paths`, so the bootstrap commit passes path ownership whether `clm init` created the file or found it (`init-root-ownership`). A missing file, or one holding only whitespace, is written as `{"permissions":{"allow":[...]}}`, pretty-printed with two-space indents and a final newline. An existing file is parsed as JSON (`clm-init` enables `serde_json`'s `preserve_order` feature, so its keys keep their order; the feature unifies across the workspace, which only changes the order other crates' maps print in, never what they parse), JSON with comments or trailing commas counting as not JSON; the rules it lacks from `permissions.allow` are appended in the list's order, creating `permissions` and `allow` when missing, and it is written back pretty-printed in the same way, only when a rule was added. A file that isn't a JSON object, or whose `permissions` isn't an object or whose `permissions.allow` isn't an array, is left as it is (`init-seeds-permissions`). Rules elsewhere in the file, the project's own, are kept.

*Checking.* `clm init --check-permissions` reads `.claude/settings.json` and `.claude/settings.local.json` there, each when present, and takes the union of their `permissions.allow` arrays, since Claude Code merges both; a file it can't read as that shape counts as allowing nothing. It compares rules as exact strings, so a rule in the older `Bash(clm land:*)` form counts as missing: the check names exactly what to add. It writes nothing and needs no `specs/`. It finds the CLM repository as `clm init` does (`init-root`). The rules it names, and the first-run seeding, are one list, so a rule added to the list is checked for in every repository at once, including those set up before it existed.


*Next steps.* On a rerun, when the check would report a rule missing, the next steps name `.claude/settings.json` and each missing rule, as the last upgrade step, right before the push step (`init-next-steps`). On a first run they name them only when the file couldn't be edited.

*Ignored settings* (`init-names-ignored-settings`). The rules reach other machines, and the unattended sessions run there, only once `.claude/settings.json` is committed, and a project's `.gitignore` often ignores all of `.claude/`. So on every run, first or rerun, after its `.gitignore` edits, `clm init` asks git whether it would leave the file out of a commit: `git check-ignore -q .claude/settings.json`, through `clm-git` in the CLM repository's root, as for the `.gitignore` probes but without `--no-index`, since a file the index already tracks is committed whatever the patterns say and isn't reported. When it is ignored, the next steps say so and name the fix: in place of a pattern that ignores the whole directory, such as `.claude/`, ignore `.claude/*` and add `!.claude/settings.json` after it (git can't re-include a file whose directory it excludes), or commit the file with `git add -f`. The step comes before the missing-rules step, which stays the last step before the push step (`init-seeds-permissions`), and right before the push step when that one isn't printed. `clm init` edits no pattern of the project's own: which of `.claude/`'s other files the project keeps out is its choice, and the line it would rewrite may sit in another ignore file. The answer is git's, so this clone's own excludes count too: a file only `.git/info/exclude` ignores is reported, which is right, since this clone's commits would leave it out.

**Migrations** (`all-migrations`, `own-migrations-frozen`, `migrations-live-with-init`). What a project set up earlier holds is `init`'s business twice over: `clm init` writes it today, and a migration brings an older project to it. So every migration written since `20261114-migrations-with-templates` lives here, whatever it carries: a changed text of a file `clm init` writes, a settings key, a format `clm` reads. Each is a directory `crates/clm-init/src/migrations/<id>/` of the shape `upgrade`'s Design gives, pulled in from `crates/clm-init/src/migrations.rs`, which lists them in the order they landed and holds this crate's supersedes pairs. A migration of a file `clm init` writes takes its `new` text from the template itself, with `include_str!` of the same file `clm init` writes from, until a later migration moves that text on; then it keeps a copy of the text as it was, in its own directory, since a shipped migration never changes. So the build of a requirement that changes such a file writes the template, the migration and the migration's tests in this component's files, in one branch, and the upgrade corpus test (`clm/upgrade-corpus`) passes on that land. `migrations()` copies `upgrade`'s thirteen and appends this crate's, so the order a project's `upgraded` rests on never changes: nothing is ever added before the last of `upgrade`'s. `crates/clm-init/migrations.lock` is this crate's lock, checked and appended to by `upgrade`'s `check_lock` and `append_lock`; it doesn't exist until a release ships a migration of this crate. A test of one migration gives `upgrade`'s engine a list holding that migration alone, or the list up to it, never the whole list, so adding a migration fails no earlier test.

## MODIFIED Requirements

### Requirement: init-files <!-- base: 35b2667c2d630d8c -->
`clm init` SHALL write `clm.toml`, `scripts/clm-check.sh` (executable), `ARCHITECTURE.md`, `CAPABILITIES.md`, `specs/SPEC.md`, `AGENTS.md` and `CLAUDE.md`, each only when it is missing, and SHALL write no file under `.github/`. For each of these files, in that order, it SHALL print the line `wrote <path>` when it wrote the file and `kept <path>` when the file was there, the lines it prints for `.gitignore` (`init-ignores-local-settings`) and then for `.claude/settings.json` (`init-seeds-permissions`) coming between `CAPABILITIES.md`'s and `specs/SPEC.md`'s. `AGENTS.md` SHALL hold the project's instructions for every agent harness (`platform`), and the `CLAUDE.md` it writes SHALL be the one line `@AGENTS.md`, which Claude Code reads as an import, so every agent harness reads one text. When the repository already has a `CLAUDE.md` with no line that is `@AGENTS.md`, white space after it aside, `clm init` SHALL keep it, still write a missing `AGENTS.md`, and print, right after the line `kept CLAUDE.md`, the line ``add `@AGENTS.md` to `CLAUDE.md` so Claude Code reads the same instructions``. `clm.toml` SHALL set `host` (from `--host`, else the host `host/host-for-remote` gives for `origin`'s URL, or `none` when there is no `origin`), under a comment line naming every value `config/host-setting` accepts, `none`, `git`, `github` and `azure`, and SHALL set `approval = "any"` and `[land] check = "scripts/clm-check.sh"`. Neither `ARCHITECTURE.md` nor `CAPABILITIES.md` SHALL hold an entry; `ARCHITECTURE.md` SHALL name, outside its `## Constraints` section, `clm guide constraints`, where CLM's carried constraints are. That the other commands accept the result is the root's to state (`clm/init-result-accepted`). When `clm init` fails it SHALL print `clm init: ` and the error on standard error, print nothing on standard output, and exit 1. Outside a git repository it SHALL exit 1 and write nothing, the error being `not in a git repository: ` followed by git's message. It SHALL call no `gh` and no `az`.

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

#### Scenario: what it lists
- GIVEN an empty git repository with no remote, and another whose own `CLAUDE.md` is `# Ours`
- WHEN `clm init` runs in each
- THEN the first's output starts with the lines `wrote clm.toml`, `wrote scripts/clm-check.sh`, `wrote ARCHITECTURE.md`, `wrote CAPABILITIES.md`, `wrote .gitignore`, `wrote .claude/settings.json`, `wrote specs/SPEC.md`, `wrote AGENTS.md` and `wrote CLAUDE.md`, in that order; and in the second's output the line after `kept CLAUDE.md` is ``add `@AGENTS.md` to `CLAUDE.md` so Claude Code reads the same instructions``

#### Scenario: a failure's message
- GIVEN a directory that isn't in a git repository
- WHEN `clm init` runs
- THEN it exits 1, standard output is empty, and standard error starts `clm init: not in a git repository: `

#### Scenario: the host comment
- GIVEN an empty git repository
- WHEN `clm init` runs
- THEN the line right above `host = "none"` in `clm.toml` is a comment naming `none`, `git`, `github` and `azure`

#### Needs
- model/human-layers
- config/host-setting
- clm/init-host-values
- host/host-for-remote

### Requirement: init-detectors <!-- base: f534f33fa8ae529c -->
`clm.toml` written by `clm init` SHALL hold one detector for each `--lang` given, each with a `hint`, and none with `--no-detectors`. A language given more than once SHALL count once, and the detectors SHALL follow the order in which the languages were first given: for `rust`, `name = "rust-unused-dependencies"`, `command = ["cargo-machete"]` and `hint = "cargo install --locked cargo-machete"`; for `typescript`, `name = "typescript-dead-code"` with the command and hint `init-without-manifest` states; and for `python`, `name = "python-dead-code"`, `command = ["vulture", "."]` and `hint = "pip install vulture"`; each with the `requires` `init-without-manifest` states. A `--lang` other than `rust`, `typescript` and `python`, compared exactly, case included, SHALL fail `clm init` before it looks for the repository or writes anything, with the error ``unknown language `<name>`; known: rust, typescript, python``.

#### Scenario: two languages
- GIVEN an empty repository
- WHEN `clm init --lang rust --lang python` runs
- THEN `clm config get detectors` lists one Rust and one Python detector

#### Scenario: existing codebase
- GIVEN a repository with code
- WHEN `clm init --lang rust --no-detectors` runs
- THEN `clm config get detectors` prints nothing

#### Scenario: languages it repeats or doesn't know
- GIVEN three empty git repositories
- WHEN `clm init --lang rust --lang rust` runs in the first, `clm init --lang python --lang rust` in the second and `clm init --lang cobol` in the third
- THEN the first's `clm.toml` holds one detector, `rust-unused-dependencies`, with `command = ["cargo-machete"]` and `hint = "cargo install --locked cargo-machete"`; the second's holds `python-dead-code`, with `command = ["vulture", "."]` and `hint = "pip install vulture"`, and then `rust-unused-dependencies`; and the third exits 1, its standard error is the line ``clm init: unknown language `cobol`; known: rust, typescript, python``, and the repository holds nothing but `.git`

### Requirement: init-next-steps <!-- base: 01bcacd1c0b87e71 -->
`clm init` SHALL end by printing, under `Next steps (yours):`, the human's remaining steps: pushing the result to `main` themselves, followed by each line `host/host-setup-steps` gives for the repository's host. It SHALL print the heading after a blank line, and each step on a line of its own starting `- `, and SHALL print neither the blank line nor the heading when there is no step. The host SHALL be the one the repository's `clm.toml` or the clone's `clm.local.toml` sets, and otherwise, or when those settings can't be loaded, the one `clm init` writes or would have written (`init-files`). It SHALL leave out the push step only when no other step comes before it, none of the files it wrote, kept or added to shows a change in `git status`, and either the repository had no `origin` when `clm init` started or those files are the same at `origin/main`, as last fetched, as on disk. It SHALL print no step for upgrading a repository an earlier `clm init` set up, which is `upgrade`'s (`upgrade-migrations`), and SHALL NOT name `CLM_RELEASES_TOKEN`, Actions, auto-merge or a status check.

#### Scenario: next steps
- GIVEN a fresh repository
- WHEN `clm init` runs
- THEN the output's next steps name pushing to `main` and `clm land`, and don't name `CLM_RELEASES_TOKEN` or `auto-merge`

#### Scenario: azure next steps
- GIVEN a fresh repository with `--host azure`
- WHEN `clm init --host azure` runs
- THEN the next steps hold each line `host/host-setup-steps` gives for `azure`, naming `az login` and `Bypass policies when pushing`

#### Scenario: an old clm.toml
- GIVEN a repository with a committed `clm.toml` holding only `approval = "any"`
- WHEN `clm init` runs
- THEN it exits 0, `clm.toml` is unchanged, and no next step names `clm.toml`, `host` or `[land]`

#### Scenario: what is already done
- GIVEN five repositories on `main`: one `clm init` set up and committed, whose `clm.toml` sets `host = "git"`, pushed to a reachable `origin` whose `main` holds that commit; a fresh one with no remote; a fresh one whose `origin` is `https://github.com/o/r.git` and whose own `clm.toml` doesn't parse as TOML; one `clm init` set up and committed, whose `clm.toml` sets `host = "git"`, with its `origin` removed; and one `clm init` set up with no remote and committed, whose `clm.toml` sets `host = "none"`, pushed to the `origin` it was given, whose `main` holds that commit
- WHEN `clm init` runs in each
- THEN the first's next steps are one step, which names that the server may protect `main`, and no push step; the second's are one step, the push step; the third exits 0 and its next steps are the push step and then each line `host/host-setup-steps` gives for `github`; the fourth's next steps are the one step `host/host-setup-steps` gives for `git`, and no push step; and the fifth's output holds no line `Next steps (yours):`

#### Needs
- host/host-setup-steps
- clm/init-host-values

### Requirement: survey-map <!-- base: 3138d8a27068cb5a -->
`clm survey` SHALL print, for the tracked files outside `specs/`, `changes/`, `backlog/` and `.github/`, each once, a file the index holds at several merge stages included, each directory with file and line counts per language, the manifests, test directories, docs and ADRs, and the import edges between directories for Rust, TypeScript and Python, and SHALL print the same bytes on every run over the same tree. It SHALL leave out a file with a NUL byte in its first 8000 bytes, and a tracked file it can't read, as one deleted from the working tree. It SHALL print five sections in this order, each under its heading line, with a blank line between two sections: `# directories`, a line `<dir>  <counts>` for each directory (`.` for the top), its counts `<language> <n> file, <m> line` for each language, joined by `; `, with `s` after `file` or `line` unless its number is 1; `# manifests`, a manifest's path per line, followed by `  workspace: ` and its members joined by `, ` when it declares workspace members; `# tests`, a directory per line; `# docs`, a `.md` file's path per line, and an ADR directory as `<dir>  <n> ADR`, with `s` unless `<n>` is 1; and `# edges`, a line `<from> -> <to>  <count>` per edge. With no file to map it SHALL print the five headings alone and exit 0. When it can't list the repository's files, as outside a git repository, it SHALL print `clm survey: ` and the error on standard error and exit 2.

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

#### Scenario: what is left out
- GIVEN a repository whose index holds `a.rs`, `c.rs` at three merge stages after a conflicted merge, a binary `assets/logo.dat` and `gone.rs`, deleted from the working tree, with `untracked.rs` beside them; an empty git repository; and a directory that isn't in a git repository
- WHEN `clm survey` runs in each
- THEN the first's `.` line counts `rust 2 files` and its output names neither `assets`, `gone.rs` nor `untracked.rs`; the second prints `# directories`, `# manifests`, `# tests`, `# docs` and `# edges`, a blank line between each two and nothing else, and exits 0; and the third exits 2 with nothing on standard output and standard error starting `clm survey: `

#### Needs
- vcs/tracked-files

### Requirement: init-root <!-- base: 79bcb63a755cf2bb -->
`clm init` SHALL write in the first of the current directory and its parents that holds `specs/SPEC.md`, searching no further up than the first directory that holds `.git` or is the top of the git work tree, and at the top of the git repository when there is none.

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

#### Scenario: a work tree whose git directory is elsewhere
- GIVEN a directory holding `specs/SPEC.md`, and under it a work tree with no `.git`, whose git directory lies elsewhere and is named by `GIT_DIR` and `GIT_WORK_TREE`
- WHEN `clm init --host git` runs in the work tree
- THEN it exits 0 and writes `specs/SPEC.md` in the work tree, and no `clm.toml` in the directory above it

### Requirement: init-ignores-local-settings <!-- base: b0683dfd14567a61 -->
`clm init` SHALL make the `.gitignore` in the directory where it writes `clm.toml` list `clm.local.toml`, `.clm/` and `.claude/worktrees/`, where an entry is listed when `git check-ignore -q --no-index <path>`, run in that directory, exits 0 for the path `clm.local.toml`, `.clm/pr-body.md` or `.claude/worktrees/x/f` respectively. When the file is missing it SHALL write it with the three lines, in that order, and list `wrote .gitignore`. When some entries are not listed it SHALL append each of them at the end of the file, on a line of its own and in that order, first adding a line ending when the file is non-empty and doesn't end with one, listing `added <entry> to .gitignore` for each, in that order; the added line endings SHALL be `\r\n` when the file holds a `\r\n`, else `\n`. Otherwise it SHALL leave the file unchanged and list `kept .gitignore`. When `git check-ignore` exits with anything but 0 or 1, or can't be run, `clm init` SHALL fail with git's message and leave `.gitignore` unchanged. When `.gitignore` is there but can't be read, as when it is a directory, `clm init` SHALL fail naming `.gitignore`. Either way, the files it wrote before the failure, those of `clm.toml`, `scripts/clm-check.sh`, `ARCHITECTURE.md` and `CAPABILITIES.md` that were missing, SHALL stay written. It SHALL leave every other byte of an existing `.gitignore` as it was, whether or not the file is UTF-8 (`init-ignores-session-files`).

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

#### Scenario: a failure part way
- GIVEN a repository with no `clm.toml` whose `.gitignore` is `node_modules/` and whose `.clm` is a symbolic link to a directory outside it, and another with no `clm.toml` whose `.gitignore` is a directory
- WHEN `clm init` runs in each
- THEN each exits 1 and `clm.toml` exists in each; the first's standard error names `check-ignore` and holds git's words `symbolic link`, and its `.gitignore` is still `node_modules/`; and the second's standard error starts `clm init: .gitignore: `

### Requirement: init-root-ownership <!-- base: 6b89737623b5155c -->
The `specs/SPEC.md` that `clm init` writes SHALL set `paths: [".claude/settings.json", ".gitignore", "clm.toml", "scripts/clm-check.sh"]`. It SHALL set `outside` to the top-level entries of the other code files the bootstrap commit will hold: the files the index holds under the CLM repository that are still on disk, and the untracked files there that git doesn't ignore once `clm init` has added its `.gitignore` lines; leaving out those under `specs/`, `changes/`, `backlog/` or `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `AGENTS.md` and the four files in `paths`; each entry written `<entry>/**` for a file in a directory and as the file's name for a file at the top, each once, sorted by bytes, each a YAML double-quoted string with `\` and `"` escaped; and `outside: []` when there is none, so path ownership is always on in a repository `clm init` sets up. So the committed result of `clm init` in a repository with code passes `clm check`'s path ownership (`check/unowned-files`), even when it commits a file that was untracked when `clm init` ran, such as an `.mcp.json`; the `.claude/settings.json` that Claude Code writes when the plugins are enabled for the project, and that `clm init` adds its allow rules to (`init-seeds-permissions`), is among the files in `paths`, and the first spec graph narrows `outside` as its components take the code (`incremental-adoption`).

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

#### Scenario: an existing AGENTS.md
- GIVEN a repository holding `AGENTS.md` and `src/a.rs`
- WHEN `clm init` runs
- THEN the root's `outside` holds `src/**` and nothing matching `AGENTS.md`

#### Scenario: a file the index holds but the disk doesn't
- GIVEN a git repository with no `specs/` whose index holds `src/main.rs` and `gone/x.rs`, the latter deleted from the working tree
- WHEN `clm init` runs
- THEN `specs/SPEC.md`'s frontmatter has `outside: ["src/**"]`

#### Serves
- capability/adopt-clm
- capability/work-from-any-agent-harness
- architecture/incremental-adoption

#### Needs

### Requirement: guide-command <!-- base: 088ebe45f26e0248 -->
`clm guide` SHALL print one line per page of the guide, as compiled into the binary, each `<name>  <title>`. The pages SHALL be every `.md` file in `docs/guide/`, `docs/spec-format.md` and the generated page `constraints` (`guide-constraints-page`), whose title is `CLM's constraints`; `<name>` is the file name without `.md` (`index` for `README.md`) and `<title>` the page's first `# ` heading; the pages read from a file SHALL come in the order `guide-table-from-files` states, then `constraints`. For a page read from a file, every page but `constraints`, `clm guide <name>` SHALL print its Markdown exactly as its file is at build time, and exit 0; `clm guide constraints` prints the page `guide-constraints-page` states. For a name that isn't a page it SHALL exit 1, printing `no guide page <name>` and the index to stderr. It SHALL read no file at run time.

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
- GIVEN this repository's `docs/guide/`
- WHEN `clm guide` runs from the binary built from it
- THEN its index names every `.md` file there, `README.md` as `index`

#### Serves
- architecture/method-in-the-plugin
- capability/adopt-clm

### Requirement: guide-table-from-files <!-- base: 5c6bd405f443ae40 -->
The table of pages `clm guide` prints SHALL be generated when `clm-init` builds, from the `.md` files in `docs/guide/` and from `docs/spec-format.md`: a page added to or removed from `docs/guide/` SHALL be in or out of the next build with no change under `crates/`. The table's order SHALL be `index`, then the pages `docs/guide/README.md` links, each in the order of its first link `](<name>.md)` or `](<name>.md#<anchor>)`, `<name>` being ASCII letters, ASCII digits, `_` and `-`, then any page it doesn't link, by name in byte order, then `spec-format`. A `docs/guide/spec-format.md`, a missing `README.md` or a page that isn't UTF-8 SHALL fail the build, naming the file.

#### Scenario: order from the index page
- GIVEN a directory holding `README.md`, which links `b.md` and then `a.md`, and the pages `a.md`, `b.md` and `z.md`
- WHEN the table's order is computed from it
- THEN it is `index`, `b`, `a`, `z`, `spec-format`

#### Scenario: a page named spec-format
- GIVEN a directory holding `README.md` and `spec-format.md`
- WHEN the table's order is computed from it
- THEN it fails, naming `spec-format.md`

#### Scenario: this repository
- GIVEN this repository's `docs/guide/` and `docs/spec-format.md`
- WHEN `clm guide` runs from the binary built from it
- THEN its index lists `index`, then the pages in the order `README.md` first links them, then `spec-format`, each with its title

#### Serves
- architecture/method-in-the-plugin

### Requirement: template-says-pull-request <!-- base: 603d3610e8e6dc1b -->
None of the files `clm init` writes SHALL call a host's pull request a page: none SHALL hold `page's` (only the guide, which `clm guide` prints, has pages).

#### Scenario: the written architecture
- GIVEN an empty git repository
- WHEN `clm init --lang rust --lang typescript --lang python` runs
- THEN none of the files it writes holds `page's`

#### Serves
- architecture/plain-terms

### Requirement: survey-components <!-- base: d0f9f634e7263b01 -->
`clm survey --components <file>` SHALL read the map from `<file>`, relative to the directory it runs in: its list of components from its top-level key `components`, and an optional top-level `outside`, a list of globs like the root's `outside` (`model/outside-globs`). Its *code files* SHALL be those `check/unowned-files` reads: the files the index holds under the repository, binary files and `.github/` included, other than those under `specs/`, `changes/`, `backlog/` or `docs/` and other than `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and `AGENTS.md`. Over them it SHALL report each code file no component covers and no `outside` glob matches (`survey/uncovered`), each code file two components cover (`survey/overlap`), and each `outside` glob that matches no code file outside every component (`survey/stale-outside`); a file a component covers and an `outside` glob matches is the component's. It SHALL report each import edge from one component's file to another component's file that its `depends_on` doesn't allow (`survey/undeclared-edge`), and each `depends_on` cycle (`survey/cycle`); an edge with an end in a file that no component covers, or that more than one covers, SHALL NOT be reported. Its import edges are those `survey-map` finds, from files that leave out `.github/` and binary files. A glob matches a file as the git pathspec `:(glob)<glob>` does. A map whose `outside` isn't a list of non-empty strings SHALL be refused as a map that doesn't parse is. A component of the map MAY set `tests` and `exemptions` as a component's frontmatter does (`model/test-ownership-keys`, `model/exemptions`): a file a component's `tests` match is that component's whatever `paths` match it, a file two components' `tests` match is `survey/overlap`, an edge from a file of a component's `tests` is judged against its `depends_on` as any of its files is, and an edge from a file an exemption of its component names for `undeclared-imports` SHALL NOT be reported (`components-own-their-tests`, `temporary-exemptions`). It SHALL judge each component's `exemptions` as `model/exemptions` judges a component's value, and hold no rule of its own for an entry: a value `model/exemptions` refuses SHALL be refused as a map that doesn't parse is, with a line on standard error for each message the model gives for that value, each `<file>: <component>: <message>` in the model's words, the first after `clm survey: ` and the others with no prefix (it MAY stop at the first component whose value is refused, as it does at a map's first parse error), and the files exempt from `undeclared-imports` SHALL be those of the entries the model accepts. A map that sets `test_depends_on` SHALL be refused as a map that doesn't parse is. A map it can't use SHALL be refused by printing `clm survey: ` and the reason on standard error, nothing on standard output, and exiting 2. It SHALL refuse a map whose file can't be read (the reason `<file>: ` and the error); one that isn't YAML of the map's shape, or that sets a top-level key other than `components`, `nodes` and `outside`, or a key in a component other than `id`, `paths`, `depends_on`, `tests` and `exemptions` (`<file>: ` and the parser's message); one with no `components` (``<file>: missing field `components` ``); one whose `depends_on` names a component the map doesn't list (``<file>: component `<id>` depends on unknown component `<dep>` ``); and one whose `outside` isn't a list of non-empty strings (``<file>: `outside` must be a list of non-empty globs``). A failure to list the repository's files SHALL be refused the same way, with git's error. Each finding SHALL be printed on standard output as a line `<file>: <rule>: <message>`, `<file>` the map's path as given, each once, ordered by rule and then by message, byte by byte, with these messages: for `survey/uncovered`, `<path> is in no component's paths`; for `survey/overlap`, `<path> is in the paths of ` and each component that covers it in backticks, in the map's order, joined by ` and `; for `survey/stale-outside`, `outside glob <glob> matches no file outside every component`; for `survey/undeclared-edge`, ``<from> (component `<a>`) imports <to> (component `<b>`), and `<a>` doesn't depend on `<b>` ``; and for `survey/cycle`, `depends_on cycle: ` and the cycle's IDs joined by ` -> `, starting at its smallest ID and ending with that ID again, each cycle once. It SHALL exit 1 when it reports any and 0 with no output otherwise. A map that lists one ID twice SHALL print only the line `<file>: survey/duplicate-component: component `<id>` appears twice` and exit 1. A map that sets `nodes`, the key `20261102-rename-node-to-component` kept beside `components`, SHALL fail with exit 1, naming the file, `nodes` and `components`, printing only the line ``<file>: survey/old-key: sets `nodes`, which is retired; list the components under `components` ``, and given the old flag, two hyphens followed by `nodes`, `clm survey` SHALL exit 2 with the usage error, as for any argument it doesn't have (`clm/node-names-retired`). Of two faults of the map, the first in this order SHALL decide: a parse failure, `nodes`, a missing `components`, an ID listed twice, an unknown `depends_on`, a wrong `outside`, a failure to list the files, a refused `exemptions`, and then the findings.

#### Scenario: tests carved out of a suite
- GIVEN a component map with `app` (`app/**`, `depends_on: [feat]`), `feat` (`feat/**`, tests `["tests/feat/**"]`) and `harness` (`tests/**`), where `tests/feat/test_x.py` imports a file under `app`
- WHEN `clm survey --components` runs
- THEN it reports no `survey/overlap` line and a `survey/undeclared-edge` line naming `feat` and `app`

#### Scenario: an exempt edge
- GIVEN the same map, with `feat` holding an exemption from `undeclared-imports` for `tests/feat/**` until `backlog/x`
- WHEN `clm survey --components` runs
- THEN it reports no `survey/undeclared-edge` line

#### Scenario: entries the model refuses
- GIVEN the same map, with `feat` holding two exemptions, the first with `until: backlog/a/b` and the second with `rule: paths-disjoint`
- WHEN `clm survey --components` runs
- THEN it exits as it does for a map that doesn't parse, with nothing on standard output, and standard error holds, each on a line naming the file and `feat`, the two messages `model/exemptions` gives for that value, word for word

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

#### Scenario: a map it can't use
- GIVEN the two-crate workspace, and in turn: no file `map.yaml`; a `map.yaml` of `components: 3`; one whose component `a` sets `test_depends_on`; one with a top-level `extra: 1` beside good components; one with no `components`; one whose `a` has `depends_on: [z]` and no component `z`; and one setting `outside: [""]`; then a good map in a directory that isn't in a git repository
- WHEN `clm survey --components map.yaml` runs on each
- THEN each exits 2 with nothing on standard output; for each map in the workspace standard error starts `clm survey: map.yaml: `, for the unknown `depends_on` it is the line ``clm survey: map.yaml: component `a` depends on unknown component `z` ``, for the map with no `components` it holds ``missing field `components` ``, and for the `outside` it holds `` `outside` must be a list of non-empty globs``; and outside git it starts `clm survey: `

#### Scenario: the lines it prints
- GIVEN the two-crate workspace, and in turn a map where `a`, `b` and `c` cover the code with `c`'s `paths` `crates/a/src/lib.rs`; one where `a` (`depends_on: [b]`) and `b` (`depends_on: [a]`) leave `README.md` uncovered; one where `a` (`depends_on: []`) and `b` cover the code; and one listing `a` twice, uncovered files besides
- WHEN `clm survey --components map.yaml` runs on each
- THEN the first prints ``map.yaml: survey/overlap: crates/a/src/lib.rs is in the paths of `a` and `c` ``; the second prints ``map.yaml: survey/cycle: depends_on cycle: a -> b -> a`` before its `map.yaml: survey/uncovered: ` line; the third prints ``map.yaml: survey/undeclared-edge: crates/a/Cargo.toml (component `a`) imports crates/b/Cargo.toml (component `b`), and `a` doesn't depend on `b` ``; and the fourth prints only ``map.yaml: survey/duplicate-component: component `a` appears twice`` and exits 1

#### Scenario: an edge into an overlap
- GIVEN components `a` (`crates/a/**`, `depends_on: []`), `b` (`crates/b/**`) and `c` (`crates/b/Cargo.toml`), covering every other code file, in the two-crate workspace, where `crates/a/Cargo.toml` imports `crates/b/Cargo.toml`
- WHEN `clm survey --components` runs
- THEN it exits 1 with a `survey/overlap` line naming `crates/b/Cargo.toml`, and no `survey/undeclared-edge` line naming it

#### Needs
- clm/component-flags
- model/exemptions

#### Serves
- capability/adopt-clm
- architecture/incremental-adoption
- architecture/plain-terms

### Requirement: template-says-architectural-constraint <!-- base: b04e76224090dc62 -->
The files `clm init` writes SHALL call an entry of `ARCHITECTURE.md` an architectural constraint: the `ARCHITECTURE.md` it writes SHALL start its preamble with `The architectural constraints:` and hold its project's entries in an empty `## Constraints` section, with no `## Requirements` section, and the `AGENTS.md` it writes SHALL say that `ARCHITECTURE.md`'s architectural constraints apply to all work. None of the files `clm init` writes SHALL hold the old term (the word `architecture` followed by a word that starts `requirement`, in any case, a word being a run of letters and digits).

#### Scenario: the written layers
- GIVEN an empty git repository
- WHEN `clm init --lang rust --lang typescript --lang python` runs
- THEN its `ARCHITECTURE.md` holds `The architectural constraints:` and `## Constraints` and no `## Requirements`, its `AGENTS.md` holds `architectural constraints`, and none of the files it writes holds the old term

#### Serves
- architecture/plain-terms

#### Needs
- model/human-layers

### Requirement: templates-say-new-words <!-- base: 5d88e5742c7bdb58 -->
The root `SPEC.md` `clm init` writes SHALL have `status: unbuilt`, SHALL be titled with the name of the CLM repository's directory as a YAML double-quoted string, `\` and `"` escaped, and SHALL hold no requirement. None of the files `clm init` writes SHALL hold a word `plain-terms-renames` replaced for a component status, a backlog kind or a `change.md` key, each compared case included: no line SHALL hold the old name of `unbuilt` (`plain-terms-renames` gives it) anywhere in it, nor `kind:`, one space and `friction` anywhere in it, nor be `kind:`, one space and `human` once the white space before and after it is trimmed, nor hold `burdens` as a word, a word being a run of letters, digits and `_`. The old name of `unbuilt` and the two `kind:` forms, the old names of `human-task` and `workaround`, are on the root's list of retired terms (`clm/retired-terms`), which is why this statement doesn't spell them.

#### Scenario: a new repository
- GIVEN an empty repository
- WHEN `clm init --lang rust --lang typescript --lang python` runs
- THEN its `specs/SPEC.md` has `status: unbuilt`, none of the files it writes holds a word `plain-terms-renames` replaced, and `clm-model` loads the tree `clm init` wrote with no violation

#### Scenario: the root's title
- GIVEN an empty git repository in a directory named `shop`, and another in a directory named `my "app"`
- WHEN `clm init` runs in each
- THEN the first's `specs/SPEC.md` has `title: "shop"` and the second's `title: "my \"app\""`, and neither holds a `### Requirement:` heading

#### Serves
- architecture/plain-terms

#### Needs
- model/valid-frontmatter
- model/parse-changes

### Requirement: says-component <!-- base: c8e61ff07f236904 -->
The messages `clm survey --components` prints and the files `clm init` writes SHALL say component, components, ancestors and placeholder component, never the old words, as `component-words` defines them: the messages of `clm survey --components` (`component `<id>` appears twice`, `depends on unknown component`, `<path> is in no component's paths`, `<from> (component `<a>`) imports <to> (component `<b>`)`, `matches no file outside every component`), `<component-id>/<slug>` in the `AGENTS.md` it writes, `How <name> is split into components.` in the root `SPEC.md`, and the preamble of the `ARCHITECTURE.md` it writes. A name CLM hasn't renamed keeps its spelling, and so does a decision a file cites. A message that names the map's old key to say it is old keeps it.

#### Scenario: a new repository
- GIVEN an empty repository
- WHEN `clm init --lang rust --lang typescript --lang python` runs
- THEN its `AGENTS.md` holds `<component-id>/<slug>`, its `specs/SPEC.md`'s Design reads `How <name> is split into components.`, and none of the files it writes holds an old word but in a constraint's or a decision's name

#### Scenario: a duplicate in the map
- GIVEN a component map listing component `a` twice
- WHEN `clm survey --components` runs on it
- THEN it exits 1 with a line naming component `a`, and no line holds an old word (`component-words`)

#### Serves
- architecture/plain-terms

### Requirement: guide-constraints-page <!-- base: 0587eab72b0a21f6 -->
`clm guide constraints` SHALL print a page titled `# CLM's constraints`, built from `model`'s compiled copy of CLM's carried entries (`model/carried-constraints`), never from the repository it runs in: a short preamble saying these are the architectural constraints every project built with CLM follows, from the `clm` it runs, what each tier means (`method`: CLM relies on it and a project can't replace it; `replaceable`: a project may replace it with its own constraint that keeps the stated guarantee; `default`: a project may drop it), and that a project replaces or drops one with a `#### Replaces` block in an entry of its own `ARCHITECTURE.md`, which a human reviews; Names in backticks that the page doesn't list are CLM's own decisions and constraints, kept in CLM's repository: they say why, and the rule is the text the page gives; then, for each carried entry in CLM's order, `## <slug>`, a line `Tier: <tier>`, its statement and, for a `replaceable` entry, a line `Guarantee: <text>`. It SHALL exit 0 and read no file at run time.

#### Scenario: the page
- GIVEN the binary built from this repository
- WHEN `clm guide constraints` runs in a directory that isn't in a repository
- THEN it exits 0, its first line is `# CLM's constraints`, it holds `## proven-by-tests` followed by `Tier: method`, `## expand-contract` with `Tier: replaceable` and a `Guarantee:` line, and `## yagni` with `Tier: default`, and it holds no `## web-optional`

#### Needs
- model/carried-constraints

#### Serves
- architecture/carried-constraints
- architecture/method-in-the-plugin

### Requirement: init-stamps-upgraded <!-- base: e2f11c8e6d0e7a64 -->
The `clm.toml` `clm init` writes SHALL set `upgraded` to the ID of the last migration `migrations()` returns (`all-migrations`), so a new repository has no migration pending. A kept `clm.toml` SHALL NOT be stamped.

#### Scenario: new repository
- GIVEN an empty directory under version control
- WHEN `clm init` runs
- THEN `clm.toml` sets `upgraded` to the newest migration's ID, and `clm upgrade` prints `up to date`

#### Scenario: kept clm.toml
- GIVEN a repository whose `clm.toml` sets no `upgraded`
- WHEN `clm init` runs
- THEN `clm.toml` is unchanged

#### Needs
- upgrade/migration-list
- config/upgrade-settings

#### Serves
- capability/upgrade-clm

### Requirement: init-seeds-permissions <!-- base: 88ca3bd9037fcc3c -->
When the CLM repository has no `specs/SPEC.md` as it starts, `clm init` SHALL add to `permissions.allow` in `.claude/settings.json` at the CLM repository's root each of its unattended allow rules the array lacks, creating the file, `permissions` and `allow` when missing: for each of the `clm` subcommands `apply`, `branch`, `build`, `check`, `claim`, `defect`, `gate`, `land`, `pr`, `rebase`, `setup`, `verify` and `workaround`, the rules `Bash(clm <subcommand>)` and `Bash(clm <subcommand> *)`. It SHALL keep every other key and rule of an existing file, SHALL add no rule naming `clm approve` or `git`. A file holding only white space SHALL count as one holding no rule. A file it writes or adds to SHALL be JSON printed with two-space indents and a final newline, keeping the order of its keys, and a file that lacks no rule SHALL be left byte for byte. Among the lines `init-files` states, it SHALL print `wrote .claude/settings.json` when it created the file, `added rules to .claude/settings.json` when it added to a file that was there, one of white space included, and `kept .claude/settings.json` when the file lacked no rule, and no line when it left a file of another shape as it is. When the file is there but can't be read, as when it is a directory, `clm init` SHALL fail naming `.claude/settings.json`. When the file isn't a JSON object, or its `permissions` or `permissions.allow` has another type, it SHALL leave the file unchanged and name `.claude/settings.json` and the rules in its next steps. When the CLM repository already has `specs/SPEC.md`, it SHALL leave `.claude/settings.json` unchanged, and when the file lacks one of the rules, its next steps SHALL name `.claude/settings.json` and each missing rule, as the last step before the push step.

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

#### Scenario: how the file is written
- GIVEN four git repositories with no `specs/`: one with no `.claude/settings.json`; one whose `.claude/settings.json` is two spaces and a newline; one whose file is the compact JSON `{"permissions":{"allow":[...]}}` holding every rule; and one whose `.claude/settings.json` is a directory
- WHEN `clm init` runs in each
- THEN the first's file ends `}\n`, indents its keys by two spaces, and the output holds `wrote .claude/settings.json`; the second's file holds exactly the twenty-six rules and the output holds `added rules to .claude/settings.json`; the third's file is unchanged byte for byte and the output holds `kept .claude/settings.json`; and the fourth exits 1 with standard error starting `clm init: .claude/settings.json: `

#### Serves
- capability/adopt-clm
- capability/build-from-spec
- capability/triage-unattended

#### Needs
- clm/session-commands

### Requirement: init-check-permissions <!-- base: 888b5f7357bd15b7 -->
`clm init --check-permissions` SHALL read `permissions.allow` from `.claude/settings.json` and `.claude/settings.local.json` at the CLM repository's root, each when present, and, comparing rules as exact strings, print `missing allow rules in .claude/settings.json:` and then each of `init-seeds-permissions`' rules that neither file allows, one per line in that requirement's order, and exit 1 when one is missing; otherwise it SHALL print nothing and exit 0. A file that isn't JSON of that shape SHALL count as allowing nothing. It SHALL write nothing, and SHALL conflict with every other option of `clm init`. Outside a git repository it SHALL print `clm init: not in a git repository: ` and git's message on standard error, nothing on standard output, and exit 1.

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
- THEN it prints the heading and every rule `init-seeds-permissions` names, in its order, exits 1, and `.claude/` still doesn't exist

#### Scenario: the older rule form
- GIVEN a repository whose `.claude/settings.json` allows `Bash(clm land:*)` and every rule but the two `clm land` ones
- WHEN `clm init --check-permissions` runs
- THEN it prints `Bash(clm land)` and `Bash(clm land *)`, and exits 1

#### Scenario: outside a repository
- GIVEN a directory that isn't in a git repository
- WHEN `clm init --check-permissions` runs
- THEN it exits 1, standard output is empty, and standard error starts `clm init: not in a git repository: `

#### Serves
- capability/build-from-spec
- capability/triage-unattended

### Requirement: init-keeps-a-remote <!-- base: c228f421a0b50df3 -->
When the host `clm init` goes by (`init-next-steps`) is `none` and the repository has no `origin`, `clm init` SHALL, after writing its files, give it the remote `host/none-keeps-a-remote` states, so the push of the result to `main` that its next steps name (`init-next-steps`) has an `origin`, and SHALL print the line `created remote origin at <path>`, `<path>` being the remote's absolute path, before its next steps. When the remote can't be made, `clm init` SHALL fail with the message of the error `host/none-keeps-a-remote`'s call returns, the files it wrote staying written. It SHALL push nothing itself.

#### Scenario: empty repository with no remote
- GIVEN an empty git repository with no remote
- WHEN `clm init` runs
- THEN `clm.toml` holds `host = "none"`, `git remote get-url origin` prints the path of `clm/remote.git` under the common git directory, which has no branch, and the output names that path

#### Scenario: a remote already there
- GIVEN an empty git repository whose `origin` is `https://github.com/o/r.git`, and in turn one with no remote run with `--host git`
- WHEN `clm init` runs
- THEN the remotes are unchanged and no `clm/remote.git` exists

#### Scenario: the line it prints and a failure
- GIVEN an empty git repository with no remote; another whose own `clm.toml` sets `host = "git"`; and an empty git repository with no remote where `clm` under the common git directory is a file
- WHEN `clm init` runs in each
- THEN the first's output holds the line `created remote origin at ` followed by the path of `clm/remote.git` under the common git directory, before the line `Next steps (yours):`; the second has no remote; and the third exits 1 with standard error starting `clm init: `, no remote, and `clm.toml` written

#### Needs
- host/none-keeps-a-remote

### Requirement: init-names-missing-main <!-- base: ae42df80ac590493 -->
When the repository's current branch isn't `main` and neither a local `main` nor the remote's `main` exists, `clm init` SHALL still write its files and SHALL print, as the first line under `Next steps (yours):`, that CLM lands only on a branch named `main` (`trunk-is-main`) and the command that renames the current branch to it, `git branch -m <current> main`, with, when the repository has an `origin`, that the remote's default branch, CI triggers and branch protection keyed to the old name move with it. It SHALL print no such line when `main` exists locally or on the remote, or when `HEAD` is detached, and SHALL rename nothing itself. When the repository has an `origin` it can't reach, a remote-tracking `origin/main`, as last fetched, SHALL count as the remote's `main`.

#### Scenario: a repository on master
- GIVEN a git repository with one commit on `master`, no `main` and no remote
- WHEN `clm init` runs
- THEN it exits 0, writes its files, the first next step names `main` and `git branch -m master main`, and `master` is still the current branch

#### Scenario: an empty repository whose unborn branch is master
- GIVEN an empty git repository whose `HEAD` names `refs/heads/master`
- WHEN `clm init` runs
- THEN the first next step names `git branch -m master main`

#### Scenario: a remote on master
- GIVEN a git repository on `master` whose `origin` has `master` and no `main`
- WHEN `clm init --host git` runs
- THEN the first next step names `git branch -m master main` and says the remote's default branch, CI triggers and branch protection move with it

#### Scenario: main already there
- GIVEN a git repository on a feature branch whose local `main` exists
- WHEN `clm init` runs
- THEN no next step names `git branch -m`

#### Scenario: what it can't see
- GIVEN a git repository with one commit on `master`, no `main` and no remote, its `HEAD` detached at that commit; and a clone on `master` with no local `main`, holding `origin/main` from an earlier fetch, whose `origin` URL now names a path that doesn't exist
- WHEN `clm init --host git` runs in each
- THEN neither's next steps name `git branch -m`

### Requirement: starter-check-land-dir <!-- base: 3e5ee659c2868ce6 -->
The `scripts/clm-check.sh` that `clm init` writes for `--lang rust` SHALL, in the lines it runs when `Cargo.toml` exists and before `cargo test`, export `CARGO_TARGET_DIR` set to the value of `CLM_LAND_DIR` and `CARGO_INCREMENTAL=0` when `CLM_LAND_DIR` is set and non-empty, and leave both as its caller has them otherwise. So a land's cargo build stays warm in the place `clm land` hands it (`land/land-dir-env`), from land to land, and a build's own check, which gets no `CLM_LAND_DIR`, compiles into its worktree's `target/`. A repository with no `Cargo.toml` SHALL still pass the script, as `init-without-manifest` says.

#### Scenario: a land hands the place
- GIVEN a repository `clm init --lang rust` set up, holding a `Cargo.toml`, and on `PATH` a `cargo` that records `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL` and exits 0
- WHEN `scripts/clm-check.sh` runs with `CLM_LAND_DIR=/tmp/x/land` and both cargo variables unset
- THEN the stub saw `CARGO_TARGET_DIR=/tmp/x/land` and `CARGO_INCREMENTAL=0`

#### Scenario: no place handed
- GIVEN the same repository and stub
- WHEN `scripts/clm-check.sh` runs with `CLM_LAND_DIR` unset, `CARGO_TARGET_DIR=/tmp/y` and `CARGO_INCREMENTAL` unset
- THEN the stub saw `CARGO_TARGET_DIR=/tmp/y` and `CARGO_INCREMENTAL` unset

#### Serves
- capability/adopt-clm
- capability/work-concurrently

### Requirement: all-migrations <!-- base: acba0c6841103399 -->
`clm-init` SHALL offer `migrations()`, every migration the binary holds: `upgrade`'s own (`upgrade/own-list-closed`) in their order, followed by this crate's own in the order they landed; `own_migrations()`, this crate's alone, which the lock step reads; and `supersedes()`, `upgrade`'s `supersedes()` followed by this crate's pairs. No two migrations SHALL share an ID, and none of this crate's SHALL be `required` with a `note`.

#### Scenario: the list today
- GIVEN the crate with no migration of its own
- WHEN `migrations()` and `supersedes()` are read
- THEN they are `upgrade`'s thirteen in their order and `upgrade`'s pairs

#### Scenario: a migration of its own
- GIVEN `upgrade`'s list followed by one migration `z`, built as `migrations()` builds its list
- WHEN it is read, and `upgrade`'s `pending_in` runs over it for a project whose `upgraded` names the last of `upgrade`'s and for which `z` is not done
- THEN `z` is last, and it is pending

#### Scenario: a duplicate ID
- GIVEN the binary's list
- WHEN `migrations()` and `own_migrations()` are read
- THEN no two migrations share an ID, and none of `own_migrations()` is `required` with a `note`

#### Serves
- capability/upgrade-clm
- architecture/upgrades-carry-migrations

#### Needs
- upgrade/migration-list
- upgrade/pending-migrations
- upgrade/own-list-closed

### Requirement: own-migrations-frozen <!-- base: 13e2d029b14ec29b -->
For each line `<id> <version> <hash>` of `crates/clm-init/migrations.lock`, this crate's own migrations (`own_migrations()`) SHALL hold `<id>`, and the hash of `crates/clm-init/src/migrations/<id>/` SHALL be `<hash>`, as `upgrade/shipped-migrations-frozen` judges a crate's lock.

#### Scenario: the crate's own lock
- GIVEN `clm-init`'s own migrations and its `migrations.lock` as they stand
- WHEN its own migrations are checked against its lock as `upgrade/shipped-migrations-frozen` checks a crate's
- THEN nothing is reported

#### Serves
- architecture/upgrades-carry-migrations

#### Needs
- upgrade/shipped-migrations-frozen

### Requirement: commands-in-crate <!-- base: 646de73d5876910c -->
`clm-init` SHALL offer each of `clm init`, `clm survey` and `clm guide` as a call of its own that takes the command's arguments as the command line gives them, the CLM repository its caller found from the current directory, if any (`clm survey`'s `--root` overrides it; `clm init` and `clm guide` don't read it), the directory it runs in, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. Given `--help`, each of the three calls SHALL write the command's help to the output stream, nothing to the error stream, and return 0, the help's first line being the command's description, then an empty line, then its `Usage:` line: for `clm init`, `Set up CLM's tooling in the CLM repository found from the current directory, or at the top of the git repository when there is none, and print the steps only a human can do` and `Usage: clm init [OPTIONS]`; for `clm survey`, `Print a compact map of the repository's code, for drafting its spec graph` and `Usage: clm survey [OPTIONS]`; and for `clm guide`, `Print CLM's guide: with no page, the index of its pages; with one, that page` and `Usage: clm guide [PAGE]`. When it can't write to a stream it SHALL return 2.

#### Scenario: a command called in a test
- GIVEN a fixture repository
- WHEN `clm-init`'s call of `clm init` is given its arguments and two buffers
- THEN the buffers hold what `clm init` prints to standard output and to standard error there, and the call returns the status `clm init` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN its caller passes it an option `clm init` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the caller's process goes on

#### Scenario: help
- GIVEN a directory that isn't in a git repository
- WHEN each of the calls of `clm init`, `clm survey` and `clm guide` is given `--help` and two buffers
- THEN each returns 0 with its error buffer empty, and its output buffer's first three lines are the command's description, an empty line and its `Usage:` line, as the statement gives them for that command

#### Scenario: a closed stream
- GIVEN a fresh git repository, and an output stream that fails every write
- WHEN each of the three calls is given that failing stream for its output, `clm init` and `clm survey` in the repository and `clm guide` with no page
- THEN each returns 2

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

#### Needs
- clm/init-result-accepted

## REMOVED Requirements

### Requirement: git-through-layer <!-- base: a0b05934319604b5 -->
### Requirement: survey-tracked-through-git <!-- base: ce420ccf2b75e452 -->
### Requirement: shared-plumbing <!-- base: d34c88aae80c9b3c -->
### Requirement: survey-agents-md <!-- base: 3dd4afc6e43c98ba -->
