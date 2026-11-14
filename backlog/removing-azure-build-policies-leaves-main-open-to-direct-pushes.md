---
title: "Removing Azure DevOps build-validation policies leaves main open to direct pushes"
kind: idea
components: [plugin]
---

## Intent

The init skill tells the human that the host's build-validation policies stop being the gate once `clm land` lands, and that the land identity needs "Bypass policies when pushing". On Azure DevOps a branch's required policies are also what reject direct pushes to it. When a project moves its pipeline's build and test stages into the check script, the human naturally deletes the now-empty build-validation policies, and if they were the only ones, `main` becomes pushable by anyone with write access: `clm land` stays the route only by convention.

In the first CLM project on Azure DevOps the human deleted all three policies knowing this, so this is advice, not a defect. Reported from that project's session on 2026-11-10.

## Open questions

- Should the skill say, where it names the policies, that removing them all unprotects `main`, and suggest keeping one inexpensive required policy (or a branch lock) with the bypass grant for the land identity?
- Does GitHub's equivalent (branch protection's required checks) need the same sentence?
