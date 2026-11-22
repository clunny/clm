---
status: done
resolves: [a-line-reusing-a-request-s-id-with-an-earlier-time-removes-the-request-from-the-fold]
---

## Intent

Accept the behaviour `backlog/a-line-reusing-a-request-s-id-with-an-earlier-time-removes-the-request-from-the-fold` reports, and delete the item: the change deletes one backlog item and changes no requirement, scenario or Design.

The item is right that a hand-written line reusing a `changes_requested` record's `id` with an earlier `at` makes the fold hold that line instead of the request (`ledger/effort-records`), so the request stops holding the land. But the only person who can write that line is one who can push a hand-written line to a `clm/ledger/*` ref on `origin`. A web user can't: the Worker writes their records with a fresh random `id`. Anyone who can push can already remove the request outright, with no `id` reuse, by committing to the requester's ledger branch a `records.jsonl` without that line. That is a fast-forward commit, the kind `clm ledger compact` pushes to the writer's own branch, so a host that blocks force pushes doesn't stop it, and nothing ties a ledger ref to the person who pushes it (`per-writer-ledgers`). The same holds for approvals, checks and lands. So the fix the item proposes, treating lines that share an `id` as one record only when they share a writer, takes no power away from anyone. It would also make little difference to who can do it: a line's `writer` is free text, so the attacker would only need to write the requester's name in it, and binding the choice to the ledger the line was read from still leaves that other route open. It would cost a Contract change, though: `Fold::lines`, keyed by `id`, can't hold two records with one `id`, and `land.approval` checks signatures through it.

What guards the records against someone who can push is the signatures and the audit (`signed-ledger-records`): in a signed repository an approval or a land counts only when its signature checks, and `clm ledger verify` reports a land whose approval or signature is missing, however it went missing. A change request withholds a land and grants nothing, so it is unsigned by design (`web-review-feedback-as-ledger-records`): any person with `write` on the repository "could already keep a branch from landing by other means", and by the same reasoning one who can push can already lift a hold. The fold's choice still does what it exists to do, which is the same answer in every clone and every read order. Random 128-bit `id`s rule out an accidental clash, and the item says so too.

## Problem

`ledger/effort-records` keeps, of differing lines that share an `id` and are not all `effort` counts, the least by `at`, `writer` and sorted text, so that a *later* line reusing a record's `id` never displaces it. A line with an *earlier* `at` does displace it. The item asks that a line never displace a record another writer wrote.

## Approaches

- **Accept it (taken).** No fold rule can protect one writer's record from someone who can push, since that person can rewrite the writer's ledger branch directly. A rule that only makes the attack longer is a workaround that guards nothing (`redesign-before-workaround`, `yagni`).
- **One record per writer and `id`.** Lines that share an `id` but name different writers become separate records. Not taken: `writer` is free text, so a line naming the requester still displaces the request; `Fold::lines` and its callers in `land.approval` and `ledger` would have to be re-keyed in expand-and-contract steps across two components; and the branch-rewrite route stays open.
- **One record per ledger ref and `id`.** Lines that share an `id` are one record only when read from ledgers of the same name. Not taken: same Contract cost, and a pusher can append the line to the requester's own ledger ref, or remove the request from it.
- **Sign change requests.** Not taken: `web-review-feedback-as-ledger-records` rejected refusing unsigned requests, since that would take away the one hold a phone can place. Signing would not stop a pusher from removing the record anyway, only from forging one.

## Outcome

- The backlog item is deleted. No requirement, Design, decision or code changes, so nothing is pending and no build follows.

## Constraints and known edge cases

- None.

## Open questions

- If a project ever wants requests to survive someone with push access, that needs the host to refuse pushes to another person's `clm/ledger/<person>/*` refs. CLM reads no host (`git-is-truth`), so that would be new intent for the human to propose, not a fold rule.
