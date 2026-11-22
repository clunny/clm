---
status: applied
resolves: [the-model-s-fence-reading-and-the-code-span-reader-s-disagree]
---

## Intent

One fence rule for every reader of a spec: the model's outline reads code fences as CommonMark does, as `check/asset-references`' code-span reader already does, so a requirement's statement (`gate/cuts`, `check.health/rule-signals`) ends where the code spans read in it end.

## Problem

Two readers decide which lines of a spec are fenced. `clm_model::outline::fence_mask`, which every model parse, `gate/cuts` and `check.health` use, opens a fence on any line starting with three backticks or tildes and closes it on the next line starting with the same three. `clm_check::code_spans` follows CommonMark: a backtick run followed by another backtick on its line opens no fence, and a fence closes only on a bare run of its character at least as long as the opening one. Where they differ, `outside-contract` reads a statement that isn't the one its spans come from: a statement line ```` ```x``` ```` opens a fence for the outline and runs the statement on through the scenarios, and a ```` ``` ```` or ```` ```rust ```` line inside a ```` ```` ```` fence closes it early, ending the statement at a `####` line still inside the fence.

No requirement states what a fence is: the model's requirements say only that a heading inside a code fence starts nothing. The fix is a rule, so it is a requirement of `model`, which owns the outline every reader takes its fences from (`code-fix-needs-a-written-rule`).

## Approaches

- **Taken: the model follows CommonMark, as the code-span reader does.** The code-span reader's rule is the right one (it is how every Markdown renderer shows the spec to a human), and it is already proven by `check/asset-references`' scenarios. `fence_mask` is the one reader `gate/cuts`, `check.health` and every model parse share, so changing it there moves them all at once, with no requirement of theirs to edit: each already says "outside a fence" and takes the fence from the model. Every Markdown file under `specs/`, `changes/` and `docs/`, `ARCHITECTURE.md` and `CAPABILITIES.md` was read both ways while drafting: no line's fencing differs, so no component's sections, requirements or hashes move.
- **Not taken: the code-span reader follows the outline's simpler rule.** It would make the two agree by making `check/asset-references` wrong against what a reader sees rendered, and undo scenarios that already hold.
- **Not taken: `check.health` reads its statements by the code-span reader's fences.** It fixes the one reported symptom and leaves `gate/cuts` and the model's own parses on a rule a renderer disagrees with.

## Outcome

`model/code-fences` states the fence rule and that `outline::fence_mask` marks by it, with scenarios for each case the item names: a ```` ```x``` ```` line opens nothing, a fence closes only on a bare run of its character at least as long, a tilde fence closes only on tildes, an unclosed fence runs to the end, and a heading inside a longer fence starts no requirement. The model's Contract names the requirement beside `outline`. Its build changes `fence_mask` alone; `gate/cuts` and `check.health/rule-signals` read the corrected fences through it unchanged.

`clm rename` holds a third reading: `rename/references` states that a fence runs from any line starting ```` ``` ```` or `~~~` to the next such line, and its code flips a fence on each. `rename/references` now reads fenced code as `model/code-fences` reads fences, with its scenario `a reference in fenced code` given a four-backtick fence holding ```` ``` ```` and `~~~` lines, and a `#### Needs` on `model/code-fences`; `term-swap` reads fenced code "as `references` gives it", so it follows unchanged. Its build reads fences through `outline::fence_mask`.

One piece of work stays out: `clm_check::code_spans` keeps its own copy of the rule, now the same as the model's, until it reads fences through the model's outline (`the-code-span-reader-keeps-a-fence-reader-of-its-own`, deferred).

## Constraints

- The rule is CommonMark's as far as a spec's text needs it: indentation before a fence marker is not limited to three spaces, as neither reader limits it now, and a fence inside a list item or block quote is read as any other line.
- No edge cases or workarounds are known.
- `outline::yaml_fence`, which takes a frontmatter section's ```` ```yaml ```` block out of a delta, keeps its own exact-line reading: it reads a block's content, not which lines are fenced, and `docs/spec-format.md` fixes that block's shape.

## Open questions

- A rename's fenced-code clause now counts a fence's own marker lines as fenced code, which the old wording left out; a marker line holds no code span, so no reference reads differently, and `term-swap` now swaps a term in a fence's info string as in its body.
