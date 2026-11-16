---
title: "Three commands-in-crate builds left listed tests in the root: redesign prove-by-tests' move?"
kind: question
components: [init, config, check.unreferenced, clm]
origin: 20261115-commands-in-their-crates
blocks: [init/commands-in-crate]
choices:
  - "Keep the plan; answer each item"
  - "Redesign how the moves end"
---

`clm workaround` exited 3 for `init/commands-in-crate`: `capability/prove-by-tests` now holds three unsettled workaround flags, each from a `commands-in-crate` build that moved most of its listed tests into its own crate but had to leave some in `crates/clm/tests/` (`redesign-before-workaround`):

- `backlog/workaround-config-commands-in-crate-1.md`: two of config's tests run other components' commands.
- `backlog/workaround-check.unreferenced-commands-in-crate-1.md`: two of check.unreferenced's tests observe only the `clm` process.
- `backlog/workaround-init-commands-in-crate-1.md`: seventeen of init's tests, eleven running `clm check`, `clm hashes`, `clm pending` or `clm setup` on what `clm init` wrote, and six guide tests that the root's `doc_readers.rs` names by path (`backlog/seventeen-init-tests-can-t-move-into-its-crate.md`).

They share one cause: a scenario that only the binary, or another component's command, can observe has no place in the component's own crate, so each build leaves the test where it was and files a question. `backlog/two-config-tests-run-other-components-commands.md` was answered with one spec change, after the build run drains, that moves every such scenario into requirements of the root. The question is whether that answer settles these flags as a class, or whether the moves need a different design (such as a test package that holds the binary and is no component's, or `depends_on` edges for tests alone).

init's build landed what moved (83 of 100 tests, the calls and the routing) and stopped work on the requirement there.
