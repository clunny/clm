---
title: "The model's fence reading and the code-span reader's disagree, so a statement can end where its spans don't"
kind: bug
components: [model, check, check.health]
---

Step: building `check.health/rule-signals` at 286d24c2. The code review found this.

What happens: two readers decide which lines of a spec are fenced. `clm_model::outline::fence_mask`, which `gate/cuts` and `check.health` use to find where a requirement's statement ends (its first `####` line outside a fence), opens a fence on any line starting with three backticks and closes it on any such line. `clm_check::code_spans` (`check/asset-references`) follows CommonMark: a backtick fence whose info string holds a backtick opens none, and a fence closes only on a bare fence line of the same marker at least as long.

Where they disagree, `outside-contract` reads text that isn't its statement, or misses some of it:
- A statement line starting ```` ```x``` ```` opens a fence for `fence_mask` but not for `code_spans`, so the statement runs on through `#### Scenario:` lines and a scenario's code names are reported as the requirement's.
- A ```` ```` ```` fence holding a ```` ``` ```` line, or a ```` ```rust ```` line inside a fence, closes the fence early for `fence_mask`, so a `####` line still inside the fence ends the statement and the names after it go unread.

`rule-signals` states both readings as written: the statement is the one `gate/cuts` reads, and its spans are read as `check/asset-references` reads them. So the fix is a spec choice of one fence rule for both, likely `fence_mask` following CommonMark, which moves `model`'s outline and every reader of it (`gate/cuts` included).
