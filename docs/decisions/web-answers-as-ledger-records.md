---
title: "An answer given in the web app is an `answered` ledger record, offered only for the answers an item lists"
date: 2026-11-12
status: accepted
---

A reviewer on the move wants to settle the backlog's `question` and `human-task` items from a phone (`20261112-web-on-the-move`), but answering most questions needs a dialogue with an agent, which the web app doesn't hold. So the app offers an answer only where a tap is the whole answer: a `question` item that lists the answers it accepts under `choices` (two to five short strings), and a `human-task` item, whose answer is `done`. Any other `question` is shown in full and marked to be answered in a session.

The answer is written as approvals and proposals are (`per-writer-ledgers`, `web-proposals-as-ledger-records`): an `answered` record, `item` and `answer`, appended to the person's web ledger with their token. The Worker refuses an answer the item doesn't offer. A record is open while its item is in the backlog; a person's later answer to the same item replaces their earlier one, so a mistaken tap is corrected by tapping again. Nothing files or deletes anything: `clm triage` prints each open answer under its item, and the `triage` skill takes it as the human's answer, recording it in `.clm/answers.md` and drafting the item, as it does an answer given in its session. The draft's change resolves the item, which closes the record.

- `choices` is written by whoever files the question (a build's or a draft's session), when a short answer settles it. A question without them stays a session's.
- `done` on a `human-task` says the person did the step; the draft that takes it up deletes the item and releases what it blocks.
- An answer gates nothing: it is a person's word to the next triage, which still drafts a change that lands as any other. So it is accepted in a signed repository as it is, where Approve is off, since only an approval decides a land (`signed-approvals`).

Rejected:
- **Free-text answers from the app.** A typed answer usually needs a follow-up question, which only a session can ask; it would turn the app into a slower chat.
- **The Worker deletes the item or edits its file.** The app can't land (`land-locally`), and an answered item still needs a draft to act on what the answer means.
- **A choice list kept by the app.** A generic yes/no on every question invites answers the question didn't ask for; the item's author knows which answers settle it.
