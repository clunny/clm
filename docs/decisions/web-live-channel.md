---
title: "Open web pages hear of changes through a Durable Object per repo"
date: 2026-10-27
status: accepted
---

The web app polls the Worker about once a minute (`web-polls-worker`). To show a change within seconds, each open page also holds a WebSocket to a Durable Object, `Live`, one per repo, which the Worker reaches by `id_from_name("<owner>/<repo>")`. Every verified webhook ends by posting the object a notice, `{"delivery"}`, which it sends to every socket; a page that receives one refreshes at once, asking the Worker for live state, the default branch's head included, keyed by that delivery (`?fresh=`) so the minute-old cache doesn't answer it. The Web Push sends of `push-subscriptions-in-ledger` run in the same object, so their crypto stays out of the webhook's 10 ms of CPU.

- The object uses the WebSocket hibernation API, so idle sockets cost no duration, and the SQLite storage class, the only one on the Free plan; it stores nothing (`git-is-truth`). One object request per webhook, none per poll (`web-on-free-plan`).
- The notice names no branch, record or item. Access is checked when the socket opens and again by every refetch, so a socket left open by someone who has since lost access learns only that the repo changed.
- `push` events cover every ref, ledger appends included, so claims, approvals, checks and lands all arrive. `check_run` and `check_suite` aren't needed, since checks are ledger records (`land-locally`), nor `create`, since a branch's creation is also a push. `pull_request` is added for page changes and review notifications (`push-subscriptions-in-ledger`).
- Polling stays as the fallback: a page whose socket can't connect is as current as before.
- `?fresh=` lets an invited person make the Worker fetch live state more than once a minute; only invited people can sign in (`invite-only-web`).

Alternatives considered:
- Server-Sent Events from the Worker. A Worker request can't be reached by another request, so the webhook couldn't tell the streams; it would need the Durable Object anyway.
- Polling more often. Most polls would find nothing changed, at a cost per tab in Worker requests and GitHub calls.
- Purging the live-state cache from the webhook. The Cache API is per data centre, so the purge would miss the pages' data centres.
