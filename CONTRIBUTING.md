# Contributing to CLM

CLM takes contributions of intent, reviewed by hand by its maintainer. A pull request that changes any file outside the spec paths, `specs/`, `changes/`, `backlog/`, `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md` and `CLAUDE.md`, is closed: a fix to `README.md` or to code arrives as an issue instead, and the project's own builds write the code.

CLM is built with CLM: read `ARCHITECTURE.md`, then `CLAUDE.md`, which says how a change flows. `clm guide contributions` says how a project takes contributions of this kind.

## The levels

- **An issue.** One line is enough: a bug, or an idea.
- **A short item.** A pull request adding `backlog/<slug>.md` with `title`, `kind: idea` and a few lines of reasoning. Leave out `priority`: its slots are the maintainer's call.
- **An elaborated item.** The same file with the sections `## Intent`, `## Settled`, `## Approaches` and `## Open questions`.
- **A full change.** `change.md` and deltas from `/clm:propose`, run in a clone of your fork.

Agree an idea in an issue first, since writing a spec costs you model use.

## Your commits

A commit counts as yours only when its email is linked to your GitHub account. Leave "Allow edits by maintainers" on. The maintainer rebases your branch when `main` has moved, so you needn't chase it.

## Set a clone up (full changes only)

You need this only for a full change, where you run CLM yourself. Run `scripts/dev-install.sh` once per clone, from any of its worktrees. It needs `jq`, `cargo` and `claude` on `PATH`, and the clone's main worktree on `main`.

It changes, and only for this clone:

- the binary, under the clone's git directory (`<git common dir>/clm-dev`);
- the plugin marketplace manifest `.clm/dev-marketplace.json` in the main worktree, which git ignores;
- `.claude/settings.local.json` in the main worktree, which sets `CLM_BIN` and enables the `clm-dev` plugins in place of the released ones;
- the `reference-transaction` hook, which rebuilds the binary from `origin/main` in the background whenever it moves;
- Claude Code's user-level marketplace list, which gains `clm-dev`.

The clone and its worktrees then run `origin/main`'s `clm` and the main checkout's plugin, while every other project on your machine keeps running the release. Restart sessions that were already running.

### Running `clm` in the clone

A plain `clm` outside a Claude Code session finds nothing of this build. Run it from the clone with `cargo run -q -p clm --`, for example `cargo run -q -p clm -- status`.

`clm check` outside a session needs the built binary on `PATH` as well, because the `unreferenced-public-items` detector in `clm.toml` runs the first `clm` it finds there. Put `<git common dir>/clm-dev/bin` first on `PATH` for that, for example `PATH="$(git rev-parse --path-format=absolute --git-common-dir)/clm-dev/bin:$PATH" clm check`.

### Undo it

`scripts/dev-install.sh --remove` undoes the setup: the hook, `clm-dev`, the manifest file, the settings entries and the marketplace record. The clone's next session runs the release.
