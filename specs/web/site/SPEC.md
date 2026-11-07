---
title: Site
status: active
owners: ["@clunny"]
depends_on: []
paths: ["web/site/**"]
successor: ""
serves: [capability/learn-clm]
---

## Contract
CLM's public site at `https://clm-app.dev`: the homepage at `/` and the user guide at `/guide/`, built with VitePress from `web/site/` and the pages `clm guide` prints, `docs/guide/*.md` and `docs/spec-format.md` (`public-site`, `guide-in-the-binary`). It is deployed as a Cloudflare Worker of its own, `clm-site`, assets only, on the `clm-app.dev` Custom Domain, by the site workflow a human dispatches (`ci/site-deploy`, `site-worker-of-its-own`); the app's Worker answers only the app's paths, ahead of it (`web/site-served-directly`, `invite-only-web`), and the site's assets serve everyone, with no session and no script. The site also holds the blog at `/blog/`: posts kept as Markdown files in `web/site/blog/`, of which only those not marked as drafts are built, so a draft is never visible, listed or searchable (`blog-drafts-left-out`). The site links to the app at `/app/` and reads nothing from the Worker's API. The released site reports page views, clicks and copied commands to Umami Cloud, cookieless and honouring Do Not Track, and only when its build is given a website ID; `/privacy` says what is collected (`site-analytics-umami`). Every page is readable without script and carries its own title, description, canonical link and social card; the build writes `robots.txt`, `sitemap.xml`, structured data, and for agents a Markdown copy of each guide page with `llms.txt` and `llms-full.txt`, all made from the guide's files and the published posts so none can fall behind them (`site-found-by-search-and-agents`). The nav and the homepage link to the GitHub repository with its star count, read at build time.

## Design
**Build.** A package of its own in `web/site/`: its `package.json` with VitePress and its build and test scripts, its committed `package-lock.json`, a `.npmrc` setting `ignore-scripts=true` (`unattended-commands`), its VitePress config and theme under `web/site/.vitepress/`, the homepage as a custom layout, and its tests with their own configs. Its own package keeps VitePress's Vite and Vue apart from the app's Vite 8. `npm run build` there writes the site to the directory `SITE_OUT` names (default `web/site/dist/`); `web`'s build doesn't build the site: the site workflow builds and deploys it on its own (`ci/site-deploy`). `npm test` runs its tests, which `ci-check.sh` runs after `web/`'s suite (`ci/web-checks`).

**The guide's pages** are the pages `clm guide` prints: each file of `docs/guide/` at `/guide/<name>`, `README.md` at `/guide/`, and `docs/spec-format.md` at `/guide/spec-format`, which the guide's files link to as `../spec-format.md`. They are read in place at build time (VitePress's `srcDir` and `rewrites`, a Vite alias or a loader, the builder's choice), never copied into the repository: they stay the only copy (`guide-from-docs`). Links between them resolve to their pages, and the build fails on a dead link. The sidebar lists them in the order `docs/guide/README.md` links them, labelled with the link text, and `spec-format` last. The directory the guide is read from and the install repository (below) can be set by environment variables, `GUIDE_DIR` and `INSTALL_REPO`, so tests can build with a fixture guide and another repository; `UMAMI_WEBSITE_ID` turns analytics on and `GITHUB_STARS` sets the star count (below). Every guide file opens, after its `#` title, with a summary paragraph of a sentence or two saying what the page is for: it is the page's description (below), and the build fails, naming the file, on a guide file whose first block after the title isn't a paragraph.

**Homepage.** `assets/mockups/homepage.html` is the reference for layout, colour, type and copy; the dashed line in it marks roughly where a laptop's first screen ends. Its copy is the starting text: wording may be polished, the message and sections may not change without a spec change. The first screen holds the headline "Declare the intent. AI builds the code to match. CLM keeps it matching.", one line on building solo or as a team with AI writing every line while the project stays maintainable, **Get started** (to the quickstart), **How it works** (to the sections below) and **GitHub** (to the repository, with its star count, below), the install commands in small type, and the loop drawn as a ring around "your intent": Declare (you approve, yes or no), Build (AI, test-first), Prove (a test cites every rule), Reconcile (drift is rebuilt). Kubernetes isn't named on the first screen. Then five sections, in order:
1. *The split*: you own the what (architecture, capabilities, one yes or no per change); AI owns the how.
2. *The loop*: "Reconciliation, borrowed from Kubernetes", opening with "CLM isn't a Kubernetes tool. It borrows one idea from it", then Kubernetes and CLM side by side.
3. *Not a vibe-coded mess*: every rule has a test, drift can't land, nothing dead.
4. *Solo or as a team*: git is the only channel.
5. *Standing on shoulders*: the Spec Growth Engine paper ([arXiv:2606.27045](https://arxiv.org/html/2606.27045v2)), Kubernetes's [controllers](https://kubernetes.io/docs/concepts/architecture/controller/), [superpowers](https://github.com/obra/superpowers) and Anthropic's [AI-native SDLC playbook](https://claude.com/blog/the-ai-native-sdlc-playbook), each with what CLM took from it, as `docs/guide/why-clm.md` says.
It ends with the install commands and a link to the quickstart, and a footer "CLM is built with CLM" with links to the guide, the web app (reading Web app) and `/privacy`; the guide's pages, whose default layout has no footer, get a small footer with the Privacy link through a slot of VitePress's layout in the theme. The nav holds How it works, Guide, Blog, Get started and Web app (to `/app/`, the invite-only web app), then the GitHub link (below). The mockup predates the GitHub button and the fourth credit; they follow its button and credit styles.

**The install repository.** The repository the homepage's install commands name (`/plugin marketplace add <repo>`) is one value in the site's config (overridable by `INSTALL_REPO` for tests): `clunny/clm`, which is public, so a visitor adopts CLM on their own (`public-site`). The blog's posts are written text, as the guide's files are, and may name it in their own install commands (`install-repository`).

**Look.** Inter for text, JetBrains Mono for commands; the app's indigo accent with a sky gradient, the mockup's dark palette and a light one. The site follows the system's light or dark setting, with VitePress's own switch kept. At phone widths the ring sits below the first screen's text, the side-by-side panels stack, and nothing scrolls sideways.

**Analytics** (`site-analytics-umami`). When the build's environment sets `UMAMI_WEBSITE_ID`, every page's head holds `<script defer src="https://cloud.umami.is/script.js" data-website-id="<id>" data-do-not-track="true">`; when it is unset or empty, no built file names Umami, so tests, local builds and forks report nothing, and the operator's ID lives only in the repository's Actions variable (`ci/site-deploy`). Umami's script records page views itself, VitePress's in-site navigation included. One module of the theme records the rest by calling `window.umami.track(name, data)`, doing nothing when `window.umami` is missing (blocked, Do Not Track, or a build without an ID), and never delaying a click's navigation:
- a click on a same-host link in the nav or among the homepage's buttons (Get started, How it works, Guide, Web app): the event `click`, with `label` the element's text and `path` the page's;
- a click on any link to another host: the event `outbound` only, with `url` the link's and `path` the page's;
- a copied command, by a code block's copy button or by copying a selection inside a command (the homepage's install commands, a guide code block): the event `copy-command`, with `path` the page's and `command` the command's first line, trimmed. VitePress's copy button writes to the clipboard without a `copy` event, so it is caught by its click, and a selection by the document's `copy` event when the selection lies inside a `pre` or code block. The script tag is added per page in the config's head (`transformHead` or `head`, the builder's choice), so the 404 page carries it too.
The app under `/app/` is a build of its own and loads no analytics. The site serves no cookie and the theme stores nothing in the browser for analytics. The funnel and the undercount (judged against Cloudflare's own traffic numbers for the zone) are read in the dashboards, not in code.

**Privacy page.** `/privacy`, a Markdown page of the site's own under `web/site/`, not part of the guide and listed in neither the nav nor the sidebar, says in plain words: the site sets no cookies; it counts page views, clicks and copied commands with Umami Cloud, which stores no IP address and no personal data; Do Not Track is honoured; and the app under `/app/` isn't measured.

**The site's address** is one value in the site's config, `https://clm-app.dev`, from which every absolute URL the build writes is made: canonical links, the sitemap, Open Graph tags, structured data, `robots.txt` and the agent files.

**Search** (`site-found-by-search-and-agents`). The site is built to be read without running script: VitePress renders every page, the homepage's custom layout included, to static HTML, and nothing a crawler should read (headings, copy, links, install commands, star count) is added in the browser alone. Every page's head holds:
- a `<title>`: the homepage's is "CLM: declare the intent, AI builds the code to match"; a guide page's is its `#` title followed by " | CLM guide", except `/guide/`'s, which is "CLM guide"; `/privacy`'s is "Privacy | CLM"; `/blog/`'s is "CLM blog" and a post's is its `title` followed by " | CLM blog"; the 404 page's is "Page not found | CLM";
- `<meta name="description">`: a guide page's summary paragraph (above), as plain text with its Markdown removed; the homepage's is its headline followed by its first screen's line, both one constant each in the site's config that `Home.vue`, the heads and `llms.txt` all read; `/privacy`'s is its own first paragraph; a post's is its `summary`, and `/blog/`'s one constant in the site's config;
- `<link rel="canonical">` to the page's clean URL on the site's address; the 404 page has none and holds `<meta name="robots" content="noindex">` instead;
- Open Graph and Twitter card tags (`og:title`, `og:description`, `og:url`, `og:type` `website`, `article` on a post's page with `article:published_time` its `date`, `og:site_name` `CLM`, `og:image`, `twitter:card` `summary_large_image`), the title and description being the page's own, and the image one social card for the whole site: a 1200×630 PNG under `web/site/public/`, made once from the homepage's first screen (the headline, the ring and the site's address), for instance by a Playwright capture, and committed; the build doesn't regenerate it;
- on a guide page or a post's page, `<link rel="alternate" type="text/markdown">` to its Markdown copy (below); on `/blog/` and every post's page, `<link rel="alternate" type="application/rss+xml">` to the blog's feed.

The build writes `sitemap.xml` (VitePress's sitemap, on the site's address), listing the homepage, every guide page, `/privacy`, `/blog/` and every published post, and no page under `/app/` and no draft, and `robots.txt`: one group for every crawler, `User-agent: *`, with no `Allow` line (what isn't disallowed is allowed), disallowing `/app/`, `/api/` and `/auth/`, the paths that belong to the invite-only app (`invite-only-web`), then the `Sitemap:` line. One group for everyone welcomes the AI crawlers too, search's (OAI-SearchBot, Claude-SearchBot, PerplexityBot) and training's (GPTBot, ClaudeBot) alike: a named group would replace the `*` group for that crawler and need its own copy of the disallows. Cloudflare can add its own blocks in front of the zone's `robots.txt`: the operator keeps the zone's AI crawler blocking (**Block AI bots**, and every crawler in **AI Crawl Control**) and Cloudflare's managed `robots.txt` off, so the site's own `robots.txt` is what every crawler reads. It is a zone setting, not code.

**Structured data.** The homepage holds one `<script type="application/ld+json">` describing CLM as a `SoftwareApplication`: `name` `CLM`, the homepage's description, `url` the site's address, `applicationCategory` `DeveloperApplication`, `operatingSystem` `Linux, macOS`, `offers` a price of `0`, `license` the MIT licence's URL, and `sameAs` the GitHub repository's URL. `/guide/faq` holds one describing an `FAQPage`, made at build time from `docs/guide/faq.md`: each `##` heading is a `Question` and the Markdown up to the next heading its `acceptedAnswer`, as plain text. A guide with no `faq.md` (a test's fixture) gets no `FAQPage` and the build doesn't fail. `/blog/` holds one describing a `Blog` whose `blogPost` lists the published posts, and each post's page one describing a `BlogPosting`: `headline` its `title`, `description` its `summary`, `datePublished` its `date`, `keywords` its `tags`, `url` its page, `author` and `publisher` an `Organization` named `CLM`. Both are made from the files and the config, never written by hand, so they can't fall behind the page.

**Agents.** The build writes, from the same files the guide's pages are built from and never committed (`guide-from-docs`):
- a Markdown copy of each guide page beside its HTML: `/guide/<name>.md`, `/guide/index.md` for `README.md` and `/guide/spec-format.md`, each the source file's text with every link to another guide file rewritten to that file's Markdown copy, as an absolute URL on the site's address, and every other relative link (into the repository) rewritten to it on the GitHub repository's `main`, `/blob/main/<path>` for a file and `/tree/main/<path>` for a directory. Only inline Markdown links outside fenced code are rewritten, and a link's `#anchor` is kept. The copies' text is otherwise the file's own, so a build with another `INSTALL_REPO` still shows the guide's own commands;
- `llms.txt`, in the `llmstxt.org` format: `# CLM`, a `>` line with the homepage's description, a paragraph saying what CLM is and how to install it (the homepage's install commands), a `## Guide` section listing every guide page's Markdown copy in the sidebar's order as `- [<title>](<url>): <summary>`, a `## Blog` section listing every published post's Markdown copy, newest first, the same way (left out when no post is published), and a `## Optional` section linking `llms-full.txt` and the GitHub repository;
- `llms-full.txt`: every guide page's Markdown copy, in the sidebar's order, each preceded by a line `<!-- <url of its page> -->` and separated by a line `---`.
A guide file added, edited or removed changes them at the next build with no edit under `web/site/`, as it changes the pages. Each published post has a Markdown copy too, at `/blog/<slug>.md`: its file's text without the frontmatter, under a `#` line with its `title`, links rewritten as a guide copy's are, a link to a draft left as its text. `llms-full.txt` holds the guide only.

**GitHub link.** The GitHub repository is the install repository (above): its URL is `https://github.com/<repo>` with no second value naming it. The nav ends with a GitHub icon linking to it, labelled with the star count, and the first screen's **GitHub** button shows the same. The count is read once per build: `GITHUB_STARS` when the environment sets it (tests set it, so they never call the network), otherwise the repository's `stargazers_count` from `<api>/repos/<repo>`, `<api>` being `GITHUB_API` when the environment sets it (tests point it at an address that refuses connections) and `https://api.github.com` otherwise, sent with `GITHUB_TOKEN` as a bearer token when the environment holds one (`ci/site-deploy` passes the workflow's), with a timeout of 10 seconds. A failed or timed-out read never fails the build: the link and button show without a count, and the build prints a warning. The count is written into the HTML at build time, so crawlers read it and visitors' browsers make no request to GitHub; it is shown as the number under 1000 and, from 1000, as thousands rounded to one decimal (`1.0k`, `1.2k` for 1234, `2.0k` for 1950), and is as old as the last site deploy. Every test build sets `GITHUB_STARS` or `GITHUB_API` (the Playwright config's web server included), so no test waits on the network. The GitHub button doesn't take the classes of the first screen's Get started button, which the first-screen test selects.

**Deploy** (`site-worker-of-its-own`). `web/site/wrangler.toml` declares the site's Worker: `name = "clm-site"`, a `compatibility_date`, no `main` (assets only, so no request to the site runs a script or spends a Worker request), `routes = [{ pattern = "clm-app.dev", custom_domain = true }]`, and `[assets]` with `directory` `./dist`, where `npm run build` writes by default, and `not_found_handling = "404-page"`, so a missing path gets the site's `/404.html` with status 404. The app's Worker (`web`) claims the app's paths on the same hostname as routes, and Cloudflare runs a route's Worker before the Custom Domain's, so those paths never reach the site's. The site holds nothing of the repositories the web app reads: no spec, backlog, change or code of any of them (`invite-only-web`). The only repository it names is CLM's own public one, in its install commands, its GitHub links and the blog's posts. `wrangler` is a dev dependency of the site's own package, so the site workflow deploys it with `npx wrangler deploy` in `web/site/` (`ci/site-deploy`).

**Blog** (`blog-drafts-left-out`). Each post is `web/site/blog/<slug>.md`, its slug the file name (the directory `BLOG_DIR` names when the environment sets it, so tests build fixture blogs as `GUIDE_DIR` builds fixture guides; an empty or missing directory builds a blog with no posts), with YAML frontmatter: `title`, `summary` (one sentence, its description), `date` (`YYYY-MM-DD`, the day it is published), `tags` (a list of words), `series_part` (optional, a positive integer: its place in the series) and `draft` (`true` or `false`). A post with `draft: false` is published; one with `draft: true`, or with no `draft` key, is a draft. The build validates every post, drafts included, and fails, naming the file, on one missing `title`, `summary` or `date`, with a `date` that isn't a real calendar date written `YYYY-MM-DD`, with a `draft` that isn't the boolean `true` or `false`, with `tags` that isn't a list or `series_part` that isn't a positive integer, with two published posts sharing a `series_part`, or with the slug `index` or `feed`, which name the blog's own page and feed. The build reads every post, leaves the drafts out before anything is rendered, and makes the blog's pages, feed, sitemap URLs, structured data and agent files from the published posts alone, so a draft's slug, title or text appears in no built file. A post's reading time is its body's word count (whitespace-separated tokens of the Markdown source, code included and frontmatter left out), divided by 230 and rounded up, at least one minute, worked out at build time and never written in the file.

`assets/mockups/blog.html` is the reference for the blog's look: mockup A for `/blog/` and the post page below it, in the homepage's palette and type. `/blog/` opens with a short heading and a link to the feed, then the published posts as a grid of cards, newest first by `date` (then by `series_part`, highest first, then by slug, A to Z), three across on a wide screen and one at phone widths; each card links to its post and shows its `series_part` as "Part <n>" (when it has one), its date as "5 Nov 2026", its reading time as "<n> min read", its `title`, its `summary` and its `tags`. With no published post it shows the heading and a line saying posts are coming, and no cards. A post's page at `/blog/<slug>` shows its `title`, its `summary` as the lede, its date, reading time and tags, then its body; beside the body (below it at phone widths), the series: every published post with a `series_part`, in part order, the page's own marked; under the body, links to the published posts before and after it in part order, where there are any. A post's body may link to another post as `<slug>.md`, which resolves to that post's page when it is published; a link to a draft is rendered as its text alone, on the page and in the post's Markdown copy, and the build doesn't fail on it, so a published post can name the next part before it is out. A link to a slug that is no post at all fails the build, as a dead guide link does.

The feed is `/blog/feed.xml`, RSS 2.0: the channel's title "CLM blog", its link `/blog/` and description, and one item per published post, newest first, with its `title`, its page's URL as `link` and `guid`, its `summary` as `description` and its `date` at 00:00:00 GMT, in RFC 822 form, as `pubDate`. The feed is not in the sitemap. With no published post, `/blog/`'s `Blog` structured data has an empty `blogPost` and the feed has no items.

## Requirements

### Requirement: homepage-first-screen
The homepage's first screen SHALL show the headline "Declare the intent. AI builds the code to match. CLM keeps it matching.", a Get started link to the guide's quickstart, and the loop's four stages Declare, Build, Prove and Reconcile, and SHALL NOT name Kubernetes.

#### Scenario: laptop first load
- GIVEN the built site, opened at `/` in a 1280×800 window and not scrolled
- WHEN a test reads what is in view
- THEN the headline, a Get started link to `/guide/quickstart` and the four stages are in view, and no text in view contains "Kubernetes"

### Requirement: homepage-sections
Below its first screen the homepage SHALL hold, in order, the sections The split, The loop, Not a vibe-coded mess, Solo or as a team and Standing on shoulders; The loop SHALL say that CLM isn't a Kubernetes tool, and Standing on shoulders SHALL link to the Spec Growth Engine paper, Kubernetes's controllers, superpowers and Anthropic's AI-native SDLC playbook.

#### Scenario: sections and credits
- GIVEN the built homepage
- WHEN a test reads its section headings and links
- THEN the five sections appear in that order, The loop holds "CLM isn't a Kubernetes tool", and Standing on shoulders links to `https://arxiv.org/html/2606.27045v2`, `https://kubernetes.io/docs/concepts/architecture/controller/`, `https://github.com/obra/superpowers` and `https://claude.com/blog/the-ai-native-sdlc-playbook`

### Requirement: guide-pages
The site SHALL have one page under `/guide/` for each page `clm guide` prints, each file of `docs/guide/` and `docs/spec-format.md`, built from that file at build time, with a sidebar listing them in the order `docs/guide/README.md` gives, and every link from one guide file to another SHALL lead to that file's page.

#### Scenario: every file has a page
- GIVEN `docs/guide/` holding `README.md`, `why-clm.md` and `quickstart.md` among others
- WHEN the site is built
- THEN there is a page for each file and for `docs/spec-format.md`, `/guide/` for `README.md`, and the sidebar lists Why CLM before Quickstart

#### Scenario: a new guide file
- GIVEN a build after a file is added to `docs/guide/`
- WHEN a test lists the built guide pages
- THEN the new file has a page, with no edit under `web/site/`

#### Scenario: links between pages
- GIVEN `docs/guide/README.md` linking to `why-clm.md`, and `docs/guide/reference.md` linking to `../spec-format.md`
- WHEN the built pages' links are followed
- THEN they open `/guide/why-clm` and `/guide/spec-format`

#### Scenario: dead link
- GIVEN a fixture guide whose file links to `missing.md`
- WHEN the site is built from it
- THEN the build fails, naming the link

#### Needs
- ci/web-checks

### Requirement: install-repository
The homepage's install commands SHALL name the repository set by one value in the site's config, by default `clunny/clm`, the public repository, and no other file under `web/site/` SHALL name it, except the blog's posts under `web/site/blog/`, which are written text, as the guide's files are, and may name it in their own install commands.

#### Scenario: repository changed
- GIVEN the site's config value set to `someone/clm-public`
- WHEN the homepage is built
- THEN its install commands read `/plugin marketplace add someone/clm-public`, and no other file under `web/site/` outside `web/site/blog/` contains `clunny/clm`

#### Scenario: the public repository
- GIVEN no `INSTALL_REPO` in the environment
- WHEN the homepage is built
- THEN its install commands read `/plugin marketplace add clunny/clm`

### Requirement: site-phone-layout
At a phone's width the homepage and the guide SHALL fit the screen without scrolling sideways, with the homepage's ring below its first screen's text.

#### Scenario: phone
- GIVEN the built site in a 390-pixel-wide window
- WHEN `/` and `/guide/quickstart` are opened
- THEN neither page is wider than the window, and on `/` the ring is below the headline

### Requirement: site-no-install-scripts
`web/site/.npmrc` SHALL set `ignore-scripts=true`, so installing the site's dependencies never runs a dependency's install script.

#### Scenario: npmrc
- GIVEN the repository
- WHEN a test reads `web/site/.npmrc`
- THEN it sets `ignore-scripts=true`

#### Serves
- architecture/deterministic-enforcement

### Requirement: analytics-script
When the build's environment sets `UMAMI_WEBSITE_ID`, every page of the built site SHALL load Umami's script from `https://cloud.umami.is/script.js` with `data-website-id` that ID and `data-do-not-track="true"`; when it is unset or empty, no built file SHALL name Umami.

#### Scenario: released build
- GIVEN `UMAMI_WEBSITE_ID=test-id` in the build's environment
- WHEN the site is built and a test reads `index.html`, `guide/quickstart.html` and `privacy.html`
- THEN each holds one script tag with `src="https://cloud.umami.is/script.js"`, `data-website-id="test-id"` and `data-do-not-track="true"`

#### Scenario: no website ID
- GIVEN no `UMAMI_WEBSITE_ID` in the build's environment
- WHEN the site is built
- THEN no file of the built site contains `umami`

#### Serves
- capability/see-site-use

### Requirement: click-events
A click on Get started, How it works, Web app or a nav link SHALL call `window.umami.track("click", { label, path })` with the element's text and the page's path, and a click on a link to another host SHALL call `window.umami.track("outbound", { url, path })`; when `window.umami` is missing, a click SHALL do what it does without analytics, with no error.

#### Scenario: Get started
- GIVEN the built homepage with a stub `window.umami.track` that records its calls, and navigation held
- WHEN the visitor clicks Get started
- THEN the stub was called with `"click"` and `{ label: "Get started", path: "/" }`, and the page goes on to `/guide/quickstart`

#### Scenario: Web app
- GIVEN the built homepage with the stub, and navigation held
- WHEN the visitor clicks the nav's Web app
- THEN the stub was called with `"click"` and `{ label: "Web app", path: "/" }`

#### Scenario: outbound link
- GIVEN the built homepage with the stub
- WHEN the visitor clicks the superpowers link
- THEN the stub was called with `"outbound"` and `{ url: "https://github.com/obra/superpowers", path: "/" }`

#### Scenario: blocked
- GIVEN the built homepage with no `window.umami`
- WHEN the visitor clicks Get started
- THEN the page goes to `/guide/quickstart` and the console shows no error

#### Serves
- capability/see-site-use

### Requirement: copy-command-event
Copying a command, by a code block's copy button or by copying a selection inside the homepage's install commands or a guide code block, SHALL call `window.umami.track("copy-command", { path, command })` with the page's path and the command's first line, trimmed.

#### Scenario: copy button in the guide
- GIVEN the built `/guide/quickstart` with the stub, its first code block starting `/plugin marketplace add clunny/clm`
- WHEN the visitor clicks that block's copy button
- THEN the stub was called with `"copy-command"` and `{ path: "/guide/quickstart", command: "/plugin marketplace add clunny/clm" }`

#### Scenario: selection copied on the homepage
- GIVEN the built homepage with the stub
- WHEN the visitor selects the first install command and copies it
- THEN the stub was called with `"copy-command"`, `path` `"/"` and `command` that command

#### Serves
- capability/see-site-use

### Requirement: privacy-page
The site SHALL have a page at `/privacy` saying that it sets no cookies, counts page views, clicks and copied commands with Umami Cloud, stores no IP address or personal data, honours Do Not Track and doesn't measure the app, and the footer of the homepage and of every guide page SHALL link to it.

#### Scenario: footer link
- GIVEN the built site
- WHEN a test opens `/` and `/guide/quickstart` and follows each footer's Privacy link
- THEN both open `/privacy`, which contains "cookies", "Umami" and "Do Not Track"

#### Serves
- capability/see-site-use

### Requirement: readable-without-script
The built site SHALL hold in its static HTML, with no script run, the homepage's headline, its section headings, its install commands and its GitHub link, and each guide page's title and text.

#### Scenario: script off
- GIVEN the built site opened in a browser with JavaScript disabled
- WHEN a test reads `/` and `/guide/quickstart`
- THEN `/` shows the headline, the five section headings, `/plugin marketplace add clunny/clm` and a link to `https://github.com/clunny/clm`, and `/guide/quickstart` shows the quickstart's title and its install commands

### Requirement: page-metadata
Every page of the built site but the 404 page SHALL have a `<title>`, a `<meta name="description">`, a canonical link to its clean URL on `https://clm-app.dev`, and Open Graph and Twitter card tags with the page's own title and description and the site's social card; a guide page's description SHALL be its file's summary paragraph as plain text and a post's page's its `summary`, a post's page SHALL have `og:type` `article` and a `<title>` of its title followed by " | CLM blog", and the build SHALL fail, naming the file, when a guide file's first block after its title isn't a paragraph.

#### Scenario: a guide page's head
- GIVEN `docs/guide/quickstart.md`, whose first paragraph is "From nothing to your first change built, for a new project or an existing one."
- WHEN the site is built and a test reads `/guide/quickstart`'s head
- THEN its title is "Quickstart | CLM guide", its description and `og:description` are that sentence, its canonical link and `og:url` are `https://clm-app.dev/guide/quickstart`, `og:image` names a 1200×630 PNG that the build holds, `twitter:card` is `summary_large_image`, and an alternate link of type `text/markdown` names `https://clm-app.dev/guide/quickstart.md`

#### Scenario: Markdown removed
- GIVEN a fixture guide file whose summary paragraph holds `` `clm guide` `` and a link `[Why CLM](why-clm.md)`
- WHEN the site is built from it
- THEN its page's description reads `clm guide` and `Why CLM` with no backticks, brackets or link target

#### Scenario: no summary
- GIVEN a fixture guide file whose title is followed by a list
- WHEN the site is built from it
- THEN the build fails, naming that file

#### Scenario: homepage and privacy
- GIVEN the built site
- WHEN a test reads the heads of `/` and `/privacy`
- THEN each has a title, a non-empty description and a canonical link, `https://clm-app.dev/` and `https://clm-app.dev/privacy`, no two pages of the site share a title, and `404.html` holds a `noindex` robots meta and no canonical link

#### Scenario: a post's head
- GIVEN a fixture published post `alpha` titled "Alpha" with the summary "First post."
- WHEN the site is built and a test reads `/blog/alpha`'s head
- THEN its title is "Alpha | CLM blog", its description and `og:description` are "First post.", its canonical link is `https://clm-app.dev/blog/alpha`, `og:type` is `article`, and an alternate link of type `text/markdown` names `https://clm-app.dev/blog/alpha.md`

### Requirement: crawler-files
The built site SHALL hold `robots.txt` with one `User-agent: *` group with no `Allow` line (what isn't disallowed is allowed), disallowing `/app/`, `/api/` and `/auth/`, followed by `Sitemap: https://clm-app.dev/sitemap.xml`, and `sitemap.xml` listing the homepage, every guide page, `/privacy`, `/blog/` and every published post on `https://clm-app.dev`, and no URL under `/app/` and no draft's.

#### Scenario: robots
- GIVEN the built site
- WHEN a test reads `robots.txt`
- THEN it has exactly one `User-agent` line, `User-agent: *`, it disallows `/app/`, `/api/` and `/auth/` and nothing else, and it names `https://clm-app.dev/sitemap.xml`

#### Scenario: sitemap
- GIVEN the built site
- WHEN a test reads `sitemap.xml`
- THEN it lists `https://clm-app.dev/`, `https://clm-app.dev/guide/quickstart`, `https://clm-app.dev/guide/spec-format` and `https://clm-app.dev/privacy`, one URL for each built guide page, and no URL containing `/app/` or `404`

#### Scenario: posts in the sitemap
- GIVEN a fixture blog with a published post `alpha` and a draft `quokka-draft`
- WHEN the site is built and a test reads `sitemap.xml`
- THEN it lists `https://clm-app.dev/blog/` and `https://clm-app.dev/blog/alpha`, and no URL containing `quokka`

### Requirement: structured-data
The homepage SHALL hold JSON-LD describing CLM as a `SoftwareApplication`, free, MIT-licensed, with `sameAs` the GitHub repository, and `/guide/faq` SHALL hold JSON-LD describing an `FAQPage` whose questions are the `##` headings of `docs/guide/faq.md`, in order, each answered by the text under it; `/blog/` SHALL hold JSON-LD describing a `Blog` listing the published posts, and each post's page JSON-LD describing a `BlogPosting` with its title, summary, date and tags.

#### Scenario: the homepage
- GIVEN the built homepage
- WHEN a test parses its `application/ld+json` script
- THEN it is a `SoftwareApplication` named `CLM` with `url` `https://clm-app.dev/`, `offers.price` `0`, a `license` naming MIT and `sameAs` holding `https://github.com/clunny/clm`

#### Scenario: the FAQ follows its file
- GIVEN a fixture guide whose `faq.md` has the headings "What is CLM?" and "Is it free?", each with a paragraph under it
- WHEN the site is built from it and a test parses `/guide/faq`'s `application/ld+json` script
- THEN it is an `FAQPage` with exactly those two questions, in that order, each answer the plain text of its paragraph

#### Scenario: a post
- GIVEN a fixture published post `alpha` titled "Alpha", dated 2026-11-05, tagged `testing`
- WHEN the site is built and a test parses `/blog/alpha`'s `application/ld+json` script
- THEN it is a `BlogPosting` with `headline` `Alpha`, `datePublished` `2026-11-05`, `keywords` holding `testing` and `url` `https://clm-app.dev/blog/alpha`, and `/blog/`'s is a `Blog` whose `blogPost` lists `alpha` alone when every other post is a draft

### Requirement: agent-markdown
The built site SHALL hold, made at build time from the guide's files, a Markdown copy of each guide page at its page's URL with `.md` added (`/guide/index.md` for `/guide/`), whose links to other guide files name their Markdown copies as absolute URLs; `llms.txt` listing every copy in the sidebar's order with its title and summary, then, under `## Blog`, the Markdown copy at `/blog/<slug>.md` of every published post, newest first, with its title and summary; and `llms-full.txt` holding every copy, in that order. None of them SHALL be committed.

#### Scenario: each copy is its file
- GIVEN the built site
- WHEN a test compares each guide file with its Markdown copy, after rewriting the file's links as the build does
- THEN they are equal, and `/guide/reference.md` links to `https://clm-app.dev/guide/spec-format.md`

#### Scenario: llms.txt
- GIVEN the built site
- WHEN a test reads `llms.txt`
- THEN it opens with `# CLM` and a `>` line, holds `/plugin marketplace add clunny/clm`, lists `https://clm-app.dev/guide/why-clm.md` before `https://clm-app.dev/guide/quickstart.md`, each with its page's summary, and links `https://clm-app.dev/llms-full.txt`

#### Scenario: the full text can't drift
- GIVEN a fixture guide built once, then a line added to one of its files and a new file linked from its `README.md`, and built again
- WHEN a test reads `llms-full.txt` and `llms.txt` after each build
- THEN after the second build `llms-full.txt` holds the added line and the new file's text, `llms.txt` lists the new file, and nothing under `web/site/` was edited

#### Scenario: not committed
- GIVEN the repository
- WHEN a test lists the files git tracks
- THEN none is named `llms.txt` or `llms-full.txt`, and no `.md` file is tracked under `web/site/public/`

#### Scenario: published posts only
- GIVEN a fixture blog with a published post `alpha`, whose body links `quokka-draft.md`, and a draft `quokka-draft`
- WHEN the site is built and a test reads `llms.txt`
- THEN its `## Blog` section lists `https://clm-app.dev/blog/alpha.md` with its summary, `/blog/alpha.md` is `alpha`'s text under a `#` line with its title, the link to the draft left as its text, and no built file names `quokka`

### Requirement: github-link
The nav of every page, and the homepage's first screen, SHALL link to `https://github.com/<repo>`, `<repo>` being the install repository, labelled with the repository's star count read at build time from `GITHUB_STARS` or GitHub's API; when the count can't be read the build SHALL still succeed, showing the link without a count. Wherever the nav bar shows its links (not in the menu VitePress opens at narrow widths), each link SHALL show GitHub's mark beside its label on one line, the mark before the label, centred on the line with the nav's other links or, on the homepage, with the first screen's other buttons; the label `GitHub`, shown without a count, is laid out the same way.

#### Scenario: stars given
- GIVEN `GITHUB_STARS=1234` in the build's environment
- WHEN the site is built and `/` is opened in a 1280×800 window, not scrolled
- THEN a link to `https://github.com/clunny/clm` reading `1.2k` is in view on the first screen, and `/guide/quickstart`'s nav holds a link to it reading `1.2k`

#### Scenario: the mark sits beside the count
- GIVEN `GITHUB_STARS=1234`
- WHEN the site is built and `/` and `/guide/quickstart` are each opened in a 1280×800 window
- THEN, measuring elements' bounding boxes (the link's `svg` for the mark, the element holding its label for the count), on both pages the nav's GitHub link, and on `/` the first screen's GitHub button, each have the mark's right edge at or left of the count's left edge and the mark's and the count's vertical centres within 2px of each other
- AND on both pages the nav GitHub link's count has its vertical centre within 2px of the label of the nav's Guide link, and on `/` the GitHub button's vertical centre is within 2px of the Get started button's

#### Scenario: a small count
- GIVEN `GITHUB_STARS=87`
- WHEN the site is built
- THEN the links read `87`

#### Scenario: GitHub unreachable
- GIVEN no `GITHUB_STARS` and the GitHub API address unreachable from the build
- WHEN the site is built
- THEN the build succeeds and prints a warning, and the nav and first screen link to the repository with no count

#### Scenario: another repository
- GIVEN `INSTALL_REPO=someone/clm-public` and `GITHUB_STARS=5`
- WHEN the site is built
- THEN the GitHub links name `https://github.com/someone/clm-public`

### Requirement: nav-web-app
The nav of every page SHALL hold, in order, How it works, Guide, Blog, Get started and Web app, Web app linking to `/app/`. Every link on the site to `/app/`, the homepage's footer's included, SHALL read Web app, and no link SHALL read Sign in.

#### Scenario: homepage and a guide page
- GIVEN the built site
- WHEN a test reads the links on `/` and on `/guide/quickstart`
- THEN on both the nav's link reading `Web app` leads to `/app/` and comes after Get started, every link to `/app/` reads `Web app`, and no link reads `Sign in`

#### Serves
- capability/learn-clm

### Requirement: site-worker
`web/site/wrangler.toml` SHALL declare the site's Worker `clm-site` with no `main`, the `clm-app.dev` Custom Domain as its one route, and static assets from the site's build output with `not_found_handling` `"404-page"`.

#### Scenario: the site's Worker
- GIVEN the repository
- WHEN a test reads `web/site/wrangler.toml`
- THEN `name` is `clm-site`, it has no `main`, its one route is `clm-app.dev` with `custom_domain` true, and its assets' `not_found_handling` is `"404-page"` and `directory` is where `npm run build` writes the site by default

#### Serves
- capability/publish-posts
- capability/learn-clm

### Requirement: blog-drafts-left-out
The site's build SHALL read every post in `web/site/blog/`, treat one whose `draft` is `true` or missing as a draft, and build the blog from the other posts alone, so that no built file holds a draft's slug, title or text; it SHALL fail, naming the file, on a post missing `title`, `summary` or `date`, with a `date` that isn't a `YYYY-MM-DD` date, or with a `draft` that isn't `true` or `false`.

#### Scenario: a draft leaves no trace
- GIVEN a fixture blog with a published post `alpha` and a draft `quokka-draft` titled "Quokka draft title", whose body holds the word `zebraphrase`
- WHEN the site is built from it and a test searches every built file
- THEN `/blog/alpha` exists, no file path contains `quokka`, and no file holds `quokka`, `Quokka draft title` or `zebraphrase`

#### Scenario: no draft key
- GIVEN a fixture post with no `draft` key
- WHEN the site is built
- THEN it is left out as a draft is

#### Scenario: a bad post
- GIVEN a fixture post with `date: soon`
- WHEN the site is built
- THEN the build fails, naming that file

#### Serves
- capability/publish-posts

### Requirement: blog-index
`/blog/` SHALL show every published post as a card linking to its page, newest first, each card holding its part (when it has a `series_part`), its date, its reading time, its title, its summary and its tags, and the nav of every page SHALL link to it as Blog, after Guide.

#### Scenario: cards in order
- GIVEN a fixture blog with published posts dated 2026-11-05 (part 1, 230 words), 2026-11-08 (part 2, 231 words) and a draft dated 2026-11-12
- WHEN the site is built and a test reads `/blog/`
- THEN it holds two cards, part 2's first, part 2's showing "Part 2", "8 Nov 2026", "2 min read", its title, summary and tags, and part 1's "1 min read"

#### Scenario: nothing published
- GIVEN a fixture blog holding only drafts
- WHEN the site is built
- THEN `/blog/` exists, holds no card, and says posts are coming

#### Scenario: phone width
- GIVEN the built `/blog/` with three published posts
- WHEN it is opened 390 px wide
- THEN its cards stand in one column and nothing scrolls sideways

#### Serves
- capability/publish-posts

### Requirement: blog-post-page
Each published post SHALL have a page at `/blog/<slug>`, readable without script, holding its title, its summary, its date, its reading time, its tags and its body, the published posts of its series in part order with its own marked, and links to the published posts before and after it in part order; a link in its body to a draft's file SHALL be rendered as its text alone.

#### Scenario: the series skips drafts
- GIVEN a fixture series of parts 1, 2 and 3, part 3 a draft, and part 2's body ending with a link `[Part three](gamma.md)` to part 3's file
- WHEN the site is built and `/blog/beta` (part 2) is opened with JavaScript disabled
- THEN it shows its title, summary, date, reading time, tags and body, its series lists parts 1 and 2 with part 2 marked, it links to part 1 as the previous post and to no next post, and "Part three" is text with no link

#### Scenario: a link to a published post
- GIVEN part 1's body linking to `beta.md`
- WHEN the site is built
- THEN on `/blog/alpha` that link leads to `/blog/beta`

#### Serves
- capability/publish-posts

### Requirement: blog-feed
The build SHALL write `/blog/feed.xml`, an RSS 2.0 feed with one item per published post, newest first, each with its title, its page's URL on the site's address as link and guid, its summary as description and its date as `pubDate`, and `/blog/` and every post's page SHALL link it as `application/rss+xml`.

#### Scenario: the feed
- GIVEN a fixture blog with published posts `alpha` (2026-11-05) and `beta` (2026-11-08) and a draft `quokka-draft`
- WHEN the site is built and a test parses `/blog/feed.xml`
- THEN it is RSS 2.0 with two items, `beta`'s first, whose link is `https://clm-app.dev/blog/beta` and whose `pubDate` is `Sun, 08 Nov 2026 00:00:00 GMT`, and `/blog/`'s head links the feed

#### Serves
- capability/publish-posts
- capability/learn-clm
