---
title: "Register clm-app.dev with Google Search Console and Bing Webmaster Tools"
kind: human-task
components: [web.site]
origin: 20261104-found-by-search-and-agents
---

Search engines find a new site faster, and report what they index, once its owner registers it. ChatGPT's search draws in part on Bing's index, so Bing matters as much as Google here. Only the operator can verify the domain. Nothing in the build waits on this.

1. In Google Search Console, add the domain property `clm-app.dev`, verified by the DNS TXT record it gives (in Cloudflare's DNS), and submit `https://clm-app.dev/sitemap.xml`.
2. In Bing Webmaster Tools, import the site from Search Console (or verify it by DNS the same way) and submit the same sitemap. Turn on IndexNow there if Bing offers it for Cloudflare.
3. A week after the release, check both report the homepage and the guide's pages as indexed, and delete this item.
