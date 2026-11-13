---
title: "Reading the loop file creates its directory"
kind: bug
components: [orchestrator.loop]
---

`clm_loop::read` and `stored_plan` find the loop file through `dir`, which runs `create_dir_all` on `<git common dir>/clm/`, so `clm view queue` on a clone that has never run the loop leaves an empty `clm/` directory in the git directory, though `view-queue` says it writes nothing and `plan-snapshot` says `stored_plan` reads without writing. The read paths should locate the file without creating its directory; only `update` needs it made.
