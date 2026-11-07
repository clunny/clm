---
status: applied
---

# The site names only CLM's own repository

## Intent

Reword `web.site`'s Design so it says what `invite-only-web` asks of the site: it holds nothing of the repositories the web app reads. Its Deploy paragraph said "Nothing the site serves names a repository", which the homepage's install commands and GitHub links already contradicted, and which blog posts now contradict too (`20261106-posts-may-name-install-repository`). The Design now says the only repository the site names is CLM's own public one, and its install-repository paragraph says posts may name it, as the requirement does.

## Problem

The sentence read the constraint's "holds nothing of any repository" as "names no repository". The constraint is about repository data behind sign-in: specs, backlog, changes and code shown in the web app. Naming CLM's own public repository in install commands and links is what `public-site` asks for.

## Approaches

1. **Reword the Design sentence (chosen).** A Design-only change: no requirement changes, nothing to build.
2. **Reword `invite-only-web` too.** Its "holds nothing of any repository" is open to the same misreading, but an `ARCHITECTURE.md` edit needs review, and the constraint's first sentence ("nothing it shows of a repository is public") already frames it as the web app's data.

## Outcome

`web.site`'s Design: the Deploy paragraph says the site holds nothing of the repositories the web app reads and names only CLM's own public repository; the install-repository paragraph says the blog's posts may name it in their own install commands.
