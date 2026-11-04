---
title: "Web Push subscriptions are ledger records"
date: 2026-10-27
status: accepted
---

The installed web app can notify a person when a `question` or `human` backlog item reaches a repo's default branch, and when a pull request on a `spec/*` branch opens for review. Sending a Web Push needs the browser's subscription (`endpoint`, `p256dh`, `auth`), which isn't derived from git. It is kept as a `subscribed` record in the person's web ledger, `clm/ledger/<login>/web`, which the Worker appends with the person's token as it appends an approval; turning notifications off appends an `unsubscribed` record naming it. The live subscriptions are the fold's, as approvals are (`per-writer-ledgers`), so `git-is-truth` holds unchanged.

- A subscription is per person and per repo, which is what a web ledger on that repo is.
- Anyone who can read the repo can read the records. That doesn't let them send to the browser: a push service accepts a message only with a VAPID signature from the key the subscription was made with, which only the Worker holds. `auth` would let a reader decrypt a message only if they could also read the push service's traffic, and a message holds only a repo name, a branch or item title and a link, which any reader of the repo sees anyway. The endpoint itself is an opaque URL at the browser vendor's push service.
- Before sending, the Worker asks GitHub, with the installation's token, whether the subscriber's login can still read the repo, so losing access stops the notifications without anyone editing the ledger.
- The Worker accepts a subscription only for an endpoint on a known push service (Google's, Mozilla's, Apple's, Microsoft's), so it never sends signed requests to an address someone made up.
- The VAPID keys are Worker secrets the operator sets once. Without them the Worker answers no key, the app hides its switch and nothing is sent, so the code builds, tests and deploys before they exist.
- A subscription the browser dropped answers 404 or 410. The Worker writes nothing then: only the person's own switch, or their app noticing the browser renewed its subscription, unsubscribes it.
- An older `clm` doesn't know the two kinds and skips their lines with a warning (`ledger`'s fold), which loses nothing it reads.

Alternatives considered:
- KV, with the app posting its subscription again on each load. Deleting the cache would silently stop someone's notifications until they next opened the app, so it isn't a cache that can be deleted with nothing lost.
- Amending `git-is-truth` to allow per-person state outside git. A change to an invariant for a few records the ledger already fits.
- Notifying only while a page is open, through the live socket and the Notification API. No push service or keys, but it misses the point: telling someone who isn't looking.
- Payload-less pushes, with the service worker fetching what happened. Browsers require every push to show a notification, and without a payload the service worker doesn't know which repo or event it is for.
