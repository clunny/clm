Unattended sessions need clm's allow rules for clm build in .claude/settings.json
`clm init` now seeds `.claude/settings.json` with allow rules for `clm build`, so the build loop isn't stopped to ask about it. Your project lacks one or both. These are the rules:

- `Bash(clm build)`
- `Bash(clm build *)`

`/clm:build` runs every step of its loop through `clm build`, and Claude Code's auto mode denies a step when no rule allows it.

`clm init --check-permissions` prints the rules your project lacks. Add them to `permissions.allow` in `.claude/settings.json` by hand: `clm upgrade --apply` never writes that file, since the settings are yours and a rule you removed on purpose isn't put back. A rule in `.claude/settings.local.json` counts too.

To keep a project that leaves the rules out on purpose as it is, run `clm upgrade --decline 20261112-build-loop-in-clm`.
