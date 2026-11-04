---
title: "The guide's entry pages are written for a newcomer; mechanisms live in How it works"
date: 2026-11-04
status: accepted
---

A newcomer reads the quickstart and then Using CLM. Both had grown the history and machinery an experienced user knows (setup files, `outside`, spec branches, review rules, `clm` commands), which buried what a newcomer needs: install, `/clm:init`, approve the first design, `/clm:build`, and the workflows to run after. So the guide splits by reader:

- **Quickstart**: install to first build, ending with the workflow skills.
- **Using CLM**: the workflows (`/clm:propose`, approving, `/clm:build`, `/clm:triage`, `/clm:health`) as the human sees them, with no `clm` command.
- **How it works**: the mechanisms underneath, for whoever wants them, commands included.

The mechanical part, no `clm <command>` in a code span on the two entry pages, and the skills and links they must hold, is a test (`init/guide-entry-pages`), so the shape doesn't drift back one helpful sentence at a time. The writing itself stays judgement.

Rejected:
- **A prose rule with no test.** Nothing would catch drift (`proven-by-tests`).
- **Word or section budgets.** Catch growth, not mechanism creeping in, and fight every honest edit.
