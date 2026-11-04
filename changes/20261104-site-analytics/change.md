---
status: applied
human_burdens:
  - "Create an Umami Cloud account (free Hobby tier) and a website for clm-app.dev; Umami becomes the processor of the site's visit data"
  - "Set the website's ID as the repository's Actions variable UMAMI_WEBSITE_ID, and define the homepage-to-install funnel in Umami's dashboard"
---

# Site analytics

## Intent

The maintainer should be able to see how many people visit `clm-app.dev`, where they come from, and how far they get from the homepage to installing CLM: which pages they read, what they click, whether they copy the install command, and where they leave. The public site gets cookieless analytics from Umami Cloud, loaded only in the released site, honouring Do Not Track, and described in a privacy note in the footer. Usage reporting from the CLI, where most friction after install lies, is a separate backlog idea.

## Problem

The site (`web.site`) measures nothing, so whether it does what `learn-clm` asks (a newcomer understands CLM within a minute and gets to the quickstart's install commands) can't be judged, and nothing shows where newcomers give up. The site's audience is developers, many of whom block trackers, so any script-based count will be low, and the design has to say how to judge by how much.

## Approaches

- **Umami Cloud, cookieless, with custom events (chosen).** One script on the site; clicks and copies are recorded as events; Umami's dashboard gives referrers, pages, events and a funnel report. Its free Hobby tier covers the site's traffic. Setting up the account is a step only the human can take. Moving to Plausible later would change only the script and its attributes.
- **Collect on our own Worker into Workers Analytics Engine, with a report page in the app.** No new vendor and the data stays in the operator's Cloudflare account, but we would build and keep collection, queries and a funnel view a standard tool already has (`reuse-first`), and every event would cost a Worker request that the site's pages don't spend today (`site-served-from-assets`). Rejected.
- **Cloudflare Web Analytics, alone or beside Umami.** Free and cookieless, but it counts pages only, with no custom events, so it can't say who copied the install command. As a second script it doesn't cover blocked visitors either: blocklists block its beacon too. Rejected.
- **Serve Umami's script through our own domain to get past blockers.** Rejected: a visitor who blocks trackers has chosen not to be tracked, and a tool asking for trust shouldn't route around that choice. It would also cost Worker requests.

## Outcome

- **Page views** of the homepage and every guide page, including VitePress's in-site navigation, with referrer, country and device, as Umami records them by itself.
- **Click events**, named by the element's label: Get started, How it works, Sign in, the nav links, and every link leaving the site (`outbound`, with its URL).
- **`copy-command`**, whenever a visitor copies a command: the homepage's install commands and any guide code block's copy button, with the page's path and the command's first line.
- **Only the released site reports.** The website ID comes from the build's environment, `UMAMI_WEBSITE_ID`, which the release's `web` job sets from the repository's Actions variable of that name. A build without it, as in tests, local builds and anyone's fork, holds no analytics script and sends nothing. This keeps the operator's ID out of the files, as `operator-ids-out-of-the-repo` wants for the others.
- **Do Not Track is honoured** (`data-do-not-track`), and the app under `/app/` is not tracked: only invited people use it.
- **A privacy note** at `/privacy`, linked from the footer: no cookies, no personal data, no IP addresses stored, Umami Cloud as the processor, Do Not Track honoured.
- **The undercount** is judged against Cloudflare's own traffic numbers for the zone, which count requests at the edge with no script and can't be blocked. That is a dashboard reading, not code.
- The funnel (homepage → `/guide/quickstart` → `copy-command`) is set up once in Umami's dashboard by the human.

## Constraints

- `web-optional` and `git-is-truth`: analytics only observes; nothing in CLM reads it, and the site works the same when Umami is down or blocked.
- `site-served-from-assets`: the site's pages still cost no Worker request; the script and its events go to Umami's own host.
- `yagni`: no setting beyond the website ID, no proxy, no second analytics tool.
