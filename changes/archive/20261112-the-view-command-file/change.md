---
status: done
resolves: [the-view-command-file-names-no-queue]
---

## Intent

Without `clm-panel`, `/clm:view` with no name shows `clm view queue`, the view the panel opens on, instead of `clm view flight`.

## Problem

`backlog/the-view-command-file-names-no-queue` found two gaps in `plugin/commands/view.md`, the text the model reads when no `clm-panel` hooks module answers `/clm:view`. The first, that its names lack `queue`, was fixed on `main` by commit `69b92e58` (`plugin: view fallback names the queue view`), under `plugin/view-fallback-wording` as it stands. The second remains: with no name the file still has the model show `clm view flight`, while the panel opens on Queue (`plugin.panel/views-panel`). No requirement states what the fallback shows for no name, so by `code-fix-needs-a-written-rule` the rule goes into `view-fallback-wording` first.

## Approaches

- **State it in `view-fallback-wording` (chosen).** The file is `plugin`'s and that requirement already governs its wording; one sentence and a scenario its build cites.
- **Code-only fix.** No requirement as written says which view the no-name fallback shows, so a test would cite nothing that states it.
- **Accept Flight.** The panel opened on Flight when the file was written; it now opens on Queue, so keeping Flight makes the two paths of the same command disagree for nothing.

## Outcome

A `plugin` build changes the no-name line of `plugin/commands/view.md` to show `clm view queue` and adds a test for the new scenario. No other component changes; no capability test reads the file.

## Constraints

- The fallback stays a thin instruction to run `clm` (`plugin-plus-rust-cli`); nothing new is decided in the plugin.
