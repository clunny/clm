---
component: web.site
base: { contract: "42db1a8d6f3188a2", design: "2cfee5c0dbce632a" }
---

## MODIFIED Contract
CLM's public site at `https://clm-app.dev`: the homepage at `/` and the user guide at `/guide/`, built with VitePress from `web/site/` and the pages `clm guide` prints, `docs/guide/*.md` and `docs/spec-format.md` (`public-site`, `guide-in-the-binary`). `web`'s build runs it and puts its output at the root of `web/dist/`, beside the app under `web/dist/app/`; the same deployment holds both, and its assets serve the site to everyone, with no session and no Worker request (`web/site-served-directly`, `invite-only-web`). The site links to the app at `/app/` and reads nothing from the Worker's API.

## MODIFIED Design
**Build.** A package of its own in `web/site/`: its `package.json` with VitePress and its build and test scripts, its committed `package-lock.json`, a `.npmrc` setting `ignore-scripts=true` (`unattended-commands`), its VitePress config and theme under `web/site/.vitepress/`, the homepage as a custom layout, and its tests with their own configs. Its own package keeps VitePress's Vite and Vue apart from the app's Vite 8. `npm run build` there writes the site to the directory `SITE_OUT` names (default `web/site/dist/`); `web`'s build runs `npm ci --ignore-scripts` and that build with `SITE_OUT` at `web/dist/` before the app's (`web/app-at-app-path`), so the site never empties the app's output. `npm test` runs its tests, which `ci-check.sh` runs after `web/`'s suite (`ci/web-checks`).

**The guide's pages** are the pages `clm guide` prints: each file of `docs/guide/` at `/guide/<name>`, `README.md` at `/guide/`, and `docs/spec-format.md` at `/guide/spec-format`, which the guide's files link to as `../spec-format.md`. They are read in place at build time (VitePress's `srcDir` and `rewrites`, a Vite alias or a loader, the builder's choice), never copied into the repository: they stay the only copy (`guide-from-docs`). Links between them resolve to their pages, and the build fails on a dead link. The sidebar lists them in the order `docs/guide/README.md` links them, labelled with the link text, and `spec-format` last. The directory the guide is read from and the install repository (below) can be set by environment variables, `GUIDE_DIR` and `INSTALL_REPO`, so tests can build with a fixture guide and another repository.

**Homepage.** `assets/mockups/homepage.html` is the reference for layout, colour, type and copy; the dashed line in it marks roughly where a laptop's first screen ends. Its copy is the starting text: wording may be polished, the message and sections may not change without a spec change. The first screen holds the headline "Declare the intent. AI builds the code to match. CLM keeps it matching.", one line on building solo or as a team with AI writing every line while the project stays maintainable, **Get started** (to the quickstart) and **How it works** (to the sections below), the install commands in small type, and the loop drawn as a ring around "your intent": Declare (you approve, yes or no), Build (AI, test-first), Prove (a test cites every rule), Reconcile (drift is rebuilt). Kubernetes isn't named on the first screen. Then five sections, in order:
1. *The split*: you own the what (architecture, capabilities, one yes or no per change); AI owns the how.
2. *The loop*: "Reconciliation, borrowed from Kubernetes", opening with "CLM isn't a Kubernetes tool. It borrows one idea from it", then Kubernetes and CLM side by side.
3. *Not a vibe-coded mess*: every rule has a test, drift can't land, nothing dead.
4. *Solo or as a team*: git is the only channel.
5. *Standing on shoulders*: the Spec Growth Engine paper ([arXiv:2606.27045](https://arxiv.org/html/2606.27045v2)), [superpowers](https://github.com/obra/superpowers) and Anthropic's [AI-native SDLC playbook](https://claude.com/blog/the-ai-native-sdlc-playbook), each with what CLM took from it, as `docs/guide/why-clm.md` says.
It ends with the install commands and a link to the quickstart, and a footer "CLM is built with CLM". The nav holds How it works, Guide, Get started and Sign in (to `/app/`).

**The install repository.** The repository the homepage's install commands name (`/plugin marketplace add <repo>`) is one value in the site's config (overridable by `INSTALL_REPO` for tests): `clunny/clm`, which is public, so a visitor adopts CLM on their own (`public-site`).

**Look.** Inter for text, JetBrains Mono for commands; the app's indigo accent with a sky gradient, the mockup's dark palette and a light one. The site follows the system's light or dark setting, with VitePress's own switch kept. At phone widths the ring sits below the first screen's text, the side-by-side panels stack, and nothing scrolls sideways.

## MODIFIED Requirements

### Requirement: install-repository <!-- base: 279812cd15963365 -->
The homepage's install commands SHALL name the repository set by one value in the site's config, by default `clunny/clm`, the public repository, and no other file under `web/site/` SHALL name it.

#### Scenario: repository changed
- GIVEN the site's config value set to `someone/clm-public`
- WHEN the homepage is built
- THEN its install commands read `/plugin marketplace add someone/clm-public`, and no other file under `web/site/` contains `clunny/clm`

#### Scenario: the public repository
- GIVEN no `INSTALL_REPO` in the environment
- WHEN the homepage is built
- THEN its install commands read `/plugin marketplace add clunny/clm`
