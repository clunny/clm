---
title: "A defect reported from the build session stays an uncommitted file and nothing says how it reaches main"
kind: bug
components: [plugin.build]
---

Step: `clm defect` run by the session that drives `clm build`, in the main checkout on `main`, as the build skill tells it to when it meets a defect.

What happened: the report was written as a new backlog item in the working tree and left untracked. The build skill says the session opens no pull request of its own and that the loop drafts reported defects with the backlog's other bugs, but it names no step that commits the item or carries it to `main`, and the session may not push `main` itself. The item was not on `origin/main` afterwards, so a propose session or a draft run working from `origin/main` could not see or claim it. The session also asked the human whether to file at all, because it could not tell what writing to the checkout's `main` would do.

Expected: the skill, or `clm defect` itself, says how a report made in the build session reaches `main` (for instance the loop carrying new untracked backlog items on its next spec or archive branch, or `clm defect` landing the item), so the session can file the moment it meets the defect without asking, and the item is draftable at once.

Rule involved: `defect-reports`, and the build skill's sentence that the loop drafts reported defects.
