---
status: done
human_burdens:
  - "Opening the site can't be taken back: search engines and archives keep what they crawl"
  - "The site opens only when you dispatch the next release, whose web job deploys it"
resolves:
  - publish-the-site
---

# Publish the public site

## Intent

Open the homepage and the guide at `clm-app.dev` to everyone, so a newcomer can learn CLM and install it on their own. The app under `/app/` and its API stay invite-only.

## The human's answers

Asked: what should be done with the deferred item publish-the-site, which waits on the human deciding CLM is ready to publicize?

The human's answer: "Draft it now" (the option described as: draft the spec change now; it needs the human's approval before it lands, and the site stays behind sign-in until it is built and deployed).

## Problem

`public-site` put the site behind the invited list until launch: `web.worker/site-behind-sign-in` answers every site path with a 401 sign-in page unless the request holds an invited session. A newcomer can't see the page written for them (`learn-clm`). The item names three steps: open the site's paths, name the public install repository, and stop the quickstart asking for access to a private repository.

The repository question has already been answered: `clunny/clm` is public (checked 2026-11-03: GitHub's API reports `visibility: public` and answers without a token). So the install repository stays `clunny/clm`, no second repository is needed, and no step waits on the human beyond the release.

## Approaches

- **The assets serve the site, with no Worker request (chosen).** `run_worker_first` narrows to `/api/*`, `/auth/*`, `/webhook`, `/_clm/*` and `/app`, and `not_found_handling` becomes `"404-page"`. Nothing decides who sees the site, so the Worker has no reason to run for it. On the Free plan a public page and any crawler then spend none of the 100,000 Worker requests a day, so traffic to the site can't take the app down (`site-served-from-assets`).
- **Keep the site on the Worker and drop the session check.** The item's wording, and the smallest edit. Rejected: every page view spends a Worker request, and the Worker keeps site code that does nothing (`no-dead-code`, `web-on-free-plan`).
- **A new public repository.** The item's other option. Rejected: `clunny/clm` is already public, and a second repository would need syncing with nothing gained (`yagni`).
- **Download releases without `gh`** (`binary-from-releases` named a mirror or bucket). Rejected for now: public release assets make a mirror needless, and the quickstart's `gh` requirement stays true of the shim. A change of its own if someone needs it (`yagni`).

## Outcome

- `web/site-served-directly` (new) holds the routing, and `web.worker/reserved-paths` (new) what the Worker keeps: `/app` redirects, and any other path its routes don't answer, `/_clm/*` included, is a bare 404. `web.worker/site-behind-sign-in` and its sign-in page go.
- `web/app-at-app-path` keeps only the built tree; its routing clause moves to the new rule. It, `web/installable` and `web/retire-root-service-worker` drop their `Needs` on the removed rule.
- `web.worker/reserved-paths` needs `web/site-served-directly`, so the Worker stops serving the site only once the routing no longer sends it there.
- `web.site/install-repository` states `clunny/clm` as the value, and the code comment saying it is private goes with that rebuild.
- `web.worker.platform/asset-responses` no longer serves `learn-clm`, and its Design stops naming the removed rule. A Serves-only edit, so nothing is built there.
- `learn-clm`'s scenario says anyone with no session, so its test drops the sign-in. It waits on `web/site-served-directly` through the queue's serving-rules order.
- `invite-only-web` is reworded with a scenario for a stranger opening the site, `ARCHITECTURE.md`'s and the decision's; `public-site`, `binary-from-releases` and `private-github` are amended; `docs/guide/quickstart.md` stops asking for access to `clunny/clm`.

## Constraints

- The invited list must stay unserved: `/_clm/*` stays on the Worker's routes, and both `web/site-served-directly` and `web.worker/reserved-paths` say so.
- The app's static files under `/app/` were already served without a session; only what they fetch from the API needs one. This change leaves that as it is, and the rewording says what is protected: what the app shows of a repository.
- Edge case accepted: a missing path under `/app/` now gets the site's `/404.html` with status 404, where it got a bare 404. The app routes by hash, so nothing links there.
- Accepted: `fetch_asset` keeps its whole-response form though only `asset(path)` still uses it for the invited list (`site-served-from-assets`).

## Open questions

None.
