---
title: "A settings key this repository sets lands before its value, through a separate requirement held by a human install item"
date: 2026-10-27
status: accepted
---

`clm-config` reads every settings file into a struct that denies unknown keys (`invalid-settings`), so a typo can't pass silently. In CLM's own repository the `clm` that runs `clm sync`, `clm check` and `clm land` is the installed one, built from an earlier `main` (`dogfood-from-main`), never the branch being built. `20261027-graph-test-selection` had one requirement, `config/affected-setting`, both teach the crate the key `affected` and require this repository's `clm.toml` to set `[affected]`. The build that did both found the installed `clm` refusing the whole file: `clm sync` failed with an unknown field, `clm land` couldn't load `[land] check`, and landing it would have broken the `clm` of every clone not yet rebuilt. The build landed the parser alone and filed a `human` item to install `clm` before a second build added the table.

That workaround becomes the rule. It covers a new key, a new field of a table key, and a new value of a key, whenever this repository's `clm.toml` sets it. The change that adds one:

1. gives the parsing its own requirement of `config`, whose scenarios use fixture files only, never this repository's `clm.toml`;
2. gives setting it here a second requirement of `config`.

The build that makes the first current leaves the second out of its work and files, as a new item in its own branch, `backlog/install-clm-for-<key>.md`: `kind: human`, `nodes: [config]`, `blocks` naming the second requirement, `origin` the change. Its body asks the human to install `clm` from a `main` holding the parser in every clone they use, then delete the item. A session lands that deletion on a backlog-only branch, and the next build writes the value. Removal runs the other way: the value goes first, the parser's removal waits on the same kind of item. It is `expand-contract` applied to the settings file.

The build files the item, not the spec branch: adding a new item never makes a code branch mixed (`gate/backlog-additions`), and editing an existing item in a code branch would make it mixed.

Consequences:
- Each such key costs one human step and a second build of `config`. Keys only adopters set, never this repository, need neither: an adopter sets a key after installing a `clm` that reads it.
- The rule is this repository's, like `dogfood-from-main`: adopters run a released `clm`, so it lives in `config`'s Design, which every change to the settings reads in its context, and here, not in the plugin's method (`method-in-the-plugin`).
- The item protects the clones its human installs; a clone that pulls by rebase, or has no `post-merge` hook, still breaks until rebuilt, and says so by naming the unknown key.
- `config/affected-setting` keeps its combined form: it is built and current, and splitting it now would only rebuild it.
- Nothing checks the rule beforehand. The installed `clm` refusing the file is what enforces it (`deterministic-enforcement`), at the build; the rule only saves the build from finding that out.

Rejected:
- **Tolerate unknown keys, or warn on them.** It would let a mistyped key pass silently, which `invalid-settings` exists to stop, in every adopter's repository, to save a step in this one.
- **`#### Needs` from the value's requirement on the parser's.** A need within one node is an error (`requirement-needs`), and the need would release when the parser is current on `main`, not when the installed `clm` reads the key.
- **Have `clm land` and `clm sync` run the branch's own `clm`.** It would cover this branch's checks but not the other clones that read `main`'s `clm.toml` after the land, and it builds the tool before every sync.
- **Put the value in a later change.** It works, but costs a second spec review per key where a backlog item costs none.
- **The spec branch files the item without `blocks`, and the build adds it.** Editing an existing item makes the build's branch mixed.

## The rule in full

`ARCHITECTURE.md`'s entry `checks-relax-before-use` states this in short (`human-layer-entries-stay-short`); the rule in full:

A new settings key or value SHALL be read by a landed requirement before this repository's `clm.toml` sets it, and only once the installed `clm` can read it.

**Scenario: new key**
- GIVEN a change adding a settings key that this repository's `clm.toml` will set
- WHEN it is built
- THEN the key's reader lands and is installed before `clm.toml` sets it, since the installed `clm` rejects a key it doesn't know

**Untested**
The order in which a reader lands, is installed and its key is set is a sequence no program sees; a key the running `clm` doesn't know is refused.
