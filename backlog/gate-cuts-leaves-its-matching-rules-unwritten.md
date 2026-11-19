---
title: "gate/cuts leaves how it reads a sentence and what loading means unwritten"
kind: question
components: [gate]
choices:
  - "Write the build's choices in as they are"
  - "Also match shall in any case"
  - "Match the phrases as substrings"
---

The build of `gate/cuts` made choices the requirement's text leaves open. They are in the code and some in tests, but no sentence keeps them, so a later rewrite can drop them unseen (`code-fix-needs-a-written-rule`):

- **The word `SHALL` is matched case-sensitively, the placement phrases without regard to case.** The text's "compared without regard to case" may cover `SHALL` too; then a lost sentence saying `shall` is a cut the gate misses.
- **Placement phrases are matched as whole words.** The text says a sentence "holds" them; as substrings, "is left intact" would hold `is left in` and lose its count.
- **A sentence ends at a `.`, `?` or `!` that any whitespace follows**, a newline included; the text says "a space".
- **"The specs don't load" means any load violation** at either side, from the human layers and the live `change.md` files too, not only one under `specs/`. It fails closed.
- **The graphs hold `ARCHITECTURE.md` and `CAPABILITIES.md`** (`backlog/workaround-gate-cuts-1`): the Design says they load as the code-fix check's, which reads neither.

A spec branch that settles them edits `gate/cuts` and the Design's **Cuts** paragraph; no requirement waits on it.
