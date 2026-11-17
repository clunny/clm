---
title: "A setting's value is offered to build before the install item settings-before-use asks for exists"
kind: bug
components: [orchestrator.run]
---

Step: the unattended build of `config` that brought `config/verify-here` to fb827562 (change 20261116-a-mutation-pass-that-stops).

What happened: the change added the `[verify]` fields `mutate_jobs` and `mutate_minutes` and has this repository's `clm.toml` set them. config's Design (`settings-before-use`) says the build that makes the parsing requirement current files `backlog/install-clm-for-<key>.md`, blocking the requirement that sets the value. That build (ba6623d4) filed no such item. Nothing checked for it, so `clm next` offered `config/verify-here` as ready, and this build's prompt told it to write the value. Only `/code-review` noticed. The build then reverted the value and filed the item itself.

Expected: the step can't be skipped silently. The build prompt for a requirement that parses a key this repository sets could name the item to file, or a check could refuse a `clm.toml` value whose key is new on the branch's `main` while no install item for it has landed and been deleted.
