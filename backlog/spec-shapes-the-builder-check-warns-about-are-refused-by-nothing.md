---
title: "The spec shapes the builder check warns about are refused by no program"
kind: idea
components: [check, plugin.propose]
---

The propose skill's builder-check step warns about shapes of a spec change that can't be built in order: a requirement that removes or renames part of a Contract another component's code uses and holds no `#### Needs` for that component's move (`contract-removal-needs`); a rename written as one requirement, not an adding one and a removing one (`rename-adds-then-removes`); a requirement that relies on work of another component that no requirement of that component states (`dependency-work-is-a-requirement`).

Today only the skill's text asks for these, and the requirement that pinned that text, `plugin.propose/builder-check`, is deleted in phase 2 of `backlog/a-passing-check-proves-less-than-the-spec-claims` as a scan of prompt text. The plan's rule is that an instruction that must hold moves into a program. Which of these shapes `clm check` can see in a change's deltas and the code, and refuse, is the question; those it can't see stay with the builder check, measured by the propose skill's evals in phase 6.

Nothing is built for this now. It is proposed after phase 2.
