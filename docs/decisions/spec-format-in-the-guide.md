---
title: "The spec format joins the guide as a page, staying at docs/spec-format.md"
date: 2026-10-27
status: accepted
---

`method-in-the-plugin` makes the guide, printed by `clm guide`, the one place an adopter learns the method, and an adopter's sessions write the spec format: nodes, requirements, scenarios, deltas, markers. Yet the format lived only in `docs/spec-format.md`, outside `docs/guide/`, so `clm guide` didn't print it and the guide's links to it led nowhere in an adopter's project. The guide also linked CLM's own `docs/decisions/` and `ARCHITECTURE.md`, which an adopter's project doesn't have.

`docs/spec-format.md` stays where it is and joins the guide as the page `spec-format`: `clm guide spec-format` prints it, and the public site serves it at `/guide/spec-format` beside the `docs/guide/` pages (`guide-from-docs`, `web.site`); the web app bundles no guide. Guide pages keep linking it as `../spec-format.md`, a link that works on GitHub, on the public site and, read as `clm guide spec-format`, in a session. No guide link leads outside the guide: a relative link resolves to a page's file, and anything else is an absolute URL or an anchor (`init/guide-links-inside`, checked by a test). CLM's own decisions and `ARCHITECTURE.md` are named in prose as CLM's repository, not linked.

Rejected:

- **Move the file to `docs/guide/spec-format.md`.** Adding a file to `docs/guide/` is a spec-branch edit, while the page table that `clm guide` compiles is code: the spec branch fails `clm-init`'s test that every page is in the table, and a code branch naming a page that doesn't exist yet doesn't compile, so the move can't land as either. It would also break the references to `docs/spec-format.md` in `CLAUDE.md`, `README.md`, `model`'s Contract and several decisions, which would all need editing for no gain to a reader.
- **Absolute links to CLM's repository on GitHub** for the decisions and `ARCHITECTURE.md`. The repository isn't public to adopters yet (`binary-from-releases`), so the link would lead nowhere for them too; prose that says where the files live stays true either way.
- **A separate, shorter format page for adopters.** Two copies of the format drift; the one file is the contract of `model` and already reads as a reference.
