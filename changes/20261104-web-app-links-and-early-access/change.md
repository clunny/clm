---
status: proposed
---

# Links between the site and the web app, and early access

## Intent

From the web app there's no way back to the homepage or the guide. The homepage's nav sends visitors to the invite-only app with a link labelled "Sign in", and anyone who isn't invited reaches a "Not invited" page that tells them to ask an operator they can't contact. This change links the app's CLM logo to the homepage, adds a Guide link to the app, renames the nav's "Sign in" to "Web app", and adds a **Request early access** route: a GitHub issue form on `clunny/clm`, linked from the app's sign-in card and the Not invited page. The same places also say that CLM works fully without the web app, by following the quickstart, and that the web app is an optional extra for convenience.

## Problem

- The app's sidebar logo (`◆ CLM`) and its sign-in card's logo are plain text. Below 900 px the sidebar is hidden, and the phone top bar holds only the repo switcher and Notify me, so a phone has no link out of the app.
- The site nav's "Sign in" doesn't say where it goes: CLM is mainly a CLI and a plugin, and the link opens a browser app most visitors can't use.
- The Worker's Not invited page (`web.worker`) says "Ask its operator to add your GitHub login" and gives no way to do that.
- Nothing a visitor sees in the app says the app is optional, so someone turned away might think they can't use CLM at all.

## Approaches

1. **A GitHub issue form for requests (chosen).** Add `.github/ISSUE_TEMPLATE/early-access.yml`, labelled `early-access`, to `clunny/clm`. The app's sign-in card and the Not invited page link to `https://github.com/clunny/clm/issues/new?template=early-access.yml`. A visitor who reaches the Not invited page has just signed in with GitHub, so they already have an account. The maintainer says yes by adding the login to `web`'s invited list, which lands on checks as today (`invite-only-web`). Nothing new is stored and no server code is added, so the request, the decision and the access all stay in git and GitHub (`git-is-truth`). The cost is that requests are public. A GitHub login is public anyway, and the form asks for no email.
2. **A form the Worker stores, with an admin list in the app.** This keeps requests private, but it adds a datastore that owns state outside git, against `git-is-truth`, and it is much more to build and maintain.
3. **A `mailto:` link or a hosted form (such as Tally).** Requests stay private and it's quick, but they arrive separately from the invited list, and a third-party form changes the privacy page and may cost money.

The human chose approach 1.

For the nav label, "Web app" was chosen over "CLM App" (the nav's brand already says CLM) and over "Sign in" (it says what the link is, not what happens after).

## Outcome

- `web`: the sidebar's `◆ CLM` logo links to `/`, the site's homepage; the phone top bar starts with a `◆` link to `/` before the repo switcher; the sidebar's foot holds a **Guide** link to `/guide/` beside Add repos; the sign-in card's logo links to `/`, and the card says CLM works fully without the web app by following the quickstart (a link to `/guide/quickstart`), that the web app is an optional extra in early access, and links to Request early access.
- `web.worker`: the Not invited page says the same, links to Request early access and to the homepage, and still sets no session cookie.
- `web.site`: the nav's "Sign in" reads **Web app**, still linking to `/app/`; the analytics `click` event's label for it is "Web app".
- `ci`: `.github/ISSUE_TEMPLATE/early-access.yml`, an issue form named "Request early access" with the label `early-access`. Its description says CLM works without the web app (with the quickstart's URL), and it has one required field asking what they'd like to use the web app for. It asks for no email.
- The request URL is written in the app and in the Worker as the one constant each, as the app already writes the GitHub App's install URL.

## Constraints

- `invite-only-web` is unchanged: only invited logins sign in, and a request grants nothing until the maintainer edits the invited list.
- The links out of the app go to the same origin's `/` and `/guide/`, outside the app's service worker scope (`/app/`), so they load the site and never the app's offline copy (`web/installable`).
- The app still loads no analytics.
