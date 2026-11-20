---
component: gate
base: { contract: "169d591024da67ba" }
---

## MODIFIED Contract
`clm gate --base <ref> [--approved]` decides whether `HEAD` may merge into `<ref>`. Its first line is the branch's kind, `spec review` for a spec branch that needs a human's review, `spec` for one that doesn't, `code` or `mixed`, then any violations in `check`'s format, and it exits 0 to allow or 1 to block. `--approved` says the ledger holds an approval of this diff that the repository's policy accepts; `clm land` computes it from the fold (`land-locally`), and the gate itself reads no ledger and no host.

`clm fingerprint`, the fingerprint an approval is pinned to, is the child `gate.fingerprint`'s (crate `clm-fingerprint`). `clm-gate` re-exports its `Entry`, `parse_ls_tree`, `fingerprint_of`, `adds_change`, `added_changes` and `fingerprint`, so `land`, `ledger`, `web` and `crates/clm` call them as `clm_gate::...` (`fingerprint-reexported`).

`clm-gate` also offers `settings_unchanged(before, after)`: given `clm.toml`'s bytes on each side of a diff (`Option<&[u8]>`, `None` where the file doesn't exist), whether its settings are the same once the upgrade notes `upgraded` and `declined_migrations`, comments and formatting are set aside, false when either side isn't UTF-8 TOML. `land` reads it to count a land whose `clm.toml` changed only so as checked (`land/settings-unchanged-is-checked`).

`clm gate` lists what a spec branch cuts, one `gate/cut` line each after the kind line: a requirement that is gone, a scenario that is gone, a scenario whose outcome changes under its name, a requirement that states fewer sentences, and a test a change's `deleted-tests.txt` lists to delete (`cuts`, `removal-needs-a-yes`). The lines change no exit status; a spec branch with any needs review. `clm-gate` offers the list as `cuts(base, head, lists)` over two spec graphs, with `widened_exemptions(base, head)`, `sentences(statement)`, `stating_sentences(statement)` and `places_a_test(sentence)`, all building for wasm, `list_lines(text)`, and `cuts_or_unloaded(base, head, lists)`, each side a `Side` holding its graph, the violations its loading gave and its revision, which returns `cuts`' result or, when a side didn't load, the one line that says so and the tests the lists add, for `web.worker.review`, `host`, `check.health`, `land` and `orchestrator.run`; each cut prints as its line, and `evaluate_at`'s result holds the cuts. A `deleted-tests.txt` line that names no citing test is the violation `gate/deleted-tests`.

`clm-gate` also offers `foreign_files(repo, base, head, component)`: the code files `head` changes against `base` that aren't `component`'s build's to change, each with its owner, none when the root at the merge base sets no `wiring` (`own-files`). The gate itself reads no branch name, so `clm gate` doesn't report them: `clm land`, which knows the branch, applies the rule (`land/build-holds-own-files`), as it applies `files-outside-spec-paths` to a spec branch.

`clm-gate` offers `clm gate` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

`clm-gate` also offers `RULES`, the version of the rules by which it decides a branch's kind and whether it needs review, and `evaluate_by(repo, base, head, approved, rules)`, the gate as it decided under the rules of version `rules`, which `evaluate_at` is with `RULES` (`rules-version`). `land` names in each merge commit the version it judged the land by (`land/gate-rules-trailer`), and `ledger`'s `verify --review` recomputes each land under the version in force when it landed (`audit-by-the-rules-in-force`).

## MODIFIED Requirements

### Requirement: cuts <!-- base: b9cced78c6eead28 -->
`clm-gate` SHALL offer `cuts(base, head, lists)`, running no process and building for `wasm32-unknown-unknown` (proven as `approval_changed`'s build is), where `base` and `head` are the spec graphs (`clm_model::Graph`) at a branch's merge base and at its head, each loaded from that side's `specs/`, `ARCHITECTURE.md`, `CAPABILITIES.md` and live `changes/<id>/change.md` files, so a rule's `serves` resolves as `clm check` resolves it, and `lists` holds, for each change with a `deleted-tests.txt` at the head, `changes/<id>/deleted-tests.txt` or `changes/archive/<id>/deleted-tests.txt`, that path and the file's text at the head and at the merge base, where an archived list with no text of its own there takes the text of the live `changes/<id>/deleted-tests.txt`, so an archive move adds no line. It SHALL return what the branch cuts, each a `Cut` that prints as its line below, in this order, the first four kinds each by requirement ID and the second and third also by the scenario's place in its requirement:
- a *gone requirement*: one `base` holds and `head` doesn't, unless each of its scenarios is moved;
- a *gone scenario*: a scenario of a requirement both graphs hold whose name the head's requirement doesn't hold, unless it is moved;
- a *changed outcome*: a scenario of a requirement both graphs hold whose name the head's requirement holds, whose outcome differs from that of the head's scenario of that name, the n-th scenario of a name at the base compared with the n-th of that name at the head, and one with no n-th of its name at the head compared with an empty outcome, so a duplicate the head loses is listed; changed outcomes are ordered by the scenario's place at the base;
- a *shortened requirement*: one both graphs hold whose statement, its text above its first `####` heading, has fewer stating sentences at the head than at the base, with both counts;
- a *test to delete*: each list line the head's text holds and the base's text doesn't, with its path, in the paths' order and then the file's.

A scenario's name is the text after `#### Scenario: ` in its requirement's block. Its *outcome* is the text of its lines from the first whose text, leading white space left out, starts `- THEN`, case kept, to the end of its block, the next `####` heading outside a fenced code block or the requirement's end, a fenced code block read as any other lines, with each run of Unicode white space, a line break included, read as one space and none at either end; a scenario with no such line has an empty outcome. A scenario is *moved* when `head` holds a scenario of the same name under a requirement of another ID that held no scenario of that name in `base`; each such scenario of `head` SHALL take one gone scenario and no more, the gone scenarios taken in the order of their requirement IDs and then of their place. A *sentence* ends at a `.`, `?` or `!` that whitespace, a line break included, or the statement's end follows. A sentence *places a test* when it holds one of the placement phrases `SHALL sit`, `sat in` and `is left in` and also the word `test` or `tests`, each phrase and word matched as whole words separated by single spaces and without regard to case, so `is left intact` holds no phrase, `places_a_test` holds no word `test` and a phrase broken across a line is not found; a sentence that holds a phrase and neither word, such as one deleting a directory once nothing else is left in it, places no test. A *stating sentence* is one that holds `SHALL` as a whole word in capitals, as the spec format writes it, and places no test. `clm-gate` SHALL offer `sentences(statement)`, returning every sentence of a statement in order, `stating_sentences(statement)`, returning its stating sentences, and `places_a_test(sentence)`, returning the first placement phrase of a sentence that places a test and nothing for any other sentence. A *list line* is `DELETE <file>::<name>`, the name ending at a tab or at the line's end; a blank line and a line starting `#` are skipped; `clm-gate` SHALL offer `list_lines(text)`, returning each list line's file and name. A list line names a test when the files agree and the names agree as `check.coverage`'s `bare_name` gives them.

For a `spec` branch, `clm gate` SHALL print one line per cut after its first line and before any violation, `evaluate_at` returning the cuts beside its verdict:
- `specs/<component path>/SPEC.md: gate/cut: requirement <id> is gone`;
- `specs/<component path>/SPEC.md: gate/cut: scenario "<name>" of <id> is gone`;
- `specs/<component path>/SPEC.md: gate/cut: scenario "<name>" of <id> changes its outcome`;
- `specs/<component path>/SPEC.md: gate/cut: <id> states <n> sentences where it stated <m>`;
- `<list path>: gate/cut: test <file>::<name> is to be deleted`.

The lines SHALL NOT change its exit status, and a branch `rename/replay` accepts SHALL have its cuts listed as any other. A `spec` branch SHALL be blocked with rule `gate/deleted-tests`, with `--approved` or without, naming the list and the line, for each line its head adds to a list that is neither skipped nor a list line, and for each list line it adds that names no test with a marker on its lines in `<file>` at the head (`check.coverage/test-lists`). The cuts SHALL be read only for a branch whose diff changes a path under `specs/` or a `deleted-tests.txt`; any other branch has none. When the graph at the merge base or at the head, loaded from the files named above and no other, loads with any violation, the requirements can't be compared: `clm gate` SHALL print `specs: gate/cut: the specs at <revision> don't load, so what the branch cuts isn't listed`, `<revision>` the commit ID of that side and the merge base's when both fail, then the *test to delete* lines of `lists`, which need no graph, and the branch SHALL need review. `clm-gate` SHALL offer that rule as `cuts_or_unloaded(base, head, lists)`, building for wasm as `cuts` does, `base` and `head` each a `Side`: the graph, the violations its loading gave and the revision as printed. It SHALL return the unloaded cut and then the tests to delete when a side holds a violation, and `cuts`' result otherwise, and SHALL be what `clm gate` lists by, so the gate and `web.worker` list one thing.

Its build updates the capability tests whose fixtures change a scenario's outcome under its name and expect `spec`, `crates/clm/tests/capability/propose_intent.rs` (`an_idea_becomes_a_spec_branch`) and `crates/clm/tests/capability/triage_unattended.rs` (`triage_a_backlog`), so each still sees the branch it builds land on checks (`capability-tests-follow-rules`).

#### Scenario: a removed requirement
- GIVEN component `a` with requirement `a/x`, whose one scenario is `first`, and a branch that only removes `a/x` from `specs/a/SPEC.md`
- WHEN `clm gate --base main` runs, and again with `--approved`
- THEN both print `spec review` and then `specs/a/SPEC.md: gate/cut: requirement a/x is gone`; the first exits 1 with `gate/approval` and the second exits 0

#### Scenario: a removed scenario
- GIVEN `a/x` with scenarios `first` and `second`, and a branch that only removes `second`
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and `specs/a/SPEC.md: gate/cut: scenario "second" of a/x is gone`

#### Scenario: a duplicate the head loses
- GIVEN `a/x` with two scenarios named `not switched on`, and a branch that only removes the second
- WHEN `clm gate --base main` runs
- THEN it prints `specs/a/SPEC.md: gate/cut: scenario "not switched on" of a/x changes its outcome`

#### Scenario: a reversed outcome
- GIVEN `a/x` with the scenario `a renamed test`, whose lines are `- GIVEN a test renamed in its file`, `- WHEN the land runs` and `- THEN it exits 1`, and a branch that only changes the last to `- THEN it lands`
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and `specs/a/SPEC.md: gate/cut: scenario "a renamed test" of a/x changes its outcome`

#### Scenario: an outcome reflowed
- GIVEN the same, and a branch that only breaks the line `- THEN it exits 1` after `it`, the rest on the next line
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and no `gate/cut` line

#### Scenario: only the setting changed
- GIVEN the same, and a branch that only changes `- GIVEN a test renamed in its file` to `- GIVEN a test renamed into another file`
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and no `gate/cut` line

#### Scenario: two scenarios of one name
- GIVEN `a/x` with two scenarios named `not switched on`, the first ending `- THEN it exits 0` and the second `- THEN it exits 1`, and a branch that only changes the second's to `- THEN it exits 2`
- WHEN `clm gate --base main` runs
- THEN it prints exactly one cut line, `specs/a/SPEC.md: gate/cut: scenario "not switched on" of a/x changes its outcome`

#### Scenario: a renamed scenario
- GIVEN the same requirement, and a branch that only changes the heading `#### Scenario: second` to `#### Scenario: the second one`
- WHEN `clm gate --base main` runs
- THEN it prints the same cut line for `second`, since a program can't tell a rename from a removal

#### Scenario: moved to another component
- GIVEN `a/x` with the one scenario `the chain`, component `b` with requirement `b/y`, and a branch that removes `a/x` and adds a scenario `the chain` to `b/y`
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and no `gate/cut` line

#### Scenario: two requirements merged
- GIVEN `a/x` and `a/y`, each with the one scenario `can edit`, and a branch that removes both and adds `a/z` with one scenario `can edit`
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exactly one cut line, `specs/a/SPEC.md: gate/cut: requirement a/y is gone`

#### Scenario: a name another requirement already held
- GIVEN `a/x` and `b/y`, each with a scenario `not switched on`, and a branch that only removes that scenario from `a/x`
- WHEN `clm gate --base main` runs
- THEN it prints `specs/a/SPEC.md: gate/cut: scenario "not switched on" of a/x is gone`

#### Scenario: a lost sentence
- GIVEN `a/x` whose statement is `The command SHALL print the list. It SHALL exit 1 when the list is empty.`, and a branch that removes the second sentence and no scenario
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and `specs/a/SPEC.md: gate/cut: a/x states 1 sentences where it stated 2`

#### Scenario: a sentence that places a test
- GIVEN `a/x` whose statement ends `Every test of a's requirements SHALL sit in a's own files.`, and a branch that removes only that sentence
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and no `gate/cut` line

#### Scenario: a sentence of behaviour that holds a phrase
- GIVEN `a/x` whose statement is `The command SHALL print the list. The directory SHALL be deleted once nothing else is left in it.`, and a branch that removes the second sentence
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and `specs/a/SPEC.md: gate/cut: a/x states 1 sentences where it stated 2`

#### Scenario: a reworded requirement
- GIVEN `a/x` whose statement is `The command SHALL print the list.`, and a branch that changes it to `` `clm list` SHALL print one line per item. `` and renames no scenario
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and no `gate/cut` line

#### Scenario: a test the change lists
- GIVEN `tests/t.rs` holding a test `two` that cites `a/x`, and a branch that only adds `changes/c/change.md` and `changes/c/deleted-tests.txt` holding a `#` line and `DELETE tests/t.rs::two`, a tab and a reason
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and `changes/c/deleted-tests.txt: gate/cut: test tests/t.rs::two is to be deleted`

#### Scenario: a line that names no test
- GIVEN the same branch, its list also holding `DELETE tests/t.rs::nope` and a line `remove the old ones`
- WHEN `clm gate --base main --approved` runs
- THEN it exits 1 with two `gate/deleted-tests` violations, one naming `tests/t.rs::nope` and one naming `remove the old ones`

#### Scenario: a list whose tests are gone
- GIVEN `main` holding `changes/c/deleted-tests.txt` with `DELETE tests/t.rs::two` and no test `two`, and a branch that only adds a requirement to `specs/a/SPEC.md`
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and no violation

#### Scenario: a list moved into the archive
- GIVEN `main` holding `changes/c/deleted-tests.txt` with `DELETE tests/t.rs::two` and no test `two`, and a branch that only moves `changes/c/` to `changes/archive/c/`
- WHEN `clm gate --base main` runs
- THEN it prints no `gate/cut` line and no `gate/deleted-tests` violation

#### Scenario: a lowercase shall
- GIVEN `a/x` whose statement is `The command SHALL print the list. It shall be quick.`, and a branch that removes the second sentence
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and no `gate/cut` line

#### Scenario: a phrase inside a word
- GIVEN `a/x` whose statement is `The command SHALL print the list. The file SHALL be what is left intact.`, and a branch that removes the second sentence
- WHEN `clm gate --base main` runs
- THEN it prints `specs/a/SPEC.md: gate/cut: a/x states 1 sentences where it stated 2`

#### Scenario: a sentence ending at a line break
- GIVEN `a/x` whose statement holds `It SHALL print the list.`, a line break, then `It SHALL exit 0.`, and a branch that removes the second
- WHEN `clm gate --base main` runs
- THEN it prints `specs/a/SPEC.md: gate/cut: a/x states 1 sentences where it stated 2`

#### Scenario: rules that serve the human layers
- GIVEN a repository whose `CAPABILITIES.md` holds entry `c`, component `a` with `serves: [capability/c]`, and a branch that only adds a requirement to `a`
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and no `gate/cut` line

#### Scenario: specs that don't load
- GIVEN a branch that only changes `specs/a/SPEC.md` so that a requirement of `a` holds no scenario
- WHEN `clm gate --base main` runs, and `cuts_or_unloaded` runs for the two graphs with their violations
- THEN the gate prints `spec review` and `specs: gate/cut: the specs at <c> don't load, so what the branch cuts isn't listed`, `<c>` the head's commit ID, and the call returns that one cut

#### Scenario: unloaded specs and a listed test
- GIVEN the same branch, also adding `changes/c/deleted-tests.txt` with `DELETE tests/t.rs::two`, a test that cites `a/x`
- WHEN `clm gate --base main` runs
- THEN it prints the unloaded line and then `changes/c/deleted-tests.txt: gate/cut: test tests/t.rs::two is to be deleted`

#### Scenario: a branch that touches no spec
- GIVEN a branch that only edits a live `changes/c/change.md` so that its frontmatter names an unknown key, and no path under `specs/`
- WHEN `clm gate --base main` runs
- THEN it prints no `gate/cut` line

#### Scenario: from two graphs
- GIVEN the graphs of *a removed scenario* loaded from texts, and no lists
- WHEN `cuts(base, head, &[])` runs
- THEN it returns one gone scenario, `second` of `a/x`

#### Serves
- architecture/removal-needs-a-yes
- capability/approve-intent
- capability/propose-intent
- capability/triage-unattended

#### Needs
- check.coverage/test-lists
