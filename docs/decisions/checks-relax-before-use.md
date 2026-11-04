---
title: "A branch is judged by the running clm's rules, so a relaxed check lands before anything relies on it"
date: 2026-10-26
status: accepted
---

`clm sync` and `clm land` both run `clm check` from the binary that runs them: `sync` links `clm-check` in, and `land` runs its own executable (`std::env::current_exe`). Wherever the installed `clm` is a release, or tracks `main` as this repository's does (`dogfood-from-main`), that is `main`'s rule, which is what a branch's merged tree will live under. In CLM's own repository a branch can change `clm check` itself, and then the rules it builds differ from the rules that judge it.

A build of `20261022-backlog-block-deadlock/model` hit this (`installed-clm-sync-rejects-a-task-that-relaxes-a-model-rule`): its task removed a `model/parse-changes` check and, in the same branch, deleted a backlog item that only the relaxed rule accepted. The installed `clm sync` failed with the violation the task existed to remove; `cargo run -p clm -- sync`, built from the branch, passed.

**Decision.** Both commands keep judging a branch by the running binary's rules, and a change that relaxes a check is landed in two steps, as `expand-contract` lands a breaking contract:

1. The branch that relaxes the check carries nothing that only the relaxed check accepts, so `main`'s rule passes it.
2. Whatever relies on the relaxation (deleting an item, a new file shape, a frontmatter value the old rule refused) goes in a later branch, once the first has landed and the installed `clm` has been rebuilt from `main` (`.git/clm-install.log` says when).

A proposing session that designs both steps puts the second's content in a separate change or backlog item, and a build that finds it can't pass `clm sync` without the relaxation leaves that content out and files it as a backlog item, rather than reaching for another binary.

The second step goes through whatever path its content needs on its own: deleting a backlog item still needs a live change that `resolves` it, or a fix branch whose commit names it with a `Resolves:` trailer (`code-fix-resolves-bug`), exactly as if the relaxation had always been there. "Rebuilt from `main`" means the `clm` that runs is built from a `main` that holds the relaxation: here the `post-merge` hook of `dogfood-from-main` rebuilds it after a pull on `main`, and since `clm land` pushes `main` without pulling, a session that needs it sooner runs `cargo install --locked --path crates/clm` from an up-to-date `main`.

Consequences:
- `clm sync` prints nothing about which rule judged the branch: the rule is always the running binary's, so there is nothing to tell apart.
- No session prompt, skill or command names `cargo run -p clm` for `clm sync` or `clm land`. A session that ran sync from its branch's build would pass sync and fail at `clm land`, later and with more work to redo.
- Tightening a check needs no split: the branch that tightens it fixes what the new rule rejects, which `main`'s rule accepts too. `clm sync` runs no tests, so it won't catch content the tightened rule rejects; `cargo test`'s `this_repository_is_clean` does, which every session runs before landing (`CLAUDE.md`) and `clm land` runs in the project's check command. That gap is the ordinary one between sync and the tests, not a disagreement between two rules.
- Remedies that ask for a `clm` built from the branch are ruled out here, including the first one `installed-clm-sync-skips-the-size-check-on-a-clm-toml-key-it-predates` offers; its other remedy, failing sync on a `clm.toml` it can't load, is untouched.
- Adopters never see the split: their branches don't change `clm`, so the running rule and the shipped rule are the same.

Rejected:
- **`clm sync` runs the check from the branch's build** (`cargo run -p clm -- check` when the repository is CLM's own). It special-cases one repository, compiles on every sync, and still disagrees with `clm land`, which by `land`'s design lands a changed gate or check only once it is on `main`.
- **The session prompt says which binary to run.** The same disagreement with `clm land`, and the prompt would carry a rule that holds only here.
- **`clm land` also runs the branch's build.** A branch could then land by passing rules it wrote itself, which `deterministic-enforcement` and `land`'s own design rule out: the rule that guards `main` is `main`'s.
