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
CLM's public site at `https://clm-app.dev`: the homepage at `/` and the user guide at `/guide/`, built with VitePress from `web/site/` and the pages `clm guide` prints, `docs/guide/*.md` and `docs/spec-format.md` (`public-site`, `guide-in-the-binary`). `web`'s build runs it and puts its output at the root of `web/dist/`, beside the app under `web/dist/app/`; the same deployment holds both, and its assets serve the site to everyone, with no session and no Worker request (`web/site-served-directly`, `invite-only-web`). The site links to the app at `/app/` and reads nothing from the Worker's API. The released site reports page views, clicks and copied commands to Umami Cloud, cookieless and honouring Do Not Track, and only when its build is given a website ID; `/privacy` says what is collected (`site-analytics-umami`). Every page is readable without script and carries its own title, description, canonical link and social card; the build writes `robots.txt`, `sitemap.xml`, structured data, and for agents a Markdown copy of each guide page with `llms.txt` and `llms-full.txt`, all made from the guide's files so none can fall behind them (`site-found-by-search-and-agents`). The nav and the homepage link to the GitHub repository with its star count, read at build time.

## Design
**Build.** A package of its own in `web/site/`: its `package.json` with VitePress and its build and test scripts, its committed `package-lock.json`, a `.npmrc` setting `ignore-scripts=true` (`unattended-commands`), its VitePress config and theme under `web/site/.vitepress/`, the homepage as a custom layout, and its tests with their own configs. Its own package keeps VitePress's Vite and Vue apart from the app's Vite 8. `npm run build` there writes the site to the directory `SITE_OUT` names (default `web/site/dist/`); `web`'s build runs `npm ci --ignore-scripts` and that build with `SITE_OUT` at `web/dist/` before the app's (`web/app-at-app-path`), so the site never empties the app's output. `npm test` runs its tests, which `ci-check.sh` runs after `web/`'s suite (`ci/web-checks`).

**The guide's pages** are the pages `clm guide` prints: each file of `docs/guide/` at `/guide/<name>`, `README.md` at `/guide/`, and `docs/spec-format.md` at `/guide/spec-format`, which the guide's files link to as `../spec-format.md`. They are read in place at build time (VitePress's `srcDir` and `rewrites`, a Vite alias or a loader, the builder's choice), never copied into the repository: they stay the only copy (`guide-from-docs`). Links between them resolve to their pages, and the build fails on a dead link. The sidebar lists them in the order `docs/guide/README.md` links them, labelled with the link text, and `spec-format` last. The directory the guide is read from and the install repository (below) can be set by environment variables, `GUIDE_DIR` and `INSTALL_REPO`, so tests can build with a fixture guide and another repository; `UMAMI_WEBSITE_ID` turns analytics on and `GITHUB_STARS` sets the star count (below). Every guide file opens, after its `#` title, with a summary paragraph of a sentence or two saying what the page is for: it is the page's description (below), and the build fails, naming the file, on a guide file whose first block after the title isn't a paragraph.

**Homepage.** `assets/mockups/homepage.html` is the reference for layout, colour, type and copy; the dashed line in it marks roughly where a laptop's first screen ends. Its copy is the starting text: wording may be polished, the message and sections may not change without a spec change. The first screen holds the headline "Declare the intent. AI builds the code to match. CLM keeps it matching.", one line on building solo or as a team with AI writing every line while the project stays maintainable, **Get started** (to the quickstart), **How it works** (to the sections below) and **GitHub** (to the repository, with its star count, below), the install commands in small type, and the loop drawn as a ring around "your intent": Declare (you approve, yes or no), Build (AI, test-first), Prove (a test cites every rule), Reconcile (drift is rebuilt). Kubernetes isn't named on the first screen. Then five sections, in order:
1. *The split*: you own the what (architecture, capabilities, one yes or no per change); AI owns the how.
2. *The loop*: "Reconciliation, borrowed from Kubernetes", opening with "CLM isn't a Kubernetes tool. It borrows one idea from it", then Kubernetes and CLM side by side.
3. *Not a vibe-coded mess*: every rule has a test, drift can't land, nothing dead.
4. *Solo or as a team*: git is the only channel.
5. *Standing on shoulders*: the Spec Growth Engine paper ([arXiv:2606.27045](https://arxiv.org/html/2606.27045v2)), Kubernetes's [controllers](https://kubernetes.io/docs/concepts/architecture/controller/), [superpowers](https://github.com/obra/superpowers) and Anthropic's [AI-native SDLC playbook](https://claude.com/blog/the-ai-native-sdlc-playbook), each with what CLM took from it, as `docs/guide/why-clm.md` says.
It ends with the install commands and a link to the quickstart, and a footer "CLM is built with CLM" with links to the guide, the web app (reading Web app) and `/privacy`; the guide's pages, whose default layout has no footer, get a small footer with the Privacy link through a slot of VitePress's layout in the theme. The nav holds How it works, Guide, Get started and Web app (to `/app/`, the invite-only web app), then the GitHub link (below). The mockup predates the GitHub button and the fourth credit; they follow its button and credit styles.

**The install repository.** The repository the homepage's install commands name (`/plugin marketplace add <repo>`) is one value in the site's config (overridable by `INSTALL_REPO` for tests): `clunny/clm`, which is public, so a visitor adopts CLM on their own (`public-site`).

**Look.** Inter for text, JetBrains Mono for commands; the app's indigo accent with a sky gradient, the mockup's dark palette and a light one. The site follows the system's light or dark setting, with VitePress's own switch kept. At phone widths the ring sits below the first screen's text, the side-by-side panels stack, and nothing scrolls sideways.

**Analytics** (`site-analytics-umami`). When the build's environment sets `UMAMI_WEBSITE_ID`, every page's head holds `<script defer src="https://cloud.umami.is/script.js" data-website-id="<id>" data-do-not-track="true">`; when it is unset or empty, no built file names Umami, so tests, local builds and forks report nothing, and the operator's ID lives only in the repository's Actions variable (`ci/web-deploy`). Umami's script records page views itself, VitePress's in-site navigation included. One module of the theme records the rest by calling `window.umami.track(name, data)`, doing nothing when `window.umami` is missing (blocked, Do Not Track, or a build without an ID), and never delaying a click's navigation:
- a click on a same-host link in the nav or among the homepage's buttons (Get started, How it works, Guide, Web app): the event `click`, with `label` the element's text and `path` the page's;
- a click on any link to another host: the event `outbound` only, with `url` the link's and `path` the page's;
- a copied command, by a code block's copy button or by copying a selection inside a command (the homepage's install commands, a guide code block): the event `copy-command`, with `path` the page's and `command` the command's first line, trimmed. VitePress's copy button writes to the clipboard without a `copy` event, so it is caught by its click, and a selection by the document's `copy` event when the selection lies inside a `pre` or code block. The script tag is added per page in the config's head (`transformHead` or `head`, the builder's choice), so the 404 page carries it too.
The app under `/app/` is a build of its own and loads no analytics. The site serves no cookie and the theme stores nothing in the browser for analytics. The funnel and the undercount (judged against Cloudflare's own traffic numbers for the zone) are read in the dashboards, not in code.

**Privacy page.** `/privacy`, a Markdown page of the site's own under `web/site/`, not part of the guide and listed in neither the nav nor the sidebar, says in plain words: the site sets no cookies; it counts page views, clicks and copied commands with Umami Cloud, which stores no IP address and no personal data; Do Not Track is honoured; and the app under `/app/` isn't measured.

**The site's address** is one value in the site's config, `https://clm-app.dev`, from which every absolute URL the build writes is made: canonical links, the sitemap, Open Graph tags, structured data, `robots.txt` and the agent files.

**Search** (`site-found-by-search-and-agents`). The site is built to be read without running script: VitePress renders every page, the homepage's custom layout included, to static HTML, and nothing a crawler should read (headings, copy, links, install commands, star count) is added in the browser alone. Every page's head holds:
- a `<title>`: the homepage's is "CLM: declare the intent, AI builds the code to match"; a guide page's is its `#` title followed by " | CLM guide", except `/guide/`'s, which is "CLM guide"; `/privacy`'s is "Privacy | CLM"; the 404 page's is "Page not found | CLM";
- `<meta name="description">`: a guide page's summary paragraph (above), as plain text with its Markdown removed; the homepage's is its headline followed by its first screen's line, both one constant each in the site's config that `Home.vue`, the heads and `llms.txt` all read; `/privacy`'s is its own first paragraph;
- `<link rel="canonical">` to the page's clean URL on the site's address; the 404 page has none and holds `<meta name="robots" content="noindex">` instead;
- Open Graph and Twitter card tags (`og:title`, `og:description`, `og:url`, `og:type` `website`, `og:site_name` `CLM`, `og:image`, `twitter:card` `summary_large_image`), the title and description being the page's own, and the image one social card for the whole site: a 1200×630 PNG under `web/site/public/`, made once from the homepage's first screen (the headline, the ring and the site's address), for instance by a Playwright capture, and committed; the build doesn't regenerate it;
- on a guide page, `<link rel="alternate" type="text/markdown">` to its Markdown copy (below).

The build writes `sitemap.xml` (VitePress's sitemap, on the site's address), listing the homepage, every guide page and `/privacy`, and no page under `/app/`, and `robots.txt`: one group for every crawler, `User-agent: *`, with no `Allow` line (what isn't disallowed is allowed), disallowing `/app/`, `/api/` and `/auth/`, the paths that belong to the invite-only app (`invite-only-web`), then the `Sitemap:` line. One group for everyone welcomes the AI crawlers too, search's (OAI-SearchBot, Claude-SearchBot, PerplexityBot) and training's (GPTBot, ClaudeBot) alike: a named group would replace the `*` group for that crawler and need its own copy of the disallows. Cloudflare can add its own blocks in front of the zone's `robots.txt`: the operator keeps the zone's AI crawler blocking (**Block AI bots**, and every crawler in **AI Crawl Control**) and Cloudflare's managed `robots.txt` off, so the site's own `robots.txt` is what every crawler reads. It is a zone setting, not code.

**Structured data.** The homepage holds one `<script type="application/ld+json">` describing CLM as a `SoftwareApplication`: `name` `CLM`, the homepage's description, `url` the site's address, `applicationCategory` `DeveloperApplication`, `operatingSystem` `Linux, macOS`, `offers` a price of `0`, `license` the MIT licence's URL, and `sameAs` the GitHub repository's URL. `/guide/faq` holds one describing an `FAQPage`, made at build time from `docs/guide/faq.md`: each `##` heading is a `Question` and the Markdown up to the next heading its `acceptedAnswer`, as plain text. A guide with no `faq.md` (a test's fixture) gets no `FAQPage` and the build doesn't fail. Both are made from the files and the config, never written by hand, so they can't fall behind the page.

**Agents.** The build writes, from the same files the guide's pages are built from and never committed (`guide-from-docs`):
- a Markdown copy of each guide page beside its HTML: `/guide/<name>.md`, `/guide/index.md` for `README.md` and `/guide/spec-format.md`, each the source file's text with every link to another guide file rewritten to that file's Markdown copy, as an absolute URL on the site's address, and every other relative link (into the repository) rewritten to it on the GitHub repository's `main`, `/blob/main/<path>` for a file and `/tree/main/<path>` for a directory. Only inline Markdown links outside fenced code are rewritten, and a link's `#anchor` is kept. The copies' text is otherwise the file's own, so a build with another `INSTALL_REPO` still shows the guide's own commands;
- `llms.txt`, in the `llmstxt.org` format: `# CLM`, a `>` line with the homepage's description, a paragraph saying what CLM is and how to install it (the homepage's install commands), a `## Guide` section listing every guide page's Markdown copy in the sidebar's order as `- [<title>](<url>): <summary>`, and a `## Optional` section linking `llms-full.txt` and the GitHub repository;
- `llms-full.txt`: every guide page's Markdown copy, in the sidebar's order, each preceded by a line `<!-- <url of its page> -->` and separated by a line `---`.
A guide file added, edited or removed changes them at the next build with no edit under `web/site/`, as it changes the pages.

**GitHub link.** The GitHub repository is the install repository (above): its URL is `https://github.com/<repo>` with no second value naming it. The nav ends with a GitHub icon linking to it, labelled with the star count, and the first screen's **GitHub** button shows the same. The count is read once per build: `GITHUB_STARS` when the environment sets it (tests set it, so they never call the network), otherwise the repository's `stargazers_count` from `<api>/repos/<repo>`, `<api>` being `GITHUB_API` when the environment sets it (tests point it at an address that refuses connections) and `https://api.github.com` otherwise, sent with `GITHUB_TOKEN` as a bearer token when the environment holds one (`ci/web-deploy` passes the workflow's), with a timeout of 10 seconds. A failed or timed-out read never fails the build: the link and button show without a count, and the build prints a warning. The count is written into the HTML at build time, so crawlers read it and visitors' browsers make no request to GitHub; it is shown as the number under 1000 and, from 1000, as thousands rounded to one decimal (`1.0k`, `1.2k` for 1234, `2.0k` for 1950), and is as old as the last release. Every test build sets `GITHUB_STARS` or `GITHUB_API` (the Playwright config's web server included), so no test waits on the network. The GitHub button doesn't take the classes of the first screen's Get started button, which the first-screen test selects.

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
The homepage's install commands SHALL name the repository set by one value in the site's config, by default `clunny/clm`, the public repository, and no other file under `web/site/` SHALL name it.

#### Scenario: repository changed
- GIVEN the site's config value set to `someone/clm-public`
- WHEN the homepage is built
- THEN its install commands read `/plugin marketplace add someone/clm-public`, and no other file under `web/site/` contains `clunny/clm`

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
Every page of the built site but the 404 page SHALL have a `<title>`, a `<meta name="description">`, a canonical link to its clean URL on `https://clm-app.dev`, and Open Graph and Twitter card tags with the page's own title and description and the site's social card; a guide page's description SHALL be its file's summary paragraph as plain text, and the build SHALL fail, naming the file, when a guide file's first block after its title isn't a paragraph.

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

### Requirement: crawler-files
The built site SHALL hold `robots.txt` with one `User-agent: *` group with no `Allow` line (what isn't disallowed is allowed), disallowing `/app/`, `/api/` and `/auth/`, followed by `Sitemap: https://clm-app.dev/sitemap.xml`, and `sitemap.xml` listing the homepage, every guide page and `/privacy` on `https://clm-app.dev`, and no URL under `/app/`.

#### Scenario: robots
- GIVEN the built site
- WHEN a test reads `robots.txt`
- THEN it has exactly one `User-agent` line, `User-agent: *`, it disallows `/app/`, `/api/` and `/auth/` and nothing else, and it names `https://clm-app.dev/sitemap.xml`

#### Scenario: sitemap
- GIVEN the built site
- WHEN a test reads `sitemap.xml`
- THEN it lists `https://clm-app.dev/`, `https://clm-app.dev/guide/quickstart`, `https://clm-app.dev/guide/spec-format` and `https://clm-app.dev/privacy`, one URL for each built guide page, and no URL containing `/app/` or `404`

### Requirement: structured-data
The homepage SHALL hold JSON-LD describing CLM as a `SoftwareApplication`, free, MIT-licensed, with `sameAs` the GitHub repository, and `/guide/faq` SHALL hold JSON-LD describing an `FAQPage` whose questions are the `##` headings of `docs/guide/faq.md`, in order, each answered by the text under it.

#### Scenario: the homepage
- GIVEN the built homepage
- WHEN a test parses its `application/ld+json` script
- THEN it is a `SoftwareApplication` named `CLM` with `url` `https://clm-app.dev/`, `offers.price` `0`, a `license` naming MIT and `sameAs` holding `https://github.com/clunny/clm`

#### Scenario: the FAQ follows its file
- GIVEN a fixture guide whose `faq.md` has the headings "What is CLM?" and "Is it free?", each with a paragraph under it
- WHEN the site is built from it and a test parses `/guide/faq`'s `application/ld+json` script
- THEN it is an `FAQPage` with exactly those two questions, in that order, each answer the plain text of its paragraph

### Requirement: agent-markdown
The built site SHALL hold, made at build time from the guide's files, a Markdown copy of each guide page at its page's URL with `.md` added (`/guide/index.md` for `/guide/`), whose links to other guide files name their Markdown copies as absolute URLs; `llms.txt` listing every copy in the sidebar's order with its title and summary; and `llms-full.txt` holding every copy, in that order. None of them SHALL be committed.

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
The nav of every page SHALL hold, in order, How it works, Guide, Get started and Web app, Web app linking to `/app/`. Every link on the site to `/app/`, the homepage's footer's included, SHALL read Web app, and no link SHALL read Sign in.

#### Scenario: homepage and a guide page
- GIVEN the built site
- WHEN a test reads the links on `/` and on `/guide/quickstart`
- THEN on both the nav's link reading `Web app` leads to `/app/` and comes after Get started, every link to `/app/` reads `Web app`, and no link reads `Sign in`

#### Serves
- capability/learn-clm
