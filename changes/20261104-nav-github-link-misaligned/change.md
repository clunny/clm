---
status: applied
priority: P3
resolves: [nav-github-link-misaligned]
---

# The nav bar's GitHub link keeps its icon beside the star count

## Intent

The GitHub link in the site's nav shows GitHub's mark on its own line above the star count, so the mark pokes above the bar and the count sits below the other links. The link should show the mark beside the count on one line, centred with the links around it, as the homepage's GitHub button means to. This change adds that to `web.site/github-link` with a scenario a browser test measures, so the fix is proven and can't regress; the build fixes the CSS.

## Problem

`web.site/github-link` states where the links go and what they read, never how the mark and count sit, so the bug isn't a breach of any written rule and a code-only fix would have no requirement to cite (`code-fix-needs-a-written-rule`). The cause is in the theme: VitePress's base styles set `svg { display: block }`, so the `.gh-icon` rule's `vertical-align` and `margin-right` do nothing and the SVG takes a line of its own. The homepage button's SVG carries the same class and may break the same way.

## Approaches

1. **Add the layout to `github-link`, with a measured scenario (chosen).** One sentence and one scenario on the existing requirement: the mark before the label on one line, centres within 2px, and the nav count centred with the Guide link. A Playwright test reads bounding boxes, which catches the stacked layout whatever CSS causes it.
2. **A screenshot comparison.** Catches more, but is brittle across fonts and platforms and needs baselines kept up; more than a P3 alignment bug earns (`yagni`).
3. **Fix the CSS with no rule.** Not allowed: nothing written would state what the test proves (`code-fix-needs-a-written-rule`).

The design isn't wrong: the link is meant to be an icon and a count, and the fix is a theme style (for example `display: inline-block` on `.gh-icon`, or an inline flex row with `align-items: center` on the link), the builder's choice.

## Outcome

`github-link` gains a sentence and the scenario "the mark sits beside the count". Its version moves, so its existing tests show `behind` until the build re-cites them.

## Constraints and edge cases

- The fallback label `GitHub` (no count) gets the same layout; the scenario tests the count case only, since the markup is the same.
- At phone widths VitePress moves nav links into its menu; the scenario measures at 1280×800 only.

## Open questions

None.
