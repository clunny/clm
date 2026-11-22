---
component: ledger
---

## MODIFIED Requirements

### Requirement: writer-identity <!-- base: deccdd4012e3e0fe -->
`writer(root, env)` SHALL return `<person>/<clone>`, where `<person>` is `config`'s `writer` or, when unset, the repository's git `user.name` as a slug, and `<clone>` is the eight hex digits in `.git/clm/writer`, written on first use and read on every later one, so every worktree of the clone shares it. It SHALL read `writer` from the layers `config` takes it from, `CLM_WRITER`, the clone's `clm.local.toml` and the user's `config.toml`, as `config/committed-settings` loads them with no repository layer, and never read the repository's `clm.toml`, which never sets `writer` (`config/writer-setting`): a `clm.toml` that can't be loaded SHALL not fail it, so a caller that never reads the working tree's file, as `verify/settings-from-main` doesn't, still appends. A `.git/clm/writer` that holds nothing but white space SHALL be taken over, fresh digits written to it. It SHALL fail, naming the `writer` setting, when the person part would be empty, and it SHALL refuse a `writer` setting that `slug` would change, one `config` accepts such as `ann--lee`, as ``` `writer` is `<setting>`, not a slug: use lowercase letters, digits and `-`, such as `<its slug>` ```.

#### Scenario: first use
- GIVEN a clone whose git `user.name` is `Christopher Lunny`, no `writer` setting and no `.git/clm/writer`
- WHEN `writer(root, env)` runs
- THEN it returns `christopher-lunny/<8 hex digits>` and `.git/clm/writer` holds those digits

#### Scenario: shared by worktrees
- GIVEN that clone and a linked worktree of it
- WHEN `writer` runs in each
- THEN both return the same identity

#### Scenario: setting wins
- GIVEN `CLM_WRITER=clunny`
- WHEN `writer(root, env)` runs
- THEN its person part is `clunny`

#### Scenario: a clm.toml that can't be loaded
- GIVEN `CLM_WRITER=clunny`, and a repository `clm.toml` that sets `[verify]` with no `check`
- WHEN `writer(root, env)` runs, and then `append` with an `approved` record
- THEN `writer` returns `clunny/<8 hex digits>`, and the record is on `clm/ledger/clunny/<those digits>`

#### Scenario: odd identities
- GIVEN, in turn, `CLM_WRITER=ann--lee`, `CLM_WRITER=Christopher/Lunny`, and a clone whose git `user.name` is `T` and whose `.git/clm/writer` is empty
- WHEN `writer(root, env)` runs on each
- THEN the first fails with ``` `writer` is `ann--lee`, not a slug: use lowercase letters, digits and `-`, such as `ann-lee` ```, the second fails naming `` `writer` ``, and the third returns `t/<8 hex digits>` and `.git/clm/writer` then holds those digits
