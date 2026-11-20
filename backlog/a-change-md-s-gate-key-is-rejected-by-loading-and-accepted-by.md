---
title: "A change.md's gate key is rejected by loading and accepted by the one-record parser"
kind: bug
components: [model]
origin: 20261119-wave-1-by-the-rule
---

Loading rejects the key `gate` on a live change (`model/parse-changes`). The model's parse of one `change.md`'s text alone, which the gate, the land and the web review call, accepts `gate` on any change. So the two readers of one file disagree, and a caller of the one-record parser never sees the refusal.

Expected: one rule, or a requirement that says why they differ. Wave 1 of phase 5 states what each does today.
