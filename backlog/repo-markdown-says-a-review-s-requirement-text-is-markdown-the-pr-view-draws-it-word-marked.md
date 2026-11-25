---
title: "`repo-markdown` says a review's requirement text is rendered as Markdown; the PR view draws it as word-marked plain text"
kind: question
components: [web.views]
choices:
  - "Narrow repo-markdown to whole texts"
  - "Render the cards as Markdown"
---

`web.views/repo-markdown` (`342a93bb`) says every text the app shows from the repository that the Worker gives as Markdown, "a requirement's text" among them, SHALL be rendered as Markdown and sanitised "in the lists and on the pages alike". The PR view's Document tab (`web/src/views/pr.ts`, `card`) draws each changed requirement, Contract, Design and frontmatter as plain text with word marks between before and after (`pr-document`), never through `markdown.ts`. That is safe (no HTML is parsed), but it is not what `repo-markdown` states. The spec auditor of the `impl/web.views` build for wave 6 found it; the build left the code as it is.

Either a spec change narrows `repo-markdown` to the texts drawn whole (the intent, a contract in the inspector, a backlog item's body, a capability's or a constraint's statement and entry), leaving `pr-document`'s word-marked cards as plain text; or the cards render Markdown while keeping the word marks, which needs a diff over the rendered tree rather than over words, and a requirement saying how a mark inside Markdown syntax reads.
