---
component: ci
---

## MODIFIED Requirements

### Requirement: web-deploy <!-- base: f966d7eed8f8658f -->
When a release is published, the release workflow SHALL deploy the `web` Worker and its assets with `wrangler deploy`, using only the Cloudflare credentials held in repository secrets, with `UMAMI_WEBSITE_ID` in the deploy step's environment set from the repository's Actions variable of that name, so the released site, and no other build, reports to the operator's analytics (`site-analytics-umami`), and with `GITHUB_TOKEN` set to the workflow's own token, so the site's build reads the repository's star count without GitHub's anonymous rate limit (`web.site/github-link`).

#### Scenario: release
- GIVEN a published release `v<workspace version>`
- WHEN the release workflow runs
- THEN its `web` job runs `wrangler deploy` with `CLOUDFLARE_API_TOKEN` from secrets

#### Scenario: website ID from a variable
- GIVEN the release workflow
- WHEN a test reads its `web` job's deploy step
- THEN its environment sets `UMAMI_WEBSITE_ID` to `${{ vars.UMAMI_WEBSITE_ID }}`, and no file under `.github/` holds a literal website ID

#### Scenario: token for the star count
- GIVEN the release workflow
- WHEN a test reads its `web` job's deploy step
- THEN its environment sets `GITHUB_TOKEN` to `${{ github.token }}`, and the job's `permissions` grant no more than `contents: read`

#### Serves
- capability/follow-from-browser
- capability/see-site-use
- capability/learn-clm
- architecture/web-optional
