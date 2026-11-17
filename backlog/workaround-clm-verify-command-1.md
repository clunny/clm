---
title: "Stop window approximated: clm-verify does not expose when its mutation pass runs"
kind: workaround
components: [clm]
requirement: clm/verify-command@57b06e54
---

clm/verify-command wants SIGINT/SIGTERM caught only while the mutation pass runs. The verify crate's verify_with exposes only announce (every outcome, before the pass) and keeps Outcome::mutate crate-private, so the root installs its handlers in announce when the outcome line starts "verify passed " and keeps them until the call returns, covering the filing after the pass and a passed run with no pass. Settling it needs the verify Contract to offer the pass's bounds: backlog/clm-verify-can-t-tell-its-caller-when-the-mutation-pass-runs.
