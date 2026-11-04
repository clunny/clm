---
title: "A change declares the node frontmatter keys it adds, so one spec PR can add a key and set it"
date: 2026-10-26
status: accepted
---

The model rejects a node frontmatter key it doesn't know (`model/valid-frontmatter`), so a typo such as `depend_on` fails loudly rather than being ignored. But the model that `clm check` and `clm apply` run in a spec PR is the released one, built before the PR's requirements. A change that adds a requirement for a new key and also sets that key on a node fails its own check with `unknown field`, so the design had to be split into two spec PRs, each with its own human review (`20261023-web-optional` split `mark-web-optional` off this way).

Now a change's `change.md` may list the keys it adds in `frontmatter_keys`. While the change is live (outside `changes/archive/`, and not `abandoned`), the model accepts each listed top-level key on any node, with any value, and leaves it out of the parsed frontmatter (the node's frontmatter hash still covers the line); `clm apply`'s validation of a delta's frontmatter accepts it the same way. A key the model already knows is validated as before, so once the change's requirements are built the declaration changes nothing. When the change is archived, which `archive-on-sync` does only once its requirements are current, the declaration stops counting and the built model must know the key; if it doesn't, `clm check` fails on `main`'s next sync, which is the loud failure the rule exists for.

Declaring a key is not itself a way round review: setting any frontmatter key other than `status` on a node already makes a spec PR `HARD` (`gate/spec-element-tiers`), so the human sees every use of a declared key before it lands.

So the reviewer sees, in one PR, the requirement that gives the key its meaning, the declaration, and the delta that sets it. The declaration is reviewed intent, like any other line of `change.md`, and typo protection stays for every key no live change declares.

The value of a declared key isn't validated until the model knows the key. If the built model rejects the value the delta set, the build that makes the requirement current fails its checks, and its session raises a `question` item as `requirement-blocks` has it; the next spec PR corrects the value.

The change that adds `frontmatter_keys` can't use it itself: the released model rejects the key in `change.md` until this change is built.

Rejected:
- **A delta that waits to be applied** until the requirements it relies on are current, applied later by `clm sync` on a code branch. It breaks `apply-in-spec-pr`: the reviewer would approve a delta whose applied result they never see, and `specs/` would change on a code branch, where `stale-base` can no longer be settled by a human.
- **Accepting any unknown key.** It removes the typo protection for every node, every day, to serve a case that comes up a few times a year.
- **The list of allowed keys kept in a spec file** (an asset of `model`) that the model reads. It is permanent, so every key is declared twice, in the list and in the code, forever; and a key listed but never built would be accepted silently. A declaration on the change expires with it.
- **Checking with a model built from the branch.** A spec PR has no code for the new requirement to build.
