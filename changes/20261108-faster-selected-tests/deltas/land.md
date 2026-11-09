---
component: land
---

## ADDED Requirements
### Requirement: fetch-retry-in-layer
`clm land` and `clm reconnect` SHALL make the fetches of `origin` that `offline-land` tries again through `clm-git`'s `fetch_retrying` (`vcs/fetch-retries`), mapping `FetchError::Offline` to the offline refusal and `FetchError::Git` to git's message, so `clm land`, `clm reconnect` and `clm approve` retry one way. No file under `crates/clm-land/src` SHALL define a function named `fetch_retrying`.

#### Scenario: one retry
- GIVEN the workspace
- WHEN a test in `clm-land` reads every file under `crates/clm-land/src`
- THEN one of them calls `clm_git::fetch_retrying`, and none defines `fn fetch_retrying`

#### Needs
- vcs/fetch-retries

#### Serves
- architecture/reuse-first
