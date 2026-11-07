---
title: "The push guard blocks pushes to main in repositories that aren't under CLM"
kind: bug
components: [plugin.hooks.guard]
---

Step: a session in a CLM repository ran `git push -u origin main` inside a separate, throwaway git repository (a fresh `git init` in the session's scratchpad, with its own GitHub remote, no `specs/` and no `clm.toml`), to spike a GitHub behaviour.

What happened: `clm hook guard` blocked it with `guard-blocks-push-to-main` ("nothing moves `main` but `clm land`"). The session had to rename the scratch repository's default branch to get past it.

Expected: the guard protects the `main` of repositories under CLM, where `clm land` is the only way onto `main` (`land-locally`). A repository with no CLM state has no `clm land`, so the guard has nothing to protect there and should let the push through, or at most ask instead of blocking.

Rule involved: `plugin.hooks.guard`'s `push-guard` requirement, which blocks a push to `main` in whatever checkout the push runs from (`guard-blocks-push-to-main`). Scoping it to checkouts under CLM needs a spec change.
