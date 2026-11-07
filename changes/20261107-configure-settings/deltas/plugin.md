---
component: plugin
---

## ADDED Requirements

### Requirement: configure-skill
The plugin SHALL ship `skills/configure/SKILL.md`, with a `description` in its frontmatter saying it reviews CLM's settings or changes one, and that it applies whenever the person wants to see or change how CLM behaves in the project, in plain words or by a key's name. It SHALL tell Claude to:
- run `clm config list` for every key's value and where it came from, and `clm guide settings` for what each key does, what each choice trades off and where it may be set; when `clm config list` fails, show the error, which names the file and the key, and offer to fix that first;
- with no argument, review every setting in the page's order: for each, show its value and where it came from and the trade-offs briefly, and ask whether to change it, leaving `upgraded`, `declined_migrations` and `skip_migrations` to `/clm:upgrade`;
- with an argument (`/clm:configure set concurrent builds`), find the key it means from the page, asking when more than one fits, and go straight to that key;
- for a change, say which files may set the key and recommend one: `clm.toml` for the project, the clone's `clm.local.toml` for this clone, the person's own `config.toml` for every project of theirs; never set a key in a file the page forbids, and say when an environment variable sets the key, since a file's value doesn't count while it does;
- edit the file in place, keeping its comments and other keys, then run `clm config list` again, and on a failure fix the edit or undo it;
- commit nothing for `clm.local.toml` or the person's own file, which take effect at once;
- gather every `clm.toml` edit of the session on one branch `config/<YYYYMMDD>-<slug>` cut from `origin/main` in a worktree of its own (`session-worktrees`), commit it there, and land it by the first line of `clm gate --base origin/main` as `propose` lands a branch: on `code` or `spec`, `clm branch push`, `clm pr open` and `clm land`; on `spec review`, which an edit to `approval` makes, show the human the edit and ask for their approval, and on a yes run `clm approve` through the guard's confirmation before `clm land`; then `clm wrap-up`.

It SHALL tell Claude never to push `main` by any means but `clm land`, and to run `clm approve` only after the human says yes in the conversation, through the guard's confirmation. A test in `crates/clm/tests/plugin.rs`, a function over the skill's text, SHALL read the skill and fail naming what it lacks.

#### Scenario: the skill
- GIVEN the plugin's `skills/configure/SKILL.md`
- WHEN a test reads it
- THEN it names `clm config list`, `clm guide settings`, `clm.local.toml`, `/clm:upgrade`, `clm gate`, `spec review`, `clm approve`, `clm land` and `clm wrap-up`

#### Scenario: a skill that doesn't validate
- GIVEN a `configure` `SKILL.md` that doesn't mention `clm config list`
- WHEN the check runs on it
- THEN it reports `clm config list`

#### Needs
- config/config-list

#### Serves
- capability/configure-clm

### Requirement: propose-adds-settings-sections
`skills/propose/SKILL.md` SHALL tell the session that when a requirement its change adds or modifies adds a settings key, or a field or value of one, the same spec branch adds or edits the key's section in the guide's settings page, `docs/guide/settings.md`, saying what it does, what each choice trades off and where it may be set: the page is a spec path, so the code branch that builds the key can't carry it, and without the section that branch's land fails `config/settings-guide-covers-keys`. It says so in one paragraph of the skill's design steps, beside the glossary's, without moving an existing step. A test in `crates/clm/tests/plugin.rs`, a function over the skill's text, SHALL read the skill and fail naming the missing text.

#### Scenario: the skill says so
- GIVEN the plugin's `skills/propose/SKILL.md`
- WHEN a test reads it
- THEN it holds `docs/guide/settings.md` and `config/settings-guide-covers-keys` in one paragraph

#### Scenario: a skill without it
- GIVEN a `propose` `SKILL.md` that doesn't mention `docs/guide/settings.md`
- WHEN the check runs on it
- THEN it reports `docs/guide/settings.md`

#### Serves
- capability/configure-clm
- capability/propose-intent

## MODIFIED Requirements

### Requirement: help-skill <!-- base: 67498eaa4bcfe398 -->
The plugin SHALL ship `skills/help/SKILL.md` with a `description` in its frontmatter saying it answers questions about CLM and how to use it, for a new user among others. It SHALL tell Claude to answer from what the installed `clm` carries, never from this repository's files or a URL (`method-in-the-plugin`): `clm guide` for the list of pages and `clm guide <page>` for the pages the question needs, `clm guide constraints` for CLM's carried constraints, and, in a repository under CLM, `clm status` and `clm backlog` for its state; outside one, to explain how to start from `clm guide quickstart` and point at `/clm:init`. It SHALL tell Claude to say, when asked why a constraint exists or whether a project can do without it, the constraint's tier and what CLM relies on, and that a `replaceable` constraint can be replaced by a constraint that keeps its guarantee and a `default` one dropped, through `/clm:propose`, while a `method` one can't. It SHALL tell Claude to point at the skill that acts (`/clm:init`, `/clm:propose`, `/clm:build`, `/clm:triage`, `/clm:health`, `/clm:contribution`, and `/clm:configure` for a question about a setting) and the view (`/clm:view`), to change no file, run no command that writes (no `clm claim`, `clm apply`, `clm land`, `clm approve` or `git commit`), and to say so when the guide doesn't answer a question rather than guess. It SHALL hold the sentence `Help changes no file.`

#### Scenario: help without the guide
- GIVEN a `skills/help/SKILL.md` that doesn't mention `clm guide constraints`
- WHEN the tests run
- THEN a test fails naming the skill and `clm guide constraints`

#### Scenario: help that writes
- GIVEN a `skills/help/SKILL.md` that doesn't hold `Help changes no file.`
- WHEN the tests run
- THEN a test fails naming the skill and `Help changes no file.`

#### Scenario: help on a setting
- GIVEN a `skills/help/SKILL.md` that doesn't mention `/clm:configure`
- WHEN the tests run
- THEN a test fails naming the skill and `/clm:configure`

#### Needs
- init/guide-constraints-page

#### Serves
- architecture/method-in-the-plugin
- architecture/carried-constraints
