---
component: config
---

## MODIFIED Requirements

### Requirement: verify-setting <!-- base: 93c5194ae1c7ea4d -->
Loading SHALL take `verify` only from the repository's `clm.toml` or the clone's `clm.local.toml`, as a table with `check`, a non-empty string; `mode`, one of `local`, `pipeline` and `import`; optional `triggers`, an array of distinct values among `change-done`, `queue-drained` and `work-done`, default empty; optional `every`, a table whose keys `lands` and `days` are each optional and a positive integer; `workflow`, a non-empty string; optional `mutate`, a non-empty string, the command of the mutation pass (`verify/mutation-pass`); optional `mutate_jobs`, a positive integer, the most requirements the pass judges at once, default 1; and optional `mutate_minutes`, a positive integer, the longest a pass runs, unset meaning no limit. It SHALL fail naming the file that sets it and `verify` for a `verify` that isn't a table, for any other key in it or in `every`, for a missing or empty `check`, a missing or unknown `mode`, an unknown or repeated trigger, `work-done` with a `mode` other than `local`, an `every` value that isn't a positive integer, a `workflow` missing with `mode = "import"` or set with another mode, an empty `mutate` or one set with `mode = "import"`, where no check runs here, and a `mutate_jobs` or `mutate_minutes` that isn't a positive integer or is set without `mutate`; and SHALL fail naming the user's `config.toml` and the key when that file sets it. `clm config get verify` SHALL print `mode=<mode>,check=<command>`, followed by `,triggers=<t>+<t>...` when there are triggers, `,every.lands=<n>` and `,every.days=<n>` when set `,workflow=<name>` when set, `,mutate=<command>` when set, and `,mutate_jobs=<n>` and `,mutate_minutes=<n>` when set, and the path of the file that sets it, or nothing and `default`.

#### Scenario: set
- GIVEN a repository `clm.toml` with `[verify]`, `check = "scripts/slow.sh"`, `mode = "local"` and `triggers = ["change-done", "work-done"]`
- WHEN `clm config get verify` runs
- THEN it prints `mode=local,check=scripts/slow.sh,triggers=change-done+work-done` and the file's path

#### Scenario: import needs a workflow
- GIVEN a repository `clm.toml` with `[verify]`, `check = "scripts/slow.sh"` and `mode = "import"`
- WHEN the settings are loaded
- THEN it fails naming `clm.toml` and `verify`

#### Scenario: work done is local only
- GIVEN a repository `clm.toml` with `[verify]`, `check = "x"`, `mode = "pipeline"` and `triggers = ["work-done"]`
- WHEN the settings are loaded
- THEN it fails naming `clm.toml` and `verify`

#### Scenario: zero lands
- GIVEN a repository `clm.toml` with `[verify]`, `check = "x"`, `mode = "local"` and `every = { lands = 0 }`
- WHEN the settings are loaded
- THEN it fails naming `clm.toml` and `verify`

#### Scenario: unset
- GIVEN a repository `clm.toml` with no `[verify]`
- WHEN `clm config get verify` runs
- THEN it prints nothing for the value and `default`

#### Scenario: from the user's file
- GIVEN a user `config.toml` with a `[verify]` table
- WHEN the settings are loaded
- THEN it fails naming `config.toml` and `verify`

#### Scenario: a mutation command
- GIVEN a repository `clm.toml` with `[verify]`, `check = "x"`, `mode = "local"` and `mutate = "scripts/mutate.sh"`
- WHEN `clm config get verify` runs
- THEN it prints `mode=local,check=x,mutate=scripts/mutate.sh` and the file's path

#### Scenario: mutation with import
- GIVEN a repository `clm.toml` with `[verify]`, `check = "x"`, `mode = "import"`, `workflow = "slow"` and `mutate = "m"`
- WHEN the settings are loaded
- THEN it fails naming `clm.toml` and `verify`

#### Scenario: a bounded pass that runs requirements at once
- GIVEN a repository `clm.toml` with `[verify]`, `check = "x"`, `mode = "local"`, `mutate = "m"`, `mutate_jobs = 6` and `mutate_minutes = 60`
- WHEN `clm config get verify` runs
- THEN it prints `mode=local,check=x,mutate=m,mutate_jobs=6,mutate_minutes=60` and the file's path

#### Scenario: jobs without a pass
- GIVEN a repository `clm.toml` with `[verify]`, `check = "x"`, `mode = "local"` and `mutate_jobs = 2`
- WHEN the settings load
- THEN loading fails naming the file and `verify`

#### Scenario: no minutes
- GIVEN the same with `mutate = "m"` and `mutate_minutes = 0`
- WHEN the settings load
- THEN loading fails naming the file and `verify`

#### Serves
- architecture/proven-by-tests
- capability/slow-tests-after-land

### Requirement: verify-here <!-- base: 826e104c37352cc5 -->
This repository's `clm.toml` SHALL set `[verify]` with `check = ".github/scripts/ci-check.sh"`, `mode = "local"`, `triggers = ["queue-drained"]`, `every = { lands = 20 }` `mutate = ".github/scripts/mutate.sh"`, `mutate_jobs = 6` and `mutate_minutes = 60`, and no other key of the table, so the whole suite runs after lands that checked only where the branch and `main` overlap (`land/overlap-recheck`), on this machine, once the build queue drains and at least every 20 lands, and the mutation pass follows each run that passed (`verify/mutation-pass`, `ci/mutate-script`), judging up to six requirements at once as free memory allows and stopping after an hour, with what it didn't reach carried to the next pass.

#### Scenario: this repository
- GIVEN this repository's `clm.toml`
- WHEN `clm config get verify` runs against it
- THEN it prints `mode=local,check=.github/scripts/ci-check.sh,triggers=queue-drained,every.lands=20,mutate=.github/scripts/mutate.sh,mutate_jobs=6,mutate_minutes=60` and the file's path

#### Serves
- capability/land-safely
- capability/prove-by-tests

#### Needs
- ci/mutate-script
