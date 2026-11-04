---
title: "The check script validates the clm plugin as it validates the panel, with whatever Claude Code the landing machine has"
date: 2026-10-28
status: accepted
---

`panel-validate-skips-without-claude` added `claude plugin validate plugin-panel` to the check script after Claude Code refused the panel's hooks module and nothing noticed. `plugin/`, the `clm` plugin itself, is loaded by the same engine and was still unvalidated: a manifest, hook or skill frontmatter it refuses would land on the plugin's tests, which read the files as text. The script now runs `claude plugin validate plugin` whenever every test is selected or the selection names a file under `plugin/`, with the same flags (neither `--strict` nor `--profile`: only what the engine refuses fails, not the plugin directory's listing checks, which today want a LICENSE and an icon) and the same skip when `claude` isn't on `PATH`.

**The validator is unpinned, and that is accepted.** The review that found the gap noted that the verdict depends on the installed `claude`, which auto-updates, so an update could fail a land of unchanged plugin code. That Claude Code is the one that loads the plugin on the landing machine, and on every user's machine soon after: a refusal after an update is the plugin failing to load for that version, which is a real finding, fixed as a code fix to the plugin. The selection keeps the blast radius small: a land that changes nothing under `plugin/` or `plugin-panel/` runs neither validation, so an update never fails a land of unrelated code; a full run (a session's own check, the release's `prepare` job, a human by hand) validates both, since both ship. Pinning Claude Code for the check would validate against a version users no longer run.

Rejected:
- **Validating in every run, selection or not:** costs a `claude` start on lands that can't have changed the plugin.
- **`--profile directory`:** fails today on the missing LICENSE and icon, which the plugin directory wants and the engine doesn't; adding them is a separate choice.
