---
status: done
resolves: [a-defect-report-naming-a-component-that-no-longer-exists-stops]
---

## Intent

Accept the behaviour the bug `a-defect-report-naming-a-component-that-no-longer-exists-stops` reports, delete the item, and design nothing. No spec, code or skill text changes.

What the item asks for already holds: `clm defect` refuses a component a backlog item may not name. `plugin.hooks/defect-backlog` states it ("a component a backlog item may not name ... it SHALL write nothing and exit 1"), its scenario *unknown node* proves it, and `crates/clm-hooks/src/defect.rs` checks each `--component` with `Graph::backlog_node_known` before it writes. So a report can't be written naming a component the checkout's specs and live changes don't hold.

The report in the item was valid when written: its component existed in that checkout, and a change that removed it reached the checkout afterwards, while the report was still an uncommitted file there. No check of `clm defect`'s can foresee that, and dropping or rewriting the item's `components` when another change lands is not something any command does to a file it doesn't own.

What follows is the model working as designed. An item naming a component that doesn't exist breaks `model/parse-backlog`, the same as an unknown `origin` or a `blocks` entry naming no requirement, and every command that loads the graph stops on it rather than show a graph it can't trust. The message names the file, the rule and the component, for example ``backlog/<slug>.md: model/parse-backlog: unknown component `<id>` ``, so the session or person who reads it knows which file to edit and how. Making the model load past such an item would weaken the guarantee every committed item gets from `clm check`, for one edge case (an uncommitted report outliving its component) that a one-line edit ends. Under `yagni` and `redesign-before-workaround`, that cost is not worth paying.

## Edge cases and workarounds

None.
