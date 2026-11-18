---
status: proposed
resolves: [a-deleted-runs-task-ids-are-forgotten]
---

## Intent

The build loop keeps the task IDs of every run of a name it has finished with, however the run left its list. They are kept by run name on the loop file, not on the run, so a run that `clm build start`, `clm build release` or a step drops still leaves its task IDs behind: a later run of the same name then refuses the old task at `clm build launched`, and a late `clm build returned <name> --task <old-task>` changes nothing, also when no run of the name is listed. The fix is outside the plan's freeze, so a question item holds its build back until the human says whether the exception made for the two rules covers it.
