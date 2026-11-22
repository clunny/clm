---
title: "clm rename reads code fences by a toggle of its own, which disagrees with the model's"
kind: bug
components: [rename]
---

Step: drafting `20261121-the-model-s-fence`, which gives the model one fence rule (`model/code-fences`) for every reader of a spec.

What happens: `crates/clm-rename/src/text.rs`'s `body` flips `fenced` on every line starting ```` ``` ```` or `~~~`, whichever marker opened the fence, at any length and whatever follows it. So a `~~~` line inside a backtick fence, a ```` ``` ```` line inside a ```` ```` ```` fence, a ```` ```rust ```` line inside a fence, or a statement line starting ```` ```x``` ```` each flips it where the model's outline doesn't. Past that line, rename reads lines the model takes as fenced as headings and `Needs`/`Serves` ID lines, rewriting them as references, and swaps terms in headings the model reads as fenced text, or the reverse. The fix reads fences through `clm-model`'s `outline::fence_mask`; whether a rename requirement must first state that (`code-fix-needs-a-written-rule`) is the draft's to judge.
