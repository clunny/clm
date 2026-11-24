---
status: proposed
resolves: [the-push-guard-lets-git-push-origin-head-branch-from-main-reach]
---

## Intent

`push-guard`'s on-`main` case now holds when any refspec of the push is `HEAD` or `@`, with or without a leading `+`, not only when every refspec is. From a checkout on `main`, `git push origin HEAD impl/x/b` and `git push origin +HEAD` push the local `main` to `origin/main`, and the guard lets both through today.
