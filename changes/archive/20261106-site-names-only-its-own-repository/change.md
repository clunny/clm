---
status: done
---

# The public site shows no repository data

## Intent

Reword `invite-only-web` and `web.site`'s Design so they say what the constraint asks of the public site: it shows none of the repository data the web app keeps behind sign-in. The constraint said the site "holds nothing of any repository", which the site, built from CLM's own documentation, never met as written. `web.site`'s Deploy paragraph said "Nothing the site serves names a repository", which the homepage's install commands and GitHub links already contradicted, and which blog posts now contradict too (`20261106-posts-may-name-install-repository`). The Design now says the site names repositories only in links and commands, and its install-repository paragraph says posts may name CLM's, as the requirement does.

## Problem

The sentence read the constraint's "holds nothing of any repository" as "names no repository". The constraint is about repository data behind sign-in: specs, backlog, changes and code shown in the web app. Naming CLM's own public repository in install commands and links is what `public-site` asks for.

## Approaches

1. **Reword the Design sentence alone.** Lands on checks, but leaves the constraint's "holds nothing of any repository" in every context bundle, where the next reader can misread it the same way.
2. **Reword `invite-only-web` too (chosen by the human).** Fixes the wording at its source; the `ARCHITECTURE.md` edit needs review. Its scenarios and `#### Enforced by` rules are unchanged: they already test sign-in and the API, not what the site names.

## Outcome

`invite-only-web`: the public site shows none of the repository data the web app keeps behind sign-in; CLM's documentation is public and the site may name and link repositories. `web.site`'s Design: the Deploy paragraph says the same and that the site names repositories only in links and commands; the install-repository paragraph says the blog's posts may name it in their own install commands.
