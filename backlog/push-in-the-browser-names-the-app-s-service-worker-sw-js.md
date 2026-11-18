---
title: "push-in-the-browser names the app's service worker /sw.js"
kind: question
components: [web]
choices:
  - "Reword it to /app/sw.js"
  - "Keep /sw.js as written"
---

`web/push-in-the-browser@b9d56673` says the service worker the app registers is `/sw.js`, and its scenario "the service worker shows a push" has "its service worker `/sw.js` active". The app registers `/app/sw.js` with scope `/app/` (`web/src/main.ts`, `installable`), and `web`'s Design gives `/sw.js` to the retiring root worker (`retire-root-service-worker`). The test in `web/e2e/app.spec.ts` matches `/\/sw\.js$/`, which holds for either, so it proves the behaviour but not the path as written. The spec-auditor found it in the build that re-cited the test. A spec branch should name `/app/sw.js` in the requirement and its scenario, after which the test can match the full path.
