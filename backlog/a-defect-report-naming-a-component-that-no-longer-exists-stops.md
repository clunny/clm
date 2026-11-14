---
title: "A defect report naming a component that no longer exists stops every clm command in that checkout"
kind: bug
components: [plugin.hooks]
origin: 20261114-proof-is-real
---

Step: a session reports a defect with `clm defect --component <component>`, naming a component a change has since removed.

What happened: the report was written as a backlog item whose `components` names the removed component, in the checkout the session ran in, uncommitted. From then on every `clm` command in that checkout failed to load the backlog and printed only `unknown component`, so `clm status`, `clm pending` and `clm backlog` showed nothing until a person edited the file.

Expected: `clm defect` refuses, or drops, a component the specs don't hold, so a report can never be the thing that stops the commands that would show it.

Rule involved: `defect-reports`.
