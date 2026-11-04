---
title: "dev-install's move off a directory marketplace leaves the clone's tracked Claude settings modified"
kind: bug
components: [clm]
---

Step: `scripts/dev-install.sh` on a clone still set up the old way, when its `clm` marketplace is a directory record, so it runs `claude plugin marketplace remove clm` and then `add clunny/clm` (`clm/dev-install` item 5).

What happened: straight after the script ran, the tracked `.claude/settings.json` in the main worktree was modified: its `enabledPlugins` and `extraKnownMarketplaces` entries for `clm` were emptied. The file's modified time is 5 seconds before `settings.local.json`'s. The script never edits that file itself, so this is most likely Claude Code's marketplace removal; that cause was inferred from the timing and not reproduced. `add` doesn't put the entries back. The main checkout is left dirty, and committing the edit would stop fresh clones being offered the plugin, against `dogfood-from-main` and `dev-install-scoped`.

Expected: the script leaves tracked files unchanged, or at least says what it changed, since `CONTRIBUTING.md`'s list of what the setup changes doesn't include this file. The human's clone was fixed with `git restore .claude/settings.json`.

Why this needs a spec change: no requirement states that the setup leaves tracked files alone (`code-fix-needs-a-written-rule`). A rule could be added to `clm/dev-install`, with the script restoring the file after the move, and its stub `claude` test made to edit that file as the real one does. Reproduce the cause on a real clone first.
