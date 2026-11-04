---
title: "The guard reads quotes, wrappers and push config, and blocks the host's merges into main"
date: 2026-10-27
status: accepted
---

Refines `guard-matches-writes` and `guard-blocks-push-to-main`. The guard judged each segment's whitespace tokens, cut at every `|`, `;`, `&` and newline whatever the quotes, and found a command only as a segment's first token after assignments and `env`. So ways of moving `main` that a session could reach by accident got through: `(git push origin main)`, `{ ...; }`, `if ...; then git push origin main; fi`, `sudo` or `time` before `git`, `bash -c '...'`, a branch cut from `origin/main` under `push.default=upstream`, `-c remote.origin.push=HEAD:main`, `GIT_DIR=...`, `git -C "../my main"`, and the host's merge routes (`repos/<o>/<r>/merges`, merging a pull request, GraphQL `updateRef`). And quoted prose was refused: `git commit -m "build; git push origin main is forbidden"` was blocked, because the text after `;` started with `git`.

The guard now:
- splits segments and tokens outside quotes, falling back to the old quote-blind split when the quotes don't balance, so an apostrophe never hides a command (`guard-quotes`);
- also splits at `(`, `)` and the backtick, so subshells and substitutions are segments, and scopes a `cd` inside `( )` to it;
- skips a fixed set of shell words and prefix commands (`!`, `{`, `if`/`then`/`elif`/`else`/`do`/`while`/`until`, `time`, `timeout`, `env`, `sudo`, `doas`, `command`, `exec`, `nohup`, `nice`, `ionice`, `stdbuf`, `xargs`, with their options) before the command, for every rule, not only the push rule (`guard-command-position`), reading assignments anywhere in that prefix and following `env -C` and `sudo -D`;
- judges a shell's `-c` script as commands, which quote-aware tokens make cheap: the script is one token (`guard-shell-scripts`);
- for an implicit push, reads `remote.<name>.push` refspecs and, with no refspec at all, asks git where the branch pushes (`@{push}`), with the segment's `--git-dir`, `--work-tree`, `GIT_DIR` and `GIT_WORK_TREE`, and only its `push.`, `remote.` and `branch.` `-c` options: the hook runs before the command is allowed, so it hands git nothing else from it;
- blocks `gh pr merge`, writes to `pulls/<n>/merge`, merges through `repos/<o>/<r>/merges` with base `main`, and the GraphQL mutations that merge or move refs. CLM never merges a page (`land-locally`), so none of them is part of its flow.

It stays a backstop for accidents, not a shell parser (`hook-logic-in-clm`). `eval`, a command in a variable, an `export`ed `GIT_DIR`, a script file, a `$( )` inside double quotes, a refspec `xargs` reads from stdin and the contents API still get through.

The label gaps raised beside these are moot: the label rules went with the labels (`per-writer-ledgers`), and a `gh` call that adds `spec-approved` or removes `spec-review` does nothing CLM reads.

Alternatives considered:
- A real shell parser (a crate such as `conch-parser` or tree-sitter-bash). Rejected for now: it would close more gaps, but it is a new dependency for a backstop, and it would still miss `eval` and variables. Reopen if the token rules keep growing.
- Skipping wrappers only for the push rule. Rejected: `(clm approve spec/x)` would then record an approval without the human's confirmation, which matters more than a push.
- Asking git with `git push --dry-run --porcelain` where an implicit push goes. Rejected: it contacts the remote, which is slow, may prompt for credentials, and fails offline; `@{push}` and the config are local.
- Blocking `gh pr merge` only when the base is `main`. Rejected: finding the base needs a host lookup, and CLM's flow merges no page into any base.
