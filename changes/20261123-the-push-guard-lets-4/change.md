---
status: applied
resolves: [the-push-guard-lets-git-push-origin-head-branch-from-main-reach]
---

## Intent

`push-guard`'s on-`main` case now holds when any refspec of the push is `HEAD` or `@`, with or without a leading `+`, not only when every refspec is. From a checkout on `main`, `git push origin HEAD impl/x/b` and `git push origin +HEAD` push the local `main` to `origin/main`, and the guard lets both through today.

## The problem

`push-guard`'s third case reads "has no refspec, or only `HEAD` or `@`, and ... the current branch is `main`". git pushes the current branch for every `HEAD` or `@` refspec, whatever else the push names, so `git push origin HEAD impl/x/b` from `main` updates `origin/main` beside `impl/x/b`. No other case catches it: `HEAD` has no destination that reaches `main` as written, and the settings cases find nothing in a plain repository. The guard exits 0 (found by the code review of the build of `push-guard@d829891a`).

A forced `+HEAD` (or `+@`) misses the case too: it isn't `HEAD` as written, and the guard reads it as a named refspec for a branch called `HEAD`, which no setting maps to `main`. So `git push origin +HEAD` from `main` force-pushes `main` with exit 0.

## Approaches

- **Any refspec that is `HEAD` or `@`, with or without a leading `+`** (chosen). This matches what git does: each such refspec pushes the current branch, whatever else the push names. It costs one `git symbolic-ref` query, which a push with only `HEAD` or `@` already runs, and a push with a `HEAD` refspec still goes on to the settings cases when the branch isn't `main`.
- **List these pushes among what gets through**: rejected. Both are plain, literal pushes to `main`, which is what `guard-blocks-push-to-main` exists to stop, and the fix is one condition.

## Outcome

`plugin.hooks.guard`:

- `push-guard`'s on-`main` case holds for a push with no refspec or with any refspec that is `HEAD` or `@`, with or without a leading `+`. The upstream-mapping case excludes `HEAD` and `@` with or without a leading `+`, so `+HEAD` is never read as a branch called `HEAD`. The sentence on `@{push}` now says it isn't asked for a push with a refspec, which is what the case's "has no refspec" already meant.
- The requirement adds the scenario "HEAD beside other refspecs".

The build changes `crates/clm-guard/src/push.rs`. `Push::head_only` becomes "no refspec, or some refspec is `HEAD` or `@` after a leading `+` is stripped". The `branches` filter drops `+HEAD` and `+@` as well. The build re-cites `push-guard`'s tests at its new version and writes the new scenario's test in `crates/clm-guard/tests/guard.rs`.

## What the requirements no longer state

- `push-guard`, the third bullet of its first `SHALL` sentence: "has no refspec, or only `HEAD` or `@` (git's name for `HEAD`), and, in the checkout it pushes from, the current branch is `main`" becomes "has no refspec, or a refspec that is `HEAD` or `@` ..., with or without a leading `+`, whatever other refspecs it has, and ... the current branch is `main`". It widens what is blocked, and no push it blocked is now allowed.
- `push-guard`, the sentence closing the `@{push}` bullet: "so `@{push}` isn't asked for a push of `HEAD` or `@`" becomes "so `@{push}` isn't asked for a push with a refspec". The bullet's "has no refspec" already limited `@{push}` to such pushes, so nothing it decides changes.

## Constraints

Edge cases:

- From `main`, a push of `HEAD` beside a named branch is blocked by the on-`main` case before the settings query is asked, as a push of `HEAD` alone is now. From another branch, the push goes on to the settings cases for its named branches.
- `HEAD:<branch>` is unaffected: it holds a `:`, so it is neither `HEAD` nor a named refspec, and it reaches `main` only as written. It stays the remedy every reason suggests.
