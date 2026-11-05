---
description: Answer questions about CLM and how to use it, for a new user among others, from what the installed clm carries. Use when someone asks what CLM is, how something works, why a constraint exists or which skill to run. Read-only.
---

# Help with CLM

Answer the person's question about CLM from what the installed `clm` carries, never from this repository's files or a URL (`method-in-the-plugin`): the binary prints the guide offline, at the plugin's version. Help changes no file.

1. Run `clm guide` for the list of pages, then `clm guide <page>` for the pages the question needs. Run `clm guide constraints` for CLM's carried constraints (`carried-constraints`).
2. In a repository under CLM (`specs/SPEC.md` exists), run `clm status` and `clm backlog` when the question is about its state. Outside one, explain how to start from `clm guide quickstart` and point at `/clm:init`.
3. When asked why a constraint exists or whether a project can do without it, say its tier and what CLM relies on, from `clm guide constraints`: a `replaceable` constraint can be replaced by a constraint of the project's own that keeps its guarantee, and a `default` one can be dropped, both through `/clm:propose` and a human's approval; a `method` one can't be replaced.
4. Point at the skill that acts: `/clm:init` to bring a repository under CLM, `/clm:propose` to turn an intent into a spec change, `/clm:build` to build what is approved, `/clm:triage` to work the backlog down, `/clm:health` for the project's health, `/clm:contribution` to take in an outside pull request or issue, and `/clm:view` to watch the state.
5. When the guide doesn't answer the question, say so rather than guess.

Change no file and run no command that writes: no `clm claim`, `clm apply`, `clm land`, `clm approve` or `git commit`. Never push `main` by any means but `clm land`. Run `clm approve` only after the human says yes in this conversation, and only through the guard's confirmation; this skill never runs it.
