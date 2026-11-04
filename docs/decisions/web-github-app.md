---
title: "One shared web instance with a public GitHub App, tokens only in an encrypted cookie"
date: 2026-10-22
status: accepted
---

There is one hosted `web` instance, run by the operator on their Cloudflare account, and one **public** GitHub App that any org or user can install on chosen repos. A private App could only be installed on the account that owns it. Teams connect by installing the App, and nobody else has to deploy anything.

People sign in through the App's user authorization flow. Each person sees only repos that are both covered by an installation and readable with their own token. Because one cache serves many organisations, isolation is the main security boundary: every request rechecks access with the requester's token, and cache keys always name the repo. Approving adds `spec-approved` with that user's token, so GitHub records the person, not a bot, as the one who approved.

Tokens live only in an encrypted `HttpOnly` cookie, so the Worker holds no sessions and no credentials (`web-derived-cache`).

Alternatives considered:
- Each team deploys its own instance with a private App. That means more setup for every team, though each would keep its own cache.
- An OAuth App. It grants broad `repo` scope over everything the user can reach, with no per-repo install.
- A server-side session store. That's state that isn't derived, against `git-is-truth`.

Labels added through the App are recorded as "via" the App. A later change could make CI accept only those approvals, which Claude's `gh` can't produce (`backlog/app-attested-approval`).
