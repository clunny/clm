---
status: applied
---

## Intent

`gate`'s Design said a git failure while reading a spec branch's files fails the gate with "exit 2", which wave 6 (`20261124-wave-6-worker-by-the-rule`) wrote. A Design states no exit (`requirements-state-what-the-contract-shows`), and `clm health` counted it under `design-exit`. The exit is `gate/evaluate-at`'s, which already states it; the Design now names that requirement instead. No requirement, scenario or behaviour changes, and nothing is cut.
