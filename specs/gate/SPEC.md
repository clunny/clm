---
title: Gate
status: active
owners: ["@clunny"]
depends_on: [model, context, vcs, gate.fingerprint, rename, check.coverage]
paths: ["crates/clm-gate/**"]
serves: [capability/land-safely, capability/approve-intent, architecture/humans-review-intent]
---

## Contract
`clm gate --base <ref> [--approved] [--root <dir>]` decides whether `HEAD` may merge into `<ref>`. Its first line is the branch's kind: `spec review` for a spec branch that needs a human's review, `spec` for one that doesn't, `code`, `light` for a branch that adds scenarios to one component's requirements together with the code that proves them (`light`), or `mixed`. After it come, for a spec branch, one `gate/cut` line for each thing the branch cuts (`cuts`), then each violation as the line `clm-model` gives for it, `<path>: <rule>: <message>` (`model/violation-line`). It exits 0 to allow and 1 to block. When git fails, as for a base that names no commit, it writes `clm gate: <message>` to standard error and nothing to standard output and exits 2, as it does, writing the usage error, for arguments it can't parse (`evaluate-at`, `commands-in-crate`). `--approved` says the ledger holds an approval of this diff that the repository's policy accepts; `clm land` computes it from the fold (`land-locally`). The gate reads no ledger, no host and no branch name: what it decides is a function of the two trees and that flag.

`clm-gate` offers, as library calls:
- `evaluate_at(repo, base, head, approved)`: the gate for `head` against `base` (`evaluate-at`), an `Outcome { kind, review, spec_part_review, violations, cuts, component }`, `kind` a `Kind` (`Spec`, `Code`, `Light` or `Mixed`, each printing as the first line's word), `review` whether a `spec` branch needs review, `spec_part_review` whether a `light` or `mixed` branch would need it were it a `spec` branch (`review` for any other kind), `violations` of the re-exported `Violation` type, `cuts` each a `Cut`, and `component` whose work the branch is (`own-files`, `light`): `Some(<component ID>)` for a `light` branch, the component whose scenarios it adds, and for a `code` branch that one component accounts for, and `None` for a `code` branch whose code files are only wiring, files no component owns and files outside the CLM root, for a `spec` or `mixed` branch, and for every `code` branch of a project whose root names no `wiring`; an error holding git's message when git fails. `land` calls it in-process to learn a branch's kind before it takes its lock, and `sync` on each `clm rebase` to learn whether a branch is a build's and whose; `ledger`'s audit reads `spec_part_review` for a land recorded light; `host` reads an outcome's kind and cuts. `evaluate_by(repo, base, head, approved, rules)` is the same under the rules of version `rules`, and `RULES` the version `evaluate_at` applies (`rules-version`): `land` names `RULES` in each merge commit (`land/gate-rules-trailer`), and `ledger`'s `verify --review` recomputes each signed land with `evaluate_by` under the version in force when it landed (`audit-by-the-rules-in-force`).
- `is_spec_path(path)`: whether a path relative to the CLM root is a spec path (`classify`), which `land` and `web.worker.review` read.
- `files_outside_spec_paths(repo, base, head)`, the files of a revision that aren't spec paths (`files-outside-spec-paths`), which `land` reads to name a spec branch's code files.
- `foreign_files(repo, base, head, component)`: the code files `head` changes that `component`'s build may not, each a `ForeignFile { path, owner }`, a file outside the CLM root among them with an empty owner (`own-files`), which `land` reads to hold a branch named `impl/<component>` to its claim (`land/build-holds-own-files`).
- Over `clm.toml`'s bytes on each side of a diff: `approval_changed(before, after)` (`approval-changed`), `upgrade_stamp_only(before, after)` (`upgrade-stamp-left-out`) and `settings_unchanged(before, after)` (`settings-unchanged`). Over a diff's paths and changes, `needs_review_of(paths, approval_changed, changes)`, and over two spec graphs `widened_exemptions(base, head)` (`needs-review-of`). Over two spec graphs, `cuts(base, head, lists)` and `cuts_or_unloaded(base, head, lists)`, each list a `TestList` and each side of the second a `Side`, with `sentences(statement)`, `stating_sentences(statement)`, `places_a_test(sentence)` and `list_lines(text)`, each line a `ListLine` (`cuts`). Each of these runs no process and builds for `wasm32-unknown-unknown`. `web.worker.review` calls `approval_changed`, `needs_review_of`, `widened_exemptions`, `cuts_or_unloaded` and `is_spec_path`, so it gives the gate's answer from the files it fetches; `land` calls `settings_unchanged` (`land/settings-unchanged-is-checked`) and `list_lines`; `check` and `orchestrator.run` call `list_lines`; `check.health` calls `sentences` and `places_a_test`.
- `gate.fingerprint`'s `Entry`, `parse_ls_tree`, `fingerprint_of`, `adds_change`, `added_changes` and `fingerprint`, under those names (`fingerprint-reexported`), which `land`, `land.approval`, `ledger`, `web` and `crates/clm` call through `clm_gate`.
- `command(args, root, env, out, err)` and its `Args`, `clm gate` as a call (`commands-in-crate`, `commands-live-in-their-crates`): `crates/clm` parses `Args` as part of its own command line, routes the command to the call and adds nothing to what it writes.

## Design
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

**Cuts** (`cuts`, `cuts-are-what-a-program-lists`). `removal-needs-a-yes` was the session's word alone. The gate lists what a program can see of a removal and asks review for it, reading the requirements at the merge base and at the head from `specs/`, where `clm apply` has already put the branch's deltas, so it reads no delta. Both graphs are loaded as **A commit's components** says, with `ARCHITECTURE.md` and `CAPABILITIES.md` beside `specs/` and the live `change.md` files, since a rule's `serves` names their entries and a graph without them loads with a violation for every such rule, and only for a `spec` branch with a spec change whose diff touches `specs/` or a `deleted-tests.txt`. A name is matched within the diff and one for one, since many requirements share a name such as "not switched on": matching against the whole head would never list one. So a rename is listed, as a program can't tell it from a removal, and a session keeps a scenario's name when it rewords or moves it. For the same reason a scenario that keeps its name and changes its outcome is listed: a program can't tell an outcome reworded from one reversed, and a reversed outcome under a kept name drops behaviour as a removal does, with no name gone and no sentence lost (`a-changed-outcome-is-a-cut`). Counting sentences that hold `SHALL` sees a requirement that states less and not one that states something else in as many sentences, which stays a reader's. The placement phrases are a constant of the crate, changed by a code PR. The tests a change deletes are declared, not computed: `clm coverage --obsolete-tests` lists every test of a merged requirement, which a build re-cites, and misses a duplicate whose requirement stays. A spec graph that doesn't load at either side can't be compared, so the branch needs review. The exemptions that make a branch need review are read from the two graphs, by `widened_exemptions`, which the Worker calls too; `exemption-names-its-item` keeps reading the frontmatter as text, so an exemption the model rejects is still held to its item. The gate checks only the lines a branch adds, so a list whose tests a build has since deleted stays valid, and `clm land` holds a code branch to the lists on its base (`land/deleted-tests-approved`).

A branch behind `<ref>` may land (`land-locally`). `clm land` merges `<ref>` into it and checks the result before anything lands. So the gate blocks only what that merge can't settle: a textual conflict, which `git merge-tree --write-tree <ref> HEAD` reports by exiting 1, and a file under `specs/` that changed on both sides since the merge base, since `specs/` is derived, regenerated by `clm rebase` and never merged (`regenerate-dont-merge`), even when the text would merge cleanly.

**Evaluating another commit** (`gate/evaluate-at`). `evaluate_at(repo, base, head, approved)` reads `head` only as a revision, through `git diff <merge-base>...<head>`, `git show <head>:./<path>`, `git ls-tree <head>`, `git log <merge-base>..<head>`, `git grep` at `<head>` and `git merge-tree --write-tree <base> <head>`, never the working tree or the index, so it gives the same verdict whatever is checked out. `clm gate` calls it with `HEAD`. `clm land` calls it in-process before its lock, for the kind its own rules read, and again as `clm gate` on the merge commit, whose tree is what lands; `sync` calls it on each `clm rebase`, for whose work a branch is. `ledger`'s `verify --review` calls `evaluate_by` for each signed land, with the land's first parent as `base`, the merge commit as `head` and the rules in force when it landed, to recompute the land's kind and whether it needed review without a checkout per land.

**A branch's component** (`own-files`, `own-files-at-the-gate`). `path-ownership` was kept by the build prompt alone, and then by `clm land` for a branch named `impl/<component>` only; a small change or a fix on a branch of another name was judged by nothing. The gate now reads the component from the files of every code branch, so the rule needs no name. The specs it reads are the merge base's; a file's owner is `check.coverage`'s `owner`, which the gate depends on for it. The candidates are the owners of the branch's code files that no `wiring` glob matches, and the branch's component is the candidate whose `foreign_files`, files outside the CLM root left out, is empty; a file of one candidate is foreign to every other unless it is a capability test the other serves and the branch only modifies, so the answer is mostly one component or none, and a tie, as between two files that each hold a marker of a capability the other component serves, goes to the first by ID. A file outside the CLM root is no component's, so the gate admits it as it admits any file no component owns; `clm land` still refuses it on a branch named `impl/<component>`, through `foreign_files`. Two kinds of file are a build's to change beside its own. The root's `wiring` names the shared ones. A file that merely cites the component is not one of them: a test sits in the files of the component it proves, so a marker gives a build no file (`commands-live-in-their-crates`). A capability test may be edited, never added or deleted, by a build whose component serves the capability it cites, since a behaviour and the test that observes it at the boundary can't land apart (`capability-tests-follow-rules`). That a wiring edit adds only the build's own entry, and that a capability-test edit keeps what its scenarios state, stay judgements no program makes. No rule picks a majority component for a branch that spans two: the gate lists every owned file and lets the author split it. The rule is off until the root sets `wiring`, as `check`'s path ownership is off until it sets `outside`, so a project adopts it when it has named its wiring; a light branch is held to one component's files whatever the root sets, since it is admitted only because it is small.

**Light branches** (`light`). A scenario added to an existing requirement states a case of what the requirement already says, and the test that proves it is the build's; making the two land on separate branches costs a review-free spec land and a build, with the scenario pending between them. So the gate lets them land together when nothing a reader must see is in the spec part: the statement, the Contract, the Design, the frontmatter and the set of requirements stay as they are, nothing is cut, nothing needs review, and the code is the component's own. The change rides the branch already archived, `done`, as a change archived in its own spec PR does, since the branch's own tests make every requirement it touched current: a light land leaves nothing live and nothing for a later build. `clm land` regenerates and compares `specs/` for such a change as for any change a branch adds (`land/derived-specs`), and the gate's run on the merge commit repeats the currency check on the tree that lands, so a `main` that moved between the two runs can't land a requirement behind. Markers are found with `git grep` at the revision, through `clm-git`, and read with `check.coverage`'s marker reading, so the gate counts a citation where `clm coverage` does. A light land is recorded as a code land that says it was light (`ledger/landed-record-fields`), so a `clm` that knows no light route reads it as a code land and, auditing with `--review`, says it can't rate it (a `Gate-Rules` above its `RULES`). The audit of a land recorded light recomputes only what `RULES` versions, whether its spec part needs review and what it cuts, through `spec_part_review` (`ledger/verify-recomputes-review`): ownership, currency and how the model loads are what other components change, and a later change to them must never make an honest light land read as understated.

**Rules versions** (`rules-version`, `audit-by-the-rules-in-force`). The audit of a signed land asks whether its record understates what the gate gave it, so it must ask the gate that judged it, not today's: a rule that makes more diffs need review would otherwise report every honest land made before it as understated and unapproved. `RULES` names the rules `evaluate_at` applies, and `evaluate_by` keeps each earlier version's answer. Version 1 is the gate as it stood before `cuts` made a branch need review; version 2 adds `cuts`; version 3 adds the changed outcome to what makes a branch need review. Version 4 counts a sentence that holds a placement phrase and not the word `test` or `tests` as a stating sentence, so a branch that drops one needs review; the lower versions decide review from a count that leaves out every sentence holding a phrase. Version 5 reads the changed paths relative to the CLM root (**Paths under the root**, above); the lower versions read them as `git diff` prints them, relative to the repository's top, and name a file at a revision as `<rev>:<path>`, as the gate did before, which in a repository whose root is a subdirectory classes a branch editing the root's spec files as `code`. Version 6 matches a placement phrase across any run of white space between its words. Version 7 adds the light route: under the lower versions a light branch is `mixed`, as it was, and a `mixed` line says nothing of the light route, so an older `clm` auditing a light land reads a version it doesn't know and says it can't rate the land (`ledger/verify-unrated-land`) rather than that the land understated a `mixed` diff. Versions 1 to 4 read every path as those versions did, pathspecs and graph loads included; in a repository whose root is a subdirectory that changes the kind, either way: a branch editing the root's spec files is `code` under the lower versions and `spec` from 5, and one editing a spec-path name at the repository's top outside the root, such as its own `CLAUDE.md`, is `spec` under the lower versions and `code` from 5. The path reading is one function given the version, not a second gate. Apart from it and the light route only the review depends on the version, and with it `gate/approval`: the kind, the cuts and the other violations are those of the gate of version `RULES` at every version, since the audit reads only the kind and the review. Reading a branch with no code file as `spec` rates no diff higher, since such a branch needs no review, and blocking a branch that spans two components is a violation the audit doesn't rate, so neither needed a version. A requirement can't name its own component in `#### Needs`, so `rules-version` relies on `cuts` and `light` by the build of `gate` taking all three. A change that makes the gate give some diff a higher kind or review than it did (`code` = `spec` = `light` < `spec review` < `mixed`) raises `RULES` by one and keeps the earlier answer behind `rules` for the lower versions, for as long as signed history may hold a land judged by them, which is always; a change that rates diffs lower raises it too when an older `clm` would otherwise call the new answer understated. The older answers are a branch on `rules` beside the rule that changed, not a copy of the gate.

## Requirements

### Requirement: classify
The gate SHALL classify a branch by the files it adds, modifies or deletes against the merge base. A *spec path* is a path, relative to the CLM root and judged by its name as git stores it whatever characters it holds, under `specs/`, `changes/`, `backlog/` or `docs/`, or the file `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` or `AGENTS.md` at the root, and `is_spec_path(path)` SHALL return whether a path is one. A *spec change* is a changed spec path that `status-only-exemption`, `archive-move` and `backlog-additions` don't leave out, or `clm.toml` when `approval-setting` counts it as one; a *code file* is a changed file that isn't a spec path, but for a `clm.toml` that `approval-setting` counts as a spec change or `upgrade-stamp-left-out` leaves out. The kind SHALL be `spec` when the branch changes no code file, an empty diff and one whose every file is left out included, or when `rename/replay` accepts its diff from the merge base; otherwise `code` when it holds no spec change, `light` when `light` takes it, and `mixed` when `light` doesn't. The gate SHALL block a `mixed` branch with rule `gate/classify`, its message naming the spec changes and the code files, each list comma-separated, telling the author to split it, telling a build that it edits no spec path but leaves a requirement that needs a spec edit unbuilt and adds a `question` backlog item whose `blocks` names it (`requirement-blocks`), and saying why `light` doesn't take it (`light`).

#### Scenario: mixed build branch
- GIVEN a branch that changes `crates/a/lib.rs` and the Contract of `specs/a/SPEC.md`
- WHEN the gate runs
- THEN it prints `mixed`, exits 1, and its line says the build leaves the requirement unbuilt and adds a `question` item

#### Scenario: rename branch
- GIVEN a branch from `main` holding only the committed result of `clm rename r`, which rewrote `specs/a/SPEC.md` and a marker in `crates/a/tests/t.rs`
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and exits 0

#### Scenario: rename branch with an extra code edit
- GIVEN that branch with one more commit changing `crates/a/src/lib.rs`
- WHEN `clm gate --base main` runs
- THEN it prints `mixed` and exits 1

#### Scenario: every spec path
- GIVEN a branch for each of `specs/b/SPEC.md`, `changes/c/change.md`, an existing `backlog/x.md` edited, `docs/x.md`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `AGENTS.md`, `docs/naïve.md` and `specs/é/SPEC.md`, each changing that file alone
- WHEN `evaluate_at` runs on each, and `is_spec_path` runs on each path and on `crates/x/src/lib.rs` and `../CLAUDE.md`
- THEN each branch's kind is `spec`, and `is_spec_path` returns true for each of the ten paths and false for the last two

#### Scenario: what the mixed line names
- GIVEN a branch changing `specs/a/SPEC.md`'s Contract and `crates/x/src/lib.rs`
- WHEN `clm gate --base main` runs
- THEN its `gate/classify` line names `specs/a/SPEC.md` among the spec changes and `crates/x/src/lib.rs` among the code files

#### Scenario: only files left out
- GIVEN three branches from `main`: one that only adds `backlog/x.md`, one that only moves `changes/c/` to `changes/archive/c/` with `status: done`, and one with no commit of its own
- WHEN `clm gate --base main` runs on each
- THEN each prints `spec` and exits 0

#### Needs
- rename/replay
- ledger/approved-constructor

### Requirement: status-only-exemption
A change to a spec path whose diff lines are only `status:` frontmatter lines SHALL NOT count as a spec change, so code PRs can record component and change status. A `status:` line changed beside any other line, a removed line that starts with `--`, and any edit of `ARCHITECTURE.md`, `CAPABILITIES.md` or `docs/allowed_signers` (a path under it included) SHALL count as a spec change, whatever its lines.

#### Scenario: code PR flips status
- GIVEN a branch changing `crates/x/src/lib.rs` and only the `status:` line of `specs/x/SPEC.md`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0

#### Scenario: edits that aren't status-only
- GIVEN branches that change `crates/x/src/lib.rs` and, in turn: the `status:` line and the Contract of `specs/x/SPEC.md`; the `status:` line of `specs/x/SPEC.md` and a line `-- note` removed from it; a line `status: ssh-ed25519 BBBB` added to `docs/allowed_signers`; only a `status:` line of `ARCHITECTURE.md`
- WHEN `clm gate --base main` runs on each
- THEN each prints `mixed` and exits 1

#### Serves
- capability/land-safely
- architecture/fold-small-defer-large

### Requirement: approval
A `spec` branch that needs review (`needs-review`) SHALL be blocked with rule `gate/approval`, naming `clm approve`, unless `--approved` is given. A `spec` branch that needs no review, a `code` branch and a `light` branch SHALL NOT require it.

#### Scenario: unapproved branch that needs review
- GIVEN a branch that only edits `ARCHITECTURE.md`
- WHEN `clm gate --base main` runs
- THEN it prints `spec review`, reports `gate/approval` naming `clm approve`, and exits 1

#### Scenario: approved branch that needs review
- GIVEN the same branch
- WHEN `clm gate --base main --approved` runs
- THEN it prints `spec review` and exits 0

#### Scenario: branch that needs no review
- GIVEN a branch that only changes the Contract of `specs/a/SPEC.md`
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and exits 0

#### Needs
- ledger/approved-constructor

### Requirement: needs-review
A `spec` branch with a spec change (`classify`) SHALL need review exactly when its diff adds, modifies or deletes `ARCHITECTURE.md`, `CAPABILITIES.md` or `docs/allowed_signers` (the keys a signed repository trusts, `signed-ledger-records`), or a path under `docs/allowed_signers/`; or `approval-setting` counts `clm.toml` as changed; or a change one of whose files it adds or modifies, a live change or a new archived one, has a `change.md` at `HEAD` that is missing, can't be read as UTF-8, whose frontmatter doesn't parse, or that declares a non-empty `human_burdens` (`yes-no-review`), so editing a change's deltas or assets alone doesn't leave its burdens unreviewed; or `cuts` returns anything for the branch (`cuts`), so a requirement, a scenario or a sentence it removes, or a test it lists to delete, is always seen by a human (`removal-needs-a-yes`); or a component's `SPEC.md` at `HEAD` holds an exemption (`model/exemptions`) whose `files` aren't all among the `files` of an exemption with the same `rule` and `until` that the component with the same ID holds at the merge base (every exemption of a component new on the branch counting), so relaxing a rule, or widening a relaxation, is always seen by a human, while removing or narrowing one, or changing only its `why`, never needs review (`temporary-exemptions`). A `spec` branch that `rename/replay` accepts SHALL instead need review exactly when its added change records a `term` rename, declares a non-empty `human_burdens`, or has a `change.md` that can't be read or parsed: its replayed edits to `ARCHITECTURE.md` and `CAPABILITIES.md` SHALL NOT count. A `spec` branch none of whose changed files is a spec change SHALL need no review and list no cut. Nothing else SHALL make a branch need review, and a `code`, `light` or `mixed` branch never does. `clm gate` SHALL print `spec review` for a `spec` branch that needs review and `spec` for one that doesn't.

#### Scenario: trusted keys
- GIVEN `main` with `docs/allowed_signers`, and a branch that only adds a key to it
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exits 1

#### Scenario: trusted keys however edited
- GIVEN `main` with `docs/allowed_signers`, and a branch that only deletes it; and `main` without it, and a branch that only adds `docs/allowed_signers/x`
- WHEN `clm gate --base main` runs on each
- THEN each prints `spec review` and exits 1

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

#### Scenario: a change with no change.md
- GIVEN a branch that only adds `changes/x/deltas/a.md`, with no `changes/x/change.md` at the merge base or at `HEAD`
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exits 1

#### Scenario: nothing else
- GIVEN a branch that changes the Contract of `specs/a/SPEC.md`, rewords one of `a`'s requirements, keeping its scenarios and the number of its stating sentences, adds `docs/decisions/d.md` and adds `changes/x/change.md`, `applied` with no `human_burdens`
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and exits 0

#### Scenario: a status flip alone
- GIVEN `main` with `changes/x/change.md`, `applied` with `human_burdens: [b]`, and a branch that only changes that file's `status:` line
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and exits 0

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
- ledger/approved-constructor
- check.coverage/test-lists
- model/exemptions
- rename/replay
- model/parse-changes

### Requirement: undeclared-burden
A `spec` branch with a spec change (`classify`), or with a `clm.toml` that `upgrade-stamp-left-out` leaves out, that adds a `backlog/` item whose `kind` at `HEAD` is `human-task`, while no change one of whose files it adds or modifies, a live change or a new archived one, declares a non-empty `human_burdens` in its `change.md` at `HEAD`, SHALL be blocked with rule `gate/undeclared-burden`, naming the item, its message naming the kind `human-task` and the key `human_burdens`, even with `--approved`. A `code` or `light` branch, and any other `spec` branch, MAY add such an item freely: a build files `question` and `human-task` items, which block only their own requirements (`requirement-blocks`).

#### Scenario: undeclared human step
- GIVEN a branch that adds `changes/x/change.md`, `proposed` with no `human_burdens`, and `backlog/h.md` of `kind: human-task`
- WHEN `clm gate --base main` runs
- THEN it reports `gate/undeclared-burden` naming `backlog/h.md`, and exits 1

#### Scenario: declared human step
- GIVEN the same branch, with `human_burdens: [a key only the human can make]` in `changes/x/change.md`
- WHEN `clm gate --base main --approved` runs
- THEN it prints `spec review` and exits 0

#### Scenario: the old words
- GIVEN a branch that adds `changes/x/change.md`, `proposed` with no burdens under either key, and `backlog/h.md` whose `kind` is the word `plain-terms-renames` replaced with `human-task`; and a second branch where `changes/x/change.md` declares a burden under the key `plain-terms-renames` replaced with `human_burdens`
- WHEN `clm gate --base main` runs on each
- THEN the first reports `gate/undeclared-burden` naming `backlog/h.md`, and the second prints `spec review`

#### Scenario: a human step filed alone
- GIVEN a branch that only adds `backlog/h.md` of `kind: human-task`; and one that adds it and changes only `upgraded` in `clm.toml`
- WHEN `clm gate --base main` runs on each
- THEN the first prints `spec`, reports no `gate/undeclared-burden`, and exits 0, and the second reports `gate/undeclared-burden` naming `backlog/h.md` and exits 1

#### Needs
- ledger/approved-constructor
- model/parse-changes
- model/parse-backlog

### Requirement: up-to-date
The gate SHALL block, with rule `gate/up-to-date` and a message telling the author to run `clm rebase`, when `HEAD` doesn't merge cleanly into `<ref>`, or when a file under `specs/` changed both between the merge base and `<ref>` and between the merge base and `HEAD`. Each such file SHALL get a line of its own, naming it relative to the CLM root, in path order. A branch that is merely behind `<ref>` SHALL NOT be blocked for that.

#### Scenario: behind, merges cleanly
- GIVEN `main` has a commit changing `crates/a.rs` that the branch lacks, and the branch changes only `crates/b.rs`
- WHEN `clm gate --base main` runs
- THEN it reports no `gate/up-to-date` violation and exits 0

#### Scenario: conflict
- GIVEN `main` and the branch each change the same line of `crates/a.rs` differently since their merge base
- WHEN `clm gate --base main` runs
- THEN it exits 1 with a `gate/up-to-date` violation naming `clm rebase`

#### Scenario: spec changed on both sides
- GIVEN `main` and the branch each change a different requirement in `specs/n/SPEC.md` since their merge base
- WHEN `clm gate --base main` runs
- THEN it exits 1 with a `gate/up-to-date` violation, although the text would merge cleanly

#### Scenario: several spec files changed on both sides
- GIVEN `main` and the branch each changing `specs/b/SPEC.md` and `specs/a/SPEC.md` since their merge base
- WHEN `clm gate --base main` runs
- THEN it prints two `gate/up-to-date` lines, the first naming `specs/a/SPEC.md` and the second `specs/b/SPEC.md`

#### Needs
- clm/renamed-commands

#### Serves
- capability/land-safely
- capability/work-concurrently
- architecture/hash-based-staleness

### Requirement: archive-move
Moving `changes/<id>/<file>` to `changes/archive/<id>/<file>` with at most `status:` lines changed SHALL NOT count as a spec change, so a code PR can finish and archive its change.

#### Scenario: impl PR archives its change
- GIVEN a branch that changes code, sets `changes/x/change.md` to `status: done` and moves `changes/x/` to `changes/archive/x/`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0

#### Serves
- capability/land-safely
- architecture/spec-is-current

### Requirement: archive-immutable
The gate SHALL block any PR that modifies or deletes a file that exists under `changes/archive/` at the merge base, and any that adds a file under a directory `changes/archive/<id>/` holding a file at the merge base other than by an archive move (`archive-move`), with rule `gate/archive-immutable` naming the file, with `--approved` or without.

#### Scenario: editing history
- GIVEN `changes/archive/x/change.md` on `main`
- WHEN a branch edits it and `clm gate --base main --approved` runs
- THEN it exits 1 naming the file

#### Scenario: adding to history
- GIVEN `changes/archive/x/change.md` on `main`, and a branch that adds `changes/archive/x/notes.md`; and a second branch that adds `changes/archive/y/change.md` with `status: done`, `y` new
- WHEN `clm gate --base main --approved` runs on each
- THEN the first exits 1 with `gate/archive-immutable` naming `changes/archive/x/notes.md`, and the second reports no `gate/archive-immutable`

#### Serves
- architecture/spec-is-current
- architecture/humans-review-intent

### Requirement: backlog-additions
A file added under `backlog/` SHALL NOT count as a spec change, so code PRs can record deferred work (`fold-small-defer-large`). A change to an existing top-level backlog item `backlog/<slug>.md` whose only effect is to add entries to its `blocks` list, the set of entries at `HEAD` holding every one at the merge base and more, SHALL NOT count as a spec change either, so a build can record that a requirement waits on an item that already exists (`requirement-blocks`); reordering `blocks`, and adding to the `blocks` of a file in a directory under `backlog/`, SHALL count. Deleting `backlog/<slug>.md` SHALL NOT count as a spec change when the same branch archives a change whose `resolves` lists `<slug>`, or when a change outside the archive at the merge base lists `<slug>` in its `resolves`, so the build that fixes an item may delete it. Nor SHALL it count when a commit between the merge base and `HEAD` carries the trailer `Resolves: backlog/<slug>`, the item at the merge base has `kind: bug` or `kind: deferred`, and a file the PR adds, modifies or deletes outside the spec paths belongs to a component the item's `components` names: the file matches, as a `:(glob)` pathspec, that component's `paths` at the merge base, or its text at the merge base or at `HEAD` contains `req:<component>/` (`fixes-count-where-cited`). A trailer value counts only when, trimmed, it is exactly `backlog/<slug>`. An item whose frontmatter doesn't parse, or none of whose components exist at the merge base, SHALL NOT be exempt this way; a named component missing there SHALL count for neither `paths` nor markers while the others count, and a component with no `paths` SHALL still qualify through a marker. A spec path matching a component's `paths` SHALL NOT count as its fix. A file whose text isn't valid UTF-8 SHALL be searched as bytes, and a side of a file that can't be read SHALL hold no marker, neither making the gate fail. So a fix that changes no spec may delete its bug in the same code PR (`code-fix-resolves-bug`), including a fix to a test kept under another component's `paths`, and a build that does a deferred item's work, such as removing code it kept for another component that no longer uses it, may delete that item (`deferred-work-resolves-in-code`). A `question`, `human-task` or `idea` item SHALL NOT be exempt this way: closing one is a decision. Any other modification or deletion of a backlog file SHALL count as a spec change.

#### Scenario: impl PR defers a discovery
- GIVEN a branch that changes code and adds `backlog/found-bug.md`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0

#### Scenario: impl PR edits an existing item
- GIVEN a branch that changes code and edits the body of an existing `backlog/x.md`
- WHEN `clm gate --base main` runs
- THEN it prints `mixed` and exits 1

#### Scenario: impl PR adds a block
- GIVEN a branch that changes code and adds `a/y` to the `blocks` of an existing `backlog/x.md`, changing nothing else in it
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0

#### Scenario: impl PR removes a block
- GIVEN a branch that changes code and removes an entry from the `blocks` of an existing `backlog/x.md`
- WHEN `clm gate --base main` runs
- THEN it prints `mixed` and exits 1

#### Scenario: blocks edits that add nothing
- GIVEN a branch that changes code and only reorders the `blocks` of an existing `backlog/x.md`; and one that changes code and only adds an entry to the `blocks` of an existing `backlog/sub/x.md`
- WHEN `clm gate --base main` runs on each
- THEN each prints `mixed` and exits 1

#### Scenario: impl PR deletes the item its change resolves
- GIVEN a branch that changes code, archives change `c` whose `resolves` is `[x]`, and deletes `backlog/x.md`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0

#### Scenario: build deletes a resolved item
- GIVEN `main` with change `c` outside the archive whose `resolves` is `[x]`, and a branch that changes code and deletes `backlog/x.md` without archiving `c`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0

#### Scenario: resolves added in the same PR
- GIVEN `main` with change `c` whose `resolves` is empty, and a branch that changes code, adds `x` to `c`'s `resolves` and deletes `backlog/x.md`
- WHEN `clm gate --base main` runs
- THEN it prints `mixed` and exits 1

#### Scenario: impl PR deletes an unrelated item
- GIVEN the branch of *impl PR deletes the item its change resolves*, which also deletes `backlog/y.md`, listed in no change's `resolves`
- WHEN `clm gate --base main` runs
- THEN it prints `mixed` and exits 1

#### Scenario: code fix deletes its bug
- GIVEN `main` with component `a` whose `paths` are `["crates/a/**"]` and `backlog/x.md` of kind `bug` whose `components` are `[a]`, and a branch that changes `crates/a/src/lib.rs` and deletes `backlog/x.md` in a commit whose message ends with the trailer `Resolves: backlog/x`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0

#### Scenario: code fix outside the bug's nodes
- GIVEN the branch of *code fix deletes its bug*, changing `crates/b/src/lib.rs`, which no `paths` of `a` match and which holds no `req:a/`, instead of `crates/a/src/lib.rs`
- WHEN `clm gate --base main` runs
- THEN it prints `mixed` and exits 1

#### Scenario: code fix to a test that cites the bug's node
- GIVEN the branch of *code fix deletes its bug*, changing instead `crates/b/tests/t.rs`, which no `paths` of `a` match and whose text on `main` holds a marker `req:a/y@1234abcd`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0

#### Scenario: code fix to a test that cites a child of the bug's node
- GIVEN the branch of *code fix to a test that cites the bug's node*, where the test's only marker cites `a.b/y` instead
- WHEN `clm gate --base main` runs
- THEN it prints `mixed` and exits 1

#### Scenario: trailer naming an idea
- GIVEN the branch of *code fix deletes its bug*, where `backlog/x.md` on `main` is of kind `idea`
- WHEN `clm gate --base main` runs
- THEN it prints `mixed` and exits 1

#### Scenario: trailer naming deferred work
- GIVEN the branch of *code fix deletes its bug*, where `backlog/x.md` on `main` is of kind `deferred`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0

#### Scenario: trailer naming a question
- GIVEN the branch of *code fix deletes its bug*, where `backlog/x.md` on `main` is of kind `question` with no `blocks`
- WHEN `clm gate --base main` runs
- THEN it prints `mixed` and exits 1

#### Scenario: trailer naming another item
- GIVEN the branch of *code fix deletes its bug*, whose trailer is `Resolves: backlog/y` instead
- WHEN `clm gate --base main` runs
- THEN it prints `mixed` and exits 1

#### Scenario: a code fix judged on odd items and files
- GIVEN the branch of *code fix deletes its bug*, changed in turn so that: `backlog/x.md` has no frontmatter; it names no component; it names only `z`, missing at the merge base; it names `z` and `a`; it names a component with no `paths`, and the branch changes, in place of `crates/a/src/lib.rs`, `crates/b/tests/t.rs`, which cites that component; it names a component whose `paths` are `["specs/b/**"]`, and the branch changes `crates/a/src/lib.rs` and only the `status:` line of `specs/b/SPEC.md`; the branch changes, in place of `crates/a/src/lib.rs`, `crates/b/tests/t.rs`, whose bytes aren't valid UTF-8 and hold `req:a/y`; the branch also changes the commit a submodule link `crates/b/sub` names, beside `crates/b/tests/t.rs`, which cites `a/y`; the trailer reads `Resolves: backlog/x.md`, `Resolves: x` or `Resolves:   backlog/x  `
- WHEN `clm gate --base main` runs on each
- THEN the first three print `mixed`; the fourth and fifth `code`; the sixth `mixed`; the seventh and eighth `code`, neither failing; and the trailers `mixed`, `mixed` and `code`

#### Serves
- capability/keep-a-backlog
- architecture/fold-small-defer-large

#### Needs
- model/parse-backlog

### Requirement: resolves-named-items
Each `resolves` entry that a branch adds to a `change.md` outside the archive, comparing the parsed lists at the merge base and at `HEAD`, and including every entry of a `change.md` the branch adds, live or in a new archived change (`needs-review`), where the list at the merge base of a new archived change `changes/archive/<id>/` is that of `changes/<id>/change.md` there, if it exists, SHALL name a backlog item that exists at the merge base or at `HEAD`. Otherwise the gate SHALL report `gate/resolves-named-items` naming the change and the entry, and exit 1. Entries already there at the merge base SHALL NOT be tested, so a task may delete an item its change resolves (`task-deletes-fixed-items`). Backlog items are named by bare slug, so an entry that isn't one, such as `backlog/x` or `x.md`, names no item even when `backlog/x.md` exists, and is reported (`check/backlog-entry-shape` says what to write). An item is a top-level `backlog/<slug>.md`, its slug matched exactly whatever characters it holds, and a file in a directory under `backlog/` is none. A `change.md` whose frontmatter doesn't parse at `HEAD` SHALL get no `gate/resolves-named-items` line, since `clm check` reports it and `needs-review` asks review for it.

#### Scenario: typo in a new change
- GIVEN a branch that adds `changes/c/change.md` with `resolves: [nope]`, and no `backlog/nope.md` at the merge base or at `HEAD`
- WHEN `clm gate --base main` runs
- THEN it reports `gate/resolves-named-items` naming `c` and `nope`, and exits 1

#### Scenario: typo added to a live change
- GIVEN `main` with change `c` whose `resolves` is `[x]`, and a branch that rewrites it as `[x, nope]` across two lines, with no `backlog/nope.md` at the merge base or at `HEAD`
- WHEN `clm gate --base main` runs
- THEN it reports `gate/resolves-named-items` naming `c` and `nope` only, and exits 1

#### Scenario: item deleted in the same PR
- GIVEN `main` with `backlog/x.md`, and a branch that adds `x` to change `c`'s `resolves` and deletes `backlog/x.md`
- WHEN `clm gate --base main --approved` runs
- THEN it doesn't report `gate/resolves-named-items`

#### Scenario: item deleted by a task
- GIVEN `main` with change `c` whose `resolves` is `[x]`, and a branch that changes code and deletes `backlog/x.md`
- WHEN `clm gate --base main` runs
- THEN it prints `code`, doesn't report `gate/resolves-named-items`, and exits 0

#### Scenario: typo in a change archived in its spec PR
- GIVEN a branch that adds `changes/archive/c/change.md`, with `status: done` and `resolves: [nope]`, and no `backlog/nope.md` at the merge base or at `HEAD`
- WHEN `clm gate --base main` runs
- THEN it reports `gate/resolves-named-items` naming `c` and `nope`, and exits 1

#### Scenario: live change moved to the archive with edits
- GIVEN `main` with change `c` outside the archive whose `resolves` is `[x]`, and no `backlog/x.md`, and a branch that moves `changes/c/` to `changes/archive/c/` and edits a line of its `## Intent`
- WHEN `clm gate --base main --approved` runs
- THEN it doesn't report `gate/resolves-named-items`

#### Scenario: entry written as a claim path
- GIVEN `main` with `backlog/x.md`, and a branch that adds `changes/c/change.md` with `resolves: [backlog/x]`
- WHEN `clm gate --base main` runs
- THEN it reports `gate/resolves-named-items` naming `c` and `backlog/x`, and exits 1

#### Scenario: entries judged on odd names
- GIVEN a branch that adds `changes/c/change.md` with `resolves: [sub/z]` and adds `backlog/sub/z.md`; one where it holds `resolves: [café]` and adds `backlog/café.md`; and one that adds a `changes/c/change.md` whose frontmatter isn't valid YAML
- WHEN `clm gate --base main` runs on each
- THEN the first reports `gate/resolves-named-items` naming `c` and `sub/z`, and the second and third report none

#### Serves
- capability/keep-a-backlog
- capability/catch-drift

### Requirement: approval-setting
When the PR's diff lists `clm.toml` (the file at the CLM root, matched as the gate matches `specs/` and `docs/` (`paths-under-the-root`), and the file `ledger verify` audits lands by) and the change alters the value of its top-level `approval` key, that file SHALL count as a spec change that needs review (`needs-review`), so the policy deciding whose approval counts changes only with an approval (`approval-setting-is-spec`). It doesn't become a spec path: the list of spec paths stays as it is, and the file is reported as any other spec change is, with no rule of its own; only `files-outside-spec-paths` leaves it off the files it returns, so such a change can land on a `spec/*` branch. The gate SHALL compare the key's value as a parsed TOML value in the root's `clm.toml` at the merge base and at `HEAD`, a missing file or a missing key counting as unset: so editing a comment, the layout or another key changes nothing, setting `approval = "any"` where it was unset is a change, deleting a file that set it is a change, and a value `config` would reject, such as `approval = 1`, is compared like any other value. When the diff lists `clm.toml` and either side's text isn't valid UTF-8 or doesn't parse as TOML, the file SHALL count as a spec change that needs review, so the gate fails closed. Any other change to `clm.toml` SHALL NOT count as a spec change, and `clm.local.toml` is never compared.

#### Scenario: code branch weakens the policy
- GIVEN `main`'s `clm.toml` sets `approval = "other"`, and a branch changes it to `approval = "any"` and changes nothing else
- WHEN `clm gate --base main` runs
- THEN it prints `spec review`, reports `gate/approval` and exits 1

#### Scenario: policy set where it was unset
- GIVEN `main`'s `clm.toml` doesn't set `approval`, and a branch adds `approval = "other"` to it and changes nothing else
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exits 1

#### Scenario: approved policy change
- GIVEN the branch of *code branch weakens the policy*
- WHEN `clm gate --base main --approved` runs
- THEN it prints `spec review` and exits 0

#### Scenario: policy change with code
- GIVEN a branch that changes `clm.toml`'s `approval` and `crates/x/src/lib.rs`
- WHEN `clm gate --base main` runs
- THEN it prints `mixed` and exits 1

#### Scenario: other setting changed
- GIVEN `main`'s `clm.toml` sets `approval = "other"`, and a branch changes its `[land] check` and a comment, leaves `approval` as it was, and changes `crates/x/src/lib.rs`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0

#### Scenario: unparseable clm.toml
- GIVEN a branch that changes `clm.toml` so that it isn't valid TOML, and changes nothing else
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exits 1

#### Serves
- capability/approve-intent
- architecture/approval-policy
- architecture/humans-review-intent

### Requirement: evaluate-at
`clm-gate` SHALL offer `evaluate_at(repo, base, head, approved)`, which SHALL return, as `clm gate --base <base>` gives them with `head` checked out, an `Outcome` holding the kind (`Kind`), whether the branch needs review, whether a `light` or `mixed` branch would need review were it a `spec` branch, by `needs-review`'s rule under the version applied, over the same diff (for any other kind, whether it needs review), the cuts (`cuts`), the violations and the branch's component (`own-files`, `light`), reading `head` only as a revision, so that what is checked out, the working tree and the index change nothing. A violation of `gate/classify` or `gate/approval`, and a `gate/up-to-date` for a conflict, SHALL name as its path the range `<base>..<head>`, the two revisions written as they were given, and every other violation its file. When git fails, as for a `base` or `head` that names no commit, `evaluate_at` SHALL return an error holding git's message, and `clm gate` SHALL write `clm gate: <message>` to standard error and nothing to standard output and exit 2. `clm gate` SHALL be `evaluate_at` with `HEAD`.

#### Scenario: another commit
- GIVEN a repository with `main`, a commit `s` on top of it that only adds a requirement to `specs/a/SPEC.md`, and `HEAD` checked out at `main` with an uncommitted edit to `ARCHITECTURE.md`
- WHEN `evaluate_at(repo, "main", s, false)` runs
- THEN it returns kind `spec` needing no review, as `clm gate --base main` prints with `s` checked out, and `HEAD`, the index and the working tree are as they were

#### Scenario: a merge commit against its first parent
- GIVEN a merge commit `m` of a branch that only changes `ARCHITECTURE.md` into `main`, with `main` as its first parent `p`
- WHEN `evaluate_at(repo, p, m, false)` runs
- THEN it returns kind `spec` needing review, with `gate/approval` among its violations

#### Scenario: a base git can't resolve
- GIVEN a repository with no ref `no-such-ref`
- WHEN `evaluate_at(repo, "no-such-ref", "HEAD", false)` runs, and `clm gate --base no-such-ref` runs
- THEN the first returns an error, and the second exits 2 with nothing on standard output and standard error starting `clm gate: `

#### Scenario: what a spec part would need
- GIVEN a branch that changes `ARCHITECTURE.md` and `crates/x/src/lib.rs`; one that changes the Contract of `specs/a/SPEC.md` and `crates/x/src/lib.rs`; and the branch of *scenarios added with their code* (`light`)
- WHEN `evaluate_at` runs on each
- THEN the first two return kind `Mixed` and review false, the first with `spec_part_review` true and the second false, and the third kind `Light` with both false

#### Scenario: the range a line names
- GIVEN a branch changing `specs/a/SPEC.md`'s Contract and `crates/x/src/lib.rs`; and a branch that changes a line of `crates/a.rs` that `main` changed otherwise
- WHEN `clm gate --base main` runs on each
- THEN the first prints a line starting `main..HEAD: gate/classify: `, and the second one starting `main..HEAD: gate/up-to-date: `

#### Needs
- ledger/approved-constructor
- check.coverage/test-lists

#### Serves
- capability/land-safely
- architecture/git-is-truth

### Requirement: base-specs-as-checkout
Where the gate reads the components of a commit, the merge base's for the component lookup of `code-fix-resolves-bug` and for `own-files`, and both sides' for `cuts` and `light`, it SHALL read `specs/` and the `change.md` of each live change as that commit holds them (`declared-keys-loaded`). Each file SHALL then hold the bytes a checkout of that commit writes (`extract-as-checkout`): a path marked `export-ignore` and a file marked `export-subst` in that commit's `.gitattributes` SHALL load as a checkout writes them. A commit without `specs/` SHALL load no component and fail nothing, whatever archived changes it holds, and a live change whose path isn't UTF-8 SHALL be left out of the load without failing the gate.

#### Scenario: specs marked export-ignore
- GIVEN `main` with component `a`, whose `paths` hold `src/a/**`, a `kind: bug` item `backlog/b.md` naming `a`, and a `.gitattributes` holding `specs/** export-ignore`, and a branch that changes `src/a/x.rs` and deletes `backlog/b.md` in a commit with the trailer `Resolves: backlog/b`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0, as it does without the attribute

#### Scenario: a spec marked export-subst
- GIVEN `main` with component `a`, whose `paths` hold `src/a/**` and whose Contract holds `$Format:%H$`, a `kind: bug` item `backlog/b.md` naming `a`, and a `.gitattributes` holding `specs/** export-subst`, and a branch that changes `src/a/x.rs` and deletes `backlog/b.md` in a commit with the trailer `Resolves: backlog/b`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0, since component `a` loads as a checkout writes it

#### Scenario: a merge base with no components to load
- GIVEN the repository of *code fix deletes its bug* (`backlog-additions`), changed in turn so that `main` holds no `specs/`; holds no `specs/` and only the archived change `changes/archive/old/`; and holds a live change whose directory's name isn't UTF-8
- WHEN `clm gate --base main` runs on each
- THEN none fails: the first two print `mixed`, since no component exempts the deletion, and the third prints `code`

#### Serves
- capability/keep-a-backlog
- architecture/fold-small-defer-large

#### Needs
- ledger/approved-constructor

### Requirement: declared-keys-loaded
When the gate loads the specs at the merge base (`base-specs-as-checkout`), it SHALL load them with the `change.md` of each live change there, so a component frontmatter key that a live change declares in `frontmatter_keys` is accepted as `clm check` accepts it (`declared-frontmatter-keys`), and a component that sets such a key is loaded, never missing for it.

#### Scenario: key declared on main
- GIVEN `main` with `changes/y/change.md`, `applied` with `frontmatter_keys: [colour]`, component `a` whose frontmatter sets `colour: blue` and whose `paths` hold `src/a/**`, and a `kind: bug` item `backlog/b.md` naming `a`, and a branch that changes `src/a/x.rs` and deletes `backlog/b.md` in a commit with the trailer `Resolves: backlog/b`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0

#### Serves
- capability/keep-a-backlog
- architecture/fold-small-defer-large

### Requirement: approval-changed
`clm-gate` SHALL offer `approval_changed(before, after)`, each the bytes of `clm.toml` at one side of a diff (`Option<&[u8]>`) or `None` where the file doesn't exist, returning whether `approval-setting` counts the file as a spec change: true exactly when the top-level `approval` values, compared as parsed TOML with a missing file or key counting as unset, differ, or either side's bytes aren't UTF-8 or don't parse as TOML. It SHALL run no process and SHALL build for `wasm32-unknown-unknown`, and the gate SHALL count `clm.toml` as `approval-setting` says exactly when `approval_changed` returns true for the file's bytes at the merge base and at `HEAD`, so the gate and `web.worker` give one answer.

#### Scenario: the value changes
- GIVEN `before` setting `approval = "other"` and `after` setting `approval = "any"`
- WHEN `approval_changed(Some(before), Some(after))` runs
- THEN it returns true, as it does for `(None, Some(after))` and `(Some(before), None)`

#### Scenario: only a comment changes
- GIVEN `before` and `after` that set the same `approval` and differ in a comment and in `[land] check`
- WHEN `approval_changed(Some(before), Some(after))` runs
- THEN it returns false, as it does for `(None, None)`

#### Scenario: a side doesn't parse
- GIVEN `after` that isn't valid TOML, and `after2` that isn't valid UTF-8
- WHEN `approval_changed(Some(before), Some(after))` and `approval_changed(Some(before), Some(after2))` run
- THEN both return true

#### Scenario: the gate agrees
- GIVEN the branch of *code branch weakens the policy* under `approval-setting`
- WHEN `clm gate --base main` runs
- THEN it prints `spec review`, the answer `approval_changed` gives for the two texts

#### Serves
- capability/approve-intent
- capability/follow-from-browser
- architecture/approval-policy

### Requirement: needs-review-of
`clm-gate` SHALL offer `needs_review_of(paths, approval_changed, changes)`, where `paths` are the paths a branch's diff adds, modifies or deletes, `approval_changed` is whether `approval-setting` counts `clm.toml` as changed, and `changes` holds, for each change one of whose files the branch adds or modifies, its `change.md` frontmatter at `HEAD`, or `None` when that file is missing, isn't UTF-8 or its frontmatter doesn't parse. It SHALL return whether a `spec` branch with that diff needs review by the part of `needs-review`'s rule that the paths and the changes decide: true exactly when a path is `ARCHITECTURE.md`, `CAPABILITIES.md` or `docs/allowed_signers`, or a path under one of them, or `approval_changed` is true, or an entry of `changes` is `None` or declares a non-empty `human_burdens`. It SHALL run no process, SHALL build for `wasm32-unknown-unknown`, and `clm-gate` SHALL offer beside it `widened_exemptions(base, head)`, over the spec graphs (`clm_model::Graph`) at the merge base and at the head, returning each exemption a component of `head` holds whose `files` aren't all among the `files` of an exemption with the same `rule` and `until` that the component of that ID holds in `base`, running no process and building for wasm too. `evaluate_at` SHALL decide a `spec` branch's review as `needs_review_of`, or `widened_exemptions` returning anything, or `cuts` returning anything (`cuts`), and by nothing else, so the gate and `web.worker`, which calls the same three over the files it fetches, give one answer, except for a branch `rename/replay` accepts, whose review `evaluate_at` decides by `needs-review`'s rule for such a branch, and a branch with no spec change, which `needs-review` says needs none.

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

#### Scenario: the gate agrees
- GIVEN a repository and four branches: one that only edits `ARCHITECTURE.md`, one that only widens an exemption of `a`, one that only removes a requirement of `a`, and one that only changes `a`'s Contract
- WHEN `evaluate_at` runs on each, and `needs_review_of`, `widened_exemptions` and `cuts` run over the same paths, changes and graphs
- THEN `evaluate_at` returns review exactly where one of the three says so: for the first three and not the last

#### Serves
- capability/approve-intent
- capability/follow-from-browser
- architecture/humans-review-intent

#### Needs
- model/parse-changes
- model/exemptions
- check.coverage/test-lists

### Requirement: fingerprint-reexported
`clm-gate` SHALL offer `gate.fingerprint`'s `Entry`, `parse_ls_tree`, `fingerprint_of`, `adds_change`, `added_changes` and `fingerprint` under the same names, each the item `clm-fingerprint` offers, so a call through `clm_gate` returns what the same call through `clm_fingerprint` returns.

#### Scenario: same function through the gate
- GIVEN two tree listings parsed with `clm_gate::parse_ls_tree`, in which one blob's ID differs
- WHEN `clm_gate::fingerprint_of` and `clm_fingerprint::fingerprint_of` run on them
- THEN both return the same fingerprint

#### Serves
- architecture/slim-components
- architecture/reuse-first

#### Needs
- gate.fingerprint/fingerprint

### Requirement: item-components
Where the gate reads a backlog item's components at a revision, for the code-fix exemption of `backlog-additions` (`code-fix-resolves-bug`), it SHALL read them from the item's `components`, or from its old key (`component-words`) when the item sets that key instead (`model/component-keys`), so an item written under either key is judged the same. An item at the merge base that sets both keys, which the model rejects, SHALL NOT be exempt, as an item whose frontmatter doesn't parse isn't, and nor SHALL one whose value under the key it sets isn't a list of strings.

#### Scenario: a bug filed under the new key
- GIVEN the repository of *code fix deletes its bug* (`backlog-additions`), with `backlog/x.md` naming its component as `components: [a]`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0

#### Scenario: a bug filed under the old key
- GIVEN the repository of *code fix deletes its bug*, with `backlog/x.md` naming `a` under the old key alone; under both keys; and as `components: a`, a string and not a list
- WHEN `clm gate --base main` runs on each
- THEN the first prints `code` and exits 0, and the second and third print `mixed` and exit 1

#### Needs
- model/component-keys

#### Serves
- architecture/plain-terms
- architecture/humans-review-intent

### Requirement: files-outside-spec-paths
`clm-gate` SHALL offer `files_outside_spec_paths(repo, base, head)`, returning the files `head` changes against `base`, from the merge base as `classify` reads them, that aren't spec paths, sorted, and returning none when `rename/replay` accepts the revision. It SHALL leave out `clm.toml` when `approval-setting` counts it as a spec change, so a branch that changes whose approval counts, which needs review, can land as a `spec/*` branch, and when `upgrade-stamp-left-out` leaves it out; any other change to `clm.toml` it SHALL return. It SHALL read the revision only through git, never the working tree or the index, as `gate/evaluate-at` does.

#### Scenario: a code file among spec files
- GIVEN a revision changing `specs/a/SPEC.md` and `crates/x/src/lib.rs` against its base
- WHEN the call runs with something else checked out
- THEN it returns `crates/x/src/lib.rs` alone

#### Scenario: only a new backlog item
- GIVEN a revision that only adds `backlog/x.md`, for which `clm gate` prints `spec`
- WHEN the call runs
- THEN it returns nothing

#### Scenario: a rename branch
- GIVEN the committed result of `clm rename r`, which rewrote a marker in `crates/a/tests/t.rs`
- WHEN the call runs
- THEN it returns nothing

#### Scenario: an approval change
- GIVEN a base whose `clm.toml` sets `approval = "other"`, and a revision that changes it to `approval = "any"` and changes `specs/a/SPEC.md`
- WHEN the call runs
- THEN it returns nothing

#### Scenario: another setting changed
- GIVEN a base whose `clm.toml` sets `approval = "other"`, and a revision that changes its `[land] check` and leaves `approval` as it was
- WHEN the call runs
- THEN it returns `clm.toml`

#### Scenario: only the upgrade stamp changed
- GIVEN a revision that changes only `upgraded` in `clm.toml` and edits `CLAUDE.md`
- WHEN the call runs
- THEN it returns nothing

#### Serves
- capability/contribute-from-outside
- capability/upgrade-clm

### Requirement: upgrade-stamp-left-out
When the diff lists `clm.toml` and its parsed TOML at the merge base and at `HEAD` differ only in the top-level keys `upgraded` and `declined_migrations`, a missing file or key counting as unset, the gate SHALL leave the file out of the files it classifies, and it SHALL NOT make the branch need review. A branch whose only changed file is such a `clm.toml` SHALL classify as `spec`. `clm-gate` SHALL offer `upgrade_stamp_only(before, after)`, of the two sides' bytes as `approval_changed` takes them, true exactly when the file differs only in those two keys and false when either side isn't UTF-8 or doesn't parse, and the gate SHALL leave the file out exactly when it returns true. The function SHALL run no process and SHALL build for `wasm32-unknown-unknown`. Any other change to `clm.toml` SHALL be judged as `approval-setting` says.

#### Scenario: notes taken
- GIVEN a branch that edits `CLAUDE.md` and changes only `upgraded` and `declined_migrations` in `clm.toml`
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and exits 0

#### Scenario: mechanical migrations taken
- GIVEN a branch that adds a line to `.gitignore` and changes only `upgraded` in `clm.toml`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0

#### Scenario: the stamp and another key
- GIVEN a branch that edits `CLAUDE.md` and changes `upgraded` and `[land] check` in `clm.toml`
- WHEN `clm gate --base main` runs
- THEN it prints `mixed` and exits 1

#### Serves
- capability/upgrade-clm

### Requirement: settings-unchanged
`clm-gate` SHALL offer `settings_unchanged(before, after)`, of `clm.toml`'s bytes on each side of a diff as `upgrade_stamp_only` takes them (`None` where the file doesn't exist), true exactly when both sides are UTF-8 and parse as TOML and their tables are equal once the top-level `upgraded` and `declined_migrations` are removed, a missing file counting as an empty table. So a change to comments, formatting or those two keys alone, or no change, gives true. `upgrade_stamp_only` SHALL return `settings_unchanged` and a difference between the parsed tables, so the two read settings alike. The function SHALL run no process and SHALL build for `wasm32-unknown-unknown`.

#### Scenario: the stamp only
- GIVEN a `clm.toml` and the same file with `upgraded` changed
- WHEN `settings_unchanged` runs on the two
- THEN it returns true, as `upgrade_stamp_only` does

#### Scenario: a comment only
- GIVEN a `clm.toml` and the same file with a commented-out table appended
- WHEN `settings_unchanged` and `upgrade_stamp_only` run on the two
- THEN `settings_unchanged` returns true and `upgrade_stamp_only` false

#### Scenario: a setting
- GIVEN a `clm.toml` and the same file with `[land] check` changed
- WHEN `settings_unchanged` runs on the two
- THEN it returns false

#### Scenario: unreadable
- GIVEN a `clm.toml` and a side that doesn't parse as TOML
- WHEN `settings_unchanged` runs on the two
- THEN it returns false

#### Serves
- capability/upgrade-clm

### Requirement: agents-md-spec-path
The gate SHALL count `AGENTS.md` at the CLM root as a spec path, as it counts `CLAUDE.md` (`classify`, `files-outside-spec-paths`), since it holds the project's instructions for every agent harness (`platform`).

#### Scenario: instructions edit
- GIVEN a branch that edits only `AGENTS.md`
- WHEN `clm gate` judges it
- THEN its first line is `spec`

#### Serves
- capability/work-from-any-agent-harness

### Requirement: exemption-names-its-item
The gate SHALL block a `spec` branch holding an exemption that `needs-review` counts as added or widened (`model/exemptions`) whose `until` names no backlog item that `backlog/<slug>.md` holds at `HEAD`, reporting `<file>: gate/exemption-names-its-item: exemption from <rule> until backlog/<slug> names no backlog item; add it` for each, so every exemption starts with the work that ends it in the queue (`temporary-exemptions`). An exemption whose item is deleted later SHALL NOT block a branch: `check.health` reports it.

#### Scenario: no item
- GIVEN a `spec` branch that adds to `specs/a/SPEC.md` an exemption from `skipped-tests` until `backlog/x`, with no `backlog/x.md`
- WHEN the gate runs
- THEN it exits 1 with the line `specs/a/SPEC.md: gate/exemption-names-its-item: exemption from skipped-tests until backlog/x names no backlog item; add it`

#### Scenario: item added with it
- GIVEN the same branch, also adding `backlog/x.md`
- WHEN the gate runs
- THEN it reports no `gate/exemption-names-its-item` violation

#### Serves
- architecture/exemptions-end

#### Needs
- model/exemptions

### Requirement: own-files
`clm-gate` SHALL offer `foreign_files(repo, base, head, component)`, returning, when the root component at the merge base of `base` and `head` sets `wiring` (`model/wiring-globs`) and the specs there hold `component`, each code file (one outside the spec paths) that `head` adds, modifies or deletes against the merge base and that is none of:
- a file `component` owns, by `check.coverage/owner` over the specs at the merge base;
- a file no component owns, which `check/unowned-files` judges;
- a file one of the root's `wiring` globs matches;
- a file `head` modifies, without adding or deleting it, whose text at the merge base holds a marker citing `capability/<slug>` anywhere in it, matched by the marker's whole ID whatever version it cites, so `capability/seeing` is not `capability/see`, for a capability that the effective `serves` of one of `component`'s requirements names (its own `#### Serves` block when it holds one, its component's `serves` otherwise);

each with the component that owns it, sorted by path. It SHALL return nothing when the root sets no `wiring` or the specs at the merge base hold no such component. It SHALL read `head` only as a revision, as `evaluate-at` does.

When the root at the merge base sets `wiring`, a `code` branch's *component* SHALL be read from its code files: of the components `check.coverage/owner` gives, over the specs at the merge base, for its code files that no `wiring` glob matches, the first by ID for which `foreign_files`, leaving out the files outside the CLM root, returns nothing. A file outside the CLM root is no component's. A `code` branch none of whose code files a component owns outside the `wiring` has no component, and nor has any `code` branch when the root sets no `wiring`. The specs at the merge base SHALL be read as they load, a load violation failing nothing and `evaluate_at` returning no error for it: a component whose `SPEC.md` doesn't parse there owns no file, and a root whose `SPEC.md` doesn't parse there names no `wiring`, so then no `code` branch has a component and none is blocked. `evaluate_at` SHALL return a `code` branch's component, a `light` branch's (`light`), and none for a `spec` or `mixed` branch. When the root at the merge base sets `wiring`, the gate SHALL block a `code` branch that has a code file a component owns outside the `wiring` and no component, with rule `gate/own-files`, with `--approved` or without, one line for each such file, sorted by path: `<file>: gate/own-files: owned by <owner>; a code branch changes one component's files, the wiring and a capability test that component serves`.

#### Scenario: another component's source
- GIVEN a root with `wiring: []`, components `a` with `paths: ["crates/a/**"]` and `b` with `paths: ["crates/b/**"]`, and a branch changing `crates/a/src/lib.rs` and `crates/b/src/lib.rs`
- WHEN `foreign_files` runs for `a`, and `clm gate --base main` runs
- THEN the first returns `crates/b/src/lib.rs` with owner `b`, and the gate prints `code`, then `gate/own-files` lines for `crates/a/src/lib.rs`, owned by `a`, and `crates/b/src/lib.rs`, owned by `b`, and exits 1

#### Scenario: wiring
- GIVEN the same with the root's `wiring: ["Cargo.lock"]` and a branch changing `crates/a/src/lib.rs` and `Cargo.lock`, which the root owns
- WHEN `foreign_files` runs for `a`
- THEN it returns nothing

#### Scenario: a test that cites the component in another's file
- GIVEN the same with `impl/a` adding a test citing `a/x` to `crates/b/tests/cli.rs`, and deleting `crates/b/tests/old.rs`, which cited `a/y` at the merge base
- WHEN `foreign_files` runs for `a`
- THEN it returns both files with owner `b`: a marker gives a build no file

#### Scenario: a capability test the component serves
- GIVEN the same, `a/x` serving `capability/see`, a component `capability` owning `tests/capability/see.rs`, which cites `capability/see`, and a branch modifying that file
- WHEN `foreign_files` runs for `a`, and again when the branch deletes the file instead
- THEN the first returns nothing, and the second returns the file with owner `capability`

#### Scenario: not switched on
- GIVEN a root with no `wiring`, and a branch changing `crates/b/src/lib.rs`
- WHEN `foreign_files` runs for `a`
- THEN it returns nothing

#### Scenario: an unknown component
- GIVEN a root with `wiring: []` and no component `z`
- WHEN `foreign_files` runs for `z`
- THEN it returns nothing

#### Scenario: the component read from the files
- GIVEN the root, `a`, `b` and `capability` of the scenarios above, with `wiring: ["Cargo.lock"]`, and branches that change, in turn: `crates/a/src/lib.rs` alone; it and `tests/capability/see.rs`, modified; `Cargo.lock` alone; `crates/a/src/lib.rs` and `crates/b/src/lib.rs` in a root that sets no `wiring`; and, in a repository whose CLM root is `tool/` and sets `wiring: []`, `tool/crates/a/src/lib.rs` and `b` at the repository's top
- WHEN `evaluate_at` runs on each, and `clm gate --base main` in the root
- THEN the first two print `code`, exit 0 and return component `a`; the third and fourth print `code`, exit 0 and return none; and the last prints `code`, exits 0 and returns component `a`

#### Scenario: specs that don't load at the merge base
- GIVEN the root, `a` and `b` of the scenarios above with `wiring: []`, and a branch changing `crates/a/src/lib.rs` and `crates/b/src/lib.rs`, on a `main` where in turn `b`'s `SPEC.md` frontmatter isn't valid YAML, and the root's `SPEC.md` frontmatter isn't
- WHEN `evaluate_at` runs on each, and `clm gate --base main`
- THEN neither errs: the first prints `code`, exits 0 and returns component `a`, and the second prints `code`, exits 0 and returns none

#### Serves
- architecture/path-ownership
- capability/land-safely

#### Needs
- ledger/approved-constructor
- model/wiring-globs
- orchestrator.run/pending-prompt

### Requirement: commands-in-crate
`clm-gate` SHALL offer `clm gate` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. Given `--help` it SHALL write the command's help to the output stream, nothing to the error stream, and return 0: its first line the description `Decide whether HEAD may merge into a base ref; exit 1 to block`, then an empty line, then `Usage: clm gate [OPTIONS] --base <BASE>`, and the options `--base`, `--approved` and `--root`. It SHALL print a violation as the line `clm-model` gives for it (`model/violation-line`), since this crate does not depend on `check`.

#### Scenario: a command called in a test
- GIVEN a fixture repository
- WHEN the call runs with the arguments of `clm gate` and two buffers
- THEN the buffers hold what `clm gate` prints to standard output and to standard error there, and the call returns the status `clm gate` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm gate` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Scenario: help
- GIVEN the same call
- WHEN it is given `--help`
- THEN it returns 0 with nothing in the error buffer, and its output buffer starts with `Decide whether HEAD may merge into a base ref; exit 1 to block`, an empty line and `Usage: clm gate [OPTIONS] --base <BASE>`, and names `--base`, `--approved` and `--root`

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

#### Needs
- model/violation-line

### Requirement: cuts
`clm-gate` SHALL offer `cuts(base, head, lists)`, running no process and building for `wasm32-unknown-unknown`, where `base` and `head` are the spec graphs (`clm_model::Graph`) at a branch's merge base and at its head, each loaded from that side's `specs/`, `ARCHITECTURE.md`, `CAPABILITIES.md` and live `changes/<id>/change.md` files, so a rule's `serves` resolves as `clm check` resolves it, and `lists` holds, for each change with a `deleted-tests.txt` at the head, `changes/<id>/deleted-tests.txt` or `changes/archive/<id>/deleted-tests.txt`, that path and the file's text at the head and at the merge base, where an archived list with no text of its own there takes the text of the live `changes/<id>/deleted-tests.txt`, so an archive move adds no line. It SHALL return what the branch cuts, each a `Cut` that prints as its line below, in this order, the first four kinds each by requirement ID and the second and third also by the scenario's place in its requirement:
- a *gone requirement*: one `base` holds and `head` doesn't, unless each of its scenarios is moved;
- a *gone scenario*: a scenario of a requirement both graphs hold whose name the head's requirement doesn't hold, unless it is moved;
- a *changed outcome*: a scenario of a requirement both graphs hold whose name the head's requirement holds, whose outcome differs from that of the head's scenario of that name, the n-th scenario of a name at the base compared with the n-th of that name at the head, and one with no n-th of its name at the head compared with an empty outcome, so a duplicate the head loses is listed; changed outcomes are ordered by the scenario's place at the base;
- a *shortened requirement*: one both graphs hold whose statement, its text above its first `####` heading, has fewer stating sentences at the head than at the base, with both counts;
- a *test to delete*: each list line the head's text holds and the base's text doesn't, with its path, in the paths' order and then the file's.

A scenario's name is the text after `#### Scenario: ` in its requirement's block. Its *outcome* is the text of its lines from the first whose text, leading white space left out, starts `- THEN`, case kept, to the end of its block, the next `####` heading outside a fenced code block or the requirement's end, a fenced code block read as any other lines, with each run of Unicode white space, a line break included, read as one space and none at either end; a scenario with no such line has an empty outcome. A scenario is *moved* when `head` holds a scenario of the same name under a requirement of another ID that held no scenario of that name in `base`; each such scenario of `head` SHALL take one gone scenario and no more, the gone scenarios taken in the order of their requirement IDs and then of their place. A *sentence* ends at a `.`, `?` or `!` that whitespace, a line break included, or the statement's end follows. The placement phrases are `SHALL sit`, `sat in` and `is left in`, each matched as whole words and without regard to case, with any run of Unicode white space, a line break included, read as the one space between two of its words, so `is left intact` holds none and a sentence with `SHALL` at the end of one line and `sit` at the start of the next holds `SHALL sit`. A sentence *places a test* when it holds one of the placement phrases and also the word `test` or `tests`, matched as a whole word, bounded by characters other than letters, digits and `_`, without regard to case, so `Tests` and the `tests` of `crates/clm/tests/` are the word and `places_a_test` and `tested` hold none; a sentence that holds a phrase and neither word, such as one deleting a directory once nothing else is left in it, places no test. A *stating sentence* is one that holds `SHALL` as a whole word in capitals, as the spec format writes it, and places no test. `clm-gate` SHALL offer `sentences(statement)`, returning every sentence of a statement that isn't empty once trimmed, in order, `stating_sentences(statement)`, returning its stating sentences, and `places_a_test(sentence)`, returning the first placement phrase of a sentence that places a test and nothing for any other sentence. A *list line* is `DELETE <file>::<name>`, the file ending at the first `::` and the name at a tab or at the line's end, neither of them empty, a carriage return that ends the line left out; a blank line and a line whose first character is `#` are skipped, and any other line is neither; `clm-gate` SHALL offer `list_lines(text)`, returning each list line's file and name. A list line the head's text holds whose file and name a list line of the base's text holds adds no *test to delete*, whatever follows its tab, and one the head holds twice is listed once. A list line names a test when the files agree and the names agree as `check.coverage`'s `bare_name` gives them.

For a `spec` branch, `clm gate` SHALL print one line per cut after its first line and before any violation, `evaluate_at` returning the cuts beside its verdict:
- `specs/<component path>/SPEC.md: gate/cut: requirement <id> is gone`;
- `specs/<component path>/SPEC.md: gate/cut: scenario "<name>" of <id> is gone`;
- `specs/<component path>/SPEC.md: gate/cut: scenario "<name>" of <id> changes its outcome`;
- `specs/<component path>/SPEC.md: gate/cut: <id> states <n> sentences where it stated <m>`;
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

### Requirement: rules-version
`clm-gate` SHALL offer `RULES`, a whole number that SHALL be 7, and `evaluate_by(repo, base, head, approved, rules)`, returning what `evaluate_at` returns as the gate decided under the rules of version `rules`, and `evaluate_at` SHALL be `evaluate_by` with `RULES`. Under version 7 a branch `light` takes SHALL be `light`, and under versions 1 to 6 it SHALL be `mixed`, blocked with `gate/classify`, and under those versions no `gate/classify` line SHALL say anything of `light`. Under versions 5 to 7 the gate SHALL read the paths a branch changes as `paths-under-the-root` states, and under versions 1 to 4 as `git diff` prints them, relative to the git repository's top, each file at a revision read as `<rev>:<path>`, as those versions did. Under versions 6 and 7 a `spec` branch SHALL need review as `needs-review` states, under versions 4 and 5 as it states with a *shortened requirement* cut counted as though a placement phrase's words matched only where one space separates each two of them, under version 3 as under version 4 with a *shortened requirement* cut counted as though every sentence holding a placement phrase, so matched, placed a test, whether or not it holds the word `test` or `tests`, under version 2 as under version 3 with a *changed outcome* cut making no branch need review, and under version 1 as it states with `cuts` making no branch need review, specs that don't load at the merge base or the head included, a branch `rename/replay` accepts keeping its own rule under each. In a repository whose CLM root is its top, the kind and the cuts SHALL be the same under every version but for a branch `light` takes, and the violations too but for `gate/approval`, which follows the review, and the `gate/classify` line, which under versions 1 to 6 says nothing of `light`. In one whose CLM root is a subdirectory, the kind, the cuts and the violations under versions 1 to 4 SHALL be those their reading of the paths gives. For a `rules` below 1 or above `RULES`, `evaluate_by` SHALL return an error naming `rules` and `RULES`.

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

### Requirement: paths-under-the-root
The gate SHALL read each path a branch changes relative to the CLM root, the directory it runs in (`root-from-cwd`), as `gate.fingerprint/fingerprint` reads its trees (`fingerprint-covers-the-clm-root`), whatever directory of its git repository that root is. Each path its rules name, the spec paths, `clm.toml`, `docs/allowed_signers`, `changes/<id>/`, `backlog/<slug>.md` and a component's `paths` among them, SHALL match the root's file, and each read of a changed file at the merge base or the head SHALL read that file. A changed file outside the root SHALL be named relative to the root, as the shortest path from the root to it, climbing out with `../` (`../b` for a file `b` at the top of a repository whose root is `tool/`), and SHALL count as a code file, never a spec path, so a spec branch that also changes one is `mixed` and blocked, and no approval, whose fingerprint covers only the root's files, lands one (`approval-policy`). `files-outside-spec-paths` SHALL return such a file in that form, and `foreign_files` SHALL return it, with an empty owner, among the files `component`'s build isn't to change, counting it as no component's whatever a component's `paths` match, since no component's `paths` can name it and `check/unowned-files` never sees it. In a repository whose CLM root is its top, every path is read as `git diff` prints it.

#### Scenario: spec branch in a subdirectory root
- GIVEN a git repository whose CLM root is its directory `tool/`, and a branch that only edits `tool/ARCHITECTURE.md`
- WHEN `clm gate --base main` runs in `tool/`
- THEN it prints `spec review`, reports `gate/approval` and exits 1

#### Scenario: a file outside the root
- GIVEN that repository, and a branch that changes `tool/specs/a/SPEC.md` and `b` at the repository's top
- WHEN `clm gate --base main` runs in `tool/`
- THEN it prints `mixed` and exits 1

#### Scenario: a file outside the root listed
- GIVEN that branch
- WHEN `files-outside-spec-paths`' call runs with the repository `tool/`
- THEN it returns `../b` alone

#### Scenario: a build changes a file outside the root
- GIVEN that repository, whose root component in `tool/specs/SPEC.md` sets `wiring`, with component `a` owning `crates/a/**`, and a commit `h` changing `tool/crates/a/src/lib.rs` and `b` at the repository's top
- WHEN `foreign_files(tool, "main", h, "a")` runs
- THEN it returns `../b` alone, with an empty owner

#### Scenario: status only in a subdirectory root
- GIVEN that repository, and a branch changing `tool/crates/x/src/lib.rs` and only the `status:` line of `tool/specs/x/SPEC.md`
- WHEN `clm gate --base main` runs in `tool/`
- THEN it prints `code` and exits 0

#### Scenario: a code fix in a subdirectory root
- GIVEN that repository, whose `tool/specs/` holds component `a` with `paths: ["crates/a/**"]`, and `tool/backlog/x.md` with `kind: bug` and `components: [a]`, and a branch that changes `tool/crates/a/src/lib.rs` and deletes `tool/backlog/x.md` in a commit with the trailer `Resolves: backlog/x`
- WHEN `clm gate --base main` runs in `tool/`
- THEN it prints `code` and exits 0

#### Serves
- capability/approve-intent
- architecture/approval-policy
- architecture/humans-review-intent

### Requirement: light
A branch whose changed files hold both spec changes and code files (`classify`) SHALL be `light`, never `mixed`, exactly when each of these holds, compared between the merge base and `HEAD`:
- Its spec changes are the files of one new archived change `changes/archive/<id>/` (`archive-in-spec-pr`), its `change.md` and `deltas/<c>.md` and no other, `specs/<path>/SPEC.md` of the one component `c` it names, and the deletion of each top-level `backlog/<slug>.md` that change's `resolves` lists; and that `change.md` at `HEAD` parses, has `status: done`, declares no `human_burdens` and records no `renames`. Beside these a light branch may hold exactly the files a `code` branch may hold beside its code, which `classify` doesn't count as spec changes, a status-only edit (`status-only-exemption`) among them.
- The spec graphs at both, loaded as `cuts` loads them, load with no violation, and were the branch a `spec` branch it would need no review (`needs-review`) and `cuts` would return nothing.
- Component `c` exists at both with the same frontmatter but for its `status`, the same Contract and Design and the same requirement IDs; each requirement's statement, its text above its first `####` heading, is the same; the `####` blocks a requirement holds at the merge base appear at `HEAD`, each the same and in the same order, with only `#### Scenario:` blocks added, anywhere among or after them; and `HEAD` adds at least one. Texts are compared with each run of white space read as one space and none at either end.
- Every code file is, by `check.coverage/owner` over the specs at the merge base, `c`'s own; a file no component owns, a file outside the CLM root among them; a file one of the root's `wiring` globs matches; or a capability test `c` serves that the branch modifies, as `own-files` reads one. This holds whether or not the root sets `wiring`.
- Each requirement of `c` to which `HEAD` adds a scenario is current at `HEAD`: every marker citing it in the files under the CLM root at `HEAD`, read as `check.coverage/markers-from-files` reads them, a file whose bytes aren't UTF-8 holding none, cites the version `HEAD`'s graph gives it, and at least one does.

A `light` branch SHALL need no review and list no cut, its component SHALL be `c`, and `clm gate` SHALL print `light` for it, with `--approved` or without, and exit 0 unless another violation blocks it. For a branch with spec changes and code files that isn't `light`, the `gate/classify` line SHALL name the first of these conditions, in the order above, that the branch fails, and the file, change, component, requirement or marker at fault.

#### Scenario: scenarios added with their code
- GIVEN `main` with a root setting `wiring: []`, component `a` with `paths: ["crates/a/**"]` whose requirement `a/x` has one scenario and is cited at its version in `crates/a/tests/t.rs`, and a branch `s` that adds `changes/archive/c/change.md`, `status: done`, and `changes/archive/c/deltas/a.md`, adds a second scenario to `a/x` in `specs/a/SPEC.md`, changes `crates/a/src/lib.rs` and cites `a/x` at its new version in `crates/a/tests/t.rs`
- WHEN `clm gate --base main` runs, again with `--approved`, and `evaluate_at(repo, "main", s, false)` runs
- THEN both runs print `light` alone and exit 0, and the call returns kind `Light`, no review, no cut, no violation and component `a`

#### Scenario: what a light branch carries beside it
- GIVEN the branch of *scenarios added with their code*, changed in turn so that: it also changes `a`'s `status:` from `unbuilt` to `active` in `specs/a/SPEC.md`; its change's `resolves` lists `q` and it deletes `backlog/q.md`; it also adds `backlog/found.md`; its only code file is `tests/capability/see.rs`, modified, which a component `capability` owns and `a/x` serves; and the root sets no `wiring`
- WHEN `evaluate_at(repo, "main", s, false)` runs on each
- THEN each returns kind `Light` and component `a`

#### Scenario: a branch light doesn't take
- GIVEN the branch of *scenarios added with their code*, changed in turn so that: it also changes `a`'s Contract; its change is live at `changes/c/` with `status: applied`; it also adds a scenario to `b/y` in `specs/b/SPEC.md`; it also changes `crates/b/src/lib.rs`, `b`'s; `crates/a/tests/u.rs` still cites `a/x` at its old version; it also changes the `- THEN` line of `a/x`'s first scenario
- WHEN `clm gate --base main --approved` runs on each
- THEN each prints `mixed` and exits 1, its `gate/classify` line naming, in turn, `a`, `changes/c/change.md`, `specs/b/SPEC.md`, `crates/b/src/lib.rs` with its owner `b`, `a/x` with `crates/a/tests/u.rs`, and a cut

#### Needs
- check.coverage/markers-from-files
- ledger/approved-constructor

#### Serves
- capability/land-safely
- architecture/proven-by-tests
