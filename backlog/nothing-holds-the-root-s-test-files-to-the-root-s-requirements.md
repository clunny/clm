---
title: "Nothing holds the root's test files to the root's requirements"
kind: deferred
components: [clm, plugin]
origin: 20261115-builds-change-only-their-own-files
---

Once `clm/routes-only` and `gate/own-files` are current, the top-level test files of `crates/clm/tests/` cite the root's requirements only, and `clm land` refuses any other build that changes them. But no check stops the root's own build from writing a test there that cites another component, and two instructions still describe the clause that went.

Due when `clm pending` lists neither `clm/routes-only` nor `gate/own-files`. Then:

- **A spec branch** sets `tests: ["crates/clm/tests/*.rs"]` in the root's frontmatter, so `check/tests-cite-their-owner` judges those files, and gives the four suite directories (`ci_suite`, `plugin_suite`, `plugin_agents_suite`, `plugin_panel_suite`) and `crates/clm/tests/capability/**` to their components as `tests` globs the same way. It also rewrites the root's Design paragraph that starts "Until `routes-only` is built", which describes a move that is over, and the sentence in this repository's `AGENTS.md` that says a fix to a test kept under another component's `paths` claims the component the test cites.
- **A code branch of `plugin`'s** takes the same sentence out of `plugin/skills/propose/SKILL.md` ("a fix to a component's test kept under another component's `paths` claims the component the test cites"), resolving this item.
