---
title: "A tracked script sets a clone up to run main's clm and the checkout's plugin, and clm --version names its commit"
date: 2026-10-27
status: accepted
---

`dogfood-from-main` runs this repository's sessions on `main`'s `clm` and the checkout's plugin. That was first set up by hand, through an untracked hook and entries in `.claude/settings.local.json` that existed only on the maintainer's machine. A new clone, another machine or another maintainer started on the release without knowing it, and `clm --version` printed the same `0.7.0` for the release and for every build of `main` since, so nobody could tell which was running.

**The script.** `scripts/dev-install.sh`, owned by the root node, does what was done by hand: refuses unless the clone's main worktree is on `main` (the setup is "run `main`"), installs `clm` from it with `cargo install --locked --path crates/clm --root <common>/clm-dev --target-dir target/install`, into the clone's common git directory and not onto `PATH` (`dev-install-scoped`), writes a `reference-transaction` hook that calls the tracked script with `--rebuild` whenever `origin/main` moves (`rebuild-on-origin-main`), adds the `clm-dev` marketplace (`dev-marketplace-in-place`), and merges `CLM_BIN` and the enabled `clm-dev` plugins into the main worktree's `settings.local.json`. `--remove` undoes the hook, the marketplace and the settings entries. Because the hook only calls the tracked script, a change to the rebuild reaches every clone with the next pull. It writes only to the main worktree, since Claude Code reads `settings.local.json` there and the hook lives in the git directory every worktree shares.

**Why a script, not a `clm` subcommand.** It only means something in a clone of CLM's own repository, so a subcommand would ship to every adopter a command that refuses everywhere they run it, and `crates/clm` is dispatch only. A script can also install `clm` on a clone that has no `clm` yet, without `cargo run` first. It merges JSON with `jq`, a common tool (`reuse-first`); a hand-written JSON edit in `sh` would be the fragile part, and the script refuses before changing anything when `jq` is missing.

**Refusing a foreign hook.** An existing `reference-transaction` hook without the script's marker line, `# clm dev-install`, is someone's own hook; the script refuses to replace it unless given `--force`.

**`clm --version` names the commit.** A build script appends the first 12 hex digits of the commit the source was built from: `clm 0.7.0 (0123456789ab)`. It does so for every build from a git checkout, release builds included, since the release workflow builds from a checkout of the release commit; telling a release from a build of `main` then means comparing the commit to the tag, and two builds of `main` differ. A source outside a git checkout prints the bare version. No `-dirty` mark: knowing whether the tree had uncommitted changes would need the build script to rerun on every file change in the workspace, and the setup builds from `main`, which has none.

Consequences:
- Setting a clone up is one command, `scripts/dev-install.sh`, run by the human in a clone of CLM. Builds still never install `clm`.
- The rebuild follows `origin/main`, so a land's push, a fetch and a pull, a rebasing one included, all rebuild (`rebuild-on-origin-main`).
- `clm --version`'s output gained a suffix; nothing in this repository parses it.

Alternatives considered:
- A `clm dev-install` subcommand: see above.
- Documenting the steps in `CLAUDE.md` only: the hook's text and the settings entries would be copied by hand, and drift from what `dogfood-from-main` describes.
- A version suffix only for builds outside a release, set by the release workflow: it needs the workflow to pass a flag, and a release binary naming its commit is no loss.
