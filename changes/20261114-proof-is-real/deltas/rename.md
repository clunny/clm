---
component: rename
---

## MODIFIED Requirements

### Requirement: marker-rewrite <!-- base: 8ae3d4f6d226ac00 -->
`clm rename` SHALL rewrite each marker, in every file `scan` reads, the markers outside a test included (`scan`'s `outside`, `check.coverage/test-spans`), found with `marker_spans`, that cites a renamed requirement or one whose text the change rewords: a marker citing the requirement's current version SHALL cite the new ID at the requirement's version after the change, and any other marker SHALL cite the new ID with its version unchanged, so a requirement current before the rename is current after it, and one behind stays behind; except that a marker citing the current version of a requirement whose text `references` rewrote SHALL keep that version, under the new ID, when a mention `finish` reports of an old ID or slug rewritten in that text lies in a file the requirement's component owns or a file holding one of its markers, so that requirement is `behind` after the rename. A mention of a term never holds a marker back. Nothing else in the file SHALL change.

#### Scenario: current and behind markers
- GIVEN requirement `a/x` at version `11111111`, a test citing `a/x@11111111` and another citing `a/x@00000000`, and a change renaming `a/x` to `a/y`, whose version becomes `22222222`
- WHEN `clm rename` runs
- THEN the tests cite `a/y@22222222` and `a/y@00000000`, and `clm pending a` lists nothing for `a/y`, which the first test keeps current

#### Scenario: reworded requirement
- GIVEN requirement `a/x` at version `11111111` reading `the sprocket`, a test citing it at `11111111`, and a change swapping `sprocket` for `cogs`
- WHEN `clm rename` runs
- THEN the test cites `a/x` at its new version, and `clm pending a` lists nothing for `a/x`

#### Scenario: code still names a rewritten entry
- GIVEN `ARCHITECTURE.md` with entry `sprocket-scoped-context`, no other requirement, entry or decision with that slug, component `a` with `paths: ["plugin/**"]` whose requirement `x` at version `11111111` reads ``(`sprocket-scoped-context`)``, a test citing `a/x@11111111`, `plugin/skills/s/SKILL.md` reading `` `sprocket-scoped-context` ``, and change `r`, `proposed`, with `renames` holding `requirement: architecture/sprocket-scoped-context` and `to: cogs-scoped-context`
- WHEN `clm rename r` runs
- THEN `a/x` reads ``(`cogs-scoped-context`)``, the test still cites `a/x@11111111`, `clm pending a` lists `a/x` as `behind`, and it prints `still names sprocket-scoped-context plugin/skills/s/SKILL.md:<line>`

#### Scenario: another component's code names it
- GIVEN the same, but `plugin/skills/s/SKILL.md` owned by component `b`, and no file holding a marker of `a/x` naming `sprocket-scoped-context`
- WHEN `clm rename r` runs
- THEN the test cites `a/x` at its new version, and `clm pending a` lists nothing for `a/x`

#### Scenario: a term left in the code
- GIVEN requirement `a/x` of component `a` with `paths: ["plugin/**"]`, at version `11111111` reading `the sprocket`, a test citing it at `11111111`, `plugin/skills/s/SKILL.md` reading `the sprocket`, and a change swapping `sprocket` for `cogs`
- WHEN `clm rename` runs
- THEN the test cites `a/x` at its new version, and it prints `still names sprocket plugin/skills/s/SKILL.md:<line>`

#### Scenario: a marker outside a test
- GIVEN `src/lib.rs` holding a marker citing `a/x` in a function with no test attribute, and a rename of `a/x` to `a/y`
- WHEN `clm rename` runs
- THEN the marker cites `a/y`

#### Needs
- check.coverage/marker-spans
- check.coverage/test-spans
