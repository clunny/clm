---
component: web.site
create: false
remove: false
base: { contract: "1dbbf844859d24d0", design: "0f9208e03e79d532" }
---

## MODIFIED Contract
CLM's public site at `https://clm-app.dev`: the homepage at `/` and the user guide at `/guide/`, built with VitePress from `web/site/` and the pages `clm guide` prints, `docs/guide/*.md` and `docs/spec-format.md` (`public-site`, `guide-in-the-binary`). `npm run build` in `web/site/` writes the site to the directory `SITE_OUT` names (default `web/site/dist/`), reading `INSTALL_REPO` (the repository it names), `UMAMI_WEBSITE_ID` (analytics), `GITHUB_STARS`, `GITHUB_API` and `GITHUB_TOKEN` (the star count), and `GUIDE_DIR` and `BLOG_DIR` (where the guide and the posts are read from); it fails on a dead guide link, naming the link, and, naming the file, on a guide file with no `#` title or no summary paragraph, a bad post or a post's link to no post; it never fails on a relative link out of the guide or a post into the repository, which its pages and Markdown copies rewrite to the file on GitHub, nor on a star count it can't read. `npm test` there runs the site's tests. It is deployed as a Cloudflare Worker of its own, `clm-site`, assets only, on the `clm-app.dev` Custom Domain, by the site workflow a human dispatches (`ci/site-deploy`, `site-worker-of-its-own`); the app's Worker answers only the app's paths, ahead of it (`web/site-served-directly`, `invite-only-web`), and the site's assets serve everyone, with no session and no script. The site also holds the blog at `/blog/`, with its RSS 2.0 feed at `/blog/feed.xml`: posts kept as Markdown files in `web/site/blog/`, of which only those not marked as drafts are built, so a draft is never visible, listed or searchable (`blog-drafts-left-out`). The site links to the app at `/app/`, each such link loading the app, and reads nothing from the Worker's API. The released site reports page views, clicks and copied commands to Umami Cloud, cookieless and honouring Do Not Track, and only when its build is given a website ID; `/privacy` says what is collected (`site-analytics-umami`). Every page is readable without script, and every page but the 404 page carries its own title, description, canonical link and social card; the build writes `robots.txt`, `sitemap.xml`, schema.org JSON-LD structured data, and for agents a Markdown copy of each guide page and published post with `llms.txt` and `llms-full.txt`, all made from the guide's files and the published posts so none can fall behind them (`site-found-by-search-and-agents`). The nav and the homepage link to the GitHub repository with its star count, read at build time.

## MODIFIED Design
**Build.** A package of its own in `web/site/`: its `package.json` with VitePress and its build and test scripts, its committed `package-lock.json`, a `.npmrc` setting `ignore-scripts=true` (`unattended-commands`), its VitePress config and theme under `web/site/.vitepress/`, the homepage as a custom layout, and its tests with their own configs. Its own package keeps VitePress's Vite and Vue apart from the app's Vite 8. `web`'s build doesn't build the site: the site workflow builds and deploys it on its own (`ci/site-deploy`). `ci-check.sh` runs `npm test` after `web/`'s suite (`ci/web-checks`).

**The guide's pages** are the pages `clm guide` prints: each file of `docs/guide/` at `/guide/<name>`, `README.md` at `/guide/`, and `docs/spec-format.md` at `/guide/spec-format`, which the guide's files link to as `../spec-format.md`. They are read in place at build time (VitePress's `srcDir` and `rewrites`, a Vite alias or a loader, the builder's choice), never copied into the repository: they stay the only copy (`guide-from-docs`). Links between them resolve to their pages. The sidebar opens with Overview, for `/guide/`, then lists the pages in the order `docs/guide/README.md` links them, labelled with the link text, then the pages it doesn't link, and `spec-format` last (`guide-pages`). The directory the guide is read from and the install repository (below) can be set by environment variables, `GUIDE_DIR` and `INSTALL_REPO`, so tests can build with a fixture guide and another repository; `UMAMI_WEBSITE_ID` turns analytics on and `GITHUB_STARS` sets the star count (below). Every guide file opens, after its `#` title, with a summary paragraph of a sentence or two saying what the page is for: it is the page's description (`page-metadata`).

**Homepage.** `assets/mockups/homepage.html` is the reference for layout, colour, type and copy; the dashed line in it marks roughly where a laptop's first screen ends. Its copy is the starting text: wording may be polished, the message and sections may not change without a spec change. The first screen holds the headline "Declare the intent. AI builds the code to match. CLM keeps it matching.", one line on building solo or as a team with AI writing every line while the project stays maintainable, **Get started** (to the quickstart), **How it works** (to the sections below) and **GitHub** (to the repository, with its star count, below), the install commands in small type, and the loop drawn as a ring around "your intent": Declare (you approve, yes or no), Build (AI, test-first), Prove (a test cites every rule), Reconcile (drift is rebuilt). Kubernetes isn't named on the first screen. Then five sections, in order:
1. *The split*: you own the what (architecture, capabilities, one yes or no per change); AI owns the how.
2. *The loop*: "Reconciliation, borrowed from Kubernetes", opening with "CLM isn't a Kubernetes tool. It borrows one idea from it", then Kubernetes and CLM side by side.
3. *Not a vibe-coded mess*: every rule has a test, drift can't land, nothing dead.
4. *Solo or as a team*: git is the only channel.
5. *Standing on shoulders*: the Spec Growth Engine paper ([arXiv:2606.27045](https://arxiv.org/html/2606.27045v2)), Kubernetes's [controllers](https://kubernetes.io/docs/concepts/architecture/controller/), [superpowers](https://github.com/obra/superpowers) and Anthropic's [AI-native SDLC playbook](https://claude.com/blog/the-ai-native-sdlc-playbook), each with what CLM took from it, as `docs/guide/why-clm.md` says.
It ends with the install commands and a **Read the quickstart** button, and a footer "CLM is built with CLM" with links to the guide, the web app (reading Web app, and loading the app as the nav's link does) and `/privacy`; the guide's pages, whose default layout has no footer, get a small footer with the Privacy link through a slot of VitePress's layout in the theme. The nav holds the site's title, How it works, Guide, Blog, Get started and Web app (to `/app/`, the invite-only web app), then the GitHub link (below). The GitHub button and the fourth credit use the mockup's button and credit styles.

**The install repository.** The repository the homepage's install commands name (`/plugin marketplace add <repo>`) is one value in the site's config (overridable by `INSTALL_REPO` for tests): `clunny/clm`, which is public, so a visitor adopts CLM on their own (`public-site`, `install-repository`).

**Look.** Inter for text, JetBrains Mono for commands; the app's indigo accent with a sky gradient, the mockup's dark palette and a light one. The site follows the system's light or dark setting, with VitePress's own switch kept. At phone widths the ring sits below the first screen's text, the side-by-side panels stack, and nothing scrolls sideways.

**Analytics** (`site-analytics-umami`). When the build's environment sets `UMAMI_WEBSITE_ID`, every page's head holds `<script defer src="https://cloud.umami.is/script.js" data-website-id="<id>" data-do-not-track="true">`; when it is unset or empty, no built file names Umami, so tests, local builds and forks report nothing, and the operator's ID lives only in the repository's Actions variable (`ci/site-deploy`). Umami's script records page views itself, VitePress's in-site navigation included. One module of the theme records the rest by calling `window.umami.track(name, data)`, doing nothing when `window.umami` or its `track` is missing (blocked, Do Not Track, or a build without an ID) and catching what `track` throws, and never delaying a click's navigation:
- a click on a same-host link in the nav (the site's title, How it works, Guide, Blog, Get started, Web app) or among the homepage's buttons (Get started, How it works, Read the quickstart): the event `click`, with `label` the link's text less a trailing arrow and `path` the page's as it was when clicked;
- a click on an `http` or `https` link to another host, the homepage's GitHub button and the nav's GitHub link among them: the event `outbound` only, with `url` the link's and `path` the page's;
- a copied command, by a code block's copy button or by copying a selection that lies inside a code block or inline code (the homepage's install commands, a guide code block): the event `copy-command`, with `path` the page's and `command` the command's first line, trimmed. VitePress's copy button writes to the clipboard without a `copy` event, so it is caught by its click, and a selection by the document's `copy` event when the selection's common ancestor lies inside a `pre` or `code` element. The script tag is added per page in the config's head (`transformHead` or `head`, the builder's choice), so the 404 page carries it too.
The app under `/app/` is a build of its own and loads no analytics. The funnel and the undercount (judged against Cloudflare's own traffic numbers for the zone) are read in the dashboards, not in code.

**Privacy page.** `/privacy`, a Markdown page of the site's own under `web/site/`, not part of the guide, says in plain words: the site sets no cookies; it counts page views, clicks and copied commands with Umami Cloud, which stores no IP address and no personal data; Do Not Track is honoured; and the app under `/app/` isn't measured.

**The site's address** is one value in the site's config, `https://clm-app.dev`, from which every absolute URL the build writes is made: canonical links, the sitemap, Open Graph tags, structured data, `robots.txt` and the agent files.

**Search** (`site-found-by-search-and-agents`). The site is built to be read without running script: VitePress renders every page, the homepage's custom layout included, to static HTML, and nothing a crawler should read (headings, copy, links, install commands, star count) is added in the browser alone. Every page's head holds:
- a `<title>` (`page-metadata` gives each page's); the 404 page's, which VitePress builds without running the config's page hooks, is set by rewriting its built HTML, and in the browser by the theme once the page is up;
- `<meta name="description">`: a guide page's summary paragraph (above), as plain text with its Markdown removed; the homepage's is its headline followed by its first screen's line, both one constant each in the site's config that `Home.vue`, the heads and `llms.txt` all read; `/privacy`'s is its own first paragraph; a post's is its `summary`, and `/blog/`'s one constant in the site's config;
- `<link rel="canonical">` to the page's clean URL on the site's address; the 404 page has none and holds `<meta name="robots" content="noindex">` instead;
- Open Graph and Twitter card tags (`og:title`, `og:description`, `og:url`, `og:type` `website`, `article` on a post's page with `article:published_time` its `date` and, when it has one, `article:modified_time` its `updated`, `og:site_name` `CLM`, `og:image`, `twitter:card` `summary_large_image`), the title and description being the page's own, and the image one social card for the whole site: a 1200×630 PNG under `web/site/public/`, made once from the homepage's first screen (the headline, the ring and the site's address), for instance by a Playwright capture, and committed; the build doesn't regenerate it;
- on a guide page or a post's page, `<link rel="alternate" type="text/markdown">` to its Markdown copy (below); on `/blog/` and every post's page, `<link rel="alternate" type="application/rss+xml">` to the blog's feed.

The build writes `sitemap.xml` (VitePress's sitemap, on the site's address) and `robots.txt`: one group for every crawler, `User-agent: *`, with no `Allow` line (what isn't disallowed is allowed), disallowing `/app/`, `/api/` and `/auth/`, the paths that belong to the invite-only app (`invite-only-web`), then the `Sitemap:` line. One group for everyone welcomes the AI crawlers too, search's (OAI-SearchBot, Claude-SearchBot, PerplexityBot) and training's (GPTBot, ClaudeBot) alike: a named group would replace the `*` group for that crawler and need its own copy of the disallows. Cloudflare can add its own blocks in front of the zone's `robots.txt`: the operator keeps the zone's AI crawler blocking (**Block AI bots**, and every crawler in **AI Crawl Control**) and Cloudflare's managed `robots.txt` off, so the site's own `robots.txt` is what every crawler reads. It is a zone setting, not code.

**Structured data.** The homepage holds one `<script type="application/ld+json">` describing CLM as a `SoftwareApplication`: `name` `CLM`, the homepage's description, `url` the site's address, `applicationCategory` `DeveloperApplication`, `operatingSystem` `Linux, macOS`, `offers` a price of `0`, `license` the MIT licence's URL, and `sameAs` the GitHub repository's URL. `/guide/faq` holds one describing an `FAQPage`, made at build time from `docs/guide/faq.md`: each `##` heading outside fenced code is a `Question` and the Markdown up to the next heading its `acceptedAnswer`, as plain text. `/blog/` holds one describing a `Blog` whose `blogPost` lists the published posts, and each post's page one describing a `BlogPosting`: `headline` its `title`, `description` its `summary`, `datePublished` its `date`, `dateModified` its `updated` (left out when it has none), `keywords` its `tags`, `url` its page, `author` and `publisher` an `Organization` named `CLM`. Both are made from the files and the config, never written by hand, so they can't fall behind the page; each is written with `<` escaped, so no text of a post's can close its script.

**Agents.** The build writes, from the same files the guide's pages are built from, at build time and never committed (`guide-from-docs`):
- a Markdown copy of each guide page beside its HTML: `/guide/<name>.md`, `/guide/index.md` for `README.md` and `/guide/spec-format.md`, each the source file's text with its links rewritten (`agent-markdown`). A link into the repository is told a file from a directory by the repository's own tree, and is rewritten the same way on the guide's and the posts' pages before VitePress reads them, so it never fails the build; a link to another guide file stays relative there, so a dead guide link still does. The copies' text is otherwise the file's own, so a build with another `INSTALL_REPO` still shows the guide's own commands;
- `llms.txt`, in the `llmstxt.org` format: `# CLM`, a `>` line with the homepage's description, a paragraph saying what CLM is and how to install it (the homepage's install commands), a `## Guide` section listing every guide page's Markdown copy in the sidebar's order as `- [<title>](<url>): <summary>`, a `## Blog` section listing every published post's Markdown copy, newest first, the same way, and a `## Optional` section linking `llms-full.txt` and the GitHub repository;
- `llms-full.txt`: every guide page's Markdown copy, in the sidebar's order, each preceded by a line `<!-- <url of its page> -->` and separated by a line `---`.
A guide file added, edited or removed changes them at the next build with no edit under `web/site/`, as it changes the pages. Each published post has a Markdown copy too, at `/blog/<slug>.md`: its file's text without the frontmatter, under a `#` line with its `title`, links rewritten as a guide copy's are, a link to a draft left as its text. `llms-full.txt` holds the guide only.

**GitHub link.** The GitHub repository is the install repository (above): its URL is `https://github.com/<repo>` with no second value naming it. The nav ends with a GitHub icon linking to it, labelled with the star count, and the first screen's **GitHub** button shows the same. The count is read once per build (`github-link`), with a timeout of 10 seconds; `ci/site-deploy` passes the workflow's `GITHUB_TOKEN`. The count is written into the HTML at build time, so crawlers read it and visitors' browsers make no request to GitHub, and is as old as the last site deploy. Every build the site's own tests make sets `GITHUB_STARS` or points `GITHUB_API` at an address of the test's (the Playwright config's web server included), so none of them waits on the network.

**Deploy** (`site-worker-of-its-own`). `web/site/wrangler.toml` declares the site's Worker: `name = "clm-site"`, a `compatibility_date`, no `main` (assets only, so no request to the site runs a script or spends a Worker request), `routes = [{ pattern = "clm-app.dev", custom_domain = true }]`, and `[assets]` with `directory` `./dist`, where `npm run build` writes by default, and `not_found_handling = "404-page"`. The app's Worker (`web`) claims the app's paths on the same hostname as routes, and Cloudflare runs a route's Worker before the Custom Domain's, so those paths never reach the site's (`web/site-served-directly`). The site shows none of the repository data the web app keeps behind sign-in, no repository's specs, backlog or changes (`invite-only-web`); the guide it builds is CLM's own documentation, public by design (`public-site`). It names repositories only in links and commands: CLM's own public one in its install commands, its GitHub links and the blog's posts, and the projects it credits. `wrangler` is a dev dependency of the site's own package, so the site workflow deploys it with `npx wrangler deploy` in `web/site/` (`ci/site-deploy`).

**Blog** (`blog-drafts-left-out`). Each post is `web/site/blog/<slug>.md`, its slug the file name (the directory `BLOG_DIR` names when the environment sets it, so tests build fixture blogs as `GUIDE_DIR` builds fixture guides), with YAML frontmatter: `title`, `summary` (one sentence, its description), `date` (`YYYY-MM-DD`, the day it is published), `tags` (a list of words), `updated` (optional, `YYYY-MM-DD`, the day its text was last changed in a way a reader should know of, such as a corrected claim; a typo fix needn't set it), `series_part` (optional, a positive integer: its place in the series) and `draft` (`true` or `false`). A post with `draft: false` is published; one with `draft: true`, or with no `draft` key, is a draft. The build reads and checks every post once, leaves the drafts out before anything is rendered, and makes the blog's pages, feed, sitemap URLs, structured data and agent files from the published posts alone. A post's reading time is its body's word count (whitespace-separated tokens of the Markdown source, code included and frontmatter left out), divided by 230 and rounded up, at least one minute, worked out at build time and never written in the file.

`assets/mockups/blog.html` is the reference for the blog's look: mockup A for `/blog/` and the post page below it, in the homepage's palette and type. `/blog/` opens with a short heading and a link to the feed, then the published posts as a grid of cards, three across on a wide screen and one at phone widths; each card links to its post and shows its `series_part` as "Part <n>" (when it has one), its date as "5 Nov 2026", its reading time as "<n> min read", its `title`, its `summary` and its `tags`. A post's page at `/blog/<slug>` shows its `title`, its `summary` as the lede, its date, then "Updated 9 Nov 2026" when it has an `updated`, its reading time and tags, then its body; beside the body (below it at phone widths), the series: every published post with a `series_part`, in part order, the page's own marked; under the body, links to the published posts before and after it in part order, where there are any. A post's body may link to another post as `<slug>.md`, which resolves to that post's page when it is published; a link to a draft is left as its text, so a published post can name the next part before it is out.

The feed is `/blog/feed.xml`, RSS 2.0: the channel's title "CLM blog", its link `/blog/` and description, and one item per published post, newest first, with its `title`, its page's URL as `link` and `guid`, its `summary` as `description` and its `date` at 00:00:00 GMT, in RFC 822 form, as `pubDate`.

## MODIFIED Requirements

### Requirement: guide-pages <!-- base: 846e6608b76e4543 -->
The site SHALL have one page under `/guide/` for each page `clm guide` prints, each file of `docs/guide/` and `docs/spec-format.md`, built from that file at build time, with a sidebar listing Overview, for `/guide/`, first, then the pages `docs/guide/README.md` links, in the order it first links them and labelled with that link's text, a link carrying an `#anchor` counted as one to its file, then each page it doesn't link, in file-name order and labelled with its `#` title, and Spec format last; every link from one guide file to another SHALL lead to that file's page, a relative link out of the guide's files SHALL lead to that file on GitHub (`agent-markdown`) and never fail the build, and the build SHALL fail, naming the link, on any other link that leads to no page of the site.

#### Scenario: every file has a page
- GIVEN `docs/guide/` holding `README.md`, `why-clm.md` and `quickstart.md` among others
- WHEN the site is built
- THEN there is a page for each file and for `docs/spec-format.md`, `/guide/` for `README.md`, and the sidebar lists Overview first, then the pages in the order `README.md` links them, each labelled with its link text, and Spec format last

#### Scenario: a new guide file
- GIVEN a fixture guide whose `README.md` links `why-clm.md#start` as "Why" and doesn't link `brand-new.md`, whose title is "Brand new"
- WHEN the site is built from it
- THEN `brand-new.md` has a page, with no edit under `web/site/`, and the sidebar reads Overview, Why, Brand new, Spec format

#### Scenario: links between pages
- GIVEN `docs/guide/README.md` linking to `why-clm.md`, and `docs/guide/reference.md` linking to `../spec-format.md`
- WHEN the built pages' links are followed
- THEN they open `/guide/why-clm` and `/guide/spec-format`

#### Scenario: dead link
- GIVEN a fixture guide whose file links to `missing.md`
- WHEN the site is built from it
- THEN the build fails, naming the link

### Requirement: install-repository <!-- base: 3b9ed6ef2d685ea3 -->
Every place the site names the repository in text of its own, the homepage's and `llms.txt`'s install commands (`/plugin marketplace add <repo>`), its GitHub links and the homepage's structured data, SHALL name `INSTALL_REPO`, or `clunny/clm`, the public repository, when it is unset or empty. Built with another repository, no built file SHALL name `clunny/clm` outside text taken from the guide's files or the posts (their pages and the scripts that carry them, their Markdown copies, `llms.txt`, `llms-full.txt`, the feed and the blog's listings), which are written text and may name it in their own install commands.

#### Scenario: repository changed
- GIVEN `INSTALL_REPO=someone/clm-public`
- WHEN the site is built
- THEN the homepage's and `llms.txt`'s install commands read `/plugin marketplace add someone/clm-public`, the homepage's structured data names `https://github.com/someone/clm-public`, and no built file holds `clunny/clm` outside text taken from the guide's files or the posts

#### Scenario: the public repository
- GIVEN no `INSTALL_REPO` in the environment, and, as a second case, `INSTALL_REPO` set empty
- WHEN the homepage is built
- THEN in each case its install commands read `/plugin marketplace add clunny/clm`

### Requirement: site-no-install-scripts <!-- base: 5ee0d14ef3eb156f -->
Installing the site's dependencies in `web/site/` SHALL run no dependency's install script: npm, run there with no `ignore-scripts` on its command line or in its environment, SHALL take `ignore-scripts` as `true` from the package's own settings, whatever the user's npm settings say.

#### Scenario: npmrc
- GIVEN `web/site/`, no `ignore-scripts` in the environment, and a user npm settings file that sets `ignore-scripts=false`
- WHEN `npm config get ignore-scripts` runs there
- THEN it prints `true`

### Requirement: analytics-script <!-- base: 8ab7cdccdf236447 -->
When the build's environment sets `UMAMI_WEBSITE_ID`, every page of the built site SHALL load Umami's script from `https://cloud.umami.is/script.js` with `data-website-id` that ID and `data-do-not-track="true"`; when it is unset or empty, no built file SHALL name Umami. The site SHALL set no cookie, and its pages SHALL store nothing in the browser but VitePress's own record of the light or dark choice.

#### Scenario: released build
- GIVEN `UMAMI_WEBSITE_ID=test-id` in the build's environment
- WHEN the site is built and a test reads `index.html`, `guide/quickstart.html` and `privacy.html`
- THEN each holds one script tag with `src="https://cloud.umami.is/script.js"`, `data-website-id="test-id"` and `data-do-not-track="true"`

#### Scenario: no website ID
- GIVEN no `UMAMI_WEBSITE_ID` in the build's environment
- WHEN the site is built
- THEN no file of the built site contains `umami`

#### Scenario: nothing stored
- GIVEN the built site with a stub `window.umami.track`
- WHEN the visitor opens `/`, clicks Get started, and copies the quickstart's first code block with its copy button
- THEN `document.cookie` is empty, and `localStorage` and `sessionStorage` hold no key but VitePress's appearance key `vitepress-theme-appearance`

### Requirement: click-events <!-- base: 395eb76af5deb1db -->
A click on a same-host link in the nav or on one of the homepage's buttons SHALL call `window.umami.track("click", { label, path })` with the link's text, less a trailing arrow, and the page's path as it was when clicked; a click on an `http` or `https` link to another host SHALL call `window.umami.track("outbound", { url, path })` and nothing else; a click on any other link SHALL report nothing; and when `window.umami` is missing, has no `track`, or its `track` throws, a click SHALL do what it does without analytics, with no error.

#### Scenario: Get started
- GIVEN the built homepage with a stub `window.umami.track` that records its calls
- WHEN the visitor clicks Get started
- THEN the stub was called with `"click"` and `{ label: "Get started", path: "/" }`, and the page goes on to `/guide/quickstart`

#### Scenario: Web app
- GIVEN the built homepage with the stub, and navigation held
- WHEN the visitor clicks the nav's Web app
- THEN the stub was called with `"click"` and `{ label: "Web app", path: "/" }`

#### Scenario: the arrow left out
- GIVEN the built homepage with the stub
- WHEN the visitor clicks How it works, which reads `How it works ↓`, and then the nav's Guide
- THEN the stub was called with `"click"` and `{ label: "How it works", path: "/" }`, then with `"click"` and `{ label: "Guide", path: "/" }`

#### Scenario: outbound link
- GIVEN the built homepage with the stub
- WHEN the visitor clicks the superpowers link
- THEN the stub was called with `"outbound"` and `{ url: "https://github.com/obra/superpowers", path: "/" }`

#### Scenario: links not reported
- GIVEN the built site with the stub, and navigation held
- WHEN the visitor clicks, in turn, a sidebar link on `/guide/quickstart`, the homepage footer's Guide link, and a `mailto:` link added to the homepage
- THEN the stub was never called

#### Scenario: blocked
- GIVEN the built homepage, in turn with no `window.umami`, with a `window.umami` that has no `track`, and with a `track` that throws
- WHEN the visitor clicks Get started
- THEN in each case the page goes to `/guide/quickstart` and the console shows no error

### Requirement: copy-command-event <!-- base: cea0c25558e9c399 -->
Copying a command, by a code block's copy button or by copying a selection that lies wholly inside a code block or inline code (the homepage's install commands among them), SHALL call `window.umami.track("copy-command", { path, command })` with the page's path and the command's first line, trimmed; copying a selection that doesn't lie inside one, or that is empty or only whitespace, SHALL report nothing.

#### Scenario: copy button in the guide
- GIVEN the built `/guide/quickstart` with the stub
- WHEN the visitor clicks its first code block's copy button
- THEN the stub was called with `"copy-command"`, `path` `"/guide/quickstart"` and `command` that block's first line, trimmed

#### Scenario: selection copied on the homepage
- GIVEN the built homepage with the stub
- WHEN the visitor selects the first install command and copies it
- THEN the stub was called with `"copy-command"`, `path` `"/"` and `command` that command

#### Scenario: not a command
- GIVEN the built homepage with the stub
- WHEN the visitor copies, in turn, a selection of the first screen's paragraph, an empty selection inside the first install command, and a selection holding only a line break inside the closing install commands' block
- THEN the stub was never called

### Requirement: privacy-page <!-- base: 7d20639a9e313953 -->
The site SHALL have a page at `/privacy` saying that it sets no cookies, counts page views, clicks and copied commands with Umami Cloud, stores no IP address or personal data, honours Do Not Track and doesn't measure the app; the footer of the homepage and of every guide page SHALL link to it, and neither the nav nor the guide's sidebar SHALL.

#### Scenario: footer link
- GIVEN the built site
- WHEN a test opens `/` and `/guide/quickstart` and follows each footer's Privacy link
- THEN both open `/privacy`, which contains "cookies", "Umami" and "Do Not Track", and neither page's nav nor `/guide/quickstart`'s sidebar holds a link to `/privacy`

### Requirement: readable-without-script <!-- base: 5e50e6d9bc646641 -->
The built site SHALL hold in its static HTML, with no script run, the homepage's headline, its section headings, its install commands and its GitHub link, and each guide page's title and text.

#### Scenario: script off
- GIVEN the built site opened in a browser with JavaScript disabled
- WHEN a test reads `/` and `/guide/quickstart`
- THEN `/` shows the headline, the five section headings, `/plugin marketplace add clunny/clm` and a link to `https://github.com/clunny/clm`, and `/guide/quickstart` shows `docs/guide/quickstart.md`'s `#` title and the text of its first code block

### Requirement: page-metadata <!-- base: 3f985405bf1148f1 -->
Every page of the built site but the 404 page SHALL have a `<title>`, a `<meta name="description">`, a canonical link to its clean URL on `https://clm-app.dev`, and Open Graph and Twitter card tags with the page's own title and description and the site's social card. The homepage's title SHALL be "CLM: declare the intent, AI builds the code to match" and its description its headline followed by the first sentence of its first screen's paragraph; a guide page's title its `#` title followed by " | CLM guide", but `/guide/`'s "CLM guide", and its description its file's summary paragraph as plain text; `/privacy`'s title "Privacy | CLM"; `/blog/`'s "CLM blog"; and a post's page's title its title followed by " | CLM blog", its description its `summary` and its `og:type` `article`. A description SHALL read in the head as written, `"`, `<` and `&` included. The 404 page SHALL be titled "Page not found | CLM", in its file and in the browser, with the description "This page does not exist.", `<meta name="robots" content="noindex">` and no canonical link or social tags. The build SHALL fail, naming the file, on a guide file with no `#` title or whose first block after its title isn't a paragraph.

#### Scenario: a guide page's head
- GIVEN `docs/guide/quickstart.md`
- WHEN the site is built and a test reads `/guide/quickstart`'s head
- THEN its title is the file's `#` title followed by " | CLM guide", its description and `og:description` are the plain text of the file's first paragraph after its title, its canonical link and `og:url` are `https://clm-app.dev/guide/quickstart`, `og:image` names a 1200×630 PNG that the build holds, `twitter:card` is `summary_large_image`, and an alternate link of type `text/markdown` names `https://clm-app.dev/guide/quickstart.md`

#### Scenario: Markdown removed
- GIVEN a fixture guide file whose summary paragraph holds `` `clm guide` `` and a link `[Why CLM](why-clm.md)`
- WHEN the site is built from it
- THEN its page's description reads `clm guide` and `Why CLM` with no backticks, brackets or link target

#### Scenario: no summary
- GIVEN a fixture guide file whose title is followed by a list
- WHEN the site is built from it
- THEN the build fails, naming that file

#### Scenario: no title
- GIVEN a fixture guide file with no `#` line
- WHEN the site is built from it
- THEN the build fails, naming that file

#### Scenario: homepage and privacy
- GIVEN the built site
- WHEN a test reads the heads of `/` and `/privacy`
- THEN each has a title, a non-empty description and a canonical link, `https://clm-app.dev/` and `https://clm-app.dev/privacy`, no two pages of the site share a title, and `404.html` holds a `noindex` robots meta and no canonical link

#### Scenario: each page's title
- GIVEN the built site
- WHEN a test reads the heads of `/`, `/guide/`, `/privacy`, `/blog/` and `404.html`, and opens a path the site doesn't have in a browser
- THEN `/` is titled "CLM: declare the intent, AI builds the code to match" with its headline and its first screen's first sentence as its description, `/guide/` "CLM guide", `/privacy` "Privacy | CLM" and `/blog/` "CLM blog"; `404.html` is titled "Page not found | CLM" with the description "This page does not exist." and no `og:` or `twitter:` tag; and the browser's title on the missing path is "Page not found | CLM"

#### Scenario: a post's head
- GIVEN a fixture published post `alpha` titled "Alpha" with the summary "First post."
- WHEN the site is built and a test reads `/blog/alpha`'s head
- THEN its title is "Alpha | CLM blog", its description and `og:description` are "First post.", its canonical link is `https://clm-app.dev/blog/alpha`, `og:type` is `article`, and an alternate link of type `text/markdown` names `https://clm-app.dev/blog/alpha.md`

### Requirement: crawler-files <!-- base: 7811e9871443aadc -->
The built site SHALL hold `robots.txt` with one `User-agent: *` group with no `Allow` line (what isn't disallowed is allowed), disallowing `/app/`, `/api/` and `/auth/`, followed by `Sitemap: https://clm-app.dev/sitemap.xml`, and `sitemap.xml` listing the homepage, every guide page, `/privacy`, `/blog/` and every published post on `https://clm-app.dev`, and no URL under `/app/`, no draft's and not the feed.

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
- THEN it lists `https://clm-app.dev/blog/` and `https://clm-app.dev/blog/alpha`, and no URL containing `quokka` or `feed`

### Requirement: structured-data <!-- base: 56e1b2dbcc97fdb1 -->
The homepage SHALL hold JSON-LD describing CLM as a `SoftwareApplication`, free, MIT-licensed, with `sameAs` the GitHub repository, and `/guide/faq` SHALL hold JSON-LD describing an `FAQPage` whose questions are the `##` headings of `docs/guide/faq.md`, in order, each answered by the text under it; with no `faq.md`, or one with no `##` heading, no page SHALL hold an `FAQPage` and the build SHALL succeed. `/blog/` SHALL hold JSON-LD describing a `Blog` listing the published posts, and each post's page JSON-LD describing a `BlogPosting` with its title, summary, date and tags.

#### Scenario: the homepage
- GIVEN the built homepage
- WHEN a test parses its `application/ld+json` script
- THEN it is a `SoftwareApplication` named `CLM` with `url` `https://clm-app.dev/`, `offers.price` `0`, a `license` naming MIT and `sameAs` holding `https://github.com/clunny/clm`

#### Scenario: the FAQ follows its file
- GIVEN a fixture guide whose `faq.md` has the headings "What is CLM?" and "Is it free?", each with a paragraph under it
- WHEN the site is built from it and a test parses `/guide/faq`'s `application/ld+json` script
- THEN it is an `FAQPage` with exactly those two questions, in that order, each answer the plain text of its paragraph

#### Scenario: no questions
- GIVEN a fixture guide with no `faq.md`, and, as a second case, one whose `faq.md` has no `##` heading
- WHEN the site is built from each
- THEN each build succeeds and no built page holds an `FAQPage`

#### Scenario: a post
- GIVEN a fixture published post `alpha` titled "Alpha", dated 2026-11-05, tagged `testing`
- WHEN the site is built and a test parses `/blog/alpha`'s `application/ld+json` script
- THEN it is a `BlogPosting` with `headline` `Alpha`, `datePublished` `2026-11-05`, `keywords` holding `testing` and `url` `https://clm-app.dev/blog/alpha`, and `/blog/`'s is a `Blog` whose `blogPost` lists `alpha` alone when every other post is a draft

### Requirement: agent-markdown <!-- base: 6ed9b7f566fba5c7 -->
The built site SHALL hold, made at build time from the guide's files, a Markdown copy of each guide page at its page's URL with `.md` added (`/guide/index.md` for `/guide/`), and no other `.md` file under `/guide/`; each copy SHALL be its file's text with only the targets of its inline links outside fenced code and code spans changed: a link to another guide file names that file's copy as an absolute URL, any other relative link (not one starting `/`) names the file on the GitHub repository's `main` (`/blob/main/<path>` for a file, `/tree/main/<path>` for a directory), and a link's `#anchor` is kept. The guide's and the posts' pages SHALL link such a file on GitHub as their copies do, and such a link SHALL never fail the build. It SHALL hold `llms.txt` listing every copy in the sidebar's order with its title and summary, then, under `## Blog`, the Markdown copy at `/blog/<slug>.md` of every published post, newest first, with its title and summary; and `llms-full.txt` holding every copy, in that order. A post's copy SHALL be its text under a `#` line with its title, its links rewritten as a guide copy's are, a link to a draft left as its text, and an image never taken for a link.

#### Scenario: each copy is its file
- GIVEN the built site
- WHEN a test compares each guide file with its Markdown copy, line by line
- THEN each line of the copy is the file's line with at most its link targets changed, no inline link outside fenced code is left relative, `/guide/reference.md` links to `https://clm-app.dev/guide/spec-format.md`, and the built `guide/` holds no `.md` file but the copies

#### Scenario: links into the repository
- GIVEN `INSTALL_REPO=someone/clm-public`, a fixture guide file linking `../spec-format.md#top`, `README.md`, `../../crates/`, `../../README.md#top`, `/privacy` and `https://example.com/a.md`, with `[not a link](../spec-format.md)` in fenced code and `` `[x](../y.md)` `` as a code span, and a fixture post linking `../../../README.md#top` and a draft's `next.md`, and holding the image `![pic](next.md)`
- WHEN the site is built from them
- THEN the build succeeds; the guide copy names `https://clm-app.dev/guide/spec-format.md#top`, `https://clm-app.dev/guide/index.md`, `https://github.com/someone/clm-public/tree/main/crates` and `https://github.com/someone/clm-public/blob/main/README.md#top`, and leaves `/privacy`, the `https` link, the fenced line and the code span as they are; the guide's page links the same two GitHub URLs; and the post's page and copy both link `https://github.com/someone/clm-public/blob/main/README.md#top`, the copy reading `next` for the draft's link and keeping `![pic](next.md)`

#### Scenario: llms.txt
- GIVEN the built site
- WHEN a test reads `llms.txt`
- THEN it opens with `# CLM` and a `>` line, holds `/plugin marketplace add clunny/clm`, lists `https://clm-app.dev/guide/why-clm.md` before `https://clm-app.dev/guide/quickstart.md`, each with its page's summary, and links `https://clm-app.dev/llms-full.txt`

#### Scenario: the full text can't drift
- GIVEN a fixture guide built once, then a line added to one of its files and a new file linked from its `README.md`, and built again
- WHEN a test reads `llms-full.txt` and `llms.txt` after each build
- THEN after the second build `llms-full.txt` holds the added line and the new file's text, `llms.txt` lists the new file, and nothing under `web/site/` was edited

#### Scenario: published posts only
- GIVEN a fixture blog with a published post `alpha`, whose body links `quokka-draft.md`, and a draft `quokka-draft`
- WHEN the site is built and a test reads `llms.txt`
- THEN its `## Blog` section lists `https://clm-app.dev/blog/alpha.md` with its summary, `/blog/alpha.md` is `alpha`'s text under a `#` line with its title, the link to the draft left as its text, and no built file names `quokka`

### Requirement: github-link <!-- base: 53d2f689ca7997bd -->
The nav of every page, and the homepage's first screen, SHALL link to `https://github.com/<repo>`, `<repo>` being the install repository, labelled with the repository's star count read at build time, shown as the number under 1000 and from 1000 as thousands rounded to one decimal with a `k` (`1.2k` for 1234). The count SHALL be `GITHUB_STARS` when the environment sets it non-empty, and otherwise the `stargazers_count` that `GET <api>/repos/<repo>` answers, `<api>` being `GITHUB_API` when set and `https://api.github.com` otherwise, sent with `Authorization: Bearer <GITHUB_TOKEN>` when `GITHUB_TOKEN` is set. When no count can be had, because the API can't be reached, answers with an error status, gives no `stargazers_count`, or doesn't answer within 10 seconds, or because the count isn't a whole number of 0 or more, the build SHALL still succeed, print a warning, and show the links labelled `GitHub` with no count. Wherever the nav bar shows its links (not in the menu VitePress opens at narrow widths), each link SHALL show GitHub's mark beside its label on one line, the mark before the label, centred on the line with the nav's other links or, on the homepage, with the first screen's other buttons; the label `GitHub`, shown without a count, is laid out the same way.

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

#### Scenario: the request
- GIVEN no `GITHUB_STARS`, `GITHUB_TOKEN=t`, and `GITHUB_API` naming a local server that answers `{"stargazers_count": 87}`
- WHEN the site is built
- THEN the server was asked `GET /repos/clunny/clm` with the header `Authorization: Bearer t`, and the links read `87`

#### Scenario: a count that can't be used
- GIVEN, in turn, `GITHUB_STARS` set to `abc`, `-5` and `12.5`, and, with no `GITHUB_STARS`, `GITHUB_API` naming a local server that answers 500, one that answers `{}`, and one that accepts the connection and never answers
- WHEN the site is built in each case
- THEN each build succeeds and prints a warning, and the links read `GitHub` with no count

#### Scenario: another repository
- GIVEN `INSTALL_REPO=someone/clm-public` and `GITHUB_STARS=5`
- WHEN the site is built
- THEN the GitHub links name `https://github.com/someone/clm-public`

### Requirement: nav-web-app <!-- base: c1d913a8891d5018 -->
The nav of every page SHALL hold, in order, How it works, Guide, Blog, Get started and Web app, Web app linking to `/app/`. Every link on the site to `/app/`, the homepage's footer's included, SHALL read Web app, and following any of them SHALL load `/app/` from the server rather than show a page of the site's own; no link SHALL read Sign in.

#### Scenario: homepage and a guide page
- GIVEN the built site
- WHEN a test reads the links on `/` and on `/guide/quickstart`
- THEN on both the nav's link reading `Web app` leads to `/app/` and comes after Get started, every link to `/app/` reads `Web app`, and no link reads `Sign in`

#### Scenario: the app is left to the app
- GIVEN the built site, served
- WHEN the visitor clicks the nav's Web app on `/guide/quickstart`, and the footer's Web app on `/`
- THEN each click makes the browser request the document `/app/` from the server

### Requirement: site-worker <!-- base: 0fb56a72403a59f7 -->
The site's Worker, as `wrangler` reads its configuration in `web/site/`, SHALL be named `clm-site`, run no script of its own, have the `clm-app.dev` Custom Domain as its one route, and serve the static assets in the directory `npm run build` writes to when no `SITE_OUT` is set, answering a path no asset holds with the site's `404.html` and status 404.

#### Scenario: the site's Worker
- GIVEN the site built in `web/site/` with no `SITE_OUT`
- WHEN `wrangler` reads `web/site/`'s configuration, and serves the Worker locally from it
- THEN its name is `clm-site`, it has no `main`, its one route is `clm-app.dev` with `custom_domain` true, its assets' directory is the one holding the build's `index.html`, `/guide/quickstart` answers 200, and a path the site doesn't have answers 404 with the body of the built `404.html`

### Requirement: blog-drafts-left-out <!-- base: b80f5701ea40a5ef -->
The site's build SHALL read every post in the blog's directory, treat one whose `draft` is `true` or missing as a draft, and build the blog from the other posts alone, so that no built file holds a draft's slug, title or text. It SHALL fail, naming the file, on any post, draft or not, that has no frontmatter, frontmatter that isn't YAML or isn't a mapping, a missing or blank `title`, `summary` or `date`, a `date` that isn't a real calendar date written `YYYY-MM-DD`, a `draft` that isn't `true` or `false`, `tags` that isn't a list of strings, a `series_part` that isn't a positive integer, or the slug `index` or `feed`; and on two published posts with one `series_part`, naming both, a draft's `series_part` clashing with none.

#### Scenario: a draft leaves no trace
- GIVEN a fixture blog with a published post `alpha` and a draft `quokka-draft` titled "Quokka draft title", whose body holds the word `zebraphrase`
- WHEN the site is built from it and a test searches every built file
- THEN `/blog/alpha` exists, no file path contains `quokka`, and no file holds `quokka`, `Quokka draft title` or `zebraphrase`

#### Scenario: no draft key
- GIVEN a fixture post with no `draft` key
- WHEN the site is built
- THEN it is left out as a draft is

#### Scenario: a bad post
- GIVEN fixture blogs each holding one post with `date: soon`, `date: 2026-02-30`, no `title`, a blank `summary`, no `date`, `draft: yes`, `tags: testing`, `series_part: 0`, `series_part: 1.5`, no frontmatter, frontmatter that isn't YAML, frontmatter that is a list, or `date: soon` and `draft: true`, or a post named `index.md` or `feed.md`; and a blog of two published posts with `series_part: 1`
- WHEN the site is built from each
- THEN each build fails, naming the post's file, the pair's naming both files; and a blog of a published post and a draft with `series_part: 1` builds

### Requirement: blog-index <!-- base: 4dfcf7596c385501 -->
`/blog/` SHALL show every published post as a card linking to its page, newest first (by `date`, then by `series_part`, highest first, a post with none after those with one, then by slug, A to Z), each card holding its part (when it has a `series_part`), its date, its reading time, its title, its summary and its tags. With no published post, or no blog directory, `/blog/` SHALL show no card and say posts are coming, its `Blog` structured data SHALL list no post, the feed SHALL hold no item, and `llms.txt` SHALL have no `## Blog` section.

#### Scenario: cards in order
- GIVEN a fixture blog with published posts dated 2026-11-05 (part 1, 230 words) and 2026-11-08 (part 2, 231 words), three more published posts with no part, and a draft dated 2026-11-12
- WHEN the site is built and a test reads `/blog/`
- THEN it holds five cards, newest first, part 2's before part 1's, part 2's showing "Part 2", "8 Nov 2026", "2 min read", its title, summary and tags, and part 1's "1 min read"

#### Scenario: same day
- GIVEN a fixture blog of published posts all dated 2026-11-05: `a` part 1, `b` part 2, and `d` and `c` with no part
- WHEN the site is built and a test reads `/blog/`
- THEN its cards are in the order `b`, `a`, `c`, `d`

#### Scenario: nothing published
- GIVEN a fixture blog holding only drafts, and, as further cases, `BLOG_DIR` naming an empty directory and one that doesn't exist
- WHEN the site is built from each
- THEN each build succeeds, `/blog/` exists, holds no card and says posts are coming, its `Blog`'s `blogPost` is empty, `/blog/feed.xml` holds no item, and `llms.txt` has no `## Blog` line

#### Scenario: phone width
- GIVEN the built `/blog/` with five published posts
- WHEN it is opened 390 px wide
- THEN its cards stand in one column and nothing scrolls sideways

### Requirement: blog-post-page <!-- base: 137f3ed3ff91b7fa -->
Each published post SHALL have a page at `/blog/<slug>`, readable without script, holding its title, its summary, its date, its reading time, its tags and its body, the published posts of its series in part order with its own marked, and links to the published posts before and after it in part order; a post with no `series_part` SHALL show no series and no previous or next link. A link in its body to a draft's file SHALL be rendered as its text alone, and the build SHALL fail, naming the post's file, on a link in its body to `<slug>.md` where no post has that slug. Every URL the site writes for a post (its page's links, the cards, the series, the feed, the sitemap, the heads, the structured data and `llms.txt`) SHALL carry its slug percent-encoded, so a slug holding `&`, a space or another character a URL or XML can't carry still leads to its page and leaves every file well-formed.

#### Scenario: the series skips drafts
- GIVEN a fixture series of parts 1, 2 and 3, part 3 a draft, and part 2's body ending with a link `[Part three](gamma.md)` to part 3's file
- WHEN the site is built and `/blog/beta` (part 2) is opened with JavaScript disabled
- THEN it shows its title, summary, date, reading time, tags and body, its series lists parts 1 and 2 with part 2 marked, it links to part 1 as the previous post and to no next post, and "Part three" is text with no link

#### Scenario: a link to a published post
- GIVEN part 1's body linking to `beta.md`
- WHEN the site is built
- THEN on `/blog/alpha` that link leads to `/blog/beta`

#### Scenario: no series
- GIVEN a fixture blog whose published post `delta` has no `series_part`, beside a published series
- WHEN the site is built and `/blog/delta` is opened
- THEN it shows no series and no previous or next link

#### Scenario: a link to no post
- GIVEN a fixture post whose body links `no-such-post.md`, and no post `no-such-post`
- WHEN the site is built
- THEN the build fails, naming the post's file

#### Scenario: an odd slug
- GIVEN a fixture published post in the file `a&b c.md`
- WHEN the site is built and served
- THEN `/blog/feed.xml` parses as XML with the item link `https://clm-app.dev/blog/a%26b%20c`, the sitemap and the post's canonical link name that URL, and `/blog/`'s card for it links `/blog/a%26b%20c`, which opens the post's page

### Requirement: blog-feed <!-- base: cdf76160d610377a -->
The build SHALL write `/blog/feed.xml`, an RSS 2.0 feed with one item per published post, newest first, each with its title, its page's URL on the site's address as link and guid, its summary as description and its date as `pubDate`, and `/blog/` and every post's page SHALL link it as `application/rss+xml`. A post's title or summary holding `&`, `<`, `>` or `"` SHALL leave the feed well-formed XML and the post's and `/blog/`'s JSON-LD parseable, each holding the text as written.

#### Scenario: the feed
- GIVEN a fixture blog with published posts `alpha` (2026-11-05) and `beta` (2026-11-08), three more published posts, and a draft `quokka-draft`
- WHEN the site is built and a test parses `/blog/feed.xml`
- THEN it is RSS 2.0 with five items, newest first, `beta`'s before `alpha`'s, whose link is `https://clm-app.dev/blog/beta` and whose `pubDate` is `Sun, 08 Nov 2026 00:00:00 GMT`, and `/blog/`'s head links the feed

#### Scenario: markup in a title
- GIVEN a fixture published post titled `A & <B> "C"`, with the summary `x < "y" </script>`
- WHEN the site is built
- THEN `/blog/feed.xml` parses as XML with an item whose title and description are those texts, the `BlogPosting` of the post's page and the `Blog` of `/blog/` each parse as JSON with that `headline`, and the post page's description meta and `og:description` read the summary as written

### Requirement: doc-readers-marked <!-- base: 730bb707f0a78189 -->
When a change touches one of this repository's guide pages under `docs/guide/` or `docs/spec-format.md`, `clm affected --readers` SHALL select every test file under `web/site/` that builds the site from, or reads, that page (not a fixture `GUIDE_DIR` points at), so a land that edits a guide page runs it even when its code was already checked (`check.affected/affected-readers`, `land/selected-tests`).

#### Scenario: the site's readers of the guide
- GIVEN the `clm:reads` markers of the files `check`'s coverage scan reads in this repository's tree
- WHEN `clm-affected`'s `readers` runs on them with `docs/guide/quickstart.md` as the only differing path
- THEN it selects `web/site/test/guide.spec.ts` and `web/site/test/seo.spec.ts`, which read the repository's guide and the site built from it

#### Scenario: the spec format page
- GIVEN the same markers
- WHEN `readers` runs on them with `docs/spec-format.md` as the only differing path
- THEN it selects `web/site/test/seo.spec.ts`

#### Needs
- check.affected/affected-readers

### Requirement: blog-updated-date <!-- base: 11514ec9802e5543 -->
A post MAY carry an `updated` date in its frontmatter; when a published post has one, its page SHALL show it beside its date as "Updated <date>", its head SHALL hold `article:modified_time` and its `BlogPosting` JSON-LD `dateModified`, each its `updated`, and a post without one SHALL show and hold none of them; the build SHALL fail, naming the file, on an `updated` that isn't a real calendar date written `YYYY-MM-DD` or that is before its `date`.

#### Scenario: an updated post
- GIVEN a fixture published post `alpha` dated 2026-11-05 with `updated: 2026-11-09`
- WHEN the site is built and a test reads `/blog/alpha`
- THEN the page shows "5 Nov 2026" and "Updated 9 Nov 2026", its head's `article:modified_time` is `2026-11-09`, and its `BlogPosting` has `datePublished` `2026-11-05` and `dateModified` `2026-11-09`

#### Scenario: no updated date
- GIVEN a fixture published post `beta` with no `updated` key
- WHEN the site is built and a test reads `/blog/beta`
- THEN the page holds no "Updated", its head no `article:modified_time`, and its `BlogPosting` no `dateModified`

#### Scenario: updated before published
- GIVEN a fixture post dated 2026-11-05 with `updated: 2026-11-01`
- WHEN the site is built
- THEN the build fails, naming that file
