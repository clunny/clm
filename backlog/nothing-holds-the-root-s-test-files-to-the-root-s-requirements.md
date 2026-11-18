---
title: "Nothing holds the root's test files to the root's requirements"
kind: deferred
components: [clm, plugin]
origin: 20261115-builds-change-only-their-own-files
---

Once `clm/routes-only` and `gate/own-files` are current, the top-level test files of `crates/clm/tests/` cite the root's requirements only, and `clm land` refuses any other build that changes them. But no check stops the root's own build from writing a test there that cites another component, and the guide, a skill and several Designs still describe the clause or the old place of the tests.

Due when `clm pending` lists neither `clm/routes-only` nor `gate/own-files`. Then:

- **A spec branch** sets `tests: ["crates/clm/tests/*.rs"]` in the root's frontmatter, so `check/tests-cite-their-owner` judges those files, and gives the four suite directories (`ci_suite`, `plugin_suite`, `plugin_agents_suite`, `plugin_panel_suite`) and `crates/clm/tests/capability/**` to their components as `tests` globs the same way. It also takes the exception "A test that cites the component" out of `docs/guide/how-it-works.md`, saying there that a project whose tests share files gives them to their components with `tests` globs before it names its `wiring`; rewrites the root's Design paragraphs that start "Until `routes-only` is built" and "Three cases need more than the helper", which describe a move that is over; and rewords each Design and requirement that still places a component's tests under `crates/clm/tests/` (`git grep -n 'crates/clm/tests/' -- specs` lists them: `check`, `check.unreferenced`, `ci`, `config`, `init`, `plugin`, `plugin.agents`, `plugin.hooks`, `plugin.hooks.guard`, `plugin.panel`, `rename`, `view.repo` and three requirements of `upgrade` on the day this was filed).
- **A code branch of `plugin`'s** takes the same sentence out of `plugin/skills/propose/SKILL.md` ("a fix to a component's test kept under another component's `paths` claims the component the test cites"), resolving this item.

**Also for this branch (2026-11-17, `20261117-tests-sit-with-the-component-they-cite`).** The Designs of `land`, `land.approval`, `web`, `web.views` and `web.worker` each hold a sentence starting "Until" that names `changes/20261117-tests-sit-with-the-component-they-cite/moved-tests.txt`. Take those sentences out once that change is archived, and reword the guide's bullet "A test that cites the component" with the exception it loses.
