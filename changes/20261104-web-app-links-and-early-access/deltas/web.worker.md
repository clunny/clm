---
component: web.worker
base: {}
---

## ADDED Requirements

### Requirement: not-invited-page
The "not invited" page the callback answers to a login that isn't invited SHALL say that the web app is in early access, that CLM works fully without it by following the quickstart, linking to `/guide/quickstart`, and that the web app is an optional extra for convenience; it SHALL link to Request early access at `https://github.com/clunny/clm/issues/new?template=early-access.yml`, the one constant in the Worker that names it, and to the homepage at `/`. It SHALL still answer 403 and set no session cookie (`web/invited-users`).

#### Scenario: someone not invited
- GIVEN the invited list holds `clunny` and GitHub completes the authorization for `someone-else`
- WHEN the browser reaches the callback
- THEN the response is 403, sets no session cookie, and its HTML links to `https://github.com/clunny/clm/issues/new?template=early-access.yml`, `/guide/quickstart` and `/`, and says the web app is optional and in early access

#### Serves
- capability/learn-clm
