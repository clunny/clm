---
title: "The public site is written for search engines and AI agents, from the guide's own files"
date: 2026-11-04
status: accepted
---

The site (`web.site`) should be found by web search and, more, by AI assistants and agents (ChatGPT, Claude, Perplexity) asked how to build software with AI from a spec, and an agent pointed at CLM should be able to read how to use it.

- **Readable without script, with metadata per page.** Most AI crawlers don't run JavaScript, so everything is in VitePress's static HTML. Every page gets its own title, description, canonical link and social card; a guide page's description is its file's opening summary paragraph, which every guide file now has and the build requires, so the guide and `clm guide` gain it too.
- **One `robots.txt` group for everyone.** AI search and AI training crawlers are both welcome: being cited and being known both serve adoption, and CLM is MIT-licensed. Only the invite-only app's paths are disallowed. A named group per crawler would replace the `*` group for it and repeat the disallows, so there is none. Cloudflare's zone-level AI blocking and its managed `robots.txt` are the operator's settings, and both are kept off (checked 2026-11-04, `20261104-cloudflare-ai-crawlers`).
- **Agent files made at build time, never committed.** `llms.txt`, `llms-full.txt` and a Markdown copy of each guide page are generated from `docs/guide/` and `docs/spec-format.md` in the same build that makes the pages, and tests compare each copy with its file, so they can't drift (`guide-from-docs`, `regenerate-dont-merge`).
- **An FAQ page in the guide** (`docs/guide/faq.md`), answering the questions people put to assistants, including how CLM differs from other spec-driven tools, since assistants quote pages that answer the question asked. It is a guide page, so `clm guide faq` prints it, and its `FAQPage` structured data is made from its headings.
- **The GitHub link with its star count, read at build time.** Crawlers see the number, visitors' browsers make no request to GitHub (as `site-analytics-umami` keeps the site's third parties to one), and a failed read never fails a release. The count is as old as the last release; a live count would need a request per visit for a number that changes slowly.
- **Credits name Kubernetes's controllers** beside the Spec Growth Engine, superpowers and Anthropic's playbook: reconciling code to a declared spec is the controller loop, and the homepage's "The loop" already says so. The playbook's credit names what CLM takes from it (intent committed first, human attention at the gates, an agent that can act up to a gate but not pass it, rules held by hooks and versioned config, a failing test first, a loop rather than handoffs) and where CLM goes further: no human reviews code.
- **Off-site presence is the operator's** (`backlog/off-site-presence`, `backlog/search-console-and-bing`): the repository's description and topics, listings, and the operator's own posts.

Rejected:
- **Prerendering or a separate static marketing site.** VitePress already renders static HTML; a second site would be a second copy of the message.
- **Committed `llms.txt` / `llms-full.txt`.** A copy that a check must keep equal, when the build can make it.
- **A live star badge (shields.io or a client-side API call).** A third-party request on every visit, invisible to crawlers that don't run script.
- **Blocking AI training crawlers while allowing AI search.** Training is how assistants come to know CLM without a search; there is nothing on the site to protect.
