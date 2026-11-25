---
title: Tests of web notice no change to its code
kind: bug
components: [web]
unproven: [web/no-install-scripts@9b7f85e6]
---

After the land `3bcbeef8` the mutation pass changed the code of `web`, and the tests that cite these requirements passed with every change it tried:

- `web/no-install-scripts`: `web/src/npmrc.test.ts` `ends up with ignore-scripts=true`

A test that proves the requirement fails when its code is wrong: strengthen these so one does, or, when they are already sound, delete this item.
