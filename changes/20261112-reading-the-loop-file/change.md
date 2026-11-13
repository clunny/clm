---
status: proposed
resolves: [reading-the-loop-file-creates-its-directory]
---

## Intent

`stored_plan` reads the loop file without creating anything: on a clone that has never run the loop, `clm view queue` leaves no empty `<git common dir>/clm/` behind. Only the commands that write the loop file make its directory.
