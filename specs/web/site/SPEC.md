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
CLM's public site at `https://clm-app.dev`: the homepage at `/` and the user guide at `/guide/`, built with VitePress from `web/site/` and the pages `clm guide` prints, `docs/guide/*.md` and `docs/spec-format.md` (`public-site`, `guide-in-the-binary`). `web`'s build runs it and puts its output at the root of `web/dist/`, beside the app under `web/dist/app/`; the same deployment holds both, and its assets serve the site to everyone, with no session and no Worker request (`web/site-served-directly`, `invite-only-web`). The site links to the app at `/app/` and reads nothing from the Worker's API. The released site reports page views, clicks and copied commands to Umami Cloud, cookieless and honouring Do Not Track, and only when its build is given a website ID; `/privacy` says what is collected (`site-analytics-umami`).

## Design
**Build.** A package of its own in `web/site/`: its `package.json` with VitePress and its build and test scripts, its committed `package-lock.json`, a `.npmrc` setting `ignore-scripts=true` (`unattended-commands`), its VitePress config and theme under `web/site/.vitepress/`, the homepage as a custom layout, and its tests with their own configs. Its own package keeps VitePress's Vite and Vue apart from the app's Vite 8. `npm run build` there writes the site to the directory `SITE_OUT` names (default `web/site/dist/`); `web`'s build runs `npm ci --ignore-scripts` and that build with `SITE_OUT` at `web/dist/` before the app's (`web/app-at-app-path`), so the site never empties the app's output. `npm test` runs its tests, which `ci-check.sh` runs after `web/`'s suite (`ci/web-checks`).

**The guide's pages** are the pages `clm guide` prints: each file of `docs/guide/` at `/guide/<name>`, `README.md` at `/guide/`, and `docs/spec-format.md` at `/guide/spec-format`, which the guide's files link to as `../spec-format.md`. They are read in place at build time (VitePress's `srcDir` and `rewrites`, a Vite alias or a loader, the builder's choice), never copied into the repository: they stay the only copy (`guide-from-docs`). Links between them resolve to their pages, and the build fails on a dead link. The sidebar lists them in the order `docs/guide/README.md` links them, labelled with the link text, and `spec-format` last. The directory the guide is read from and the install repository (below) can be set by environment variables, `GUIDE_DIR` and `INSTALL_REPO`, so tests can build with a fixture guide and another repository; `UMAMI_WEBSITE_ID` turns analytics on (below).

**Homepage.** `assets/mockups/homepage.html` is the reference for layout, colour, type and copy; the dashed line in it marks roughly where a laptop's first screen ends. Its copy is the starting text: wording may be polished, the message and sections may not change without a spec change. The first screen holds the headline "Declare the intent. AI builds the code to match. CLM keeps it matching.", one line on building solo or as a team with AI writing every line while the project stays maintainable, **Get started** (to the quickstart) and **How it works** (to the sections below), the install commands in small type, and the loop drawn as a ring around "your intent": Declare (you approve, yes or no), Build (AI, test-first), Prove (a test cites every rule), Reconcile (drift is rebuilt). Kubernetes isn't named on the first screen. Then five sections, in order:
1. *The split*: you own the what (architecture, capabilities, one yes or no per change); AI owns the how.
2. *The loop*: "Reconciliation, borrowed from Kubernetes", opening with "CLM isn't a Kubernetes tool. It borrows one idea from it", then Kubernetes and CLM side by side.
3. *Not a vibe-coded mess*: every rule has a test, drift can't land, nothing dead.
4. *Solo or as a team*: git is the only channel.
5. *Standing on shoulders*: the Spec Growth Engine paper ([arXiv:2606.27045](https://arxiv.org/html/2606.27045v2)), [superpowers](https://github.com/obra/superpowers) and Anthropic's [AI-native SDLC playbook](https://claude.com/blog/the-ai-native-sdlc-playbook), each with what CLM took from it, as `docs/guide/why-clm.md` says.
It ends with the install commands and a link to the quickstart, and a footer "CLM is built with CLM" with a Privacy link to `/privacy`; the guide's pages, whose default layout has no footer, get a small footer with the same link through a slot of VitePress's layout in the theme. The nav holds How it works, Guide, Get started and Sign in (to `/app/`).

**The install repository.** The repository the homepage's install commands name (`/plugin marketplace add <repo>`) is one value in the site's config (overridable by `INSTALL_REPO` for tests): `clunny/clm`, which is public, so a visitor adopts CLM on their own (`public-site`).

**Look.** Inter for text, JetBrains Mono for commands; the app's indigo accent with a sky gradient, the mockup's dark palette and a light one. The site follows the system's light or dark setting, with VitePress's own switch kept. At phone widths the ring sits below the first screen's text, the side-by-side panels stack, and nothing scrolls sideways.

**Analytics** (`site-analytics-umami`). When the build's environment sets `UMAMI_WEBSITE_ID`, every page's head holds `<script defer src="https://cloud.umami.is/script.js" data-website-id="<id>" data-do-not-track="true">`; when it is unset or empty, no built file names Umami, so tests, local builds and forks report nothing, and the operator's ID lives only in the repository's Actions variable (`ci/web-deploy`). Umami's script records page views itself, VitePress's in-site navigation included. One module of the theme records the rest by calling `window.umami.track(name, data)`, doing nothing when `window.umami` is missing (blocked, Do Not Track, or a build without an ID), and never delaying a click's navigation:
- a click on a same-host link in the nav or among the homepage's buttons (Get started, How it works, Guide, Sign in): the event `click`, with `label` the element's text and `path` the page's;
- a click on any link to another host: the event `outbound` only, with `url` the link's and `path` the page's;
- a copied command, by a code block's copy button or by copying a selection inside a command (the homepage's install commands, a guide code block): the event `copy-command`, with `path` the page's and `command` the command's first line, trimmed. VitePress's copy button writes to the clipboard without a `copy` event, so it is caught by its click, and a selection by the document's `copy` event when the selection lies inside a `pre` or code block. The script tag is added per page in the config's head (`transformHead` or `head`, the builder's choice), so the 404 page carries it too.
The app under `/app/` is a build of its own and loads no analytics. The site serves no cookie and the theme stores nothing in the browser for analytics. The funnel and the undercount (judged against Cloudflare's own traffic numbers for the zone) are read in the dashboards, not in code.

**Privacy page.** `/privacy`, a Markdown page of the site's own under `web/site/`, not part of the guide and listed in neither the nav nor the sidebar, says in plain words: the site sets no cookies; it counts page views, clicks and copied commands with Umami Cloud, which stores no IP address and no personal data; Do Not Track is honoured; and the app under `/app/` isn't measured.

## Requirements

### Requirement: homepage-first-screen
The homepage's first screen SHALL show the headline "Declare the intent. AI builds the code to match. CLM keeps it matching.", a Get started link to the guide's quickstart, and the loop's four stages Declare, Build, Prove and Reconcile, and SHALL NOT name Kubernetes.

#### Scenario: laptop first load
- GIVEN the built site, opened at `/` in a 1280×800 window and not scrolled
- WHEN a test reads what is in view
- THEN the headline, a Get started link to `/guide/quickstart` and the four stages are in view, and no text in view contains "Kubernetes"

### Requirement: homepage-sections
Below its first screen the homepage SHALL hold, in order, the sections The split, The loop, Not a vibe-coded mess, Solo or as a team and Standing on shoulders; The loop SHALL say that CLM isn't a Kubernetes tool, and Standing on shoulders SHALL link to the Spec Growth Engine paper, superpowers and Anthropic's AI-native SDLC playbook.

#### Scenario: sections and credits
- GIVEN the built homepage
- WHEN a test reads its section headings and links
- THEN the five sections appear in that order, The loop holds "CLM isn't a Kubernetes tool", and Standing on shoulders links to `https://arxiv.org/html/2606.27045v2`, `https://github.com/obra/superpowers` and `https://claude.com/blog/the-ai-native-sdlc-playbook`

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
A click on Get started, How it works, Sign in or a nav link SHALL call `window.umami.track("click", { label, path })` with the element's text and the page's path, and a click on a link to another host SHALL call `window.umami.track("outbound", { url, path })`; when `window.umami` is missing, a click SHALL do what it does without analytics, with no error.

#### Scenario: Get started
- GIVEN the built homepage with a stub `window.umami.track` that records its calls, and navigation held
- WHEN the visitor clicks Get started
- THEN the stub was called with `"click"` and `{ label: "Get started", path: "/" }`, and the page goes on to `/guide/quickstart`

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
