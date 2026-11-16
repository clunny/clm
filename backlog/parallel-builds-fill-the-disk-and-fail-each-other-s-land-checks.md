---
title: "Parallel builds fill the disk and fail each other's land checks"
kind: bug
components: [orchestrator.loop]
---

During the unattended build of check.health (slot 4), `clm land --check-only` failed twice with ENOSPC: once in web/site (playwright artifacts) and once in capabilities/web (vitepress build). The checks themselves passed up to that point. The root filesystem was at 100% (196G, 50M free) because of the builds running beside it. Clearing this worktree's own target/debug/incremental freed 6.6G, and the concurrent builds used that up within minutes. The loop starts builds without checking that the free disk covers what each worktree's target directory and check run need (`clm machine` reads free disk, but nothing seems to hold a build back on it). So builds fail for reasons that have nothing to do with their code, and no build can land.
