---
title: "A person's web subscriptions are the records of their own web ledger branch"
date: 2026-10-28
status: accepted
---

`web.worker` chose notification recipients from the whole ledger fold's live subscriptions "whose `writer` is `<login>/web`", and answered the app's switch the same way. A record's `writer` is a claim (`per-writer-ledgers`): anyone with push access could commit, to a ledger branch of their own, a `subscribed` record naming another person's login and their own endpoint, and keep receiving that repo's notifications after losing access, for as long as the named person keeps theirs; or an `unsubscribed` record naming someone else's subscription, and silence them. The permission check before each send asks GitHub about the login the record names, so it didn't help.

A person's *web subscriptions* are now the live subscriptions of `clm/ledger/<login>/web` folded alone: the branch names the person, the `writer` field is not consulted, and only that branch's `unsubscribed` records end its subscriptions. `push-subscriptions-in-ledger` already put each person's subscription in their own web ledger, and the Worker's `POST` and `DELETE` already decided what to write from that branch alone, so this is the reading side catching up with the writing side rather than a new place for the records. The permission check stays: it is what stops notifications to someone who lost access.

What this doesn't do: it doesn't stop someone with push access from committing to `clm/ledger/alice/web` itself, which git lets any pusher do. That is the trust the ledger gives every writer, which `signed-ledger-records` tightens for approvals and lands where it matters; a subscription moves no decision, and a forged one there is noticed by its owner when a notification they turned off arrives, or by `clm ledger show`. Signing subscriptions, or a host-side rule for the branches, is not worth a key touch per switch.

Rejected: having `ledger`'s fold count an `unsubscribed` record only when its writer is the subscription's. It closes the silencing half in `clm ledger show` too, but leaves the receiving half as it was, needs a Contract change in `ledger` for a rule only `web.worker` acts on, and still trusts `writer`, the field the forgery sets.
