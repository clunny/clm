---
title: "An idea is elaborated in its own backlog item, landed on main once per session"
date: 2026-11-01
status: accepted
---

A design that takes more than one session had nowhere shared to live between sessions. `propose` writes `change.md` only once the human agrees to the design, so a brainstorm that stopped halfway survived only in the conversation, or in one machine's agent memory, which no teammate and no other clone sees (`git-is-truth`).

So an idea is elaborated in its own `backlog/<slug>.md` item, `kind: idea`. Its body grows with the design under four sections: `## Intent`, `## Settled` (choices made, with reasons), `## Approaches` (options still open) and `## Open questions` (where the next session starts). During a session, `propose` commits the edits on a branch of its own, `spec/<YYYYMMDD>-idea-<slug>`, pushed as it goes so nothing is lost if the session dies, and lands that branch once, at the end of the session or when the human asks for a checkpoint, before `clm checkpoint` (`checkpoint-before-done`). The branch edits only `backlog/`, so it lands on checks (`yes-no-review`). An unlanded one stays on `origin` and the next session resumes it. Once the human agrees to the whole design, the idea becomes a change as any other (`from: backlog/<slug>`), seeded from the item's sections.

After landing, the session deletes its claim, `propose/<slug>`, and the branch's `autosync/spec/<id>` snapshot itself, since `clm checkpoint` deletes them only once the item leaves the backlog or a change names it, which an elaborated idea doesn't. The state of every idea is therefore on `main`, visible to `clm backlog`, `clm triage` and the web, and a paused idea holds no branch and no claim. An unlanded idea branch is found by its name, `spec/*-idea-<slug>`, since no change on it shows in `clm proposals`.

Rejected:
- **A long-lived spec branch holding a `proposed` change.** Its state is visible only on the branch, it drifts from `main`, and it holds its items for as long as it lives.
- **A half-designed change on `main`.** A change on `main` reads as applied work ready to build (`clm triage`'s `main` lines, `clm pending`); a draft status would have every command learn it.
- **Landing at every pause.** A commit on `main` per pause adds noise for no gain: the pushed branch already keeps the work safe until the session ends.
- **Agent memory.** Local to one machine and one tool.
