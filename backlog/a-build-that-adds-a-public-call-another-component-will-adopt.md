---
title: "A build that adds a public call another component will adopt cannot land on its own"
kind: bug
components: [check.unreferenced, gate]
---

A requirement asks a library for a new public call that another component adopts in a later build (expand, then contract). The build of the providing component fails `clm check`: the unreferenced-public-items detector reports the call as public with no use outside its library, and uses in the library's own tests do not count. The allowance is an `--allow` entry in `clm.toml`, which is one component's file, so the providing build may not add it (`path-ownership`), and an entry added first is itself a finding because the item does not exist yet. So the only way to land an expand step is one branch that changes two components' files, which `land/build-holds-own-files` refuses for a claim and which the routes agreed on 2026-11-21 refuse for every code branch. Expected: a build can declare, in its own files, that a new public call awaits a caller, naming the change that brings the caller, and the declaration goes stale when the caller arrives or the change is archived (`expand-contract`, `no-dead-code`, `own-files-at-the-gate`).
