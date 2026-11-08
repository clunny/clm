---
component: web.site
---

## ADDED Requirements
### Requirement: doc-readers-marked
Each test file under `web/site/` that builds the site from, or reads, this repository's own `docs/guide/` or `docs/spec-format.md` (not a fixture `GUIDE_DIR` points at), and each capability test under `capabilities/web/` that opens a guide page of the site built from them, SHALL hold a `clm:reads` marker (`check.affected/affected-readers`) whose glob matches every such path it reads, `docs/guide/**` for one that builds the whole guide, so a land that edits a guide page runs it even when its code was already checked (`land/selected-tests`). The capability tests' markers are part of this requirement's build (`capability-tests-follow-rules`).

#### Scenario: the site's readers of the guide
- GIVEN the `clm:reads` markers of the files `check`'s coverage scan reads in this repository's tree
- WHEN `clm-affected`'s `readers` runs on them with `docs/guide/quickstart.md` as the only differing path
- THEN it selects `web/site/test/guide.spec.ts` and `web/site/test/seo.spec.ts`, which read the repository's guide and the site built from it, and `capabilities/web/learn-clm.spec.ts`, which follows the homepage to the quickstart

#### Scenario: the spec format page
- GIVEN the same markers
- WHEN `readers` runs on them with `docs/spec-format.md` as the only differing path
- THEN it selects `web/site/test/seo.spec.ts`

#### Serves
- capability/land-safely

#### Needs
- check.affected/affected-readers
