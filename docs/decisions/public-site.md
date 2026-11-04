---
title: "CLM's public site: homepage and guide at clm-app.dev's root, built with VitePress; the app moves to /app/"
date: 2026-11-01
status: accepted
---

CLM gets a public site (`learn-clm`): a homepage at `https://clm-app.dev/` that makes CLM's value clear within a minute, and the user guide at `/guide/`, one page per file of `docs/guide/`. It lives in the new node `web.site`.

**Where.** At the root of `clm-app.dev`: no second domain to buy or point. The browser app moves to `/app/`, and the root is always the homepage, signed in or not. The Worker's own routes (`/api/*`, `/auth/*`, `/webhook`) stay, so the GitHub App's manifest doesn't change; sign-in returns to `/app/`.

**The installed app is the app only.** Its manifest's `id`, `start_url` and `scope` are `/app/`, and its service worker's scope is `/app/`, so it never caches the site and opens the site's links in a browser tab, as installed web apps normally do. The explicit `id` keeps the installed app's identity through any later move; this move needs one reinstall, a burden of the change that makes it. The guide isn't offline in the installed app: `clm guide` covers that.

**Built with VitePress** (`reuse-first`). It gives the guide its sidebar, search, links between pages, light and dark, and phone layouts, and hosts the custom homepage. Rejected: our own pages in the app's Vite build (our own Markdown step, sidebar, search and phone layout, own code with no reason to write it), and the guide inside the app as the cut Guide view had it (slow first paint, unreadable by search engines).

**The homepage leads with reconciliation.** Of three first-screen directions (the promise, the reconciliation loop, the contrast), the human picked the loop: "Declare the intent. AI builds the code to match. CLM keeps it matching." The first screen must catch attention alone; the page scrolls, and five short sections elaborate. Kubernetes is credited as the inspiration for reconciliation only, below the first screen, never so that CLM reads as a tool for Kubernetes. The homepage credits the Spec Growth Engine paper, superpowers and Anthropic's AI-native SDLC playbook. `web.site`'s `assets/mockups/homepage.html` is the reference.

**Invite-only until launch** (`invite-only-web`). The Worker serves the site only to signed-in invited people until the human decides CLM is ready to publicize; `backlog/publish-the-site` holds that step. At launch a visitor adopts CLM on their own, installing from `clunny/clm` made public or a new public repository, so the homepage names the install repository in one place.
