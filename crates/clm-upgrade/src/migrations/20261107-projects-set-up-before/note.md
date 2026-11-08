Unattended sessions need clm's allow rules in .claude/settings.json
`clm init` now seeds `.claude/settings.json` with allow rules for the `clm` commands unattended sessions run, so a headless build isn't stopped to ask about each one. Your project lacks some of them. These are the rules:

- `Bash(clm apply)` and `Bash(clm apply *)`
- `Bash(clm branch)` and `Bash(clm branch *)`
- `Bash(clm check)` and `Bash(clm check *)`
- `Bash(clm claim)` and `Bash(clm claim *)`
- `Bash(clm defect)` and `Bash(clm defect *)`
- `Bash(clm gate)` and `Bash(clm gate *)`
- `Bash(clm land)` and `Bash(clm land *)`
- `Bash(clm pr)` and `Bash(clm pr *)`
- `Bash(clm rebase)` and `Bash(clm rebase *)`
- `Bash(clm verify)` and `Bash(clm verify *)`
- `Bash(clm workaround)` and `Bash(clm workaround *)`

`clm init --check-permissions` prints the ones your project lacks. Add them to `permissions.allow` in `.claude/settings.json` by hand: `clm upgrade --apply` never writes that file, since the settings are yours and a rule you removed on purpose isn't put back. A rule in `.claude/settings.local.json` counts too.

If your root component's `paths` don't list `.claude/settings.json`, as in a project set up before `clm init` seeded the rules, a new settings file needs adding there through a spec change before `clm check` passes.

To keep a project that leaves rules out on purpose as it is, run `clm upgrade --decline 20261107-projects-set-up-before`.
