---
title: "Skills say to run clm pr open without the arguments it requires"
kind: bug
components: [host, plugin, plugin.upgrade, plugin.propose, plugin.triage, plugin.init]
issues: [426]
---

Five skills say to open the pull request with a bare `clm pr open`: upgrade (its landing step), configure, init, propose (the small-change route) and triage (its intake step). The command requires a branch, `--title` and `--body-file`, so a session that follows the text exactly gets a usage error. The land still goes through, since `clm land` doesn't depend on the pull request, but the step as written can't succeed and leaves the session guessing. Patching each skill's text leaves the next skill written the same way to repeat it.

Expected: the step works as the skills write it. The recommended design: `clm pr open` with no arguments opens the pull request for the current branch, its title and body taken from the branch's change (`change.md`'s intent) or, for a branch without one, from its commits; `--title` and `--body-file` still override it, for the drafts and folds that write their own body. This fixes every skill in one place, and fits a pull request being only for reading the diff. The alternative, every skill giving the full form with a test that keeps new skills from going bare, repeats the arguments in each skill.
