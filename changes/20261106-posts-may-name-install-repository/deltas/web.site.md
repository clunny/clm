---
component: web.site
---

## MODIFIED Requirements

### Requirement: install-repository <!-- base: b53df95fe20d4d65 -->
The homepage's install commands SHALL name the repository set by one value in the site's config, by default `clunny/clm`, the public repository, and no other file under `web/site/` SHALL name it, except the blog's posts under `web/site/blog/`, which are written text, as the guide's files are, and may name it in their own install commands.

#### Scenario: repository changed
- GIVEN the site's config value set to `someone/clm-public`
- WHEN the homepage is built
- THEN its install commands read `/plugin marketplace add someone/clm-public`, and no other file under `web/site/` outside `web/site/blog/` contains `clunny/clm`

#### Scenario: the public repository
- GIVEN no `INSTALL_REPO` in the environment
- WHEN the homepage is built
- THEN its install commands read `/plugin marketplace add clunny/clm`
