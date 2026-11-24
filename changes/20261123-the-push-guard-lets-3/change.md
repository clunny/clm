---
status: applied
resolves: [the-push-guard-lets-a-push-reach-main-through-include-path-matching-or-a-mapped-branch]
---

## Intent

`push-guard` now reads three more routes by which a push reaches `main`. A push that names a branch with no destination (`git push origin impl/x/b`) is judged as a push of the current branch is, since git maps it through `remote.<name>.push`, or under `push.default=upstream` to the branch's upstream. A push with no refspec is blocked when `push.default` is `matching` or a remote mirrors, since git then pushes `main` with every other branch. And a `-c` or `--config-env` `include.path` or `includeIf.<condition>.path` is treated like the `--config-env` settings the guard can't read: such a push is blocked, and it counts as naming another work tree for `push-guard-scoped-to-clm`. One widened `git config --get-regexp` query reads every setting these cases need.

## The problem

Each of these is allowed today, and git pushes to `main`:

- `git -c include.path=/tmp/p push`, or `P=/tmp/p git --config-env include.path=P push`, where the file sets `remote.origin.push = HEAD:main` (or `includeIf.<cond>.path`). The setting's name isn't under `push.`, `remote.` or `branch.`, so the guard neither passes it to the queries nor treats it as a setting it can't read.
- `git -c push.default=matching push`, or `git -c remote.origin.mirror=true push`, from `impl/x/b` while the local `main` is ahead of `origin/main`. `@{push}` names the branch's own name, and nothing asks about `matching` or `mirror`. The repository's own config does the same.
- `git -c remote.origin.push=refs/heads/impl/x/b:refs/heads/main push origin impl/x/b`, its `--config-env` form, or `git -c push.default=upstream push origin impl/x/c` on a branch tracking `main`. git maps a refspec with no `:` that names a local branch (`builtin/push.c`'s `refspec_append_mapped`), but the guard judges an explicit refspec as written. The scenario "settings from the environment" says the `--config-env` form exits 0.

## Approaches

- **Read the three routes in `push-guard`** (chosen). The `remote.<name>.push` check (from the `-c` options or the repository's config) now covers a named refspec too, as well as no refspec, `HEAD` and `@`. A new case covers `push.default=upstream` (or its old name `tracking`) with a named branch whose `branch.<b>.merge` reaches `main`. Another covers `push.default=matching` or a `remote.<name>.mirror` that isn't false when there's no refspec. The settings these cases read come from one query: `git config --get-regexp '^(push\.default|remote\..*\.(push|mirror)|branch\..*\.merge)$'` replaces today's `^remote\..*\.push$`, so a push of the current branch pays nothing extra. The `include.` and `includeif.` options join the `--config-env` case, and the `--config-env` and `GIT_CONFIG_*` cases now also apply to a named refspec. This keeps the single remedy every reason already suggests: an explicit `HEAD:<branch>` is never mapped.
- **Pass `-c include.path` to the queries and let git read the file**: rejected. It reads a file the command names before the command is allowed, can't help with the `--config-env` form (the path is in a variable), and the included file can also move the work tree.
- **Name the routes in the Design among what gets through**: rejected. Each is a documented git setting given on the command line or kept in the repository's config, which is exactly the kind of push to `main` `guard-blocks-push-to-main` is there to stop. The guard already blocks the `-c push.default=upstream` and `-c remote.origin.push` forms of a push of the current branch.
- **Match git's refspec rules exactly** (only a `remote.<name>.push` whose source matches the branch counts; `matching` only when the local `main` would move): rejected. A backstop takes the safe side (`yagni`), and every over-block costs only a rerun with `HEAD:<branch>`.

## Outcome

`plugin.hooks.guard`:

- `push-guard` defines a **named refspec**: one with no `:`, `*`, `$` or backtick, so `HEAD`, `@` or a branch's name. It splits the current-branch bullet so the `remote.<name>.push` check also covers named refspecs, and adds the upstream-mapping case and the matching/mirror case. It widens the `--config-env` case to `include.`/`includeif.` names and to `-c` `include.`/`includeif.` options, and applies that case and the `GIT_CONFIG_*` case to named refspecs. It names the settings query and says it is asked at most once, and that every line it prints counts. The reason's list of tests gains the upstream and matching/mirror cases and names the `-c` option. The requirement adds the scenarios "a branch named without its destination", "matching and mirroring" and "settings from an included file".
- `push-guard-scoped-to-clm` counts a `-c` or `--config-env` `include.`/`includeif.` option as naming another work tree. It adds the scenario "a work tree set by an included file".
- The Design says why includes are blocked rather than read, why named branches and matching/mirror are judged and by which query, and which over-blocks are accepted. It adds a refspec in a variable that a setting maps to `main` to what gets through.

The build changes `crates/clm-guard/src/push.rs`: the widened query and its parsing, named refspecs, the new cases and their `Reach` reasons, and the include options. It re-cites the two requirements' tests at their new versions and writes the new scenarios' tests. Its tests for "settings from the environment", "configuration from variables" and "a push git doesn't answer" follow the reworded scenarios.

## What the requirements no longer state

- `push-guard`, scenario "settings from the environment": `git --config-env remote.origin.push=B push origin impl/x/b` now exits 2 instead of 0, since git maps a named branch through that setting. The `THEN` now reads "the first four each exit 2 ... and the last exits 0". The reason's wording "says the guard can't read its setting from the environment" becomes "says the guard can't read its setting", since an include option is no setting from the environment.
- `push-guard`, scenario "configuration from variables": its last command was `... git push origin impl/x/b`, which now exits 2. The scenario keeps its outcome (exit 0) with the command `... git push origin HEAD:impl/x/b`.
- `push-guard`, scenario "a push git doesn't answer": its second command was `git push origin impl/x/b`, which now runs the settings query. The scenario keeps its outcome (`Allow` without running the fake `git`) with `git push origin HEAD:impl/x/b`.
- `push-guard`'s third bullet ("no refspec, or only `HEAD` or `@`, and ... the current branch is `main`, or a `remote.<name>.push` value ... from `git config --get-regexp '^remote\..*\.push$'`") is split in two. The `remote.<name>.push` half now also covers a named refspec and reads the wider settings query. No case it blocked is now allowed.

## Constraints

Edge cases and workarounds:

- The guard takes the safe side and over-blocks in two places. A named branch is blocked whenever any `remote.<name>.push` destination reaches `main`, even if that refspec's source is another branch. `matching` and mirror are blocked even when the local `main` isn't ahead. The remedy in both cases is `HEAD:<branch>`.
- `push.default`'s values are compared as git compares them (exactly, lowercase). `remote.<name>.mirror` counts unless its value is `false`, `no`, `off` or `0` in any case, so an empty value (`-c remote.origin.mirror`, which git reads as true) counts.
- A refspec holding `$` or a backtick isn't mapped: it stays judged as written, which the Design lists among what gets through.
- A named refspec git couldn't match to exactly one local branch (a tag, an unknown name) is judged as a branch anyway. That only matters when a setting would map it to `main`, which is the safe side.
- The repository's own `include.path` is followed by the settings query itself. Only the command-line forms are blocked unread.
