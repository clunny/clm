---
title: "clm runner launches builds with the agent harness the human picks, not only claude -p"
kind: idea
components: [orchestrator.runner]
---

Stage 3 of `20261112-any-harness` (`any-agent-harness`). `clm runner` starts `claude -p` for each build (`machine-runners`). Let it take the agent harness to launch, such as `codex exec` or the Antigravity CLI's headless mode, with that agent harness's own permission option passed through unchanged, as `--permission-mode` is today, and the guard wired through that agent harness's hooks (`orchestrator.runner/runner-needs-guard`).
