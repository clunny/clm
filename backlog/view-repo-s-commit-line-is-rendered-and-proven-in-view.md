---
title: "view.repo states how a commit line renders, which view's code does and view's file proves"
kind: deferred
components: [view.repo, view]
blocks: [gate/own-files]
---

`view.repo/view-at-rev` states, beside what `clm view --at <rev>` reads, how the line naming the commit is rendered: `<rev> <hash>`, ` · fetched <n>m ago` under an hour and `<n>h` from then on, ` · checkout <branch>` or `detached`, never a `⚠`, the same words in the SVG, and an `at` object in the JSON with no `checkout`. That rendering is `clm-view`'s code (`At` and `render`), and `view`'s spec doesn't state it.

One test proves it: `the_commit_line_heads_every_chart_at_a_commit` in `view`'s file `crates/clm-view/tests/views.rs`, which calls `clm_view::render` and cites only `view.repo/view-at-rev`. `some-tests-prove-a-component-from-another-component-s-files` asked for it to move to `view.repo`'s files. It was left unchanged: what it asserts is `view`'s code, and `view.repo`'s own seven tests of the requirement go through its crate and don't cover the ages, the SVG or the JSON's absent fields.

**What settles it** is a spec change, by the human's answer of 2026-11-17 for a mixed scenario (`commands-live-in-their-crates`): where a component's half is something its crate offers and nothing states, its own requirement states that half. So `view` gains the statement of how a commit line renders from an `At` it is given (a requirement, or a scenario of `view-command`), the test is re-cited to it where it stands, and `view.repo/view-at-rev` keeps what the command reads and names `view` for the line's form.

It can be proposed with `twenty-view-tests-show-what-only-the-commands-do`, which moves clauses the other way between the same two components. The change that carries it out names this item in `from`.
