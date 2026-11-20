---
component: gate
base: { design: "818ee6ed4579a0db" }
---

## MODIFIED Design
Changes come from `git diff --name-status --no-renames <merge-base>...HEAD`, run in the CLM root. **Paths under the root** (`paths-under-the-root`, `fingerprint-covers-the-clm-root`). `git diff` prints each path relative to the git repository's top, whatever directory it runs in, while every rule the gate applies names a path relative to the CLM root, as the fingerprint an approval is pinned to does. So the gate turns each path any `git diff` prints, filtered by a pathspec or not, into one relative to the root, with `git rev-parse --show-prefix` giving the root's place in the repository: a path under the root loses that prefix, and a path outside it becomes the shortest path from the root to it, climbing out with `../` (`../x` for `a/x` with the root at `a/b/`). Every later read names a path in that form: a pathspec as it stands, since git reads pathspecs relative to the directory it runs in, and a file at a revision as `<rev>:./<path>`, which git reads the same way, never as `<rev>:<path>`, which git reads from the repository's top. `clm-git`'s `extract_paths` is the exception: it takes paths relative to the repository's top and refuses `..`, so the graph loads (the merge base's for the code-fix check and `foreign_files`, both sides' for the cuts) pass the prefix before each root path (`<prefix>specs`, `<prefix>changes/<id>/change.md`) and load the graph from the prefix's directory in the extraction. A file outside the root is no component's, whatever a component's `paths` would match, since `paths` name files under the root. A path outside the root is never a spec path, so it counts as code; that is what keeps `approval-policy` whole for a subdirectory root, whose approvals cover only the root's files. At a repository whose root is its top the prefix is empty and nothing changes. Spec paths are `specs/`, `changes/`, `backlog/`, `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and `AGENTS.md`. File contents at the merge base and at `HEAD` are compared with `git diff <rev>:./<path> <rev>:./<path>`. Uses the `git` CLI, not a git library.

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

**Evaluating another commit** (`gate/evaluate-at`). `evaluate_at(repo, base, head, approved)` is the gate for `head` against `base` as `clm gate --base <base>` gives it with `head` checked out: it reads `head` only as a revision, through `git diff <merge-base>...<head>`, `git show <head>:./<path>`, `git ls-tree <head>`, `git log <merge-base>..<head>` and `git merge-tree --write-tree <base> <head>`, never the working tree or the index, so it gives the same verdict whatever is checked out. A violation's path names the range as `<base>..<head>` with the revisions as given, so `clm gate`'s output stays `<base>..HEAD`. `clm gate` calls it with `HEAD`. `ledger`'s `verify --review` calls `evaluate_by` for each signed land, with the land's first parent as `base`, the merge commit as `head` and the rules in force when it landed, to recompute the land's kind and whether it needed review without a checkout per land.

**A build's own files** (`own-files`, `own-files-at-the-gate`). `path-ownership` was kept by the build prompt alone; a program now refuses what the prompt asked for. The gate stays a function of two trees and one flag and reads no branch name (`spec-branches-hold-spec-paths`), so `foreign_files` takes the component as an argument and `clm land`, which knows the branch, calls it for a branch named `impl/<component>`: no other branch has a component, and a small change on a branch of another name is not judged. The specs it reads are the merge base's, loaded as for the code-fix check; a file's owner is `check.coverage`'s `owner`, which the gate depends on for it. Two kinds of file are the build's to change beside its own. The root's `wiring` names the shared ones. A file that merely cites the component is not one of them: a test sits in the files of the component it proves, so a marker gives a build no file (`commands-live-in-their-crates`); the clause that once opened such a file, for the tests Cargo kept in the root package's directory, went when each crate came to prove its requirements through its own call. A capability test may be edited, never added or deleted, by a build whose component serves the capability it cites, since a behaviour and the test that observes it at the boundary can't land apart (`capability-tests-follow-rules`). That a wiring edit adds only the build's own entry, and that a capability-test edit keeps what its scenarios state, stay judgements no program makes. The rule is off until the root sets `wiring`, as `check`'s path ownership is off until it sets `outside`, so a project adopts it when it has named its wiring.

**Rules versions** (`rules-version`, `audit-by-the-rules-in-force`). The audit of a signed land asks whether its record understates what the gate gave it, so it must ask the gate that judged it, not today's: a rule that makes more diffs need review would otherwise report every honest land made before it as understated and unapproved. `RULES` names the rules `evaluate_at` applies, and `evaluate_by` keeps each earlier version's answer. Version 1 is the gate as it stood before `cuts` made a branch need review; version 2 adds `cuts`; version 3 adds the changed outcome to what makes a branch need review. Version 4 counts a sentence that holds a placement phrase and not the word `test` or `tests` as a stating sentence, so a branch that drops one needs review; the lower versions decide review from a count that leaves out every sentence holding a phrase. Version 5 reads the changed paths relative to the CLM root (**Paths under the root**, above); the lower versions read them as `git diff` prints them, relative to the repository's top, and name a file at a revision as `<rev>:<path>`, as the gate did before, which in a repository whose root is a subdirectory classes a branch editing the root's spec files as `code`. Versions 1 to 4 read every path as those versions did, pathspecs and graph loads included. In such a repository that is the one place a version changes the kind, either way: a branch editing the root's spec files is `code` under the lower versions and `spec` under 5, and one editing a spec-path name at the repository's top outside the root, such as its own `CLAUDE.md`, is `spec` under the lower versions and `code` under 5. The path reading is one function given the version, not a second gate. Apart from it only the review depends on the version, and with it `gate/approval`: the kind, the cuts and the other violations are those of today's gate at every version, since the audit reads only the kind and the review. A requirement can't name its own component in `#### Needs`, so `rules-version` relies on `cuts` by the build of `gate` taking both. A change that makes the gate give some diff a higher kind or review than it did (`code` = `spec` < `spec review` < `mixed`) raises `RULES` by one and keeps the earlier answer behind `rules` for the lower versions, for as long as signed history may hold a land judged by them, which is always; a change that rates diffs lower or the same needs no new version, since the audit reports only understatement. The older answers are a branch on `rules` beside the rule that changed, not a copy of the gate. A `rules` below 1 or above `RULES` is an error naming both, which `ledger` reports as a land it can't rate (`ledger/verify-unrated-land`): a newer `clm` judged it.

## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: approval-setting <!-- base: c9f7ac40a9d3c1ef -->
When the PR's diff lists `clm.toml` (the file at the CLM root, matched as the gate matches `specs/` and `docs/` (`paths-under-the-root`), and the file `ledger verify` audits lands by) and the change alters the value of its top-level `approval` key, that file SHALL count as a spec change that needs review (`needs-review`), so the policy deciding whose approval counts changes only with an approval (`approval-setting-is-spec`). It doesn't become a spec path: the list of spec paths stays as it is, and the file is reported as any other spec change is, with no rule of its own; only `files-outside-spec-paths` leaves it off the files it returns, so such a change can land on a `spec/*` branch. The gate SHALL compare the key's value as a parsed TOML value at the merge base and at `HEAD`, read with `git show <rev>:./clm.toml` in the CLM root, a missing file or a missing key counting as unset: so editing a comment, the layout or another key changes nothing, setting `approval = "any"` where it was unset is a change, deleting a file that set it is a change, and a value `config` would reject, such as `approval = 1`, is compared like any other value. When the diff lists `clm.toml` and either side's text isn't valid UTF-8 or doesn't parse as TOML, the file SHALL count as a spec change that needs review, so the gate fails closed. Any other change to `clm.toml` SHALL NOT count as a spec change, and `clm.local.toml` is never compared.

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

### Requirement: rules-version <!-- base: b8613216ec5f4d31 -->
`clm-gate` SHALL offer `RULES`, a whole number that SHALL be 5, and `evaluate_by(repo, base, head, approved, rules)`, returning what `evaluate_at` returns as the gate decided under the rules of version `rules`, and `evaluate_at` SHALL be `evaluate_by` with `RULES`. Under version 5 the gate SHALL read the paths a branch changes as `paths-under-the-root` states, and under versions 1 to 4 as `git diff` prints them, relative to the git repository's top, each file at a revision read as `<rev>:<path>`, as those versions did. Under versions 4 and 5 a `spec` branch SHALL need review as `needs-review` states, under version 3 as it states with a *shortened requirement* cut counted as though every sentence holding a placement phrase placed a test, whether or not it holds the word `test` or `tests`, under version 2 as under version 3 with a *changed outcome* cut making no branch need review, and under version 1 as it states with `cuts` making no branch need review, specs that don't load at the merge base or the head included, a branch `rename/replay` accepts keeping its own rule under each. In a repository whose CLM root is its top, the kind and the cuts SHALL be the same under every version, and the violations too but for `gate/approval`, which follows the review. In one whose CLM root is a subdirectory, the kind, the cuts and the violations under versions 1 to 4 SHALL be those their reading of the paths gives. For a `rules` below 1 or above `RULES`, `evaluate_by` SHALL return an error naming `rules` and `RULES`.

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

#### Scenario: a sentence of behaviour that holds a phrase under each version
- GIVEN component `a` with requirement `a/x` whose statement is `The command SHALL print the list. The directory SHALL be deleted once nothing else is left in it.`, and a branch `s` that only removes the second sentence
- WHEN `evaluate_by(repo, "main", s, false, 3)` and `evaluate_by(repo, "main", s, false, RULES)` run
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
