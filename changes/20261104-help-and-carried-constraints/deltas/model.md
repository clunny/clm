---
component: model
---

## ADDED Requirements

### Requirement: carried-constraints
The model SHALL compile in, with `include_str!`, the text of CLM's own `ARCHITECTURE.md` at this repository's root as it is when the crate is built, and expose its *carried entries*: each architecture entry holding a `#### Carries` block, in file order, with its slug, its tier (the block's one line, `- method`, `- replaceable` or `- default`), its statement (the entry's text from after its heading to its first `####` heading, trimmed) and, for a `replaceable` entry, its `#### Guarantee` block's text, trimmed. A repository whose own `ARCHITECTURE.md` holds an entry with a `#### Carries` block is the *source*, CLM's own, and its graph takes the carried entries from its own file; any other repository's graph, including one with no `ARCHITECTURE.md`, takes them from the compiled copy, so it holds the set its `clm` enforces. Outside the source, an entry of the repository's `ARCHITECTURE.md` MAY hold one `#### Replaces` block of `- <slug>` lines, each naming a carried entry it replaces or drops. The graph SHALL say whether it is the source, and expose the carried entries and, outside the source, the carried entries in force: those no entry's `#### Replaces` names, in the carried order. `#### Carries`, `#### Guarantee` and `#### Replaces` blocks are part of the entry's block, so of its hash, as `#### Enforced by` is (`human-layers`). The model SHALL report as `model/human-layers`, naming `ARCHITECTURE.md` and the entry's slug: a `#### Carries` block that isn't exactly one line naming a tier; a `replaceable` entry without exactly one `#### Guarantee` block, and a `#### Guarantee` block in any other entry; a second `#### Carries` or `#### Replaces` block in one entry; a `#### Replaces` block in the source, or one that is empty; a `#### Replaces` line that isn't a `- <slug>` bullet, that names no carried entry, or that names a `method` entry, naming that slug; and, outside the source, an entry whose slug is a carried entry's slug. It SHALL report as `model/human-layers`, naming `CAPABILITIES.md` and the slug, a capability holding any of the three blocks. A test of the compiled copy reads this repository's `ARCHITECTURE.md`, so it holds a `clm:reads` marker naming the file (`doc-readers-marked`).

#### Scenario: the source
- GIVEN an `ARCHITECTURE.md` with entry `a` holding `#### Carries` `- method`, entry `b` holding `#### Carries` `- replaceable` and a `#### Guarantee` paragraph, and entry `c` with no `#### Carries`
- WHEN the model loads
- THEN the graph is the source, its carried entries are `a` (`method`) then `b` (`replaceable`, with its guarantee text), `c` is not among them, and loading reports nothing

#### Scenario: a project
- GIVEN an `ARCHITECTURE.md` with no `#### Carries` block, whose entry `x` holds `#### Replaces` naming `expand-contract` and `yagni`
- WHEN the model loads
- THEN the graph is not the source, its carried entries are the compiled copy's, holding `proven-by-tests`, `expand-contract` and `yagni`, the entries in force hold `proven-by-tests` and neither `expand-contract` nor `yagni`, and loading reports nothing

#### Scenario: no architecture file
- GIVEN a repository with no `ARCHITECTURE.md`
- WHEN the model loads
- THEN the entries in force are every carried entry of the compiled copy

#### Scenario: bad replaces
- GIVEN a project `ARCHITECTURE.md` whose entry `x` holds `#### Replaces` naming `proven-by-tests` and `no-such-entry`, and whose entry `yagni` is its own
- WHEN the model loads
- THEN loading reports `model/human-layers` for `x` with `proven-by-tests`, for `x` with `no-such-entry`, and for `yagni`

#### Scenario: bad carries
- GIVEN a source `ARCHITECTURE.md` with entry `a` holding `#### Carries` `- optional`, entry `b` holding `#### Carries` `- replaceable` and no `#### Guarantee`, entry `c` holding `#### Carries` `- method` and a `#### Guarantee`, and entry `d` holding `#### Replaces`
- WHEN the model loads
- THEN loading reports `model/human-layers` for each of `a`, `b`, `c` and `d`

#### Serves
- architecture/carried-constraints
