---
component: init
base: { contract: "43cbe535c8f07fbf" }
---

## MODIFIED Contract

Bringing a repository under CLM (`init-cli-and-skill`). The deterministic half; the conversation and the drafting are `plugin`'s `/clm:init` skill.

- `clm init [--lang <language>]... [--no-detectors] [--host none|git|github|azure]` sets up CLM's tooling in the CLM repository found from the current directory, or at the top of the git repository when there is none (`init-root`), and prints the steps only a human can do. Run again, even after upgrading `clm`, it regenerates nothing: every file it writes belongs to the project once written, since no workflow is generated (`land-locally`). The one file it edits rather than writes whole is `.gitignore`, to which it adds each of `clm.local.toml`, `.clm/` and `.claude/worktrees/` that git doesn't already ignore there, or all three when it has no `.gitignore` (`local-settings`, `init-ignores-session-files`). On its first run, before the repository has a `specs/SPEC.md`, it also adds the narrow allow rules for the `clm` commands unattended sessions run to `.claude/settings.json` (`init-seeds-permissions`, `unattended-steps-run-clm`). The root component it writes owns the files `clm init` writes, and sets `outside`, which turns on `clm check`'s path ownership, to the repository's existing code, so the bootstrap commit passes it before the first spec graph lands (`incremental-adoption`).
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

`clm-init` also holds the migrations written since `20261114-migrations-with-templates` and offers the whole list (`all-migrations`): `migrations()`, `upgrade`'s own followed by its own in the order they landed, and `supersedes()`, the pairs saying which migration supersedes which note. Every caller of `upgrade`'s engine passes these two: the root for `clm upgrade` and the check before each command, `plugin.hooks` for the session-start reminder, and `clm init` itself for the `upgraded` it stamps.

`clm-init` offers each of `clm init`, `clm survey` and `clm guide` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the directory it runs in, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## ADDED Requirements

### Requirement: commands-in-crate
`clm-init` SHALL offer each of `clm init`, `clm survey` and `clm guide` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, which the command's `--root` overrides, the directory it runs in, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. What `crates/clm` passes it today, of the settings and of other components' results, it takes as further arguments, since a crate loads only the settings of a component it depends on. Every test of `init`'s requirements SHALL call `clm-init` itself, this call where the requirement states what a command prints or how it exits, and SHALL sit in `init`'s own files: none runs the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `init`'s own files
- WHEN it calls `clm-init`'s call with the arguments of `clm init` and two buffers
- THEN the buffers hold what `clm init` prints to standard output and to standard error there, and the call returns the status `clm init` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm init` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on
