---
component: web.site
base: {}
---

## MODIFIED Requirements

### Requirement: github-link <!-- base: 37bdaa86c59602ae -->
The nav of every page, and the homepage's first screen, SHALL link to `https://github.com/<repo>`, `<repo>` being the install repository, labelled with the repository's star count read at build time from `GITHUB_STARS` or GitHub's API; when the count can't be read the build SHALL still succeed, showing the link without a count. Each link SHALL show GitHub's mark beside its label on one line, the mark before the label, both centred on the same line as the links and buttons around it.

#### Scenario: stars given
- GIVEN `GITHUB_STARS=1234` in the build's environment
- WHEN the site is built and `/` is opened in a 1280×800 window, not scrolled
- THEN a link to `https://github.com/clunny/clm` reading `1.2k` is in view on the first screen, and `/guide/quickstart`'s nav holds a link to it reading `1.2k`

#### Scenario: the mark sits beside the count
- GIVEN `GITHUB_STARS=1234`
- WHEN the site is built and `/` and `/guide/quickstart` are each opened in a 1280×800 window
- THEN in the nav's GitHub link, and in the homepage's GitHub button, the mark's right edge is at or left of the count's left edge and the vertical centres of the mark and the count are within 2px of each other; and the nav GitHub link's count is vertically centred within 2px of the nav's Guide link text

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
