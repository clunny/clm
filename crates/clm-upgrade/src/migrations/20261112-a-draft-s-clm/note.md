Unattended sessions need clm's allow rules for clm setup in .claude/settings.json
`clm init` now seeds `.claude/settings.json` with allow rules for `clm setup`, so a headless build or draft isn't stopped to ask about it. Your project lacks one or both. These are the rules:

- `Bash(clm setup)`
- `Bash(clm setup *)`

Builds and drafts run `clm setup` before their first test run on a code branch, so that the tests run against the worktree's own dependencies (`tests-run-in-their-own-tree`). Claude Code's auto mode denies the command when no rule allows it, which leaves a draft's code-only item unfixed.

`clm init --check-permissions` prints the rules your project lacks. Add them to `permissions.allow` in `.claude/settings.json` by hand: `clm upgrade --apply` never writes that file, since the settings are yours and a rule you removed on purpose isn't put back. A rule in `.claude/settings.local.json` counts too.

To keep a project that leaves the rules out on purpose as it is, run `clm upgrade --decline 20261112-a-draft-s-clm`.
