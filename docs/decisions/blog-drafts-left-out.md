---
title: "Blog posts carry a draft flag, and the build leaves drafts out entirely"
date: 2026-11-05
status: accepted
---

Posts live in `web/site/blog/`, one Markdown file each, released one at a time (`publish-posts`). Each has `draft: true` or `false` in its frontmatter; a missing flag counts as a draft, so a post is never published by forgetting a key. The site's build reads every post and drops the drafts before rendering anything, so pages, cards, the series list, previous and next links, the feed, the sitemap, structured data, Markdown copies and `llms.txt` are all made from published posts alone (`web.site/blog-drafts-left-out`). A test searches the whole built site for a draft's slug, title and a phrase from its body.

**Why left out, not hidden.** Building a draft and keeping it off the index (a `noindex` tag, an unlisted URL) still deploys it, where anyone who guesses the URL, any crawler that ignores the tag, and any link from a published post would find it. A draft that isn't built can't leak.

**Publishing is a land plus a deploy.** Setting `draft: false` and a `date` is an ordinary edit that lands on checks; the post goes public when the human dispatches the site workflow (`site-worker-of-its-own`). A published post may name the next part before it is out: a link to a draft's file renders as text, so the series' teasers don't have to be rewritten as each part appears.

**Reading time is computed** from the body at build time (words ÷ 230, rounded up), so it can't fall behind an edit.

Alternatives considered:
- *A publish date instead of a flag, hiding future posts.* A static site only changes when it is built, so the date would publish nothing until the next deploy; the flag says the same thing without implying a schedule.
- *Drafts in a separate directory.* Publishing would move a file, losing its history in diffs, for no gain over a flag.
