---
title: "A scenario whose outcome changes under its name is a cut"
date: 2026-11-18
status: accepted
---

`cuts-are-what-a-program-lists` had the gate list a scenario only when its name was gone, and left "a reword that keeps the count of sentences" to the reader. The draft of `20261118-renaming-a-citing-test` met the gap: it reversed the outcome of two scenarios named *a renamed test*, one from "returns `tests/t.rs::two`" to "returns nothing" and one from "exits 1" to "lands", so the land stopped refusing what it had refused. No name was gone and no `SHALL` sentence was lost, so `clm gate` printed `spec`, and an unattended draft, which has no human to ask and is told to land a `spec` branch on checks, had no way to ask for the yes `removal-needs-a-yes` wants. It left the change for review against the gate's line.

**What the gate lists** (`gate/cuts`). A scenario of a requirement held on both sides, under the same name there, whose outcome differs: its lines from the first `- THEN` to the end of its block, whitespace read as one space. The line is `scenario "<name>" of <id> changes its outcome`, and the branch needs review. The n-th scenario of a name is compared with the n-th of that name at the head, since a requirement may hold two of one name.

**Why the outcome and not the whole scenario.** A scenario's outcome is what it promises. A changed setting or action (`GIVEN`, `WHEN`) with the same outcome promises the same thing of another case, which a removal yes doesn't protect; a changed outcome promises something else of the same case, which is what reversing a scenario does. A program can't tell an outcome reworded from one reversed, as it can't tell a renamed scenario from a removed one, so both are listed and the human reads the line.

**What it costs.** Of the last 93 spec lands that edited `specs/`, 30 changed the outcome of a scenario under its name, so about one spec branch in three that edits specs comes to need review. The triage folds every change that needs review into one pull request with one approval, which bounds what each one costs the human.

**Rules version.** It makes more diffs need review, so `RULES` goes to 3 and version 2 keeps the answer without it (`rules-version`).

**What stays a reader's.** A reword of a statement that keeps the count of sentences; a scenario moved to another requirement whose outcome changes on the way, since a move is matched by name and count, not one scenario to another; a sentence of behaviour worded like one that places a test; whether a merged scenario states what two did.

Considered:
- **A key in `change.md` a session sets to ask for review of a relaxation the gate can't see.** Rejected: it rests on the session noticing, which is the failure met; the draft noticed only through the architecture audit. A program can see a changed outcome in every branch.
- **Listing only an outcome that loses a line or a word.** Rejected: "exits 1" to "lands" loses nothing countable.
- **Comparing the whole scenario.** Rejected: a changed setting under the same outcome drops nothing, and would add reviews for no yes.
