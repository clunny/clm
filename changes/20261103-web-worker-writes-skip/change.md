---
status: proposed
resolves:
  - web-worker-writes-skip-the-replica-object
---

# The Worker's writes go through the Replica object

## Intent

`web.worker`'s Design and Contract say a proposal's, a decision's and a withdrawal's append goes through the repo's `Replica` object, and nothing in `crates/clm-web` does it. Design in progress.
