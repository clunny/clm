---
status: done
resolves: [cloudflare-token-workers-routes]
---

# The Cloudflare API token holds the zone's Workers Routes permission

## Intent

The human has done the `human-task` `cloudflare-token-workers-routes`: the token behind the repository secret `CLOUDFLARE_API_TOKEN` now holds the `clm-app.dev` zone's Workers Routes permission, which deploying the app's Worker on routes needs (`site-worker-of-its-own`, `web/site-served-directly`). This change records that fact in the decision `site-worker-of-its-own`, in place of its pointer to the item, and deletes the item, which unblocks `web/site-served-directly`. It changes the backlog and that decision only: no requirement, scenario or design changes, so there is nothing to build. `web`'s Design already states the token as it now is ("with the zone's Workers Routes permission").

## The human's answers

> the zone config hasn't been done yet but I'll do it right now if you guide me
>
> the token is updated
>
> (Guided: the token behind CLOUDFLARE_API_TOKEN was edited in place to include Zone → Workers Routes → Edit and Zone → Zone → Read, with Zone Resources including clm-app.dev; the secret did not change.)

## Problem

- `web/site-served-directly` waited on a token permission only the human could grant. It is granted, so the block is stale.

## Approaches

- **A. Delete the item and record the fact in the decision (chosen).** The decision is where the move's prerequisites are written, and it named the item.
- **B. Also restate the token in `web`'s Design.** Its Setup list already says the token has the zone's Workers Routes permission; adding Zone → Zone → Read there would rewrite the whole Design for one detail that builds nothing.

## Outcome

- `backlog/cloudflare-token-workers-routes.md` is gone, so `web/site-served-directly` is no longer blocked.
- `docs/decisions/site-worker-of-its-own.md` says the token holds Zone → Workers Routes → Edit and Zone → Zone → Read on `clm-app.dev`, edited in place, the secret unchanged.

## Constraints

- `requirement-blocks`: a settled `human-task` is deleted and its fact written down, so the requirement it blocked can be built.

## Edge cases and workarounds

None.
