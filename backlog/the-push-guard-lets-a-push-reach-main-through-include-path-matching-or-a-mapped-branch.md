---
title: "The push guard lets a push reach main through an included config, push.default=matching, a mirror remote or a branch refspec a push setting maps"
kind: bug
components: [plugin.hooks.guard]
---

`push-guard` reads only `push.`, `remote.` and `branch.` settings from a segment's `-c` and `--config-env` options, and judges a push setting only for a push with no refspec or only `HEAD`/`@`. Found by the code review of the build of `20261123-the-push-guard-lets`. Each of these is allowed, and git pushes to `main`:

- `P=/tmp/p git --config-env include.path=P push`, or `git -c include.path=/tmp/p push`, where the included file sets `remote.origin.push = HEAD:main` (also `includeIf.<cond>.path`). The setting's name isn't under `push.`, `remote.` or `branch.`, so it is neither passed to the queries nor taken as a setting from the environment.
- `git -c push.default=matching push`, or `git -c remote.origin.mirror=true push`, from `impl/x/b` while the local `main` is ahead of `origin/main`. `@{push}` names the branch's own name, and nothing asks about `matching` or `mirror`. This is also true of the repository's own config.
- `B=refs/heads/impl/x/b:refs/heads/main git --config-env remote.origin.push=B push origin impl/x/b`, or the `-c` form, or `-c push.default=upstream` on a branch tracking `main`. git maps a refspec with no `:` that names a local branch through `remote.<name>.push`, or through the upstream under `push.default=upstream`. The guard judges an explicit refspec as written. The scenario "settings from the environment" states that the `--config-env` form exits 0.

Needs a spec change. Either read these routes in `push-guard`, or name them in the Design among what a backstop lets through. See also `the-push-guard-lets-a-push-through-whose-settings-come-by-git-config-count`.
