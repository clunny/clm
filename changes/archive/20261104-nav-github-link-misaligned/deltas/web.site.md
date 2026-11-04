---
component: web.site
base: {}
---

## MODIFIED Requirements

### Requirement: github-link <!-- base: 37bdaa86c59602ae -->
The nav of every page, and the homepage's first screen, SHALL link to `https://github.com/<repo>`, `<repo>` being the install repository, labelled with the repository's star count read at build time from `GITHUB_STARS` or GitHub's API; when the count can't be read the build SHALL still succeed, showing the link without a count. Wherever the nav bar shows its links (not in the menu VitePress opens at narrow widths), each link SHALL show GitHub's mark beside its label on one line, the mark before the label, centred on the line with the nav's other links or, on the homepage, with the first screen's other buttons; the label `GitHub`, shown without a count, is laid out the same way.

#### Scenario: stars given
- GIVEN `GITHUB_STARS=1234` in the build's environment
- WHEN the site is built and `/` is opened in a 1280×800 window, not scrolled
- THEN a link to `https://github.com/clunny/clm` reading `1.2k` is in view on the first screen, and `/guide/quickstart`'s nav holds a link to it reading `1.2k`

#### Scenario: the mark sits beside the count
- GIVEN `GITHUB_STARS=1234`
- WHEN the site is built and `/` and `/guide/quickstart` are each opened in a 1280×800 window
- THEN, measuring elements' bounding boxes (the link's `svg` for the mark, the element holding its label for the count), on both pages the nav's GitHub link, and on `/` the first screen's GitHub button, each have the mark's right edge at or left of the count's left edge and the mark's and the count's vertical centres within 2px of each other
- AND on both pages the nav GitHub link's count has its vertical centre within 2px of the label of the nav's Guide link, and on `/` the GitHub button's vertical centre is within 2px of the Get started button's

#### Scenario: a small count
- GIVEN `GITHUB_STARS=87`
- WHEN the site is built
- THEN the links read `87`

#### Scenario: GitHub unreachable
- GIVEN no `GITHUB_STARS` and the GitHub API address unreachable from the build
- WHEN the site is built
- THEN the build succeeds and prints a warning, and the nav and first screen link to the repository with no count

#### Scenario: another repository
- GIVEN `INSTALL_REPO=someone/clm-public` and `GITHUB_STARS=5`
- WHEN the site is built
- THEN the GitHub links name `https://github.com/someone/clm-public`
