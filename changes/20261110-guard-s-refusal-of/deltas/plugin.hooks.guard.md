---
component: plugin.hooks.guard
---

## ADDED Requirements

### Requirement: block-refuses-whole-command
When `clm hook guard` blocks a `Bash` command (exit 2), under any rule, the reason it writes to stderr SHALL end with one line saying that the whole command line was refused and nothing in it ran, the commands chained before and after the refused part included, so the session must run again any of them it still needs. Claude Code cancels the whole tool call on a `PreToolUse` hook's exit 2, while each rule's reason speaks only of the part it judged, so without this line a session reads the commands before the refused one as having run. The line SHALL come once, after every rule's reason, however many rules refused the command. A command the guard allows or asks about SHALL NOT carry it.

#### Scenario: a commit chained before a piped land
- GIVEN the commands `git commit -m m && clm land impl/x | tail` and `git add f && git push origin HEAD:main`, from a repository under CLM
- WHEN `clm hook guard` runs on each
- THEN each exits 2, and the last line of its reason says the whole command line was refused and nothing in it ran, naming `nothing in it ran`

#### Scenario: two rules refuse one command
- GIVEN the command `git push origin main; clm land spec/x | tail`, from a repository under CLM
- WHEN `clm hook guard` runs on it
- THEN it exits 2 with a reason naming `guard-blocks-push-to-main` and `guard-refuses-piped-land`, and `nothing in it ran` appears once, on its last line

#### Scenario: allowed and asked commands
- GIVEN the commands `git commit -m m && clm land impl/x` and `clm approve spec/x`
- WHEN `clm hook guard` runs on each
- THEN neither's output names `nothing in it ran`
