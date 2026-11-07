---
status: applied
---

# Blog posts may name the install repository

## Intent

Let a blog post under `web/site/blog/` name the install repository, `clunny/clm`, in its own text, so a post can end with the commands that install CLM, as the guide's quickstart and FAQ already do. Every other file under `web/site/` still leaves the repository to the one value in the site's config, so the homepage's install commands keep following it.

## Problem

`web.site/install-repository` says that no file under `web/site/` but the site's config names the repository. The rule was written before the blog existed, to keep the homepage's commands on one value that tests can override with `INSTALL_REPO`. Posts now live under `web/site/blog/`, and the first one, `humans-own-intent`, ends with `/plugin marketplace add clunny/clm` beside a link to the quickstart; its land fails the requirement's test. A post is prose kept in the repository like a guide page, and the guide's files, which sit outside `web/site/`, already name the repository in their commands.

## Approaches

1. **Leave posts out of the rule (chosen).** Posts are written text, as the guide is; a build with another `INSTALL_REPO` shows a post's own commands, as it shows the guide's. The homepage, the theme and the config stay under the rule.
2. **A placeholder in posts that the build fills from the config.** Every post would follow `INSTALL_REPO`, but a post's file would no longer read as what it publishes, its Markdown copy would need the same substitution, and the guide's commands don't follow the value either.
3. **No commands in posts, only a link to the quickstart.** Keeps the rule as written, but the human wants the commands in the post.

## Outcome

`install-repository` exempts the files under `web/site/blog/`; its test skips that directory when it looks for other files naming the repository. Nothing else changes.
