---
title: "land's Contract still lists run_check after runner-names-its-caller removed it"
kind: bug
components: [land]
---

Change `20261122-the-check-runner-s` had `land`'s Contract list both runner forms while both existed, with `run_check` "offered only until `runner-names-its-caller` removes it". No delta drops that entry once the function is gone. The build of `land/runner-names-its-caller` removed `clm_land::run_check`, and `clm rebase` archived the change, so `specs/land/SPEC.md`'s Contract (the `run_check(command, dir, selection, env, log)` bullet) now describes a call that doesn't exist. That contradicts `land/runner-names-its-caller`. Fixing it takes a spec change that drops the bullet. An impl branch can't make that change.
