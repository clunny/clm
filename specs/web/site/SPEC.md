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
CLM's public site at `https://clm-app.dev`: the homepage at `/` and the user guide at `/guide/`, built with VitePress from `web/site/` and the pages `clm guide` prints, `docs/guide/*.md` and `docs/spec-format.md` (`public-site`, `guide-in-the-binary`). `web`'s build runs it and puts its output at the root of `web/dist/`, beside the app under `web/dist/app/`; the same Worker serves both, and until launch only to signed-in invited people (`web.worker/site-behind-sign-in`, `invite-only-web`). The site links to the app at `/app/` and reads nothing from the Worker's API.

## Design
**Build.** A package of its own in `web/site/`: its `package.json` with VitePress and its build and test scripts, its committed `package-lock.json`, a `.npmrc` setting `ignore-scripts=true` (`unattended-commands`), its VitePress config and theme under `web/site/.vitepress/`, the homepage as a custom layout, and its tests with their own configs. Its own package keeps VitePress's Vite and Vue apart from the app's Vite 8. `npm run build` there writes the site to the directory `SITE_OUT` names (default `web/site/dist/`); `web`'s build runs `npm ci --ignore-scripts` and that build with `SITE_OUT` at `web/dist/` before the app's (`web/app-at-app-path`), so the site never empties the app's output. `npm test` runs its tests, which `ci-check.sh` runs after `web/`'s suite (`ci/web-checks`).

**The guide's pages** are the pages `clm guide` prints: each file of `docs/guide/` at `/guide/<name>`, `README.md` at `/guide/`, and `docs/spec-format.md` at `/guide/spec-format`, which the guide's files link to as `../spec-format.md`. They are read in place at build time (VitePress's `srcDir` and `rewrites`, a Vite alias or a loader, the builder's choice), never copied into the repository: they stay the only copy (`guide-from-docs`). Links between them resolve to their pages, and the build fails on a dead link. The sidebar lists them in the order `docs/guide/README.md` links them, labelled with the link text, and `spec-format` last. The directory the guide is read from and the install repository (below) can be set by environment variables, `GUIDE_DIR` and `INSTALL_REPO`, so tests can build with a fixture guide and another repository.

**Homepage.** `assets/mockups/homepage.html` is the reference for layout, colour, type and copy; the dashed line in it marks roughly where a laptop's first screen ends. Its copy is the starting text: wording may be polished, the message and sections may not change without a spec change. The first screen holds the headline "Declare the intent. AI builds the code to match. CLM keeps it matching.", one line on building solo or as a team with AI writing every line while the project stays maintainable, **Get started** (to the quickstart) and **How it works** (to the sections below), the install commands in small type, and the loop drawn as a ring around "your intent": Declare (you approve, yes or no), Build (AI, test-first), Prove (a test cites every rule), Reconcile (drift is rebuilt). Kubernetes isn't named on the first screen. Then five sections, in order:
1. *The split*: you own the what (architecture, capabilities, one yes or no per change); AI owns the how.
2. *The loop*: "Reconciliation, borrowed from Kubernetes", opening with "CLM isn't a Kubernetes tool. It borrows one idea from it", then Kubernetes and CLM side by side.
3. *Not a vibe-coded mess*: every rule has a test, drift can't land, nothing dead.
4. *Solo or as a team*: git is the only channel.
5. *Standing on shoulders*: the Spec Growth Engine paper ([arXiv:2606.27045](https://arxiv.org/html/2606.27045v2)), [superpowers](https://github.com/obra/superpowers) and Anthropic's [AI-native SDLC playbook](https://claude.com/blog/the-ai-native-sdlc-playbook), each with what CLM took from it, as `docs/guide/why-clm.md` says.
It ends with the install commands and a link to the quickstart, and a footer "CLM is built with CLM". The nav holds How it works, Guide, Get started and Sign in (to `/app/`).

**The install repository.** The repository the homepage's install commands name (`/plugin marketplace add <repo>`) is one value in the site's config (overridable by `INSTALL_REPO` for tests), since at launch it is `clunny/clm` made public or a new public repository. Until launch it is `clunny/clm`.

**Look.** Inter for text, JetBrains Mono for commands; the app's indigo accent with a sky gradient, the mockup's dark palette and a light one. The site follows the system's light or dark setting, with VitePress's own switch kept. At phone widths the ring sits below the first screen's text, the side-by-side panels stack, and nothing scrolls sideways.

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
The homepage's install commands SHALL name the repository set by one value in the site's config, and no other file under `web/site/` SHALL name it.

#### Scenario: repository changed
- GIVEN the site's config value set to `someone/clm-public`
- WHEN the homepage is built
- THEN its install commands read `/plugin marketplace add someone/clm-public`, and no other file under `web/site/` contains `clunny/clm`

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
