---
title: "A task is claimed by pushing its branch"
date: 2026-10-21
status: accepted
---

Claiming task `<task>` of change `<id>` means pushing a new branch `impl/<id>/<task>`. Git rejects creating a branch that already exists, so the push is an atomic lock with no extra files or state. The latest commit on the branch is the heartbeat. A claim with no commit for longer than the expiry (default 4 hours, configurable) has lapsed, and another session may take the task over.

This resolves the question of how claims expire when a session crashes.
