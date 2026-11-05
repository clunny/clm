---
title: "Check runs started together on one clone fail each other over shared host resources"
kind: bug
components: [land, orchestrator.run, orchestrator.triage, plugin]
issues: [73, 91, 139]
---

Several builds and drafts on one clone run the project's full check at the same time: `clm land --check-only` from build and draft prompts, and `clm land` itself. Lands queue behind one another on `.clm/land.lock` (`land-target-lock`), but `--check-only` takes no lock, so it overlaps other check-only runs and lands. When a project's tests bind a fixed local port, as notespeed's browser tests do, whichever run starts second fails for a reason unrelated to its branch. A build then either retries until the port is free or uses up its stop budget and leaves its work unlanded.

Expected: builds and drafts that CLM runs together on one clone can't fail each other's checks through shared host resources. Options to weigh: put check-only runs under the same per-clone lock lands take, give each check run its own port or environment variable, or at least have the method warn that fixed-port tests will fail intermittently under parallel builds. Whichever is chosen, a failure caused by another run's check shouldn't count against a build's retry budget.
