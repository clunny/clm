---
title: "People without write access contribute intent only, read by hand by the maintainer"
date: 2026-11-03
status: accepted
---

A project under CLM can take issues, backlog suggestions and spec changes from people without write access, in pull requests from forks. A pull request changing any file outside the spec paths is closed. The maintainer reads every one, including those the gate would let land on checks, because `yes-no-review` judges a project's own sessions, and text from a stranger instructs the project's future sessions: a malicious edit to a spec or `CLAUDE.md` is prompt injection, which only a person can catch.

Why not code: nobody reviews code (`ai-writes-code`), and `clm land` runs the merged tree's check command on the lander's machine (`land-locally`), so landing a stranger's code runs it. A spec-paths branch can't change anything a land runs.

This is method, so it's in the guide's `contributions` page and the plugin's skills (`method-in-the-plugin`), and each project chooses whether to take contributions. `/clm:contribution` is the maintainer's side and `/clm:propose` the contributor's, stopping at the pull request when the work goes back to a parent. Asking whether it does keeps a hard fork, whose owner lands on its own `main`, working as any other project.

An idea is agreed in an issue first, since a spec costs its author model use. The maintainer's session rebases, so contributors needn't chase `main`. Credit is the commit's author: `clm land` keeps the commits, and a rebase keeps their author.

Left for later: collaborators with write access, which on this repository can run `clm land` with nothing on the host limiting them. Before the first, turn on `docs/allowed_signers` and `approval = "other"`.

Rejected:
- **Fork pull requests with code, landed after reading them.** It asks the maintainer to review code, which `ai-writes-code` says no human does, and they'd be running it.
- **A `clm pr accept <number>` command.** A product feature before any pull request has arrived (`yagni`). `spec-branches-hold-spec-paths` holds at the last step; revisit if volume justifies it.
- **An Actions workflow closing code pull requests.** It needs `pull_request_target`, a known risk, is a second hosted decision-maker beside `land-locally`, and copies the gate's path list.
- **A rule in `CLAUDE.md` alone.** Other projects would never read it.
