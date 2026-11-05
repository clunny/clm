---
status: applied
resolves: []
---

# Trim the search-console task to the indexing check

## Intent

`backlog/search-console-and-bing` asks the human to register `clm-app.dev` with Google Search Console and Bing Webmaster Tools, submit the sitemap to both, and a week after the release check that both report the homepage and the guide's pages as indexed. The human did the Google and Bing steps in the triage session; `https://clm-app.dev/sitemap.xml` returns 200 and `clm-app.dev` carries the `google-site-verification` TXT record. This change edits the item down to its last step, the indexing check a week after the release, which then deletes the item. It changes only the backlog.

`resolves` names no item: the item stays open with the step left, and a change that resolves it would delete it when it is archived (`task-deletes-fixed-items`), losing that step. The draft prompt's lack of a route for this case is already reported as `backlog/draft-prompt-has-no-route-for-an-answer-that-edits-a-human-task`.

## The human's answers

> "Walk me through it now (Recommended)" — Claude shows the Search Console and Bing steps; once done, Claude drafts a change that trims the item down to the one-week indexing check. Then: "I did the google and bing steps".
>
> (Context Claude checked: https://clm-app.dev/sitemap.xml returns 200, and clm-app.dev now has the TXT record `google-site-verification=…`. Steps 1 and 2 are done; keep step 3, the check about a week after release that Google and Bing report the homepage and the guide's pages as indexed, then delete the item. Do not name this item in `resolves`, since resolving deletes it; edit its body and leave it open.)
