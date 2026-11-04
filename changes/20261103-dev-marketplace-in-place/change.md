---
status: proposed
human_burdens:
  - "Once the change is built, run `scripts/dev-install.sh` once in each set-up clone, then restart its sessions: until then the clone's panel stays unloaded."
---

# The clm-dev marketplace is the main worktree

## Intent

A clone that `scripts/dev-install.sh` set up should load every plugin of its `clm-dev` marketplace from the checkout, the panel included. Today the panel fails to load, so `/clm:view` and the panel beside the chat are missing from the very sessions that build them. This change makes the marketplace the main worktree itself, through a renamed copy of the tracked manifest, and has the setup check that each `clm-dev` plugin loads, so a failure like this stops the setup instead of showing up sessions later.

## Problem

Setup builds the marketplace at `<common>/clm-dev/marketplace` and symlinks each plugin's folder to the main worktree (`clm/dev-install`, step 4). Claude Code refuses a plugin hook module whose real path lies outside the marketplace's directory. The panel's hooks module, `./register.tsx`, resolves through the symlink to `<main worktree>/plugin-panel/hooks/register.tsx`, so `claude plugin list` reports `clm-panel@clm-dev` as failing to load with `Path escapes plugin directory`. `clm@clm-dev`, linked the same way, still loads: the check bites on hook modules. `20261103-dev-install-scoped` measured both plugins loading. Since then either the panel gained its hooks module or Claude Code gained the check; the setup has no check of its own that would have said which.

## What we measured

On this machine's Claude Code, each case under a throwaway `CLAUDE_CONFIG_DIR`, with a copy of `plugin-panel` in a scratch repository:

- Today's layout, the plugin folder a symlink out of the marketplace: fails to load with the same error.
- A source written `../plugin-panel`: refused at install, since a source must start with `./`.
- The repository itself as a directory marketplace: loads in place, but its manifest is the tracked one, named `clm`, the name of the shared GitHub record.
- `claude plugin marketplace add <repo>/.clm/dev-marketplace.json`, a manifest file naming the marketplace `clm-dev` with source `./plugin-panel`: loads in place. Claude Code resolves a manifest file's `./` sources against the folder above the file's own folder, as it does for `.claude-plugin/marketplace.json`. `claude plugin list --json` reads the plugin from `<repo>/plugin-panel`, and `marketplace list --json` records `source: "file"`, `path` the file and `installLocation` the repository.
- `marketplace add` of the file under a name a directory record already holds re-points that record to the file and keeps the plugins installed from it, which then load from the new source.
- `claude plugin list --json` gives a plugin that fails to load a non-empty `errors` array, one entry per project path at local scope. The two `clm-panel@clm-dev` lines `claude plugin list` prints are those records, one for the main worktree and one for a linked worktree, and not a fault.

## Approaches

- **The main worktree as the marketplace, through a manifest file (chosen).** The setup writes the tracked manifest, renamed `clm-dev`, to `<main worktree>/.clm/dev-marketplace.json`, which git ignores, and adds that file. The plugins really lie inside the marketplace, so Claude Code's path check has nothing to refuse, and they load in place, so a skill edit still applies at the next session. It relies on how Claude Code resolves a manifest file's sources, which was measured, not read in its documentation: so the setup checks that each plugin loads and stops with Claude Code's errors when one doesn't.
- **Copy the plugins into `<common>/clm-dev/marketplace` on each setup and rebuild.** It passes the check, but an edit reaches sessions only after the next copy, not at the next session, which `dev-install-scoped` exists to give.
- **Keep the symlinks and enable the release panel.** The clone's sessions would run the release's panel, against `dev-install-scoped`.
- **The tracked `.claude-plugin/` holding a second, ignored manifest.** It resolves the same way, but puts an untracked file beside a tracked one, and needs a `.gitignore` entry of its own; `.clm/` is already ignored.

## Outcome

- Setup writes `<main worktree>/.clm/dev-marketplace.json`, adds it as `clm-dev` (which also re-points a record an earlier setup made at `<common>/clm-dev/marketplace`), installs the plugins, deletes `<common>/clm-dev/marketplace` and then checks `claude plugin list --json`, exiting 1 with each failing plugin's errors before it touches the settings file.
- `--remove` also deletes the manifest file, and reads the plugins to uninstall from it, or from an earlier setup's manifest when only that exists.
- The binary stays at `<common>/clm-dev/bin/clm`; the hook, the rebuild and the settings entries don't change.

## Constraints

- `dev-install-scoped`: nothing outside the clone changes, and the shared `clm` record stays on GitHub.
- `no-shared-install`: only the human runs the script, hence the burden.
- `CONTRIBUTING.md`'s list of what the setup changes names the manifest file in the main worktree beside `<common>/clm-dev`.
