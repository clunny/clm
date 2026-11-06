---
title: "ci-check does not install web/site deps for capabilities/web-only changes"
kind: bug
components: [ci]
---

ci-check.sh installs web/site's dependencies only when web/ changed. capabilities/web's see-site-use and publish-posts specs run 'npx vitepress build' in web/site, so a change touching only capabilities/web/ runs them without web/site/node_modules. Fix: also run web/site's npm ci when cap_web is true.
