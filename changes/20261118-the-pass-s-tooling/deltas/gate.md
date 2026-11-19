---
component: gate
base: { contract: "fc6200cf16f25b22", design: "049e61317962f154" }
---

## MODIFIED Contract
`clm gate --base <ref> [--approved]` decides whether `HEAD` may merge into `<ref>`. Its first line is the branch's kind, `spec review` for a spec branch that needs a human's review, `spec` for one that doesn't, `code` or `mixed`, then any violations in `check`'s format, and it exits 0 to allow or 1 to block. `--approved` says the ledger holds an approval of this diff that the repository's policy accepts; `clm land` computes it from the fold (`land-locally`), and the gate itself reads no ledger and no host.

`clm fingerprint`, the fingerprint an approval is pinned to, is the child `gate.fingerprint`'s (crate `clm-fingerprint`). `clm-gate` re-exports its `Entry`, `parse_ls_tree`, `fingerprint_of`, `adds_change`, `added_changes` and `fingerprint`, so `land`, `ledger`, `web` and `crates/clm` call them as `clm_gate::...` (`fingerprint-reexported`).

`clm-gate` also offers `settings_unchanged(before, after)`: given `clm.toml`'s bytes on each side of a diff (`Option<&[u8]>`, `None` where the file doesn't exist), whether its settings are the same once the upgrade notes `upgraded` and `declined_migrations`, comments and formatting are set aside, false when either side isn't UTF-8 TOML. `land` reads it to count a land whose `clm.toml` changed only so as checked (`land/settings-unchanged-is-checked`).

`clm gate` lists what a spec branch cuts, one `gate/cut` line each after the kind line: a requirement that is gone, a scenario that is gone, a requirement that states fewer sentences, and a test a change's `deleted-tests.txt` lists to delete (`cuts`, `removal-needs-a-yes`). The lines change no exit status; a spec branch with any needs review. `clm-gate` offers the list as `cuts(base, head, lists)` over two spec graphs, with `widened_exemptions(base, head)`, `stating_sentences(statement)` and `places_a_test(sentence)`, all building for wasm, for `web.worker.review`, `host` and `check.health`; `evaluate_at`'s result holds the cuts. A `deleted-tests.txt` line that names no citing test is the violation `gate/deleted-tests`.

`clm-gate` also offers `foreign_files(repo, base, head, component)`: the code files `head` changes against `base` that aren't `component`'s build's to change, each with its owner, none when the root at the merge base sets no `wiring` (`own-files`). The gate itself reads no branch name, so `clm gate` doesn't report them: `clm land`, which knows the branch, applies the rule (`land/build-holds-own-files`), as it applies `files-outside-spec-paths` to a spec branch.

`clm-gate` offers `clm gate` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## MODIFIED Design
Changes come from `git diff --name-status --no-renames <merge-base>...HEAD`. Spec paths are `specs/`, `changes/`, `backlog/`, `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and `AGENTS.md`. File contents at the merge base and at `HEAD` are compared with `git diff <rev>:<path> <rev>:<path>`. Uses the `git` CLI, not a git library.

**Review is yes or no** (`yes-no-review`). A spec branch needs a human's review only for what a program can't judge for the human: the two human layers, `ARCHITECTURE.md` and `CAPABILITIES.md`, edited whole in the branch; the keys a signed repository trusts, `docs/allowed_signers` (any entry at or under that path, never counted as a status-only edit; `signed-ledger-records`); whose approval counts (`approval-setting`); and a change that declares a burden, something that spends the human's money or time or can't be undone. Everything else, component specs, decisions, backlog edits and changes with no burdens, lands on checks, and nothing is reviewed after the fact. Whether a branch needs review is read from the diff's paths, `clm.toml` at both sides and the `change.md` texts at `HEAD`, so no graph is loaded. A `change.md` that can't be read or parsed needs review, so the gate fails closed; a live one that holds `gate` parses (the model sinks the key) and `clm check` rejects it (`model/parse-changes`). `undeclared_burdens` checks what a program can see of a burden: a `kind: human-task` item the branch adds is a step only the human can take, so a spec branch that adds one must say so in `human_burdens`, which then makes it need review. Builds file `question` and `human-task` items freely; those block only their own requirements.

**Rename branches** (`rename-by-replay`). A branch whose whole diff `rename/replay` reproduces, from the merge base and the renames its one added change records, is classified `spec` though it rewrites test markers in code files, since every edit follows from the pairs a human or the gate can read. Its edits to `ARCHITECTURE.md` and `CAPABILITIES.md` are replayed renames of IDs, which change no intent, so they don't make it need review; a `term` pair does, since re-citing a reworded requirement skips the build that would confirm its tests, and only a human can judge that a swapped word keeps a rule's meaning (`hash-based-staleness`). Its `human_burdens`, and a `change.md` that doesn't parse, count as for any change. `classify` asks `clm_rename::replays` only when the diff holds code files and the branch adds exactly one change whose `change.md` at `HEAD` sets `renames`, so no other branch pays for the extraction; when it answers no, the branch is classified by its paths. `needs_review_of` stays the path rule the Worker shares; `evaluate_at` uses it, with `widened_exemptions` and `cuts`, for every `spec` branch but a replayed one, whose review is its own rule and whose cuts are still listed.

The gate loads `specs/` at the merge base only for the code-fix check's component lookup (`code-fix-resolves-bug`), with `clm_model::load`, extracted into a temporary directory with `clm-git`'s `extract_paths`, so each file holds the bytes a checkout of that commit writes (`extract-as-checkout`, `base-specs-as-checkout`). The paths passed are `specs`, when it exists at the merge base, and each path of the form `changes/<id>/change.md` that `git ls-tree -r --name-only <rev> -- changes` lists, since `extract_paths` takes its paths literally and errors on one that matches nothing. The live changes' `change.md` files carry the component frontmatter keys they declare in `frontmatter_keys`, so a component that sets one loads as `clm check` loads it (`declared-keys-loaded`, `declared-frontmatter-keys`). Their deltas and assets aren't extracted, since nothing the gate reads from the loaded graph needs them.

A *change the PR adds* is a live `changes/<id>/` directory, or a *new archived change*: `changes/archive/<id>/` with `change.md` among the files the PR adds there other than by an archive move (`archive-move`). The second is a change that needs nothing built, such as a design-only one, marked `done` and archived in its own spec PR (`archive-in-spec-pr`). Since `archive-immutable` forbids editing anything under `changes/archive/`, the gate treats a new archived change's files as it treats a live change's: its `human_burdens` count for `needs-review` and `undeclared-burden`, and its `resolves` is checked by `resolves-named-items`. The PR adds the whole of such a `change.md`, so every one of its `resolves` entries is new and tested, as for a live `change.md` the PR adds. The exception is a live change moved into the archive with more than `status:` lines edited, which isn't an archive move: its entries are compared with the live `changes/<id>/change.md` at the merge base, so items a task deleted (`task-deletes-fixed-items`) aren't reported.

A code PR records its progress in its tests' markers, `req:<id>@<version>` (`versioned-markers`), and nothing under `changes/` but an archive move (`archive-move`) when `clm rebase` archived a complete change on its branch (`archive-on-sync`).

**Code fixes** (`code-fix-resolves-bug`). A code PR may delete the backlog item of a bug it fixes, or of deferred work it does, when it changes no spec (`deferred-work-resolves-in-code`). A commit names the item with the trailer `Resolves: backlog/<slug>`, read with `git log --format=%(trailers:key=Resolves,valueonly) <merge-base>..HEAD`, one slug per value. Values are trimmed, and only those of the exact form `backlog/<slug>` count; a value naming an item the PR doesn't delete is ignored. The deletion is exempt when `backlog/<slug>.md` at the merge base has `kind: bug` or `kind: deferred` and its `components` name a component, in `specs/` at the merge base, that a code file (one outside the spec paths) the PR adds, modifies or deletes belongs to. A file belongs to a component when one of the component's `paths` matches it, or when its text at the merge base or at `HEAD` holds `req:<component>/`, the start of a marker citing one of the component's requirements (`fixes-count-where-cited`). The second way is for a test that lives in another component's `paths`, as a project's tests do until it gives them to their components: a fix to one of them belongs to the component it cites, as `tests-count-where-cited` counts it toward that component's size. It says whose fix it is, not whose file: `clm land` still holds the branch to its own files (`land/build-holds-own-files`). Either version counts, so a fix that removes or adds a file's last marker belongs where the file did or does. The text is searched as is, with no marker parsing: `req:plugin/` doesn't occur in `req:plugin.hooks/x`, and a fixture that builds its marker from pieces cites nothing. `kind` and `components` are read from the item's frontmatter alone, so a key the model rejects can't hide them; an item whose frontmatter doesn't parse, or none of whose components exist at the merge base, isn't exempt; a named component missing there counts for neither `paths` nor markers, and the others count. A component with no `paths` can still qualify through markers. A file whose text isn't valid UTF-8 is searched as bytes, and one that can't be read on a side holds no marker on that side; neither makes the gate fail. The search runs only when a trailer names an item the PR deletes. Only the item's own components count, not their parents or children, since `paths` are disjoint and a marker names one component. A path matches as a git `:(glob)` pathspec, as `clm size` matches `paths`, tested against the PR's diff (`git diff --name-only <merge-base> HEAD -- ':(glob)<path>'`) so a deleted file matches too. Unlike `resolves`, which a change must have merged before the branch deletes the item, the branch here both names and deletes it. What bounds it is that deleting a bug or deferred item needs no review anyway, and that the PR's code must touch a component the item names, by its `paths` or by a test that proves it.

**Approval.** The host plays no part in review (`per-writer-ledgers`). `--approved` is the flag, and it is `clm land`'s word: the gate doesn't know who approved or under which policy, which keeps it a pure function of the two trees and one flag, and keeps every caller's verdict the same. The `gate/approval` message names `clm approve <branch>`, so a session knows what to ask the human for.

**Fingerprint.** `gate.fingerprint` holds it (`tree-diff-fingerprint`); `clm-gate` keeps only the re-export (`fingerprint-reexported`). The gate itself computes no fingerprint.

**Where the gate's tests live.** All of `gate`'s tests are its crate's, under `crates/clm-gate/tests/`; those that proved a rule by running the `clm` binary call the crate's own command (`commands-in-crate`).

**Cuts** (`cuts`, `cuts-are-what-a-program-lists`). `removal-needs-a-yes` was the session's word alone. The gate now lists what a program can see of a removal and asks review for it, reading the requirements at the merge base and at the head from `specs/`, where `clm apply` has already put the branch's deltas, so it reads no delta. Both graphs are loaded as the code-fix check loads the base's, the head's from `head`'s tree, and only for a `spec` branch whose diff touches `specs/` or a `deleted-tests.txt`. A scenario's name is the text of its `#### Scenario:` heading in the requirement's block. A name is matched within the diff and one for one, since many requirements share a name such as "not switched on": matching against the whole head would never list one. So a rename is listed, as a program can't tell it from a removal, and a session keeps a scenario's name when it rewords or moves it. Counting sentences that hold `SHALL` sees a requirement that states less and not one that states something else in as many sentences, which stays a reader's. The placement phrases are a constant of the crate, changed by a code PR. The tests a change deletes are declared, not computed: `clm coverage --obsolete-tests` lists every test of a merged requirement, which a build re-cites, and misses a duplicate whose requirement stays. The gate checks only the lines a branch adds, so a list whose tests a build has since deleted stays valid, and `clm land` holds a code branch to the lists on its base (`land/deleted-tests-approved`).

A branch behind `<ref>` may land (`land-locally`). `clm land` merges `<ref>` into it and checks the result before anything lands. So the gate blocks only what that merge can't settle. `git merge-tree --write-tree <ref> HEAD` exits 1 on a textual conflict. A file under `specs/` that changed on both sides since the merge base is also blocked: `specs/` is derived, so it is regenerated by `clm rebase` and never merged (`regenerate-dont-merge`), even when the text would merge cleanly. Both report rule `gate/up-to-date` and tell the author to run `clm rebase`.

**Evaluating another commit** (`gate/evaluate-at`). `evaluate_at(repo, base, head, approved)` is the gate for `head` against `base` as `clm gate --base <base>` gives it with `head` checked out: it reads `head` only as a revision, through `git diff <merge-base>...<head>`, `git show <head>:<path>`, `git ls-tree <head>`, `git log <merge-base>..<head>` and `git merge-tree --write-tree <base> <head>`, never the working tree or the index, so it gives the same verdict whatever is checked out. A violation's path names the range as `<base>..<head>` with the revisions as given, so `clm gate`'s output stays `<base>..HEAD`. `clm gate` calls it with `HEAD`. `ledger`'s `verify --review` calls it for each signed land, with the land's first parent as `base` and the merge commit as `head`, to recompute the land's kind and whether it needed review without a checkout per land.

**A build's own files** (`own-files`, `own-files-at-the-gate`). `path-ownership` was kept by the build prompt alone; a program now refuses what the prompt asked for. The gate stays a function of two trees and one flag and reads no branch name (`spec-branches-hold-spec-paths`), so `foreign_files` takes the component as an argument and `clm land`, which knows the branch, calls it for a branch named `impl/<component>`: no other branch has a component, and a small change on a branch of another name is not judged. The specs it reads are the merge base's, loaded as for the code-fix check; a file's owner is `check.coverage`'s `owner`, which the gate depends on for it. Two kinds of file are the build's to change beside its own. The root's `wiring` names the shared ones. A file that merely cites the component is not one of them: a test sits in the files of the component it proves, so a marker gives a build no file (`commands-live-in-their-crates`); the clause that once opened such a file, for the tests Cargo kept in the root package's directory, went when each crate came to prove its requirements through its own call. A capability test may be edited, never added or deleted, by a build whose component serves the capability it cites, since a behaviour and the test that observes it at the boundary can't land apart (`capability-tests-follow-rules`). That a wiring edit adds only the build's own entry, and that a capability-test edit keeps what its scenarios state, stay judgements no program makes. The rule is off until the root sets `wiring`, as `check`'s path ownership is off until it sets `outside`, so a project adopts it when it has named its wiring.

## ADDED Requirements

### Requirement: cuts
`clm-gate` SHALL offer `cuts(base, head, lists)`, running no process and building for `wasm32-unknown-unknown` (proven as `approval_changed`'s build is), where `base` and `head` are the spec graphs (`clm_model::Graph`) at a branch's merge base and at its head, and `lists` holds, for each change with a `deleted-tests.txt` at the head, `changes/<id>/deleted-tests.txt` or `changes/archive/<id>/deleted-tests.txt`, that path and the file's text at the merge base, when it has one, and at the head. It SHALL return what the branch cuts, in this order, the first three kinds each by requirement ID and the second also by the scenario's place in its requirement:
- a *gone requirement*: one `base` holds and `head` doesn't, unless each of its scenarios is moved;
- a *gone scenario*: a scenario of a requirement both graphs hold whose name the head's requirement doesn't hold, unless it is moved;
- a *shortened requirement*: one both graphs hold whose statement, its text above its first `####` heading, has fewer stating sentences at the head than at the base, with both counts;
- a *test to delete*: each list line the head's text holds and the base's text doesn't, with its path, in the paths' order and then the file's.

A scenario's name is the text after `#### Scenario: ` in its requirement's block. A scenario is *moved* when `head` holds a scenario of the same name under a requirement of another ID that held no scenario of that name in `base`; each such scenario of `head` SHALL take one gone scenario and no more, the gone scenarios taken in the order of their requirement IDs and then of their place. A *sentence* ends at a `.`, `?` or `!` that a space or the statement's end follows, and a *stating sentence* is one that holds the word `SHALL` and none of the placement phrases `SHALL sit`, `sat in` and `is left in`, compared without regard to case; `clm-gate` SHALL offer `stating_sentences(statement)`, returning them, and `places_a_test(sentence)`, returning the first placement phrase a sentence holds. A *list line* is `DELETE <file>::<name>`, the name ending at a tab or at the line's end; a blank line and a line starting `#` are skipped.

For a `spec` branch, `clm gate` SHALL print one line per cut after its first line and before any violation, `evaluate_at` returning the cuts beside its verdict:
- `specs/<component path>/SPEC.md: gate/cut: requirement <id> is gone`;
- `specs/<component path>/SPEC.md: gate/cut: scenario "<name>" of <id> is gone`;
- `specs/<component path>/SPEC.md: gate/cut: <id> states <n> sentences where it stated <m>`;
- `<list path>: gate/cut: test <file>::<name> is to be deleted`.

The lines SHALL NOT change its exit status, and a branch `rename/replay` accepts SHALL have its cuts listed as any other. A `spec` branch SHALL be blocked with rule `gate/deleted-tests`, with `--approved` or without, naming the list and the line, for each line its head adds to a list that is neither skipped nor a list line, and for each list line it adds whose `<file>` at the head holds no test of that name with a marker on its lines (`check.coverage/test-lists`).

#### Scenario: a removed requirement
- GIVEN component `a` with requirement `a/x`, whose one scenario is `first`, and a branch that only removes `a/x` from `specs/a/SPEC.md`
- WHEN `clm gate --base main` runs, and again with `--approved`
- THEN both print `spec review` and then `specs/a/SPEC.md: gate/cut: requirement a/x is gone`; the first exits 1 with `gate/approval` and the second exits 0

#### Scenario: a removed scenario
- GIVEN `a/x` with scenarios `first` and `second`, and a branch that only removes `second`
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and `specs/a/SPEC.md: gate/cut: scenario "second" of a/x is gone`

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

#### Scenario: from two graphs
- GIVEN the graphs of *a removed scenario* loaded from texts, and no lists
- WHEN `cuts(base, head, &[])` runs
- THEN it returns one gone scenario, `second` of `a/x`

#### Serves
- architecture/removal-needs-a-yes
- capability/approve-intent

#### Needs
- check.coverage/test-lists

## MODIFIED Requirements

### Requirement: needs-review <!-- base: 48a7ac7663b01e08 -->
A `spec` branch SHALL need review exactly when its diff adds, modifies or deletes `ARCHITECTURE.md`, `CAPABILITIES.md` or `docs/allowed_signers` (the keys a signed repository trusts, `signed-ledger-records`); or `approval-setting` counts `clm.toml` as changed; or a change one of whose files it adds or modifies, a live change or a new archived one, has a `change.md` at `HEAD` that can't be read as UTF-8, whose frontmatter doesn't parse, or that declares a non-empty `human_burdens` (`yes-no-review`), so editing a change's deltas or assets alone doesn't leave its burdens unreviewed; or `cuts` returns anything for the branch (`cuts`), so a requirement, a scenario or a sentence it removes, or a test it lists to delete, is always seen by a human (`removal-needs-a-yes`); or a component's `SPEC.md` at `HEAD` holds an exemption (`model/exemptions`) whose `files` aren't all among the `files` of an exemption with the same `rule` and `until` that the component with the same ID holds at the merge base (every exemption of a component new on the branch counting), so relaxing a rule, or widening a relaxation, is always seen by a human, while removing or narrowing one, or changing only its `why`, never needs review (`temporary-exemptions`). A `spec` branch that `rename/replay` accepts SHALL instead need review exactly when its added change records a `term` rename, declares a non-empty `human_burdens`, or has a `change.md` that can't be read or parsed: its replayed edits to `ARCHITECTURE.md` and `CAPABILITIES.md` SHALL NOT count. Nothing else SHALL make a branch need review, and a `code` or `mixed` branch never does. `clm gate` SHALL print `spec review` for a `spec` branch that needs review and `spec` for one that doesn't.

#### Scenario: architecture
- GIVEN a branch that only edits `ARCHITECTURE.md`
- WHEN `clm gate --base main --approved` runs
- THEN it prints `spec review` and exits 0

#### Scenario: trusted keys
- GIVEN `main` with `docs/allowed_signers`, and a branch that only adds a key to it
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exits 1

#### Scenario: capabilities added
- GIVEN `main` without `CAPABILITIES.md`, and a branch that only adds it
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exits 1

#### Scenario: burdens
- GIVEN a branch that only adds `changes/x/change.md`, `proposed` with `human_burdens: [a paid model run on every land]`
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exits 1

#### Scenario: deltas of a change with burdens
- GIVEN `main` with `changes/x/change.md`, `applied` with `human_burdens: [b]`, and `changes/x/deltas/a.md`, and a branch that only edits that delta
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exits 1

#### Scenario: change that doesn't parse
- GIVEN a branch that only adds `changes/x/change.md` whose frontmatter isn't valid YAML
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exits 1

#### Scenario: nothing else
- GIVEN a branch that changes the Contract of `specs/a/SPEC.md`, rewords one of `a`'s requirements, keeping its scenarios and the number of its stating sentences, adds `docs/decisions/d.md` and adds `changes/x/change.md`, `applied` with no `human_burdens`
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and exits 0

#### Scenario: a cut
- GIVEN a branch that only removes one of `a`'s requirements from `specs/a/SPEC.md`
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exits 1, and with `--approved` it prints `spec review` and exits 0

#### Scenario: rename of IDs through the human layers
- GIVEN a branch holding only the committed result of `clm rename r`, whose `renames` hold one `requirement` rename that rewrote a line of `ARCHITECTURE.md`'s `#### Enforced by` and a marker in a test
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and exits 0

#### Scenario: term rename
- GIVEN a branch holding only the committed result of `clm rename r`, whose `renames` hold one `term` rename
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exits 1

#### Scenario: an added exemption
- GIVEN a `spec` branch whose only change adds an exemption to `specs/a/SPEC.md`'s frontmatter
- WHEN the gate runs
- THEN it prints `spec review`

#### Scenario: a narrowed exemption
- GIVEN a `spec` branch whose only change removes one of the globs an exemption in `specs/a/SPEC.md` lists
- WHEN the gate runs
- THEN it prints `spec`

#### Scenario: a removed exemption
- GIVEN a `spec` branch whose only change removes an exemption from `specs/a/SPEC.md`'s frontmatter
- WHEN the gate runs
- THEN it prints `spec`

#### Needs
- model/exemptions
- rename/replay
- model/parse-changes

### Requirement: needs-review-of <!-- base: 988ab216ed2f89de -->
`clm-gate` SHALL offer `needs_review_of(paths, approval_changed, changes)`, where `paths` are the paths a branch's diff adds, modifies or deletes, `approval_changed` is whether `approval-setting` counts `clm.toml` as changed, and `changes` holds, for each change one of whose files the branch adds or modifies, its `change.md` frontmatter at `HEAD`, or `None` when that file is missing, isn't UTF-8 or its frontmatter doesn't parse. It SHALL return whether a `spec` branch with that diff needs review by the part of `needs-review`'s rule that the paths and the changes decide: true exactly when a path is `ARCHITECTURE.md`, `CAPABILITIES.md` or `docs/allowed_signers`, or a path under one of them, or `approval_changed` is true, or an entry of `changes` is `None` or declares a non-empty `human_burdens`. It SHALL run no process, SHALL build for `wasm32-unknown-unknown` (proven as `approval_changed`'s build is, by the wasm build of `clm-web` the checks run), and `clm-gate` SHALL offer beside it `widened_exemptions(base, head)`, over the spec graphs (`clm_model::Graph`) at the merge base and at the head, returning each exemption a component of `head` holds whose `files` aren't all among the `files` of an exemption with the same `rule` and `until` that the component of that ID holds in `base`, built and proven for wasm the same way. `evaluate_at` SHALL decide a `spec` branch's review as `needs_review_of`, or `widened_exemptions` returning anything, or `cuts` returning anything (`cuts`), and by nothing else, so the gate and `web.worker`, which calls the same three over the files it fetches, give one answer, except for a branch `rename/replay` accepts, whose review `evaluate_at` decides by `needs-review`'s rule for such a branch: replaying needs the base's whole tree, which the Worker doesn't have, so the Worker's answer for a rename branch can say it needs review when the gate says it doesn't, and never the reverse. `change_needs_review` stays public only while a crate outside `clm-gate` calls it (`no-dead-code`).

#### Scenario: a human layer
- GIVEN `paths` `["ARCHITECTURE.md"]`, `approval_changed` false and no `changes`
- WHEN `needs_review_of` runs
- THEN it returns true, as it does for `["docs/allowed_signers"]`, and for no paths with `approval_changed` true

#### Scenario: burdens and an unreadable change
- GIVEN `paths` `["changes/x/change.md"]`, `approval_changed` false, and `changes` holding one frontmatter with `human_burdens: [b]`
- WHEN `needs_review_of` runs
- THEN it returns true, as it does when `changes` holds `None` instead

#### Scenario: nothing else
- GIVEN `paths` `["specs/a/SPEC.md", "docs/decisions/d.md", "CLAUDE.md", "changes/x/change.md"]`, `approval_changed` false, and `changes` holding one frontmatter with no `human_burdens`
- WHEN `needs_review_of` runs
- THEN it returns false

#### Scenario: an exemption, from two graphs
- GIVEN a graph `base` whose component `a` holds an exemption from `undeclared-imports` for `tests/a/**` until `backlog/r`, and a graph `head` in which that exemption's `files` also list `tests/b/**`
- WHEN `widened_exemptions(base, head)` runs, and again with the two graphs exchanged
- THEN the first returns that exemption and the second returns nothing

#### Serves
- capability/approve-intent
- capability/follow-from-browser
- architecture/humans-review-intent

#### Needs
- model/parse-changes
- model/exemptions
