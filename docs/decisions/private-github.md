---
title: "Hosting on GitHub, in a public repository"
date: 2026-10-21
status: accepted
---

CLM is hosted on GitHub. The repository, `clunny/clm`, is public, and it is the install repository of `public-site`. It was private until 2026-11-03, and the later decisions that give "the repo is private" as a reason record why they were made then.

Of the GitHub features first relied on (branch protection, CODEOWNERS, Actions and auto-merge), only Actions is still used, for the release workflow alone (`host-ci-for-releases`): every check and every land runs on the lander's machine (`land-locally`), and what CLM asks of a host goes through an adapter (`host-adapters`).
