---
component: gate
base: { contract: "35840f7388520c48", design: "1f22fcd7d80663df" }
---

## MODIFIED Contract
`clm gate --base <ref> [--approved]` decides whether `HEAD` may merge into `<ref>`. Its first line is the branch's kind, `spec review` for a spec branch that needs a human's review, `spec` for one that doesn't, `code` or `mixed`, then any violations in `check`'s format, and it exits 0 to allow or 1 to block. `--approved` says the ledger holds an approval of this diff that the repository's policy accepts; `clm land` computes it from the fold (`land-locally`), and the gate itself reads no ledger and no host.

`clm fingerprint`, the fingerprint an approval is pinned to, is the child `gate.fingerprint`'s (crate `clm-fingerprint`). `clm-gate` re-exports its `Entry`, `parse_ls_tree`, `fingerprint_of`, `adds_change`, `added_changes` and `fingerprint`, so `land`, `ledger`, `web` and `crates/clm` call them as `clm_gate::...` (`fingerprint-reexported`).

`clm-gate` also offers `settings_unchanged(before, after)`: given `clm.toml`'s bytes on each side of a diff (`Option<&[u8]>`, `None` where the file doesn't exist), whether its settings are the same once the upgrade notes `upgraded` and `declined_migrations`, comments and formatting are set aside, false when either side isn't UTF-8 TOML. `land` reads it to count a land whose `clm.toml` changed only so as checked (`land/settings-unchanged-is-checked`).

`clm gate` lists what a spec branch cuts, one `gate/cut` line each after the kind line: a requirement that is gone, a scenario that is gone, a scenario whose outcome changes under its name, a requirement that states fewer sentences, and a test a change's `deleted-tests.txt` lists to delete (`cuts`, `removal-needs-a-yes`). The lines change no exit status; a spec branch with any needs review. `clm-gate` offers the list as `cuts(base, head, lists)` over two spec graphs, with `widened_exemptions(base, head)`, `stating_sentences(statement)` and `places_a_test(sentence)`, all building for wasm, `list_lines(text)`, and `cuts_or_unloaded(base, head, lists)`, each side a `Side` holding its graph, the violations its loading gave and its revision, which returns `cuts`' result or, when a side didn't load, the one line that says so and the tests the lists add, for `web.worker.review`, `host`, `check.health`, `land` and `orchestrator.run`; each cut prints as its line, and `evaluate_at`'s result holds the cuts. A `deleted-tests.txt` line that names no citing test is the violation `gate/deleted-tests`.

`clm-gate` also offers `foreign_files(repo, base, head, component)`: the code files `head` changes against `base` that aren't `component`'s build's to change, each with its owner, none when the root at the merge base sets no `wiring` (`own-files`). The gate itself reads no branch name, so `clm gate` doesn't report them: `clm land`, which knows the branch, applies the rule (`land/build-holds-own-files`), as it applies `files-outside-spec-paths` to a spec branch.

`clm-gate` offers `clm gate` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

`clm-gate` also offers `RULES`, the version of the rules by which it decides a branch's kind and whether it needs review, and `evaluate_by(repo, base, head, approved, rules)`, the gate as it decided under the rules of version `rules`, which `evaluate_at` is with `RULES` (`rules-version`). `land` names in each merge commit the version it judged the land by (`land/gate-rules-trailer`), and `ledger`'s `verify --review` recomputes each land under the version in force when it landed (`audit-by-the-rules-in-force`).

## MODIFIED Design
Changes come from `git diff --name-status --no-renames <merge-base>...HEAD`. Spec paths are `specs/`, `changes/`, `backlog/`, `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and `AGENTS.md`. File contents at the merge base and at `HEAD` are compared with `git diff <rev>:<path> <rev>:<path>`. Uses the `git` CLI, not a git library.

**Review is yes or no** (`yes-no-review`). A spec branch needs a human's review only for what a program can't judge for the human: the two human layers, `ARCHITECTURE.md` and `CAPABILITIES.md`, edited whole in the branch; the keys a signed repository trusts, `docs/allowed_signers` (any entry at or under that path, never counted as a status-only edit; `signed-ledger-records`); whose approval counts (`approval-setting`); and a change that declares a burden, something that spends the human's money or time or can't be undone. Everything else, component specs that cut nothing, decisions, backlog edits and changes with no burdens, lands on checks, and nothing is reviewed after the fact. A spec branch also needs review for what it cuts, which a program lists for the human (**Cuts**, below). Apart from the cuts and the exemptions, which are read from the two spec graphs, whether a branch needs review is read from the diff's paths, `clm.toml` at both sides and the `change.md` texts at `HEAD`. A `change.md` that can't be read or parsed needs review, so the gate fails closed; a live one that holds `gate` parses (the model sinks the key) and `clm check` rejects it (`model/parse-changes`). `undeclared_burdens` checks what a program can see of a burden: a `kind: human-task` item the branch adds is a step only the human can take, so a spec branch that adds one must say so in `human_burdens`, which then makes it need review. Builds file `question` and `human-task` items freely; those block only their own requirements.

**Rename branches** (`rename-by-replay`). A branch whose whole diff `rename/replay` reproduces, from the merge base and the renames its one added change records, is classified `spec` though it rewrites test markers in code files, since every edit follows from the pairs a human or the gate can read. Its edits to `ARCHITECTURE.md` and `CAPABILITIES.md` are replayed renames of IDs, which change no intent, so they don't make it need review; a `term` pair does, since re-citing a reworded requirement skips the build that would confirm its tests, and only a human can judge that a swapped word keeps a rule's meaning (`hash-based-staleness`). Its `human_burdens`, and a `change.md` that doesn't parse, count as for any change. `classify` asks `clm_rename::replays` only when the diff holds code files and the branch adds exactly one change whose `change.md` at `HEAD` sets `renames`, so no other branch pays for the extraction; when it answers no, the branch is classified by its paths. `needs_review_of` stays the path rule the Worker shares; `evaluate_at` uses it, with `widened_exemptions` and `cuts`, for every `spec` branch but a replayed one, whose review is its own rule and whose cuts are still listed.

The gate loads `specs/` at the merge base only for the code-fix check's component lookup (`code-fix-resolves-bug`), with `clm_model::load`, extracted into a temporary directory with `clm-git`'s `extract_paths`, so each file holds the bytes a checkout of that commit writes (`extract-as-checkout`, `base-specs-as-checkout`). The paths passed are `specs`, when it exists at the merge base, and each path of the form `changes/<id>/change.md` that `git ls-tree -r --name-only <rev> -- changes` lists, since `extract_paths` takes its paths literally and errors on one that matches nothing. The live changes' `change.md` files carry the component frontmatter keys they declare in `frontmatter_keys`, so a component that sets one loads as `clm check` loads it (`declared-keys-loaded`, `declared-frontmatter-keys`). Their deltas and assets aren't extracted, since nothing the gate reads from the loaded graph needs them.

A *change the PR adds* is a live `changes/<id>/` directory, or a *new archived change*: `changes/archive/<id>/` with `change.md` among the files the PR adds there other than by an archive move (`archive-move`). The second is a change that needs nothing built, such as a design-only one, marked `done` and archived in its own spec PR (`archive-in-spec-pr`). Since `archive-immutable` forbids editing anything under `changes/archive/`, the gate treats a new archived change's files as it treats a live change's: its `human_burdens` count for `needs-review` and `undeclared-burden`, and its `resolves` is checked by `resolves-named-items`. The PR adds the whole of such a `change.md`, so every one of its `resolves` entries is new and tested, as for a live `change.md` the PR adds. The exception is a live change moved into the archive with more than `status:` lines edited, which isn't an archive move: its entries are compared with the live `changes/<id>/change.md` at the merge base, so items a task deleted (`task-deletes-fixed-items`) aren't reported.

A code PR records its progress in its tests' markers, `req:<id>@<version>` (`versioned-markers`), and nothing under `changes/` but an archive move (`archive-move`) when `clm rebase` archived a complete change on its branch (`archive-on-sync`).

**Code fixes** (`code-fix-resolves-bug`). A code PR may delete the backlog item of a bug it fixes, or of deferred work it does, when it changes no spec (`deferred-work-resolves-in-code`). A commit names the item with the trailer `Resolves: backlog/<slug>`, read with `git log --format=%(trailers:key=Resolves,valueonly) <merge-base>..HEAD`, one slug per value. Values are trimmed, and only those of the exact form `backlog/<slug>` count; a value naming an item the PR doesn't delete is ignored. The deletion is exempt when `backlog/<slug>.md` at the merge base has `kind: bug` or `kind: deferred` and its `components` name a component, in `specs/` at the merge base, that a code file (one outside the spec paths) the PR adds, modifies or deletes belongs to. A file belongs to a component when one of the component's `paths` matches it, or when its text at the merge base or at `HEAD` holds `req:<component>/`, the start of a marker citing one of the component's requirements (`fixes-count-where-cited`). The second way is for a test that lives in another component's `paths`, as a project's tests do until it gives them to their components: a fix to one of them belongs to the component it cites, as `tests-count-where-cited` counts it toward that component's size. It says whose fix it is, not whose file: `clm land` still holds the branch to its own files (`land/build-holds-own-files`). Either version counts, so a fix that removes or adds a file's last marker belongs where the file did or does. The text is searched as is, with no marker parsing: `req:plugin/` doesn't occur in `req:plugin.hooks/x`, and a fixture that builds its marker from pieces cites nothing. `kind` and `components` are read from the item's frontmatter alone, so a key the model rejects can't hide them; an item whose frontmatter doesn't parse, or none of whose components exist at the merge base, isn't exempt; a named component missing there counts for neither `paths` nor markers, and the others count. A component with no `paths` can still qualify through markers. A file whose text isn't valid UTF-8 is searched as bytes, and one that can't be read on a side holds no marker on that side; neither makes the gate fail. The search runs only when a trailer names an item the PR deletes. Only the item's own components count, not their parents or children, since `paths` are disjoint and a marker names one component. A path matches as a git `:(glob)` pathspec, as `clm size` matches `paths`, tested against the PR's diff (`git diff --name-only <merge-base> HEAD -- ':(glob)<path>'`) so a deleted file matches too. Unlike `resolves`, which a change must have merged before the branch deletes the item, the branch here both names and deletes it. What bounds it is that deleting a bug or deferred item needs no review anyway, and that the PR's code must touch a component the item names, by its `paths` or by a test that proves it.

**Approval.** The host plays no part in review (`per-writer-ledgers`). `--approved` is the flag, and it is `clm land`'s word: the gate doesn't know who approved or under which policy, which keeps it a pure function of the two trees and one flag, and keeps every caller's verdict the same. The `gate/approval` message names `clm approve <branch>`, so a session knows what to ask the human for.

**Fingerprint.** `gate.fingerprint` holds it (`tree-diff-fingerprint`); `clm-gate` keeps only the re-export (`fingerprint-reexported`). The gate itself computes no fingerprint.

**Where the gate's tests live.** All of `gate`'s tests are its crate's, under `crates/clm-gate/tests/`; those that proved a rule by running the `clm` binary call the crate's own command (`commands-in-crate`).

**Cuts** (`cuts`, `cuts-are-what-a-program-lists`). `removal-needs-a-yes` was the session's word alone. The gate now lists what a program can see of a removal and asks review for it, reading the requirements at the merge base and at the head from `specs/`, where `clm apply` has already put the branch's deltas, so it reads no delta. Both graphs are extracted as the code-fix check extracts the base's, the head's from `head`'s tree, with `ARCHITECTURE.md` and `CAPABILITIES.md` beside `specs/` and the live `change.md` files, since a rule's `serves` names their entries and a graph without them loads with a violation for every such rule, and only for a `spec` branch whose diff touches `specs/` or a `deleted-tests.txt`. A scenario's name is the text of its `#### Scenario:` heading in the requirement's block. A name is matched within the diff and one for one, since many requirements share a name such as "not switched on": matching against the whole head would never list one. So a rename is listed, as a program can't tell it from a removal, and a session keeps a scenario's name when it rewords or moves it. For the same reason a scenario that keeps its name and changes its outcome, its lines from `- THEN` on, is listed: a program can't tell an outcome reworded from one reversed, and a reversed outcome under a kept name drops behaviour as a removal does, with no name gone and no sentence lost (`a-changed-outcome-is-a-cut`). Whitespace is read as one space, so a reflowed line is not a change. Only a scenario held under the same requirement on both sides is compared; a moved one is not. Counting sentences that hold `SHALL` sees a requirement that states less and not one that states something else in as many sentences, which stays a reader's. The placement phrases are a constant of the crate, changed by a code PR. The tests a change deletes are declared, not computed: `clm coverage --obsolete-tests` lists every test of a merged requirement, which a build re-cites, and misses a duplicate whose requirement stays. A spec graph that doesn't load at either side can't be compared, so the branch needs review and one `gate/cut` line says so. The exemptions that make a branch need review are now read from the two graphs, by `widened_exemptions`, which the Worker calls too; `exemption-names-its-item` keeps reading the frontmatter as text, so an exemption the model rejects is still held to its item. The gate checks only the lines a branch adds, so a list whose tests a build has since deleted stays valid, and `clm land` holds a code branch to the lists on its base (`land/deleted-tests-approved`).

A branch behind `<ref>` may land (`land-locally`). `clm land` merges `<ref>` into it and checks the result before anything lands. So the gate blocks only what that merge can't settle. `git merge-tree --write-tree <ref> HEAD` exits 1 on a textual conflict. A file under `specs/` that changed on both sides since the merge base is also blocked: `specs/` is derived, so it is regenerated by `clm rebase` and never merged (`regenerate-dont-merge`), even when the text would merge cleanly. Both report rule `gate/up-to-date` and tell the author to run `clm rebase`.

**Evaluating another commit** (`gate/evaluate-at`). `evaluate_at(repo, base, head, approved)` is the gate for `head` against `base` as `clm gate --base <base>` gives it with `head` checked out: it reads `head` only as a revision, through `git diff <merge-base>...<head>`, `git show <head>:<path>`, `git ls-tree <head>`, `git log <merge-base>..<head>` and `git merge-tree --write-tree <base> <head>`, never the working tree or the index, so it gives the same verdict whatever is checked out. A violation's path names the range as `<base>..<head>` with the revisions as given, so `clm gate`'s output stays `<base>..HEAD`. `clm gate` calls it with `HEAD`. `ledger`'s `verify --review` calls `evaluate_by` for each signed land, with the land's first parent as `base`, the merge commit as `head` and the rules in force when it landed, to recompute the land's kind and whether it needed review without a checkout per land.

**A build's own files** (`own-files`, `own-files-at-the-gate`). `path-ownership` was kept by the build prompt alone; a program now refuses what the prompt asked for. The gate stays a function of two trees and one flag and reads no branch name (`spec-branches-hold-spec-paths`), so `foreign_files` takes the component as an argument and `clm land`, which knows the branch, calls it for a branch named `impl/<component>`: no other branch has a component, and a small change on a branch of another name is not judged. The specs it reads are the merge base's, loaded as for the code-fix check; a file's owner is `check.coverage`'s `owner`, which the gate depends on for it. Two kinds of file are the build's to change beside its own. The root's `wiring` names the shared ones. A file that merely cites the component is not one of them: a test sits in the files of the component it proves, so a marker gives a build no file (`commands-live-in-their-crates`); the clause that once opened such a file, for the tests Cargo kept in the root package's directory, went when each crate came to prove its requirements through its own call. A capability test may be edited, never added or deleted, by a build whose component serves the capability it cites, since a behaviour and the test that observes it at the boundary can't land apart (`capability-tests-follow-rules`). That a wiring edit adds only the build's own entry, and that a capability-test edit keeps what its scenarios state, stay judgements no program makes. The rule is off until the root sets `wiring`, as `check`'s path ownership is off until it sets `outside`, so a project adopts it when it has named its wiring.

**Rules versions** (`rules-version`, `audit-by-the-rules-in-force`). The audit of a signed land asks whether its record understates what the gate gave it, so it must ask the gate that judged it, not today's: a rule that makes more diffs need review would otherwise report every honest land made before it as understated and unapproved. `RULES` names the rules `evaluate_at` applies, and `evaluate_by` keeps each earlier version's answer. Version 1 is the gate as it stood before `cuts` made a branch need review; version 2 adds `cuts`; version 3 adds the changed outcome to what makes a branch need review. Only the review depends on the version, and with it `gate/approval`: the kind, the cuts and the other violations are those of today's gate at every version, since the audit reads only the kind and the review. A requirement can't name its own component in `#### Needs`, so `rules-version` relies on `cuts` by the build of `gate` taking both. A change that makes the gate give some diff a higher kind or review than it did (`code` = `spec` < `spec review` < `mixed`) raises `RULES` by one and keeps the earlier answer behind `rules` for the lower versions, for as long as signed history may hold a land judged by them, which is always; a change that rates diffs lower or the same needs no new version, since the audit reports only understatement. The older answers are a branch on `rules` beside the rule that changed, not a copy of the gate. A `rules` below 1 or above `RULES` is an error naming both, which `ledger` reports as a land it can't rate (`ledger/verify-unrated-land`): a newer `clm` judged it.

## MODIFIED Requirements

### Requirement: cuts <!-- base: 02dbf77fa45080a2 -->
`clm-gate` SHALL offer `cuts(base, head, lists)`, running no process and building for `wasm32-unknown-unknown` (proven as `approval_changed`'s build is), where `base` and `head` are the spec graphs (`clm_model::Graph`) at a branch's merge base and at its head, each loaded from that side's `specs/`, `ARCHITECTURE.md`, `CAPABILITIES.md` and live `changes/<id>/change.md` files, so a rule's `serves` resolves as `clm check` resolves it, and `lists` holds, for each change with a `deleted-tests.txt` at the head, `changes/<id>/deleted-tests.txt` or `changes/archive/<id>/deleted-tests.txt`, that path and the file's text at the head and at the merge base, where an archived list with no text of its own there takes the text of the live `changes/<id>/deleted-tests.txt`, so an archive move adds no line. It SHALL return what the branch cuts, each a `Cut` that prints as its line below, in this order, the first four kinds each by requirement ID and the second and third also by the scenario's place in its requirement:
- a *gone requirement*: one `base` holds and `head` doesn't, unless each of its scenarios is moved;
- a *gone scenario*: a scenario of a requirement both graphs hold whose name the head's requirement doesn't hold, unless it is moved;
- a *changed outcome*: a scenario of a requirement both graphs hold whose name the head's requirement holds, whose outcome differs from that of the head's scenario of that name, the n-th scenario of a name at the base compared with the n-th of that name at the head, and one with no n-th of its name at the head compared with an empty outcome, so a duplicate the head loses is listed; changed outcomes are ordered by the scenario's place at the base;
- a *shortened requirement*: one both graphs hold whose statement, its text above its first `####` heading, has fewer stating sentences at the head than at the base, with both counts;
- a *test to delete*: each list line the head's text holds and the base's text doesn't, with its path, in the paths' order and then the file's.

A scenario's name is the text after `#### Scenario: ` in its requirement's block. Its *outcome* is the text of its lines from the first whose text, leading white space left out, starts `- THEN`, case kept, to the end of its block, the next `####` heading outside a fenced code block or the requirement's end, a fenced code block read as any other lines, with each run of Unicode white space, a line break included, read as one space and none at either end; a scenario with no such line has an empty outcome. A scenario is *moved* when `head` holds a scenario of the same name under a requirement of another ID that held no scenario of that name in `base`; each such scenario of `head` SHALL take one gone scenario and no more, the gone scenarios taken in the order of their requirement IDs and then of their place. A *sentence* ends at a `.`, `?` or `!` that whitespace, a line break included, or the statement's end follows. A *stating sentence* is one that holds `SHALL` as a whole word in capitals, as the spec format writes it, and none of the placement phrases `SHALL sit`, `sat in` and `is left in`, each matched as whole words separated by single spaces and without regard to case, so `is left intact` holds none and a phrase broken across a line is not found; `clm-gate` SHALL offer `stating_sentences(statement)`, returning them, and `places_a_test(sentence)`, returning the first placement phrase a sentence holds. A *list line* is `DELETE <file>::<name>`, the name ending at a tab or at the line's end; a blank line and a line starting `#` are skipped; `clm-gate` SHALL offer `list_lines(text)`, returning each list line's file and name. A list line names a test when the files agree and the names agree as `check.coverage`'s `bare_name` gives them.

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

### Requirement: rules-version <!-- base: 494cf9a03d1be2c5 -->
`clm-gate` SHALL offer `RULES`, a whole number that SHALL be 3, and `evaluate_by(repo, base, head, approved, rules)`, returning what `evaluate_at` returns as the gate decided under the rules of version `rules`, and `evaluate_at` SHALL be `evaluate_by` with `RULES`. Under version 3 a `spec` branch SHALL need review as `needs-review` states, under version 2 as it states with a *changed outcome* cut making no branch need review, and under version 1 as it states with `cuts` making no branch need review, specs that don't load at the merge base or the head included, a branch `rename/replay` accepts keeping its own rule under each. The kind and the cuts SHALL be the same under every version, and the violations too but for `gate/approval`, which follows the review. For a `rules` below 1 or above `RULES`, `evaluate_by` SHALL return an error naming `rules` and `RULES`.

#### Serves
- architecture/signed-ledger-records
- architecture/humans-review-intent

#### Needs
- check.coverage/test-lists

#### Scenario: a removal under each version
- GIVEN component `a` with requirement `a/x`, and a branch `s` that only removes `a/x` from `specs/a/SPEC.md`
- WHEN `evaluate_by(repo, "main", s, false, 1)` and `evaluate_by(repo, "main", s, false, RULES)` run
- THEN the first gives `spec` without review and the second `spec` with review, and each holds the cut `requirement a/x is gone`

#### Scenario: a changed outcome under each version
- GIVEN component `a` with requirement `a/x` whose one scenario, `first`, ends `- THEN it exits 1`, and a branch `s` that only changes that line to `- THEN it lands`
- WHEN `evaluate_by(repo, "main", s, false, 2)` and `evaluate_by(repo, "main", s, false, RULES)` run
- THEN the first gives `spec` without review and the second `spec` with review, and each holds the cut `scenario "first" of a/x changes its outcome`

#### Scenario: the same verdict under both
- GIVEN a branch that only edits `ARCHITECTURE.md`
- WHEN `evaluate_by` runs on it with `1` and with `RULES`
- THEN both give `spec` with review

#### Scenario: a version this gate doesn't know
- GIVEN any branch
- WHEN `evaluate_by` runs on it with `RULES + 1`, and again with `0`
- THEN each returns an error whose message names the version asked for and `RULES`
