---
status: applied
resolves: []
---

# Trim the off-site presence task to what's left

## Intent

`backlog/off-site-presence` asks the human to set the GitHub repository's description, website and topics, upload its social preview, submit listings and link their own posts to `https://clm-app.dev`. The description, the website and the topics are already set on `clunny/clm` (checked with `gh repo view` in the triage session). The human chose to trim the item to what's left, so this change edits its first step down to the social-preview upload, which couldn't be checked, and keeps the listings and the posts. It changes only the backlog.

`resolves` names no item: the item stays open with the steps left, and a change that resolves it would delete it when it is archived (`task-deletes-fixed-items`), losing those steps.

## The human's answers

> "Trim done steps (Recommended)" — Draft a change that edits the item down to what's left: the social preview (if not done), listings and posts.
>
> (Context Claude checked with `gh repo view`: the repository's description, website https://clm-app.dev and topics ai-coding, claude-code, developer-tools, rust, spec-driven-development, ai-agents, claude-code-plugin, requirements are already set. The social-preview upload could not be checked, so keep it in the item.)
