---
title: "A host's text is judged by a verifier that can only read, and reaches the backlog only through the human"
date: 2026-11-04
status: accepted
---

An issue's or pull request's text is a stranger's, and any instruction hidden in it would reach every session that reads what is made from it: a prompt injection. Triage contains it in four ways:

- **`clm intake` prints no host text.** Its lines hold numbers, slugs, change IDs, commits and logins only, so a title can't forge a line a session parses. A new dependency alert's line also carries its severity, ecosystem, package and manifest path (`dependency-alerts-through-intake`): the names of the project's own dependencies and files and a fixed set of severities, each one word, since `host` replaces whitespace and control characters, so no field can start another line; the advisory's own text is never printed. An alert has no stranger's text, so it skips the verifier and goes to the human straight from its line.
- **The verifier can only read.** `issue-verifier` has `Read`, `Glob` and `Grep` and nothing else: no `Bash`, so no `gh`, no `clm` and no command an issue asks for; no `Edit` or `Write`. The text it reads can at most skew its verdict, which advises (`deterministic-enforcement`). It reports passages that read as instructions.
- **The item is written in other words.** The verifier's proposed item says what happens and what's expected without copying the issue's sentences, and names the issue by number in `issues`.
- **The human reads the source.** Before anything lands or is written to the host, the human sees each issue's own text whole beside the verdict and the whole item, as `/clm:contribution` shows a pull request's whole diff (`outside-contributions-are-intent`). Their reading is the defence no program gives.

Rejected:
- **Showing the human only the verifier's summary.** The summary is written by a model that read the injected text; only the source shows what it said.
- **A verifier with `Bash`, to run `clm context`.** It could then run whatever an issue asks. It reads `specs/` directly instead.
- **Stripping instructions from the text by a filter.** No program can tell an instruction from a bug report's description of one.
