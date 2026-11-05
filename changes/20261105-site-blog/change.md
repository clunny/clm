---
status: proposed
human_burdens:
  - "Moves the clm-app.dev Custom Domain from the app's Worker to a new site Worker: a one-time production change to the live zone, with a possible brief outage while it moves."
  - "The Cloudflare API token in the repository's secrets may need the zone's Workers Routes permission (backlog/cloudflare-token-workers-routes)."
  - "Each post goes public only when the human dispatches the site workflow: a step only the human takes, once per post."
---

# A blog on clm-app.dev, released one post at a time

## Intent

Add a blog to the public site at `clm-app.dev/blog/`, so the posts written about CLM can be released there one at a time as a series. Each post is a Markdown file in the repository with a `draft` flag; a draft is left out of the build entirely, so it is never visible, linked, listed or searchable. Published posts get the same search and AI treatment as the guide's pages. To release a post without cutting a CLM release, the site moves to a Cloudflare Worker of its own, deployed by a workflow the human dispatches, while the app's Worker keeps `/app/`, the API, sign-in and the webhook and is still deployed by the release.

## Problem

- The site has no place for writing about CLM. Seventeen posts exist outside the repository, written to be released as a series.
- The site and the app are one Worker with one set of assets, deployed only by the release workflow's `web` job (`ci/web-deploy`). Publishing anything on the site today means dispatching a CLM release, which bumps the version and ships the plugin.
- A post must be able to sit in the repository unpublished without leaking: not on a page, not in the sitemap, `llms.txt`, a feed or a Markdown copy.

## Approaches

1. **A deploy workflow of its own for the one Worker.** A dispatched workflow builds `web/` from `main` and runs the same `wrangler deploy`; the release stops deploying. Smallest change, but every site deploy also ships the app from `main`, and the two still can't be released apart.
2. **The site as a Worker of its own (chosen).** The site becomes an assets-only Worker that takes over the `clm-app.dev` Custom Domain. The app's Worker drops the Custom Domain and claims its own paths as routes; Cloudflare runs a route's Worker before the Custom Domain's, so `/app*`, `/api/*`, `/auth/*`, `/webhook`, `/_clm/*` and `/sw.js` reach the app and everything else reaches the site (`site-worker-of-its-own`). The release deploys the app; a new dispatched workflow deploys the site. The two release independently, and a site deploy can never change the app.
3. **Option 1 plus scheduled publishing.** Each post gets a date, and a daily scheduled deploy publishes the posts whose date has passed. The series releases itself, at the cost of a daily Actions run and posts going live with nobody watching.

The human chose approach 2.

For the blog's main page the human chose a grid of cards, newest first (mockup A of `assets/mockups/blog.html`), over a featured latest post with the series listed below it.

## Outcome

- **Posts** live in `web/site/blog/`, one Markdown file each, named `<slug>.md`. Frontmatter: `title`, `summary`, `date` (the publication date, `YYYY-MM-DD`), `tags`, optional `series_part`, and `draft`. The reading time is worked out at build time from the body, never written by hand (`blog-drafts-left-out`).
- **Drafts** (`draft: true`, or `draft` missing) are left out of the build: no page, no card, no series entry, no sitemap URL, no feed item, no Markdown copy, no `llms.txt` line, and no link to one from a published post's series list or previous/next links.
- **`/blog/`**: published posts as cards, newest first, each with its part, date, reading time, title, summary and tags (mockup A). The nav gains **Blog** after Guide.
- **A post's page** at `/blog/<slug>`: title, summary as the lede, date, reading time, tags, the body, the series in a side box (published parts only), and previous and next links to published posts in the series.
- **Search and agents**: each published post has its title (`<title> | CLM blog`), description (its `summary`), canonical link, Open Graph tags with `og:type` `article`, `BlogPosting` structured data, a Markdown copy at `/blog/<slug>.md`, a line in a `## Blog` section of `llms.txt`, a sitemap URL, and an item in an RSS feed at `/blog/feed.xml`. `/blog/` is in the sitemap and holds `Blog` structured data.
- **Deployment**: `web/site/wrangler.toml` declares the site's Worker `clm-site`, assets only, on the `clm-app.dev` Custom Domain. `web/wrangler.toml` drops the Custom Domain and routes only the app's paths. `web`'s build no longer builds the site. The release workflow's `web` job deploys the app only; `.github/workflows/site.yml`, run only on `workflow_dispatch`, builds `web/site/` from `main` and deploys the site with the Umami website ID and the workflow's token.
- **The guard** treats `site.yml` as it treats `release.yml`: a session can't dispatch it or re-run one of its runs (`humans-cut-releases`), so only the human makes the site public.
- **Capability** `publish-posts`: the maintainer can keep posts as drafts in the repository and publish them one at a time, with no CLM release.
- The seventeen posts move into `web/site/blog/` as drafts on a content branch once the blog is built. No requirement names them.

## Constraints

- `invite-only-web` holds: the site's Worker serves only public files, and every path that reaches a repository's data is still the app Worker's.
- The order of the move matters on the live zone: the app Worker's routes must exist before the site Worker takes the Custom Domain, or `/app/` and the API answer with the site's 404 in between. The builder states the order in the pull request; the human runs the first site deploy.
- The site's build stays readable without script, and its pages keep every head tag `page-metadata` lists.
