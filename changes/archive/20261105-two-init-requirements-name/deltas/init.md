---
component: init
---

## MODIFIED Requirements

### Requirement: template-says-pull-request <!-- base: f22d765fea1a3713 -->
No file under `crates/clm-init/templates/` SHALL call a host's pull request a page: none SHALL hold `page's` (only the guide, which `clm guide` prints, has pages).

#### Scenario: the written architecture
- GIVEN an empty repository
- WHEN `clm init` runs
- THEN its `ARCHITECTURE.md` holds no `page's URL`, and no line of a file under `crates/clm-init/templates/` holds `page's`

#### Serves
- architecture/plain-terms

### Requirement: templates-say-new-words <!-- base: 46b3713f2cd719fc -->
The root `SPEC.md` `clm init` writes SHALL have `status: unbuilt`; no file under `crates/clm-init/templates/` SHALL hold a word `plain-terms-renames` replaced for a component status, a backlog kind or a `change.md` key.

#### Scenario: a new repository
- GIVEN an empty repository
- WHEN `clm init` runs
- THEN its `specs/SPEC.md` has `status: unbuilt`, no file under `crates/clm-init/templates/` holds a word `plain-terms-renames` replaced, and the model loads the tree with no violation

#### Serves
- architecture/plain-terms

#### Needs
- model/valid-frontmatter
- model/parse-changes
