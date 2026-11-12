---
title: "A draft's clm setup is denied by Claude Code's auto-mode classifier"
kind: bug
components: [orchestrator.triage]
origin: 20261112-platform-fake-dead-helpers
---

In an unattended draft (`20261112-platform-fake-dead-helpers`, a worktree-isolated subagent of a triage session in auto mode), `clm setup`, run as one plain command right after `clm claim web.worker.platform --fix backlog/platform-fake-dead-helpers` as the draft prompt's `draft-prompt-setup` text asks, was denied by Claude Code's auto-mode classifier with the reason "Unauthorized Persistence". The project's `[build] setup` runs `npm ci --ignore-scripts` in five directories of the worktree. With setup denied, the draft must leave its code-only item (`left platform-fake-dead-helpers: setup failed`), so a deferred item that touches only Rust and needs none of the npm packages can't be fixed by any draft. The draft or triage flow should get `clm setup` allowed for draft sessions, for example by a permission rule the plugin or `clm init` installs, or by telling the triage session to ask for that permission before it launches drafts. Also, a component whose tests need no npm package could skip the setup.
