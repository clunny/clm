---
component: config
---

## MODIFIED Requirements

### Requirement: verify-setting <!-- base: 5bf87fcc5903e829 -->
Loading SHALL take `verify` only from the repository's `clm.toml` or the clone's `clm.local.toml`, as a table with `check`, a non-empty string; `mode`, one of `local`, `pipeline` and `import`; optional `triggers`, an array of distinct values among `change-done`, `queue-drained` and `work-done`, default empty; optional `every`, a table whose keys `lands` and `days` are each optional and a positive integer; `workflow`, a non-empty string; and optional `mutate`, a non-empty string, the command of the mutation pass (`verify/mutation-pass`). It SHALL fail naming the file that sets it and `verify` for a `verify` that isn't a table, for any other key in it or in `every`, for a missing or empty `check`, a missing or unknown `mode`, an unknown or repeated trigger, `work-done` with a `mode` other than `local`, an `every` value that isn't a positive integer, a `workflow` missing with `mode = "import"` or set with another mode, an empty `mutate` or one set with `mode = "import"`, where no check runs here; and SHALL fail naming the user's `config.toml` and the key when that file sets it. `clm config get verify` SHALL print `mode=<mode>,check=<command>`, followed by `,triggers=<t>+<t>...` when there are triggers, `,every.lands=<n>` and `,every.days=<n>` when set `,workflow=<name>` when set and `,mutate=<command>` when set, and the path of the file that sets it, or nothing and `default`.

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

#### Serves
- architecture/proven-by-tests
- capability/slow-tests-after-land
