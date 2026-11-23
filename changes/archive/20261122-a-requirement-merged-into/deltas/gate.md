---
component: gate
base: { design: "5852d201ddc7c8d4" }
---

## MODIFIED Design
Changes come from `git diff --name-status --no-renames <merge-base>...HEAD`, run in the CLM root. **Paths under the root** (`paths-under-the-root`, `fingerprint-covers-the-clm-root`). `git diff` prints each path relative to the git repository's top, whatever directory it runs in, while every rule the gate applies names a path relative to the CLM root, as the fingerprint an approval is pinned to does. So the gate turns each path any `git diff` prints, filtered by a pathspec or not, into one relative to the root, with `git rev-parse --show-prefix` giving the root's place in the repository: a path under the root loses that prefix, and a path outside it becomes the shortest path from the root to it, climbing out with `../` (`../x` for `a/x` with the root at `a/b/`). Every later read names a path in that form: a pathspec as it stands, since git reads pathspecs relative to the directory it runs in, and a file at a revision as `<rev>:./<path>`, which git reads the same way, never as `<rev>:<path>`, which git reads from the repository's top. `clm-git`'s `extract_paths` is the exception: it takes paths relative to the repository's top and refuses `..`, so the graph loads pass the prefix before each root path (`<prefix>specs`, `<prefix>changes/<id>/change.md`) and load the graph from the prefix's directory in the extraction. A file outside the root is no component's, whatever a component's `paths` would match, since `paths` name files under the root. A path outside the root is never a spec path, so it counts as code; that is what keeps `approval-policy` whole for a subdirectory root, whose approvals cover only the root's files. At a repository whose root is its top the prefix is empty and nothing changes. File contents at the merge base and at `HEAD` are compared with `git diff <rev>:./<path> <rev>:./<path>`. Uses the `git` CLI, not a git library.

**The routes onto `main`.** Every land takes one of three routes, and the gate tells which from the files alone: a branch's name is a claim, and no rule rests on it. *Spec*: a branch with no code file is a spec branch, a program deciding whether it needs review; one whose files are all exempt, such as a backlog item filed alone, an archive move or no change at all, is a spec branch that needs nothing. *Code*: a branch with code files and no spec change is one component's work, read from the files it changes (**A branch's component**, below). *Light*: spec and code together are refused, except for a branch whose spec part only adds scenarios to requirements of the component whose code it changes, in one change it archives, and which leaves those requirements proven (**Light branches**, below). A build named `impl/<component>` is held to its claim by `clm land`, which refuses it when `foreign_files` for that component returns anything (`land/build-holds-own-files`); the component `evaluate_at` returns says whose work a branch is, for `sync` and the records, and is not the claim's test, since a build may edit a capability test its component serves, a file another component owns.

**Review is yes or no** (`yes-no-review`). A spec branch needs a human's review only for what a program can't judge for the human: the two human layers, `ARCHITECTURE.md` and `CAPABILITIES.md`, edited whole in the branch; the keys a signed repository trusts, `docs/allowed_signers` (`signed-ledger-records`); whose approval counts (`approval-setting`); and a change that declares a burden, something that spends the human's money or time or can't be undone. Everything else, component specs that cut nothing, decisions, backlog edits and changes with no burdens, lands on checks, and nothing is reviewed after the fact. A spec branch also needs review for what it cuts, which a program lists for the human (**Cuts**, below). Apart from the cuts and the exemptions, which are read from the two spec graphs, whether a branch needs review is read from the diff's paths, `clm.toml` at both sides and the `change.md` texts at `HEAD`. A `change.md` that is missing, can't be read or doesn't parse needs review, so the gate fails closed; a live one that holds `gate` parses (the model sinks the key) and `clm check` rejects it (`model/parse-changes`). `undeclared_burdens` checks what a program can see of a burden: a `kind: human-task` item the branch adds is a step only the human can take, so a spec branch with a spec change that adds one must say so in `human_burdens`, which then makes it need review. Builds file `question` and `human-task` items freely; those block only their own requirements, and a branch that only files them is held to nothing.

**Rename branches** (`rename-by-replay`). A branch whose whole diff `rename/replay` reproduces, from the merge base and the renames its one added change records, is classified `spec` though it rewrites test markers in code files, since every edit follows from the pairs a human or the gate can read. Its edits to `ARCHITECTURE.md` and `CAPABILITIES.md` are replayed renames of IDs, which change no intent, so they don't make it need review; a `term` pair does, since re-citing a reworded requirement skips the build that would confirm its tests, and only a human can judge that a swapped word keeps a rule's meaning (`hash-based-staleness`). Its `human_burdens`, and a `change.md` that doesn't parse, count as for any change. `classify` asks `clm_rename::replays` only when the branch adds exactly one change whose `change.md` at `HEAD` sets `renames`, so no other branch pays for the extraction; when it answers no, the branch is classified by its paths. `needs_review_of` stays the path rule the Worker shares; `evaluate_at` uses it, with `widened_exemptions` and `cuts`, for every `spec` branch but a replayed one, whose review is its own rule and whose cuts are still listed.

**A commit's components.** The gate loads the components of a commit for three reads: the merge base's, for the code-fix check's component lookup (`code-fix-resolves-bug`) and for a branch's component (`own-files`); both sides', for the cuts and the light route. It loads with `clm_model::load`, from a temporary directory `clm-git`'s `extract_paths` fills, so each file holds the bytes a checkout of that commit writes (`extract-as-checkout`, `base-specs-as-checkout`). The paths passed are `specs`, when it exists at the commit, and each path of the form `changes/<id>/change.md` that `git ls-tree -r --name-only <rev> -- changes` lists, since `extract_paths` takes its paths literally and errors on one that matches nothing; the cuts and the light route add `ARCHITECTURE.md` and `CAPABILITIES.md`. The live changes' `change.md` files carry the component frontmatter keys they declare in `frontmatter_keys`, so a component that sets one loads as `clm check` loads it (`declared-keys-loaded`, `declared-frontmatter-keys`). Their deltas and assets aren't extracted, since nothing the gate reads from the loaded graph needs them.

A *change the PR adds* is a live `changes/<id>/` directory, or a *new archived change*: `changes/archive/<id>/` with `change.md` among the files the PR adds there other than by an archive move (`archive-move`). The second is a change that needs nothing built, such as a design-only one, marked `done` and archived in its own spec PR (`archive-in-spec-pr`), or the change a light branch carries. Since `archive-immutable` forbids editing anything under `changes/archive/`, the gate treats a new archived change's files as it treats a live change's: its `human_burdens` count for `needs-review` and `undeclared-burden`, and its `resolves` is checked by `resolves-named-items`. The PR adds the whole of such a `change.md`, so every one of its `resolves` entries is new and tested, as for a live `change.md` the PR adds. The exception is a live change moved into the archive with more than `status:` lines edited, which isn't an archive move: its entries are compared with the live `changes/<id>/change.md` at the merge base, so items a task deleted (`task-deletes-fixed-items`) aren't reported.

A code PR records its progress in its tests' markers, `req:<id>@<version>` (`versioned-markers`), and nothing under `changes/` but an archive move (`archive-move`) when `clm rebase` archived a complete change on its branch (`archive-on-sync`).

**Code fixes** (`code-fix-resolves-bug`). A code PR may delete the backlog item of a bug it fixes, or of deferred work it does, when it changes no spec (`deferred-work-resolves-in-code`). A commit names the item with the trailer `Resolves: backlog/<slug>`, read with `git log --format=%(trailers:key=Resolves,valueonly) <merge-base>..HEAD`, one slug per value. A file belongs to a component when one of the component's `paths` matches it, or when its text at the merge base or at `HEAD` holds `req:<component>/`, the start of a marker citing one of the component's requirements (`fixes-count-where-cited`). The second way is for a test that lives in another component's `paths`, as a project's tests do until it gives them to their components: a fix to one of them belongs to the component it cites, as `tests-count-where-cited` counts it toward that component's size. It says whose fix it is, not whose file: `own-files` still holds the branch to one component's files. Either version counts, so a fix that removes or adds a file's last marker belongs where the file did or does. The text is searched as is, with no marker parsing: `req:plugin/` doesn't occur in `req:plugin.hooks/x`, and a fixture that builds its marker from pieces cites nothing. `kind` and `components` are read from the item's frontmatter alone, so a key the model rejects can't hide them. The search runs only when a trailer names an item the PR deletes. Only the item's own components count, not their parents or children, since `paths` are disjoint and a marker names one component. A path matches as a git `:(glob)` pathspec, as `clm size` matches `paths`, tested against the PR's diff (`git diff --name-only <merge-base> HEAD -- ':(glob)<path>'`) so a deleted file matches too. Unlike `resolves`, which a change must have merged before the branch deletes the item, the branch here both names and deletes it. What bounds it is that deleting a bug or deferred item needs no review anyway, and that the PR's code must touch a component the item names, by its `paths` or by a test that proves it.

**Approval.** The host plays no part in review (`per-writer-ledgers`). `--approved` is the flag, and it is `clm land`'s word: the gate doesn't know who approved or under which policy, which keeps it a pure function of the two trees and one flag, and keeps every caller's verdict the same.

**Fingerprint.** `gate.fingerprint` holds it (`tree-diff-fingerprint`); `clm-gate` keeps only the re-export (`fingerprint-reexported`). The gate itself computes no fingerprint.

**Cuts** (`cuts`, `cuts-are-what-a-program-lists`). `removal-needs-a-yes` was the session's word alone. The gate lists what a program can see of a removal and asks review for it, reading the requirements at the merge base and at the head from `specs/`, where `clm apply` has already put the branch's deltas, so it reads no delta. Both graphs are loaded as **A commit's components** says, with `ARCHITECTURE.md` and `CAPABILITIES.md` beside `specs/` and the live `change.md` files, since a rule's `serves` names their entries and a graph without them loads with a violation for every such rule, and only for a `spec` branch with a spec change whose diff touches `specs/` or a `deleted-tests.txt`. A name is matched within the diff and one for one, since many requirements share a name such as "not switched on": matching against the whole head would never list one. So a rename is listed, as a program can't tell it from a removal, and a session keeps a scenario's name when it rewords or moves it. For the same reason a scenario that keeps its name and changes its outcome is listed: a program can't tell an outcome reworded from one reversed, and a reversed outcome under a kept name drops behaviour as a removal does, with no name gone and no sentence lost (`a-changed-outcome-is-a-cut`). Counting sentences that hold `SHALL` sees a requirement that states less and not one that states something else in as many sentences, which stays a reader's. A requirement whose every scenario moves is read as merged, not gone, so its name alone says nothing of what it stated: its sentences are counted with those of the requirements that took its scenarios, and a merge that states fewer in all is listed (`a-merge-is-counted-whole`). A merge is counted whole, every merged requirement and every requirement that took from one, since two requirements merged into a third would each look kept against the third's count alone. A requirement both sides hold that took scenarios is still compared on its own too, so the shortened requirement each earlier rules version asked review for is listed as it was. The placement phrases are a constant of the crate, changed by a code PR. The tests a change deletes are declared, not computed: `clm coverage --obsolete-tests` lists every test of a merged requirement, which a build re-cites, and misses a duplicate whose requirement stays. A spec graph that doesn't load at either side can't be compared, so the branch needs review. The exemptions that make a branch need review are read from the two graphs, by `widened_exemptions`, which the Worker calls too; `exemption-names-its-item` keeps reading the frontmatter as text, so an exemption the model rejects is still held to its item. The gate checks only the lines a branch adds, so a list whose tests a build has since deleted stays valid, and `clm land` holds a code branch to the lists on its base (`land/deleted-tests-approved`).

A branch behind `<ref>` may land (`land-locally`). `clm land` merges `<ref>` into it and checks the result before anything lands. So the gate blocks only what that merge can't settle: a textual conflict, which `git merge-tree --write-tree <ref> HEAD` reports by exiting 1, and a file under `specs/` that changed on both sides since the merge base, since `specs/` is derived, regenerated by `clm rebase` and never merged (`regenerate-dont-merge`), even when the text would merge cleanly.

**Evaluating another commit** (`gate/evaluate-at`). `evaluate_at(repo, base, head, approved)` reads `head` only as a revision, through `git diff <merge-base>...<head>`, `git show <head>:./<path>`, `git ls-tree <head>`, `git log <merge-base>..<head>`, `git grep` at `<head>` and `git merge-tree --write-tree <base> <head>`, never the working tree or the index, so it gives the same verdict whatever is checked out. `clm gate` calls it with `HEAD`. `clm land` calls it in-process before its lock, for the kind its own rules read, and again as `clm gate` on the merge commit, whose tree is what lands; `sync` calls it on each `clm rebase`, for whose work a branch is. `ledger`'s `verify --review` calls `evaluate_by` for each signed land, with the land's first parent as `base`, the merge commit as `head` and the rules in force when it landed, to recompute the land's kind and whether it needed review without a checkout per land.

**A branch's component** (`own-files`, `own-files-at-the-gate`). `path-ownership` was kept by the build prompt alone, and then by `clm land` for a branch named `impl/<component>` only; a small change or a fix on a branch of another name was judged by nothing. The gate now reads the component from the files of every code branch, so the rule needs no name. The specs it reads are the merge base's; a file's owner is `check.coverage`'s `owner`, which the gate depends on for it. The candidates are the owners of the branch's code files that no `wiring` glob matches, and the branch's component is the candidate whose `foreign_files`, files outside the CLM root left out, is empty; a file of one candidate is foreign to every other unless it is a capability test the other serves and the branch only modifies, so the answer is mostly one component or none, and a tie, as between two files that each hold a marker of a capability the other component serves, goes to the first by ID. A file outside the CLM root is no component's, so the gate admits it as it admits any file no component owns; `clm land` still refuses it on a branch named `impl/<component>`, through `foreign_files`. Three kinds of file are a build's to change beside its own. The root's `wiring` names the shared ones. A file that merely cites the component is not one of them: a test sits in the files of the component it proves, so a marker gives a build no file (`commands-live-in-their-crates`). A capability test may be edited, never added or deleted, by a build whose component serves the capability it cites, since a behaviour and the test that observes it at the boundary can't land apart (`capability-tests-follow-rules`). And a build may take out of the root's `clm.toml` a detector's `--allow` entry that its own work makes stale, such as the entry for a public item it narrows, since `check.unreferenced` reports a stale entry and `clm.toml` is another component's file: an edit that only deletes whole lines and, read as TOML, only drops `--allow` pairs from detectors' commands can excuse nothing, since a dropped allowance only makes a detector stricter, and the check of the merged tree judges whether the entry was still needed (`a-build-drops-its-allowance`). That a wiring edit adds only the build's own entry, and that a capability-test edit keeps what its scenarios state, stay judgements no program makes. No rule picks a majority component for a branch that spans two: the gate lists every owned file and lets the author split it. The rule is off until the root sets `wiring`, as `check`'s path ownership is off until it sets `outside`, so a project adopts it when it has named its wiring; a light branch is held to one component's files whatever the root sets, since it is admitted only because it is small.

**Light branches** (`light`). A scenario added to an existing requirement states a case of what the requirement already says, and the test that proves it is the build's; making the two land on separate branches costs a review-free spec land and a build, with the scenario pending between them. So the gate lets them land together when nothing a reader must see is in the spec part: the statement, the Contract, the Design, the frontmatter and the set of requirements stay as they are, nothing is cut, nothing needs review, and the code is the component's own. The change rides the branch already archived, `done`, as a change archived in its own spec PR does, since the branch's own tests make every requirement it touched current: a light land leaves nothing live and nothing for a later build. `clm land` regenerates and compares `specs/` for such a change as for any change a branch adds (`land/derived-specs`), and the gate's run on the merge commit repeats the currency check on the tree that lands, so a `main` that moved between the two runs can't land a requirement behind. Markers are found with `git grep` at the revision, through `clm-git`, and read with `check.coverage`'s marker reading, so the gate counts a citation where `clm coverage` does. A light land is recorded as a code land that says it was light (`ledger/landed-record-fields`), so a `clm` that knows no light route reads it as a code land and, auditing with `--review`, says it can't rate it (a `Gate-Rules` above its `RULES`). The audit of a land recorded light recomputes only what `RULES` versions, whether its spec part needs review and what it cuts, through `spec_part_review` (`ledger/verify-recomputes-review`): ownership, currency and how the model loads are what other components change, and a later change to them must never make an honest light land read as understated.

**Rules versions** (`rules-version`, `audit-by-the-rules-in-force`). The audit of a signed land asks whether its record understates what the gate gave it, so it must ask the gate that judged it, not today's: a rule that makes more diffs need review would otherwise report every honest land made before it as understated and unapproved. `RULES` names the rules `evaluate_at` applies, and `evaluate_by` keeps each earlier version's answer. Version 1 is the gate as it stood before `cuts` made a branch need review; version 2 adds `cuts`; version 3 adds the changed outcome to what makes a branch need review. Version 4 counts a sentence that holds a placement phrase and not the word `test` or `tests` as a stating sentence, so a branch that drops one needs review; the lower versions decide review from a count that leaves out every sentence holding a phrase. Version 5 reads the changed paths relative to the CLM root (**Paths under the root**, above); the lower versions read them as `git diff` prints them, relative to the repository's top, and name a file at a revision as `<rev>:<path>`, as the gate did before, which in a repository whose root is a subdirectory classes a branch editing the root's spec files as `code`. Version 6 matches a placement phrase across any run of white space between its words. Version 7 adds the light route: under the lower versions a light branch is `mixed`, as it was, and a `mixed` line says nothing of the light route, so an older `clm` auditing a light land reads a version it doesn't know and says it can't rate the land (`ledger/verify-unrated-land`) rather than that the land understated a `mixed` diff. Version 8 adds the shortened merge to what makes a branch need review. Versions 1 to 4 read every path as those versions did, pathspecs and graph loads included; in a repository whose root is a subdirectory that changes the kind, either way: a branch editing the root's spec files is `code` under the lower versions and `spec` from 5, and one editing a spec-path name at the repository's top outside the root, such as its own `CLAUDE.md`, is `spec` under the lower versions and `code` from 5. The path reading is one function given the version, not a second gate. Apart from it and the light route only the review depends on the version, and with it `gate/approval`: the kind, the cuts and the other violations are those of the gate of version `RULES` at every version, since the audit reads only the kind and the review. Reading a branch with no code file as `spec` rates no diff higher, since such a branch needs no review, and blocking a branch that spans two components is a violation the audit doesn't rate, so neither needed a version. A requirement can't name its own component in `#### Needs`, so `rules-version` relies on `cuts` and `light` by the build of `gate` taking all three. A change that makes the gate give some diff a higher kind or review than it did (`code` = `spec` = `light` < `spec review` < `mixed`) raises `RULES` by one and keeps the earlier answer behind `rules` for the lower versions, for as long as signed history may hold a land judged by them, which is always; a change that rates diffs lower raises it too when an older `clm` would otherwise call the new answer understated. The older answers are a branch on `rules` beside the rule that changed, not a copy of the gate.

## MODIFIED Requirements

### Requirement: cuts <!-- base: 53fe4864102cfb4d -->
`clm-gate` SHALL offer `cuts(base, head, lists)`, running no process and building for `wasm32-unknown-unknown`, where `base` and `head` are the spec graphs (`clm_model::Graph`) at a branch's merge base and at its head, each loaded from that side's `specs/`, `ARCHITECTURE.md`, `CAPABILITIES.md` and live `changes/<id>/change.md` files, so a rule's `serves` resolves as `clm check` resolves it, and `lists` holds, for each change with a `deleted-tests.txt` at the head, `changes/<id>/deleted-tests.txt` or `changes/archive/<id>/deleted-tests.txt`, that path and the file's text at the head and at the merge base, where an archived list with no text of its own there takes the text of the live `changes/<id>/deleted-tests.txt`, so an archive move adds no line. It SHALL return what the branch cuts, each a `Cut` that prints as its line below, in this order, the first four kinds each by requirement ID and the second and third also by the scenario's place in its requirement, and the fifth by the ID of its first merged requirement:
- a *gone requirement*: one `base` holds and `head` doesn't, unless each of its scenarios is moved;
- a *gone scenario*: a scenario of a requirement both graphs hold whose name the head's requirement doesn't hold, unless it is moved;
- a *changed outcome*: a scenario of a requirement both graphs hold whose name the head's requirement holds, whose outcome differs from that of the head's scenario of that name, the n-th scenario of a name at the base compared with the n-th of that name at the head, and one with no n-th of its name at the head compared with an empty outcome, so a duplicate the head loses is listed; changed outcomes are ordered by the scenario's place at the base;
- a *shortened requirement*: one both graphs hold whose statement, its text above its first `####` heading, has fewer stating sentences at the head than at the base, with both counts;
- a *shortened merge*: a merge whose requirements, all counted together, have fewer stating sentences at the head than at the base, a requirement a side doesn't hold counting none there, with both counts;
- a *test to delete*: each list line the head's text holds and the base's text doesn't, with its path, in the paths' order and then the file's.

A scenario's name is the text after `#### Scenario: ` in its requirement's block. Its *outcome* is the text of its lines from the first whose text, leading white space left out, starts `- THEN`, case kept, to the end of its block, the next `####` heading outside a fenced code block or the requirement's end, a fenced code block read as any other lines, with each run of Unicode white space, a line break included, read as one space and none at either end; a scenario with no such line has an empty outcome. A scenario is *moved* when `head` holds a scenario of the same name under a requirement of another ID that held no scenario of that name in `base`; each such scenario of `head` SHALL take one gone scenario and no more, the gone scenarios taken in the order of their requirement IDs and then of their place, and the scenarios of a name that take them in the same order. The requirement of `head` that holds a scenario that takes a gone scenario *took* it. A *merged requirement* is one `base` holds and `head` doesn't, each of whose scenarios is moved. A *merge* is the smallest set of requirements that holds a merged requirement, each requirement that took one of the scenarios of a merged requirement it holds, and each merged requirement one of whose scenarios a requirement it holds took; so each merged requirement is in one merge. A *sentence* ends at a `.`, `?` or `!` that whitespace, a line break included, or the statement's end follows. The placement phrases are `SHALL sit`, `sat in` and `is left in`, each matched as whole words and without regard to case, with any run of Unicode white space, a line break included, read as the one space between two of its words, so `is left intact` holds none and a sentence with `SHALL` at the end of one line and `sit` at the start of the next holds `SHALL sit`. A sentence *places a test* when it holds one of the placement phrases and also the word `test` or `tests`, matched as a whole word, bounded by characters other than letters, digits and `_`, without regard to case, so `Tests` and the `tests` of `crates/clm/tests/` are the word and `places_a_test` and `tested` hold none; a sentence that holds a phrase and neither word, such as one deleting a directory once nothing else is left in it, places no test. A *stating sentence* is one that holds `SHALL` as a whole word in capitals, as the spec format writes it, and places no test. `clm-gate` SHALL offer `sentences(statement)`, returning every sentence of a statement that isn't empty once trimmed, in order, `stating_sentences(statement)`, returning its stating sentences, and `places_a_test(sentence)`, returning the first placement phrase of a sentence that places a test and nothing for any other sentence. A *list line* is `DELETE <file>::<name>`, the file ending at the first `::` and the name at a tab or at the line's end, neither of them empty, a carriage return that ends the line left out; a blank line and a line whose first character is `#` are skipped, and any other line is neither; `clm-gate` SHALL offer `list_lines(text)`, returning each list line's file and name. A list line the head's text holds whose file and name a list line of the base's text holds adds no *test to delete*, whatever follows its tab, and one the head holds twice is listed once. A list line names a test when the files agree and the names agree as `check.coverage`'s `bare_name` gives them.

For a `spec` branch, `clm gate` SHALL print one line per cut after its first line and before any violation, `evaluate_at` returning the cuts beside its verdict:
- `specs/<component path>/SPEC.md: gate/cut: requirement <id> is gone`;
- `specs/<component path>/SPEC.md: gate/cut: scenario "<name>" of <id> is gone`;
- `specs/<component path>/SPEC.md: gate/cut: scenario "<name>" of <id> changes its outcome`;
- `specs/<component path>/SPEC.md: gate/cut: <id> states <n> sentences where it stated <m>`;
- `specs/<component path>/SPEC.md: gate/cut: <ids> merged into <ids> state <n> sentences where they stated <m>`, the first `<ids>` the merge's merged requirements and the second the requirements of `head` it holds, each in ID order and joined by `, `, and the path that of the first merged requirement's component;
- `<list path>: gate/cut: test <file>::<name> is to be deleted`.

The lines SHALL NOT change its exit status, and a branch `rename/replay` accepts SHALL have its cuts listed as any other. A `spec` branch SHALL be blocked with rule `gate/deleted-tests`, with `--approved` or without, naming the list and the line, for each line its head adds to a list that is neither skipped nor a list line, and for each list line it adds that names no test with a marker on its lines in `<file>` at the head (`check.coverage/test-lists`). The cuts SHALL be read only for a `spec` branch that has a spec change (`classify`) and whose diff changes a path under `specs/` or a `deleted-tests.txt`; any other branch has none. When the graph at the merge base or at the head, loaded from the files named above and no other, loads with any violation, the requirements can't be compared: `clm gate` SHALL print `specs: gate/cut: the specs at <revision> don't load, so what the branch cuts isn't listed`, `<revision>` the commit ID of that side and the merge base's when both fail, then the *test to delete* lines of `lists`, which need no graph, and the branch SHALL need review. `clm-gate` SHALL offer that rule as `cuts_or_unloaded(base, head, lists)`, building for wasm as `cuts` does, `base` and `head` each a `Side`: the graph, the violations its loading gave and the revision as printed. It SHALL return the unloaded cut and then the tests to delete when a side holds a violation, and `cuts`' result otherwise, and SHALL be what `clm gate` lists by, so the gate and `web.worker` list one thing.

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
- GIVEN `a/x` with the one scenario `the chain`, component `b` with requirement `b/y`, and a branch that removes `a/x` and adds a scenario `the chain` to `b/y` and `a/x`'s statement to `b/y`'s
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and no `gate/cut` line

#### Scenario: two requirements merged
- GIVEN `a/x` and `a/y`, each with the one scenario `can edit`, and a branch that removes both and adds `a/z` with one scenario `can edit`, the three stating the same one sentence
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exactly one cut line, `specs/a/SPEC.md: gate/cut: requirement a/y is gone`

#### Scenario: a merge that loses a sentence
- GIVEN `a/x` whose statement is `It SHALL print the list.` and whose one scenario is `first`, `a/y` whose statement is `It SHALL exit 0.` and whose one scenario is `second`, and a branch that removes `a/x` and adds a scenario `first` to `a/y`, leaving `a/y`'s statement as it is
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exactly one cut line, `specs/a/SPEC.md: gate/cut: a/x merged into a/y state 1 sentences where they stated 2`

#### Scenario: a merge that keeps its sentences
- GIVEN the same, and a branch that also puts `It SHALL print the list.` before `a/y`'s statement
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and no `gate/cut` line

#### Scenario: two merged into a new requirement
- GIVEN `a/x` whose statement is `It SHALL print the list.` and whose one scenario is `first`, `a/y` whose statement is `It SHALL exit 0.` and whose one scenario is `second`, and a branch that removes both and adds `a/z`, whose statement is `It SHALL print the list.` and whose scenarios are `first` and `second`
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exactly one cut line, `specs/a/SPEC.md: gate/cut: a/x, a/y merged into a/z state 1 sentences where they stated 2`

#### Scenario: a merge into another component
- GIVEN `a/x` with the one scenario `the chain`, component `b` with requirement `b/y`, each stating one sentence, and a branch that removes `a/x` and adds a scenario `the chain` to `b/y`, leaving `b/y`'s statement as it is
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exactly one cut line, `specs/a/SPEC.md: gate/cut: a/x merged into b/y state 1 sentences where they stated 2`

#### Scenario: a requirement that took scenarios and lost a sentence
- GIVEN `a/x` whose statement is `It SHALL print the list.` and whose one scenario is `first`, `a/y` whose statement is `It SHALL exit 0. It SHALL be quick.` and whose one scenario is `second`, and a branch that removes `a/x`, adds a scenario `first` to `a/y` and removes `It SHALL be quick.`
- WHEN `clm gate --base main` runs
- THEN it prints exactly two cut lines, `specs/a/SPEC.md: gate/cut: a/y states 1 sentences where it stated 2` and then `specs/a/SPEC.md: gate/cut: a/x merged into a/y state 1 sentences where they stated 3`

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

#### Scenario: a sentence that places a test across a line
- GIVEN `a/x` whose statement is `The command SHALL print the list.` on one line, `Every test of a's requirements SHALL` on the next and `sit in a's own files.` on the one after, and a branch that removes only the last two lines
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

#### Scenario: odd list lines
- GIVEN list texts holding `DELETE b.rs::mod::three` ending in a carriage return; a line the base already held with another reason after its tab; `DELETE f::` and `DELETE ::n`; `  # note`; and one line twice in the head
- WHEN `list_lines` runs on them, and `cuts` with them as a list's head text over a base text
- THEN `list_lines` gives file `b.rs` and name `mod::three` for the first, and neither empty-sided line nor the indented `#` line as a list line; and `cuts` lists no test for the line the base held and the repeated line once

#### Serves
- architecture/removal-needs-a-yes
- capability/approve-intent
- capability/propose-intent
- capability/triage-unattended

#### Needs
- check.coverage/test-lists

### Requirement: rules-version <!-- base: ad92df64f6d8f62c -->
`clm-gate` SHALL offer `RULES`, a whole number that SHALL be 8, and `evaluate_by(repo, base, head, approved, rules)`, returning what `evaluate_at` returns as the gate decided under the rules of version `rules`, and `evaluate_at` SHALL be `evaluate_by` with `RULES`. Under versions 7 and 8 a branch `light` takes SHALL be `light`, and under versions 1 to 6 it SHALL be `mixed`, blocked with `gate/classify`, and under those versions no `gate/classify` line SHALL say anything of `light`. Under versions 5 to 8 the gate SHALL read the paths a branch changes as `paths-under-the-root` states, and under versions 1 to 4 as `git diff` prints them, relative to the git repository's top, each file at a revision read as `<rev>:<path>`, as those versions did. Under version 8 a `spec` branch SHALL need review as `needs-review` states, under versions 6 and 7 as it states with a *shortened merge* cut making no branch need review, under versions 4 and 5 as under version 6 with a *shortened requirement* cut counted as though a placement phrase's words matched only where one space separates each two of them, under version 3 as under version 4 with a *shortened requirement* cut counted as though every sentence holding a placement phrase, so matched, placed a test, whether or not it holds the word `test` or `tests`, under version 2 as under version 3 with a *changed outcome* cut making no branch need review, and under version 1 as it states with `cuts` making no branch need review, specs that don't load at the merge base or the head included, a branch `rename/replay` accepts keeping its own rule under each. In a repository whose CLM root is its top, the kind and the cuts SHALL be the same under every version but for a branch `light` takes, and the violations too but for `gate/approval`, which follows the review, and the `gate/classify` line, which under versions 1 to 6 says nothing of `light`. In one whose CLM root is a subdirectory, the kind, the cuts and the violations under versions 1 to 4 SHALL be those their reading of the paths gives. For a `rules` below 1 or above `RULES`, `evaluate_by` SHALL return an error naming `rules` and `RULES`.

#### Serves
- architecture/signed-ledger-records
- architecture/humans-review-intent

#### Needs
- check.coverage/test-lists
- ledger/approved-constructor

#### Scenario: a removal under each version
- GIVEN component `a` with requirement `a/x`, and a branch `s` that only removes `a/x` from `specs/a/SPEC.md`
- WHEN `evaluate_by(repo, "main", s, false, 1)` and `evaluate_by(repo, "main", s, false, RULES)` run
- THEN the first gives `spec` without review and the second `spec` with review, and each holds the cut `requirement a/x is gone`

#### Scenario: a changed outcome under each version
- GIVEN component `a` with requirement `a/x` whose one scenario, `first`, ends `- THEN it exits 1`, and a branch `s` that only changes that line to `- THEN it lands`
- WHEN `evaluate_by(repo, "main", s, false, 2)` and `evaluate_by(repo, "main", s, false, RULES)` run
- THEN the first gives `spec` without review and the second `spec` with review, and each holds the cut `scenario "first" of a/x changes its outcome`

#### Scenario: a sentence of behaviour that holds a phrase under each version
- GIVEN component `a` with requirement `a/x` whose statement is `The command SHALL print the list. The directory SHALL be deleted once nothing else is left in it.`, and a branch `s` that only removes the second sentence
- WHEN `evaluate_by(repo, "main", s, false, 3)` and `evaluate_by(repo, "main", s, false, RULES)` run
- THEN the first gives `spec` without review and the second `spec` with review, and each holds the cut `a/x states 1 sentences where it stated 2`

#### Scenario: a placement phrase across a line under each version
- GIVEN component `a` with requirement `a/x` whose statement is `The command SHALL print the list. It SHALL exit 1 when the list is empty.`, and a branch `s` that only replaces the second sentence with `Every test of a's requirements SHALL`, a line break and `sit in a's own files.`
- WHEN `evaluate_by(repo, "main", s, false, 5)` and `evaluate_by(repo, "main", s, false, RULES)` run
- THEN the first gives `spec` without review and the second `spec` with review, and each holds the cut `a/x states 1 sentences where it stated 2`

#### Scenario: a merge that loses a sentence under each version
- GIVEN the branch `s` of *a merge that loses a sentence* (`cuts`)
- WHEN `evaluate_by(repo, "main", s, false, 7)` and `evaluate_by(repo, "main", s, false, RULES)` run
- THEN the first gives `spec` without review and the second `spec` with review, and each holds the cut `a/x merged into a/y state 1 sentences where they stated 2`

#### Scenario: the same verdict under both
- GIVEN a branch that only edits `ARCHITECTURE.md`
- WHEN `evaluate_by` runs on it with `1` and with `RULES`
- THEN both give `spec` with review

#### Scenario: a subdirectory root under each version
- GIVEN a git repository whose CLM root is its directory `tool/`, and a branch `s` that only edits `tool/ARCHITECTURE.md`
- WHEN `evaluate_by(tool, "main", s, false, 4)` and `evaluate_by(tool, "main", s, false, RULES)` run
- THEN the first gives `code` without review and the second `spec` with review

#### Scenario: a version this gate doesn't know
- GIVEN any branch
- WHEN `evaluate_by` runs on it with `RULES + 1`, and again with `0`
- THEN each returns an error whose message names the version asked for and `RULES`

#### Scenario: a light branch under each version
- GIVEN the branch `s` of *scenarios added with their code* (`light`)
- WHEN `evaluate_by(repo, "main", s, false, 6)` and `evaluate_by(repo, "main", s, false, RULES)` run
- THEN the first gives `mixed` with a `gate/classify` violation, and the second `light` without review and with no violation
