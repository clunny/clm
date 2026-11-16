---
title: "apply's test that `clm hash` is no command observes only the root's parser"
kind: question
components: [apply, clm]
origin: 20261115-commands-in-their-crates
blocks: [clm/routes-only]
choices:
  - "It becomes the root's"
  - "clm-apply names its commands"
---

`apply/commands-in-crate` moved 8 of the 9 tests `changes/20261115-commands-in-their-crates/moved-tests.txt` lists for `apply` into `crates/clm-apply/tests/command.rs`, where they call `clm-apply`'s calls. One stays in `crates/clm/tests/cli.rs`:

- `hash_is_no_command` (`apply/hash-command`, scenario "old name gone"): it asserts that `clm hash a` fails exactly as an unknown subcommand does and that `clm --help` names `hashes` and no `hash`. Command names, the unknown-subcommand error and the top-level help are `crates/clm`'s one parser; `clm-apply`'s calls take the arguments after the name, so no call of the crate can receive `hash` (the root's Design: a test of something only a process has stays, with a `question` item).

What is needed: the spec change that moves the scenario into a requirement of the root (as `commands-live-in-their-crates` answered for the other such tests), or one that has `clm-apply` define its commands' names as a subcommand type the root flattens, so its own tests can show `hash` is none of them.

`backlog_and_archive_commands` moved with its `clm archive` half only: its `clm backlog` half is `check`'s and is already proven word for word by `crates/clm-check/tests/cli.rs`'s `backlog_lists_each_item`.

## The human's answer (2026-11-15)

The first choice, under the standing answer for tests of this kind (`commands-live-in-their-crates`): the scenario "old name gone" becomes the root's, whose one parser holds the command names and the top-level help. The batched spec change that follows the build run moves it, or folds it into `clm/old-command-names-retired` where that requirement already states it, and deletes this item.
