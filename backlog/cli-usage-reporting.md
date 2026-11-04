---
title: "Usage reporting: how CLM is used after install, and where people stop"
kind: idea
components: [clm, plugin]
---

## Intent

The site's analytics (`site-analytics-umami`) show newcomers up to a copied install command. What happens next, in people's own repositories, is where most friction lies: whether `/clm:init` finishes, whether a first change is proposed, approved and landed, which checks misfire, and where people stop using CLM. None of it reaches the maintainer today: each repository's ledger and `clm defect` items stay in that repository.

## Open questions

- Opt-in telemetry from the `clm` binary, a voluntary `clm feedback`/report command, or both? Consent, what is sent, and where it goes (`git-is-truth`, `web-optional`: CLM must work the same with it off or unreachable).
- What would be counted: commands run and their exit codes, the steps of the propose → approve → land path reached, defects filed?
- Installs: read GitHub's release download counts and repository traffic (views, clones, stars), which no browser setting affects; GitHub keeps traffic for 14 days only, so it would need a periodic snapshot.
