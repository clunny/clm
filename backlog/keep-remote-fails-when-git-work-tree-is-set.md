---
title: "keep_remote fails when GIT_WORK_TREE is set"
kind: bug
components: [host]
---

keep_remote runs its bare repository setup through clm-git, which inherits GIT_WORK_TREE from the environment. With GIT_DIR and GIT_WORK_TREE set (a work tree whose git directory is elsewhere) that command is refused, so clm init fails on host none. It should run without those variables. Found building init/init-keeps-a-remote; the root's init test search_stops_at_the_top_without_a_git_entry now passes --host git to avoid it.
