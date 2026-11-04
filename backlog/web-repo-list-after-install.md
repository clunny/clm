---
title: "Web: new repos show at once after Add repos, say why a repo is hidden, and sign out"
kind: idea
components: [web, web.worker]
---

Three gaps the human hit after adding a repo through the web app's "Add repos" link:
- **New repos take up to 5 minutes to appear.** The repo list is memoised for 5 minutes per token hash unless empty (`web.worker`'s repo list requirement), so a user who already has repos and installs the App on another waits out the memo. Coming back from GitHub's install page (its setup URL redirect) could skip the memo, or the client could offer a refresh that does.
- **The picker doesn't say why a repo is hidden.** Repos without `specs/SPEC.md` on their default branch are left out silently, so a granted repo that hasn't run `clm init` looks like a failed install. The picker or the Add repos link could say that only repos with `specs/SPEC.md` are listed.
- **There is no sign-out.** Nothing clears the session cookie, so a user can't sign out, or sign in again to start a fresh session.

Raised by the human on 2026-11-03.
