---
component: orchestrator
---

## MODIFIED Requirements

### Requirement: next-wait <!-- base: 1946593e40a09e00 -->
`clm next` SHALL refuse `--wait` and `--interval` as usage errors, and SHALL hold no code that waits for work: `clm build wait` waits for everything the build loop launches (`orchestrator.loop/build-wait`), and the skill that ran `clm next --wait` runs the loop instead. This is the contract step of moving the wait into the loop (`expand-contract`), so it waits until the skill no longer names `clm next --wait`; while it waits, this pending requirement stands in for a `clm:remove-by 20261112-build-loop-in-clm` marker on the `--wait` code, which `clm pending` keeps in sight until it is built.

#### Scenario: wait refused
- WHEN `clm next --wait` runs, and then `clm next --interval 5`
- THEN each exits 2 with usage

#### Serves
- capability/build-from-spec

#### Needs
- orchestrator.loop/build-wait
