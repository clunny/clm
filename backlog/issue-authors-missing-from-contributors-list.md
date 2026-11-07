---
title: "Someone who only opens an issue may not appear in the project's contributors list"
kind: idea
components: [plugin]
---

## Problem

When an issue is taken in, through `/clm:contribution` or `/clm:triage`'s inbox step, the commit that lands its backlog item credits the issue's author with a `Co-authored-by` trailer (`outside-contributions-are-intent`). `docs/guide/contributions.md`'s Credit section says this may not add them to the project's contributors list, and that only a pull request of their own gets them full credit.

## Idea

Make the issue's author the **author** of the commit that adds their backlog item, with the maintainer as committer and the maintainer and Claude as `Co-authored-by`:

```
git commit --author="<login> <<id>+<login>@users.noreply.github.com>"
```

GitHub links every `<id>+<login>@users.noreply.github.com` address to its account, so the author needs no setup and does nothing. `clm land` keeps commits, and a rebase keeps their author, so the authorship reaches `main`.

Points to settle in the proposal:
- Only the commit that adds their item carries their name, never the later spec or code work, since its wording is the maintainer's or Claude's.
- An `intake/` branch taking in several issues needs one commit per issue author. When duplicates merge into one item, one author is the commit's author and the rest stay co-authors.
- Whether to tell the person in the `accepted` comment, or ask first.
- A deleted account resolves to GitHub's ghost user. That's harmless.
- It changes the `contribution` and `triage` skills' specs (`specs/plugin/SPEC.md`), the Credit section of `docs/guide/contributions.md` and `outside-contributions-are-intent`, so it goes through `/clm:propose`.

## Spike so far (2026-11-07)

A private throwaway repository, `clunny/clm-author-spike`, with default branch `trunk`. The push guard blocked `main` there; see `backlog/the-push-guard-blocks-pushes-to-main-in-repositories-that-aren-t.md`. Its commits:
- A control commit by an identity linked to no account, with only a `Co-authored-by` trailer for the maintainer's account.
- Two commits authored by the maintainer's account through its noreply address, with that unlinked identity as committer: one pushed directly, and one through a `--no-ff` merge like `clm land`'s.
- One commit authored by a second account the maintainer owns (used with their agreement), committed by the maintainer's account, through its noreply address. This is the real setup.

Findings:
- **Making them the author works with no action on their side.** The commits API links each commit to its author's account, through the merge too.
- **Insights → Contributors (`stats/contributors`) counts co-authors as well as authors.** The maintainer was counted 4 times but authored only 2 commits. The other 2 are the control commit, linked only by its trailer, and the second account's commit, linked by its trailer and as committer. The second account was counted once. So today's `Co-authored-by` trailer probably already credits issue authors in that graph, and the guide's Credit section undersells it either way.
- **The sidebar list (`/contributors`) wasn't settled.** It kept serving the response cached at the first call, "the unlinked identity, 2 commits", for over 10 minutes, even with a different query string. That cache can take hours to refresh. The sidebar list probably counts authors only, which is where this idea would still matter. The branch name `trunk` isn't the cause: both lists read the default branch.

## When this is picked up again

1. Re-check the sidebar list: `gh api 'repos/clunny/clm-author-spike/contributors?anon=1' --jq '.[] | "\(.login // .name) \(.type) \(.contributions)"'`, and the repository's page while signed in. If the maintainer is listed but the second account isn't, or the reverse, that answers whether co-authors count there.
2. Decide from that whether the idea is worth a change, or whether rewording the guide's Credit section is enough.
3. **Clean up:** delete `clunny/clm-author-spike`, which holds the second account's noreply address in its history. Run `gh auth refresh -h github.com -s delete_repo` then `gh repo delete clunny/clm-author-spike --yes`, or delete it in the web UI. The local clone was in a session's scratchpad and is gone already.
