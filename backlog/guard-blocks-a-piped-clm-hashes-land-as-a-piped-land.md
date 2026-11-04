---
title: "Guard blocks a piped clm hashes land as a piped land"
kind: bug
components: [plugin.hooks.guard]
origin: 20261104-spec-paths-vs-clm
---

`clm hook guard` blocked `cargo run -q -p clm -- hashes land | grep spec-branch` under `guard-refuses-piped-land`, though that runs `clm hashes` with the component `land` as its argument and lands nothing. `plugin.hooks.guard/guard-refuses-piped-land` says a `cargo run` segment runs `clm land` when "a token after the subcommand" is exactly `land`, which matches any later argument named `land` (a component ID such as `land` or a branch named `land`), not only `clm`'s own subcommand, the first token after `--`. A draft hit it while reading `clm hashes land`, and had to redirect the output to a file instead. The fix needs the rule to say that `land` must be `clm`'s first argument after `cargo run`'s `--`, so it needs a spec change.
