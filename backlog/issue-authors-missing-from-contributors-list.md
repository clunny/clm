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

## Spike resumed (2026-11-11)

The API cache had refreshed. `gh api 'repos/clunny/clm-author-spike/contributors?anon=1'` returned:

| Contributor | Type | Contributions |
|---|---|---|
| The unlinked identity | Anonymous | 3 (initial, control and merge commits) |
| The maintainer's account | User | 2 |
| The second account | User | 1 |

`stats/contributors` still gave the maintainer 4 and the second account 1.

Findings:
- **The sidebar list's API counts authors only.** The maintainer's 2 are exactly the 2 commits it authored. Neither the control commit, where it is only a `Co-authored-by` trailer, nor the second account's commit, where it is co-author and committer, counts. So today's trailer does **not** put an issue author in this list.
- **Authorship does.** The second account is listed with its one commit and did nothing to get there.
- **The repository's page still didn't show the second account**, for either account signed in. The page's sidebar may hide an account that can't see a private repository, or keep its own cache. To test the first, the second account was invited as a collaborator and accepted: a personal account's repository can only invite with write access, since read-only access is for organisations. Being a collaborator doesn't list anyone by itself, because the list comes from commits on the default branch. Straight after accepting, neither account's view of the page listed it, but the caches had had little time to clear.

## When this is picked up again

1. Reload the repository's page while signed in. If the second account now shows, a private repository's page was hiding it, and a public repository such as CLM's would list an issue author made a commit's author. If it still doesn't after a day, making the spike repository public for a few minutes is the last check: it shows the second account's noreply address, so it needs the maintainer's yes.
2. The API result already favours the idea over rewording the guide's Credit section alone, though that section should also say that the Insights graph counts co-authors. Unless the page contradicts it, take the idea to `/clm:propose`.
3. **Clean up:** delete `clunny/clm-author-spike`, which holds the second account's noreply address in its history and now the second account as a collaborator. Run `gh auth refresh -h github.com -s delete_repo` then `gh repo delete clunny/clm-author-spike --yes`, or delete it in the web UI. The local clone was in a session's scratchpad and is gone already.
