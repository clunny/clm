---
status: done
---

## Intent

`gate`'s Design, as wave 6 (`20261124-wave-6-worker-by-the-rule`) wrote it, said the gate's own reading gives the model's files "read as `clm_model::read_files` reads a directory". That function is private to `clm-model`, so the gate's build can't call it and its build prompt had to tell it so; the public call that reads them the same way is `clm_model::load`. The Design now names it. No requirement, scenario or behaviour changes, and nothing is cut.
