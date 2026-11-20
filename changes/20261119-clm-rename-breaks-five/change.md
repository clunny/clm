---
status: applied
resolves: [clm-rename-breaks-five-of-its-own-requirements-at-the-edges]
---

## Intent

`clm rename` does what its requirements say at six edges the second builder check of wave 1 found by reading the code (`backlog/clm-rename-breaks-five-of-its-own-requirements-at-the-edges`): a failure after the apply leaves the tree as found, a marker under `changes/archive/` or `docs/decisions/` is rewritten while nothing else there is, `successor` is no reference while a block-style list's items are, a flag's version is found however its value is written, another change's delta for a component only the rename creates is left alone, and a line of fenced code is never a heading. The requirements where the code and the text part, or the text is silent or contradicts itself, gain the words and a scenario each, so `rename`'s build, which every one of them already waits on as `behind`, fixes each with a test that proves it.

## Problem

Each read from the code on 2026-11-19, none run:

- **A failure after the apply.** `crates/clm-rename/src/build.rs` undoes only its own writes, not the apply's, and runs the archive outside the undo. Already fixed in the spec: `rename/undo-on-failure`, added by `20261119-an-i-o-failure`, states the undo "up to and including the archive", and waits on `apply/journal` as `unbuilt`. This change adds nothing for it.
- **Markers under the archive.** The marker rewrite edits every file `check.coverage`'s `scan` reads, non-Markdown files under `changes/archive/` and `docs/decisions/` among them, where `references` says it "SHALL edit no file under" them and `marker-rewrite` says it rewrites the markers in "every file the coverage reads markers from". The two requirements contradict each other there, so no code satisfies both: a spec change (`code-fix-needs-a-written-rule`).
- **`successor` and block-style lists.** The frontmatter edit treats `successor`, which holds a change ID, as a reference, against "it SHALL rewrite nothing else"; and it reads only `key: value` lines, so a list in block style (`depends_on:` then `  - a`) keeps its old IDs, against "whole values in `depends_on`…".
- **A flag written another way.** `flag-rerecord` matches the text `requirement: <id>@<version>` literally, so `requirement: "a/x@…"` or a second space keeps the old version and the flag goes stale.
- **A component only the rename creates.** `rebase_others` rebases a delta for `b` once `a` is renamed to `b`, where `other-deltas` rebases only an element whose base the graph before the rename accepted, and the Design says so of the component.
- **A heading in a fence.** `text.rs`' `body` reads a fenced line starting with `#` as a heading, so it is never swapped, and a fenced `#### Needs` turns the lines after the fence into ID lines, never swapped either. `term-swap` both leaves alone "a line that starts with `#`" and swaps a fenced line "as on any other line", so the fenced `#` line is the text's own contradiction.

## Approaches

- **Taken: one spec change, a scenario per edge.** The item is one, and its second and sixth bullets need the text changed, so the item can't go to a code fix branch. The other edges are stated by the text already, but only in general words a build has read past once: a scenario each names the edge, and the build of `rename`, already pending on every requirement this change touches, writes the test that proves it.
- **Rejected for the archive markers: keep `references`' clause whole and leave those markers.** A marker left citing an old ID names no requirement, and `check/dangling-markers` judges every file `scan` reads, so the rename branch would fail `clm check` over a file its author never touched. A marker is a claim the coverage counts, not history's prose, so `marker-rewrite` wins and `references`' clause yields to it.
- **Rejected for the fenced `#` line: keep it unswapped, as the term-swap clause reads literally.** A heading is left alone because it holds a slug only a `requirement` pair renames; a fenced line holds an example, whose words the Design already swaps "since a word is the same word in an example". A `# comment` in a fence is such a word.
- **Rejected: a code fix branch for the edges the text already states, beside this change.** One item can't be both resolved by a commit's trailer and by this change, and `rename` is being rebuilt wholesale for wave 1 anyway; a second branch on its code would only race that build.

## Outcome

- `rename/refusals` (modified): a component whose `depends_on` or `serves` value spans several lines and names a renamed ID is refused, naming the component and key, since its delta can't modify that value (`apply/frontmatter`). Scenario *a block-style list in a component*.
- `rename/references` (modified): a list's items are values in flow style, and in a backlog item, which `rename` writes itself, in block style too, a quoted value is rewritten keeping its quotes, no other key (`successor`) is rewritten, and a marker under `changes/archive/` or `docs/decisions/` is the one edit made there. Scenarios *a list in block style*, *a key that holds no reference*, *a marker in the archive*.
- `rename/term-swap` (modified): a heading and a block's ID lines are judged outside fenced code only; a fenced line starting with `#` is swapped and starts or ends no block. Scenario *a heading in fenced code*.
- `rename/flag-rerecord` (modified): the value is found as the frontmatter parse reads it, quoted or bare, however spaced, and its quotes kept. Scenario *a quoted flag*.
- `rename/other-deltas` (modified): never an element of a component the graph before the rename doesn't hold. Scenario *a component only the rename creates*.
- `rename`'s Design: **What a reference is**, **Terms**, **Markers** and **Other changes' deltas** say the same.
- `docs/decisions/rename-by-replay.md`: its **Scope** keeps archived changes and decisions as history but for a test marker in one of their non-Markdown files, which is rewritten (architecture audit).

## Constraints

- No scenario is renamed or dropped, and no requirement removed.
- `marker-rewrite`'s text is unchanged: it already rewrites every marker the coverage reads.

## Edge cases and workarounds

- A quoted value in `depends_on` or `serves` is rewritten keeping its quotes, as a flag's is.
- A fence opened inside a `#### Needs` block holds no ID line; the block's lines after the fence closes are still ID lines until the next heading outside fenced code.
- No spec on `main` holds a block-style `depends_on` or `serves`, so the refusal costs nothing today; teaching `apply/frontmatter` multi-line values would be a larger change for no current case (`yagni`).
- *a marker in the archive* passes against today's code: it settles the two requirements' contradiction in the text, not a code fault.
- A marker in a Markdown file under `changes/archive/` isn't one the coverage counts, so it is left, as every Markdown marker is.

## What the requirements no longer state

- `rename/references`: "it SHALL edit no file under `changes/archive/` or `docs/decisions/`" now ends "but to rewrite a marker there as `marker-rewrite` does".
- `rename/term-swap`: "It SHALL leave alone every heading, a line that starts with `#`" now leaves alone such a line only outside fenced code; a fenced one is swapped.

## Open questions

None left. The builder check's findings were settled in the change: a block-style list in a component's frontmatter can't pass through a delta, so it is a refusal and the block-style scenario is a backlog item's; *a component only the rename creates* now gives `a` a Design the rename rewrites, so the test fails against today's code; *a marker in the archive* states what `clm-rename`'s own tests can observe, since `clm-check` depends on it through `clm-gate`; `flag-rerecord` names the line it edits. The architecture audit's finding (`rename-by-replay`'s **Scope**) is fixed by amending the decision.
