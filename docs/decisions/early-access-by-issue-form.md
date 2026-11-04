---
title: "Early access to the web app is requested with a GitHub issue form"
date: 2026-11-04
status: accepted
---

The web app under `/app/` is invite-only (`invite-only-web`). Until now, someone who wasn't invited signed in with GitHub and reached a page telling them to ask an operator they had no way to contact. Visitors now request access with an issue form on `clunny/clm`, `.github/ISSUE_TEMPLATE/early-access.yml`, labelled `early-access`. The app's sign-in page and the Worker's Not invited page link to it (`20261104-web-app-links-and-early-access`). A maintainer grants a request by adding the login to `web`'s invited list, which lands on checks as before. Closing the issue is the maintainer's choice and nothing reads it.

The same pages say that CLM works fully without the web app, by following the quickstart, and that the web app is an optional extra for convenience. Someone turned away shouldn't think they can't use CLM.

Why an issue form: a person who reaches the Not invited page has just signed in with GitHub, so they already have an account. The request, the decision (the invited list) and the access all stay in git and GitHub, and no new service holds state (`git-is-truth`). Requests are public. A GitHub login is public anyway, and the form asks for no email.

Rejected:
- requests stored by the Worker, with an admin list in the app: a datastore owning state outside git, and much more to build;
- a `mailto:` link or a hosted form: requests arrive apart from the invited list, and a third-party form changes what `/privacy` must say and may cost money.

The site's nav names the link to the app "Web app" rather than "Sign in", which said what happens and not where the link goes, or "CLM App", which repeats the nav's brand.
