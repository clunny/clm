---
status: done
---

# Blog posts can say when they were updated

## Intent

Let a blog post carry an optional `updated` date, shown on its page beside its published date and given to search engines as `article:modified_time` and the `BlogPosting`'s `dateModified`, so a reader can tell when a published post was corrected. The first use is part 1, "Stop reviewing AI's code. Review its intent.", reworded on 2026-11-09 to say humans own the intent rather than the spec: its build sets `updated: 2026-11-09` on `web/site/blog/humans-own-intent.md`.

## Problem

A post's only date is `date`, the day it was published. Part 1 was corrected after publication (its tagline said humans own the spec, which `humans-review-intent` and `human-layers` no longer hold), and nothing on the page tells a returning reader or a crawler that it changed.

## Approaches

1. **One added requirement, `blog-updated-date` (chosen).** It states the key, its validation, the page text and both head entries in one place. The existing requirements (`blog-drafts-left-out`, `blog-post-page`, `page-metadata`, `structured-data`) stay as written and current, since each still holds for a post with no `updated`.
2. **Modify each of those four requirements.** Same behaviour, but four requirements go `behind` and their tests must be re-cited for a small optional field.
3. **Also put it in the feed, the cards and the sitemap.** RSS 2.0 has no modified date for an item; the cards order by `date` and stay compact; a sitemap `lastmod` would need one for every page, guide included. Left out.

## Outcome

`updated` is optional, `YYYY-MM-DD`, never before `date`. The page shows "Updated 9 Nov 2026" after the published date; the head and JSON-LD carry it; the feed, `/blog/` cards, ordering, sitemap and Markdown copies are unchanged. Setting it is the author's call: a corrected claim warrants one, a typo fix needn't.
