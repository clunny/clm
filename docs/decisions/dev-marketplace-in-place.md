---
title: "The clm-dev marketplace is the main worktree, added through a manifest file"
date: 2026-11-03
status: accepted
---

Refines `dev-install-scoped`, replacing its "In place, through symlinks". That setup built `clm-dev` at `<common>/clm-dev/marketplace` and symlinked each plugin's folder to the main worktree. Claude Code refuses a plugin hook module whose real path lies outside the marketplace's directory, so `clm-panel@clm-dev` failed to load (`Path escapes plugin directory: ./register.tsx (hooks)`), and the clone's sessions had no panel.

**The marketplace's root is the main worktree.** `claude plugin marketplace add` takes a path to a manifest file, and resolves its `./` sources against the folder above the file's own, as it does for `.claude-plugin/marketplace.json`. So `scripts/dev-install.sh` writes the tracked manifest, renamed `clm-dev`, to `<main worktree>/.clm/dev-marketplace.json` and adds that file. The plugins then lie inside the marketplace, so nothing escapes it, and Claude Code reads them from the main worktree, so an edit still applies at the next session. `.clm/` is already ignored, so nothing tracked changes, and the shared `clm` record stays on GitHub. Adding the file under the name `clm-dev` re-points the record an earlier setup made and keeps its installed plugins.

**The setup checks that the plugins load.** The resolution was measured on this machine's Claude Code, not read in its documentation, and the symlinked layout broke without anything saying so. So after installing, the setup reads `claude plugin list --json` and stops, before writing the settings file, when a `clm-dev` plugin for the main worktree has `errors`.

Consequences:
- The marketplace manifest lives in the main worktree, not in `<common>/clm-dev`, which keeps only the binary.
- A clone set up earlier keeps a panel that doesn't load until its human runs the script again.
- A future Claude Code that resolves a manifest file's sources differently makes the setup fail with Claude Code's own error, not a silently missing plugin.

Alternatives considered:
- **Copy the plugins into `<common>/clm-dev/marketplace`.** It loads, but an edit reaches sessions only at the next copy.
- **Keep the symlinks and enable the release panel.** The clone would run the release's panel, against `dev-install-scoped`.
- **A second, ignored manifest in the tracked `.claude-plugin/`.** It resolves the same, but sits an untracked file beside a tracked one and needs a `.gitignore` line of its own.
- **A `../` source.** Claude Code refuses a source that doesn't start with `./`.
