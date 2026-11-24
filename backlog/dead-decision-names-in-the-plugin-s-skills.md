---
title: "The plugin's skills cite decisions and requirements that are deleted"
kind: deferred
components: [plugin]
origin: 20261113-phase-2-delete
---

`20261113-phase-2-delete` deleted the superseded decisions and repointed their citations in the specs and the other decisions; the plugin's skills still cite them. Split from `backlog/dead-decision-names-in-code-comments-and-skills` so that `plugin`'s build resolves its half with a `Resolves:` trailer. Each sentence stays; the name goes or becomes the live one:

- `approve-in-session` → `approval-policy`: `plugin/skills/build/SKILL.md`, `plugin/skills/propose/SKILL.md`, `plugin/skills/triage/SKILL.md`.
- `forced-fixes-cover-additions` (the paragraph of `plugin/skills/propose/SKILL.md` that still teaches the withdrawn rule): rewrite the bullet as `coupled-changes-sequence` states it: the user first adds a default arm marked `clm:remove-by` the change, in a requirement of its own; the widening needs it; the user's requirement that handles the new value needs the widening and removes the arm.
- `skills-follow-agent-harness` → `agent-harness-adapters`: the build, configure, contribution, health, init, propose, triage and verify skills.
- `unattended-permissions-check` → `init/init-check-permissions`: the build and triage skills.
- `propose-elaborates-ideas` → `elaborate-ideas-on-main` and `triage-enters-branch-worktree` → `session-worktrees`: the triage skill.
- Names dropped, not renamed: `architecture-audit` (the propose skill already names `spec-architecture-audit` beside it), `triage-folds-renames-alone`, `recommend-the-right-design`, `propose-harness-only-work`, `triage-revises-declined-fold`, `draft-backlog-only-change`, `background-work-stopped`, `triage-archives-complete` and `init-skill-asks-verify`.

`change-done` in the init skill is a `[verify] triggers` value, not a dead name: leave it. Prompt text, fixed by component with no test (`prompt-text-is-source`).
