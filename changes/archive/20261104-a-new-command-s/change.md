---
status: done
resolves:
  - a-new-command-s-glossary-entry-can-t-ride-on-its-code-branch
---

# A new command's glossary entry rides with its requirement

## Intent

A requirement that adds a `clm` command, or any other term `clm/glossary-covers-terms` covers, should be built and landed on one code branch. The spec branch that adds the requirement adds the term's glossary entry too: the `propose` skill and the prompt `clm draft` writes say so. The gate stays as it is: a code branch still can't carry `docs/guide/glossary.md`.

## Problem

`clm/glossary-covers-terms` fails the land's checks when a top-level command, requirement state, component status, change status, backlog kind or `change.md` key is missing from `docs/guide/glossary.md`. The glossary is under `docs/`, a spec path, so a build that adds the command and its entry on `impl/*` makes `clm gate` print `mixed`. The build had to land the entry on a branch of its own first: two branches, two lands and an extra check run for one requirement.

## Approaches

1. **The spec branch adds the entry** (chosen). The term is decided, and named, by the requirement that adds it, so its definition belongs on the same branch. The glossary test only fails on a missing entry, never on an extra one, so an entry that lands before its code passes the checks, and the code branch then lands alone. It needs only two sentences of instruction, in the two places that design changes (the `propose` skill and the draft prompt), and no new mechanism (`reuse-first`, `yagni`).
2. **Let a code branch carry glossary entries.** The gate would exempt additions to `docs/guide/glossary.md` from the spec-path count, as it exempts added backlog items. Rejected: it is a second exemption in the classification for one file, it would have to tell added entries from edited ones, and a definition is words a human reads, which the spec branch is where they get written. The term's meaning would also be first written by the build, after the design was settled.

## Outcome

- `plugin/propose-adds-glossary-terms`: `skills/propose/SKILL.md` tells the session to add the entry on the spec branch.
- `orchestrator.triage/draft-adds-glossary-terms`: the draft prompt says the same in its design paragraph.

## Edge cases

- A requirement already on `main` that adds a term without its entry, or a build that introduces a term no requirement named: the build still lands the entry on a spec branch of its own, as today. That should become rare once the designers add entries; it needs no rule.
- A glossary entry for a command that a later change drops before it's built stays in the glossary until someone removes it; the glossary test doesn't flag extra entries, and `clm/retired-terms` covers only renames.

## Constraints

`plain-terms`, `yes-no-review`, `guide-in-the-binary`, `reuse-first`, `yagni`.
