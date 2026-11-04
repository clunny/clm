---
component: web
base: {}
---

## ADDED Requirements

### Requirement: links-to-site
The sidebar's `◆ CLM` logo SHALL be a link to `/`, the site's homepage, and the sidebar's foot SHALL hold a Guide link to `/guide/` beside Add repos. Below 900 px wide, the top bar SHALL begin with a `◆` link to `/`, labelled for assistive technology as the CLM homepage, before the repo switcher. These links SHALL be plain links that load the site, never routes of the app's hash router.

#### Scenario: desktop sidebar
- GIVEN the app open on a repo's spec graph in a 1280×800 window
- WHEN a test reads the sidebar
- THEN its logo is a link whose `href` is `/`, and a link reading `Guide` with `href` `/guide/` sits beside Add repos

#### Scenario: phone top bar
- GIVEN the app open on a repo's spec graph in a 390×844 window
- WHEN a test reads the top bar
- THEN its first element is a link whose `href` is `/` with an accessible name naming the CLM homepage, and it comes before the repo switcher

#### Serves
- capability/learn-clm

### Requirement: sign-in-early-access
The sign-in page's logo SHALL be a link to `/`. The page SHALL say that CLM works fully without the web app by following the quickstart, linking to `/guide/quickstart`, and that the web app is an optional extra for convenience, in early access, and SHALL link to Request early access at `https://github.com/clunny/clm/issues/new?template=early-access.yml`, the one constant in the app that names it. Sign in with GitHub and Install the GitHub App SHALL stay as they are.

#### Scenario: signed out
- GIVEN a Worker that answers the repo list 401
- WHEN the app is opened
- THEN the sign-in page's logo links to `/`, its text says CLM works without the web app and that the web app is an optional extra in early access, it links to `/guide/quickstart` and to `https://github.com/clunny/clm/issues/new?template=early-access.yml` with a link reading `Request early access`, and Sign in with GitHub still leads to `/auth/login`

#### Serves
- capability/learn-clm
