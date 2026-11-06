---
title: "clm pending describes the checkout without naming its commit"
kind: bug
components: [view, plugin]
---

Step: `/clm:build`, whose final summary reads blocked requirements from `clm pending`, in a session whose checkout the skill forbids syncing.
What happened: on a checkout behind origin/main, `clm pending` listed a requirement as behind and others as blocked after builds had landed them or their blockers, while `clm next` read origin/main and disagreed. Its output names no commit and gives no behind warning, and it has no way to read origin/main.
Expected: `views-name-their-commit` says a view must say which commit it describes and when that commit is behind origin/main, but its rule names only `clm status` and `clm view`. `clm pending` should follow it, or the skill should get the summary's blocked requirements from something that reads origin/main. No requirement states this yet, so it needs a spec change.
