---
title: "Some commands decide a file's component from paths alone, so a test file a tests glob owns is attributed to the component whose paths hold it"
kind: idea
components: [effort, gate]
---

## Intent

`check.coverage/owner` gives a file's owner, `tests` before `paths` (`components-own-their-tests`), and `pending`, `affected`, size and `check` read it. A few readers still match `paths` themselves: effort's history (`crates/clm-effort/src/history.rs`) and the gate's code-fix attribution (`crates/clm-gate/src/lib.rs`, `paths_match`). For a test file a `tests` glob owns inside another component's `paths`, each attributes it to that other component. The gate still attributes it correctly through its markers, since such a file cites only its owner. Effort's history then misattributes effort. The verify stop's path map (`crates/clm-land/src/verify_stop.rs`) went through `owner` in `20261121-wave-3-land-by-the-rule` (`land/verify-stop`), and `clm rebase`'s list of components whose spec changed takes its component from the gate in `20261121-wave-3-sync-by-the-rule` (`sync/build-branches`).

## Open questions

- Should each go through `owner`, and does any of them need `paths` alone on purpose?
