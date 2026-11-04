---
title: "A crate's scan for the remote's name looks for git arguments, not for the word"
date: 2026-10-27
status: accepted
---

`shared-plumbing-in-git` makes `clm-git`'s `REMOTE` the one spelling of the remote's name in the code, and each crate that moves onto it proves so with a test that reads its own `src/`. `sync/shared-plumbing` and `orchestrator/shared-plumbing` look for a string literal beginning `"origin`, which works there because neither crate prints `origin` to a person. It doesn't work for `land`, whose messages begin `"origin can't be reached` and ``"origin's `main` has moved``, and it misses the refs spelled inside a longer literal, such as `"refs/remotes/origin/main"` and `"+refs/heads/main:refs/remotes/origin/main"`.

So the crates moved after those two (`land`, `ledger`, `host`, `init`, `view`) scan each line of their `src/` outside a `//` comment for the three shapes the name takes as a git argument:

- `"origin"`: the remote itself, as `fetch`, `push`, `ls-remote` or `remote get-url` takes it, or as compared with `git remote`'s output;
- `"origin/`: a remote-tracking ref by its short name, such as `"origin/main"`;
- `remotes/origin`: a remote-tracking ref by its full name, at any place in a literal, including a refspec.

Prose that names `origin` for a person or a session to read, such as `origin can't be reached`, `on origin is no longer at` or `behind origin/main`, matches none of them and keeps its wording: it tells a person where something is, and when the remote's name ever changes it changes with the rest of the documentation, not with the code. Doc comments are skipped because they describe the refs in the same words.

Rejected:
- **Building every message from `REMOTE` too.** It would make the scan a plain search for `origin`, but turns fixed messages that tests and the plugin's prompts match into `format!` calls, for a name CLM doesn't let anyone change.
- **Keeping the `"origin` prefix.** It flags `land`'s messages and misses `refs/remotes/origin/...`, so it is both too strict and too loose for these crates.
- **One workspace-wide test.** It would sit in no node, so no node's requirement would cite it, and a crate that legitimately needs `origin` in a fixture couldn't be scoped out without an exception list.
