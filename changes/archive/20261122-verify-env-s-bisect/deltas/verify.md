---
component: verify
---

## MODIFIED Requirements

### Requirement: verify-env <!-- base: 877031f2b36e59a7 -->
Every run of `[verify] check`, a bisect probe's included, SHALL set in the command's environment each variable of the `env` that `config/env-setting` gives from the CLM root's `clm.toml` at the commit it checks out (`origin/main`'s head, the commit `--commit` names, or the probe's land) and the clone's `clm.local.toml`, never the working tree's `clm.toml`, each replacing any value the caller's environment holds for that name. When there is a last green commit and the `env` of the commit it verifies differs from the one `config/env-setting` gives from the green commit's `clm.toml` and the same `clm.local.toml`, every run of the check for that verification, its probes included, SHALL have `CLM_SELECTION` unset, since a variable can change any test's result. `CLM_LAND_DIR` (`verify-build-dir`) SHALL be as it is without it. An `env` can't be used when `config/env-setting` refuses the table; a git command that fails while reading it ends the run as `settings-from-main` says. When the `env` of the commit whose check it runs as the verification, not as a probe, can't be used, it SHALL refuse with exit 1 and `config`'s message, which names `env` and that commit's `clm.toml` as `<short sha>:clm.toml`, with the CLM root's place in its git repository before `clm.toml` as `settings-from-main` names its file, running no check and appending nothing; for `origin/main`'s head `settings-from-main`'s load refuses first, naming its own file, so this refusal is reached for a `--commit` of an older commit. A bisect probe on a land whose `env` can't be used SHALL run no check there and count as untestable (`verify-bisect`), and where two `env`s are compared, a last green commit's that can't be used, or, under `clm verify --bisect`, the red commit's, SHALL count as differing.

#### Scenario: main's table and the clone's
- GIVEN a `[verify] check` that writes `$A` and `$B` to a file outside the repository, a checkout whose `clm.toml` sets `[env]` with `A = "checkout"`, an `origin/main` whose `clm.toml` sets `[verify]` with that check and `[env]` with `A = "main"` and `B = "main"`, a `clm.local.toml` with `[env]` setting `B = "local"`, `A=caller` in the caller's environment, and a run due
- WHEN `clm verify --due` runs in the checkout
- THEN the file holds `main` for `A` and `local` for `B`

#### Scenario: a probe takes its land's variables
- GIVEN a passing record, then a land setting `[env]` `A = "1"`, then a land setting `A = "2"`, and a `[verify] check` that fails when `$A` is `2`
- WHEN `clm verify --due` runs and bisects
- THEN the probe of the first land ran with `A` set to `1` and passed, and the record names the second land as the culprit

#### Scenario: a changed variable runs every test
- GIVEN a passing record, then one land that changes only `clm.toml`'s `[env]`, and a `[verify] check` that writes whether `CLM_SELECTION` is set
- WHEN `clm verify --due` runs
- THEN the check ran with `CLM_SELECTION` unset

#### Scenario: a table that can't be used
- GIVEN `mode = "local"`, a passing record for `g`, a land `bad` whose `clm.toml` adds `[env]` with `CLM_X = "1"`, which `config/env-setting` refuses, a land `good` after it that restores `g`'s `clm.toml` and changes a file of component `a`, and a `[verify] check` that records whether `CLM_SELECTION` is set and fails on any tree holding `good`'s change
- WHEN `clm verify --commit <bad>` runs; then `clm verify --due` with a trigger holding; then, with a ledger whose only `verified` record is a passing one for `bad`, `clm verify --commit <good>`; and, in a fixture of its own holding a passing record for `g`, a land `l1` changing a file of `a`, then `bad`, then a land that restores `g`'s `clm.toml`, so that `bad` is not `origin/main`'s head, a red record for `bad`, and a check that fails on any tree holding `l1`'s change, `clm verify --bisect <bad>`
- THEN the first exits 1 naming `<short bad>:clm.toml` and `env`, runs no check and appends nothing; the second prints `verify failed <good> culprit unknown components a`, stderr holds `culprit unknown: [verify] check could not run on 1 lands, <bad> to <bad>, before the first red one, <good>`, and its red record is appended; the third ran the check with `CLM_SELECTION` unset and appended a red record naming `good` as the culprit; and the fourth ran its probe of `l1` with `CLM_SELECTION` unset and appended a red record for `bad` naming `l1` and `[a]`

#### Needs
- config/env-setting
