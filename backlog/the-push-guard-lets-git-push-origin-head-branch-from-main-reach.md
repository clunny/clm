---
title: "The push guard lets git push origin HEAD <branch> from main reach main"
kind: bug
components: [plugin.hooks.guard]
---

`push-guard`'s on-`main` case holds only for a push with no refspec or "only `HEAD` or `@`". From a checkout on `main`, `git push origin HEAD impl/x/b` pushes the local `main` to `origin/main` beside `impl/x/b`, yet no case of the requirement blocks it: the refspecs aren't only `HEAD`, `HEAD` has no destination that reaches `main` as written, and the settings cases find nothing. The guard exits 0. Found by the code review of the build of `push-guard@d829891a`. A fix needs the requirement to say the on-`main` case holds when any refspec is `HEAD` or `@` (`code-fix-needs-a-written-rule`).
