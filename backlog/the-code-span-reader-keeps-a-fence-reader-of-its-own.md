---
title: "The code-span reader keeps a fence reader of its own beside the model's"
kind: deferred
components: [check]
origin: 20261121-the-model-s-fence
---

**The problem.** `20261121-the-model-s-fence` gives `model/code-fences` the CommonMark fence reading that `clm_check::code_spans` (`crates/clm-check/src/spans.rs`, its `fence` and `paragraphs`) already follows, so the two now agree, but by two copies of one rule (`reuse-first`). A later edit to either one brings back the disagreement the change fixed, with nothing to catch it but each side's own tests.

**The work.** Once `model/code-fences` is current, `spans.rs` reads which lines are fenced through `clm-model`'s `outline` (`fence_mask`, or a per-line reader the model exposes beside it) and drops its own `fence`. No requirement or scenario changes: `check/asset-references` reads the same spans before and after.
