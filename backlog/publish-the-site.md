---
title: "Publish the public site: open the homepage and guide to everyone"
kind: deferred
components: [web.worker, web.site]
---

The public site (`public-site`) is behind the invited list until the human decides CLM is ready to publicize. Then, in one spec change:

- `web.worker/site-behind-sign-in` changes so the site's paths are served to everyone; the app under `/app/` and the API stay invite-only (`invite-only-web`, amended to say so, and `ARCHITECTURE.md`'s `invite-only-web` reworded).
- The site's install repository (`web.site/install-repository`) names the public repository: `clunny/clm` made public, or a new repository for the public version.
- The guide's quickstart stops asking for access to a private repository.

Only the human decides when; nothing builds this before a spec change says so.
