---
title: "The web app stays current by polling the Worker"
date: 2026-10-23
status: accepted
---

While a page is visible, the web app asks the Worker again about every 60 seconds and whenever the page becomes visible, and redraws the view in place when the answers change. The Worker's 60-second live-state cache answers almost every poll, so an open tab costs no GitHub requests or KV writes beyond what that cache already makes (`web-on-free-plan`). Every GitHub call stays in the Worker, so the user's token never leaves its `HttpOnly` cookie (`web-github-app`).

Alternatives considered:
- WebSockets through a Durable Object that the webhook notifies. Changes would show within seconds, but it adds a Cloudflare component and needs the GitHub App to subscribe to PR, check and branch events, a settings change the operator makes by hand. Kept as `backlog/web-live-push`.
- The browser calling GitHub directly. Its script would have to hold the user's token, so any injected script could take it. GitHub offers browsers no push channel, so it would still poll, and the unauthenticated limit for public repos (60 requests an hour per address) is too low for that.
