---
title: "Skill and prompt text is source, and a requirement states only what a program computes"
date: 2026-11-13
status: accepted
---

About 120 requirements were satisfied by a sentence existing in a skill, an agent definition, a prompt a program writes or an instruction file. Their tests read the file and looked for strings. Such a test fails only when someone edits the sentence, and it says nothing about whether a session follows it, so a passing check proved less than the spec claimed (`deterministic-enforcement`: a sentence in a prompt enforces nothing).

The skills under `plugin/skills/`, the agent definitions, the command files, the fixed text of the prompts `clm run`, `clm draft` and the build loop write, and the instruction files `AGENTS.md` and `CLAUDE.md` are source files their component owns. What they tell a session is not a requirement. A build edits them as it edits code, and the edit shows in the diff.

A requirement about one of these files states only what a program computes or does:

- that the file ships, with the frontmatter the agent harness reads (a skill's `description`, an agent's `name`, `tools` and `model`);
- a value a program puts into a prompt: a component ID, a branch, a path, a model name, a slot, a list derived from the repository;
- text that is present or absent under a computed condition, such as the setup step only when `[build] setup` is set.

A requirement that states both is split: the computed part stays under its ID and the wording goes.

An instruction that must hold moves into a program that refuses the bad outcome (a check, the gate, the guard), and until one exists it is a backlog item naming that program, not a requirement. What only a model's behaviour can show, such as whether a session treats an issue's text as data, is measured by an eval once the method has them (`capability/measure-by-evals`).

Considered:
- Keeping a requirement per skill for the steps that matter most. Rejected: it is still a scan of prompt text, and choosing which sentences matter is the judgement an eval should measure.
- Deleting every requirement about a written prompt. Rejected: that a prompt names the right branch, model or check command is behaviour a test can prove.
