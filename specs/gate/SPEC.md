---
title: Gate
status: active
owners: ["@clunny"]
depends_on: [model, context, vcs, gate.fingerprint, rename]
paths: ["crates/clm-gate/**"]
serves: [capability/land-safely, capability/approve-intent, architecture/humans-review-intent]
---

## Contract
`clm gate --base <ref> [--approved]` decides whether `HEAD` may merge into `<ref>`. Its first line is the branch's kind, `spec review` for a spec branch that needs a human's review, `spec` for one that doesn't, `code` or `mixed`, then any violations in `check`'s format, and it exits 0 to allow or 1 to block. `--approved` says the ledger holds an approval of this diff that the repository's policy accepts; `clm land` computes it from the fold (`land-locally`), and the gate itself reads no ledger and no host.

`clm fingerprint`, the fingerprint an approval is pinned to, is the child `gate.fingerprint`'s (crate `clm-fingerprint`). `clm-gate` re-exports its `Entry`, `parse_ls_tree`, `fingerprint_of`, `adds_change`, `added_changes` and `fingerprint`, so `land`, `ledger`, `web` and `crates/clm` call them as `clm_gate::...` (`fingerprint-reexported`).

`clm-gate` also offers `settings_unchanged(before, after)`: given `clm.toml`'s bytes on each side of a diff (`Option<&[u8]>`, `None` where the file doesn't exist), whether its settings are the same once the upgrade notes `upgraded` and `declined_migrations`, comments and formatting are set aside, false when either side isn't UTF-8 TOML. `land` reads it to count a land whose `clm.toml` changed only so as checked (`land/settings-unchanged-is-checked`).

## Design
Changes come from `git diff --name-status --no-renames <merge-base>...HEAD`. Spec paths are `specs/`, `changes/`, `backlog/`, `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and `AGENTS.md`. File contents at the merge base and at `HEAD` are compared with `git diff <rev>:<path> <rev>:<path>`. Uses the `git` CLI, not a git library.

**Review is yes or no** (`yes-no-review`). A spec branch needs a human's review only for what a program can't judge for the human: the two human layers, `ARCHITECTURE.md` and `CAPABILITIES.md`, edited whole in the branch; the keys a signed repository trusts, `docs/allowed_signers` (any entry at or under that path, never counted as a status-only edit; `signed-ledger-records`); whose approval counts (`approval-setting`); and a change that declares a burden, something that spends the human's money or time or can't be undone. Everything else, component specs, decisions, backlog edits and changes with no burdens, lands on checks, and nothing is reviewed after the fact. Whether a branch needs review is read from the diff's paths, `clm.toml` at both sides and the `change.md` texts at `HEAD`, so no graph is loaded. A `change.md` that can't be read or parsed needs review, so the gate fails closed; a live one that holds `gate` parses (the model sinks the key) and `clm check` rejects it (`model/parse-changes`). `undeclared_burdens` checks what a program can see of a burden: a `kind: human-task` item the branch adds is a step only the human can take, so a spec branch that adds one must say so in `human_burdens`, which then makes it need review. Builds file `question` and `human-task` items freely; those block only their own requirements.

**Rename branches** (`rename-by-replay`). A branch whose whole diff `rename/replay` reproduces, from the merge base and the renames its one added change records, is classified `spec` though it rewrites test markers in code files, since every edit follows from the pairs a human or the gate can read. Its edits to `ARCHITECTURE.md` and `CAPABILITIES.md` are replayed renames of IDs, which change no intent, so they don't make it need review; a `term` pair does, since re-citing a reworded requirement skips the build that would confirm its tests, and only a human can judge that a swapped word keeps a rule's meaning (`hash-based-staleness`). Its `human_burdens`, and a `change.md` that doesn't parse, count as for any change. `classify` asks `clm_rename::replays` only when the diff holds code files and the branch adds exactly one change whose `change.md` at `HEAD` sets `renames`, so no other branch pays for the extraction; when it answers no, the branch is classified by its paths. `needs_review_of` stays the path rule the Worker shares; `evaluate_at` uses it for every `spec` branch but a replayed one.

The gate loads `specs/` at the merge base only for the code-fix check's component lookup (`code-fix-resolves-bug`), with `clm_model::load`, extracted into a temporary directory with `clm-git`'s `extract_paths`, so each file holds the bytes a checkout of that commit writes (`extract-as-checkout`, `base-specs-as-checkout`). The paths passed are `specs`, when it exists at the merge base, and each path of the form `changes/<id>/change.md` that `git ls-tree -r --name-only <rev> -- changes` lists, since `extract_paths` takes its paths literally and errors on one that matches nothing. The live changes' `change.md` files carry the component frontmatter keys they declare in `frontmatter_keys`, so a component that sets one loads as `clm check` loads it (`declared-keys-loaded`, `declared-frontmatter-keys`). Their deltas and assets aren't extracted, since nothing the gate reads from the loaded graph needs them.

A *change the PR adds* is a live `changes/<id>/` directory, or a *new archived change*: `changes/archive/<id>/` with `change.md` among the files the PR adds there other than by an archive move (`archive-move`). The second is a change that needs nothing built, such as a design-only one, marked `done` and archived in its own spec PR (`archive-in-spec-pr`). Since `archive-immutable` forbids editing anything under `changes/archive/`, the gate treats a new archived change's files as it treats a live change's: its `human_burdens` count for `needs-review` and `undeclared-burden`, and its `resolves` is checked by `resolves-named-items`. The PR adds the whole of such a `change.md`, so every one of its `resolves` entries is new and tested, as for a live `change.md` the PR adds. The exception is a live change moved into the archive with more than `status:` lines edited, which isn't an archive move: its entries are compared with the live `changes/<id>/change.md` at the merge base, so items a task deleted (`task-deletes-fixed-items`) aren't reported.

A code PR records its progress in its tests' markers, `req:<id>@<version>` (`versioned-markers`), and nothing under `changes/` but an archive move (`archive-move`) when `clm rebase` archived a complete change on its branch (`archive-on-sync`).

**Code fixes** (`code-fix-resolves-bug`). A code PR may delete the backlog item of a bug it fixes, or of deferred work it does, when it changes no spec (`deferred-work-resolves-in-code`). A commit names the item with the trailer `Resolves: backlog/<slug>`, read with `git log --format=%(trailers:key=Resolves,valueonly) <merge-base>..HEAD`, one slug per value. Values are trimmed, and only those of the exact form `backlog/<slug>` count; a value naming an item the PR doesn't delete is ignored. The deletion is exempt when `backlog/<slug>.md` at the merge base has `kind: bug` or `kind: deferred` and its `components` name a component, in `specs/` at the merge base, that a code file (one outside the spec paths) the PR adds, modifies or deletes belongs to. A file belongs to a component when one of the component's `paths` matches it, or when its text at the merge base or at `HEAD` holds `req:<component>/`, the start of a marker citing one of the component's requirements (`fixes-count-where-cited`). The second way is for tests that live in another component's `paths`: `plugin`'s skill tests are the root's `crates/clm/tests/plugin.rs`, because Cargo gives only the root package's tests the binary, and they cite `req:plugin/`, so a fix to one of them is `plugin`'s, as `tests-count-where-cited` counts them toward `plugin`'s size. Either version counts, so a fix that removes or adds a file's last marker belongs where the file did or does. The text is searched as is, with no marker parsing: `req:plugin/` doesn't occur in `req:plugin.hooks/x`, and a fixture that builds its marker from pieces cites nothing. `kind` and `components` are read from the item's frontmatter alone, so a key the model rejects can't hide them; an item whose frontmatter doesn't parse, or none of whose components exist at the merge base, isn't exempt; a named component missing there counts for neither `paths` nor markers, and the others count. A component with no `paths` can still qualify through markers. A file whose text isn't valid UTF-8 is searched as bytes, and one that can't be read on a side holds no marker on that side; neither makes the gate fail. The search runs only when a trailer names an item the PR deletes. Only the item's own components count, not their parents or children, since `paths` are disjoint and a marker names one component. A path matches as a git `:(glob)` pathspec, as `clm size` matches `paths`, tested against the PR's diff (`git diff --name-only <merge-base> HEAD -- ':(glob)<path>'`) so a deleted file matches too. Unlike `resolves`, which a change must have merged before the branch deletes the item, the branch here both names and deletes it. What bounds it is that deleting a bug or deferred item needs no review anyway, and that the PR's code must touch a component the item names, by its `paths` or by a test that proves it.

**Approval.** The host plays no part in review (`per-writer-ledgers`). `--approved` is the flag, and it is `clm land`'s word: the gate doesn't know who approved or under which policy, which keeps it a pure function of the two trees and one flag, and keeps every caller's verdict the same. The `gate/approval` message names `clm approve <branch>`, so a session knows what to ask the human for.

**Fingerprint.** `gate.fingerprint` holds it (`tree-diff-fingerprint`); `clm-gate` keeps only the re-export (`fingerprint-reexported`). The gate itself computes no fingerprint.

**Where the binary's tests live** (`binary-tests-apart`). A test that proves a `gate` rule by running the `clm` binary sits under the root's `crates/clm/tests/`, since Cargo gives only the root package's tests the binary, and there in `gate.rs` alone. A file counts whole toward every component it cites (`tests-count-where-cited`), so a `gate` test in `cli.rs` would charge all of `cli.rs`, mostly `check`'s and `config`'s tests, to `gate`'s size. The rest of `gate`'s tests are its library's, under `crates/clm-gate/tests/`.

A branch behind `<ref>` may land (`land-locally`). `clm land` merges `<ref>` into it and checks the result before anything lands. So the gate blocks only what that merge can't settle. `git merge-tree --write-tree <ref> HEAD` exits 1 on a textual conflict. A file under `specs/` that changed on both sides since the merge base is also blocked: `specs/` is derived, so it is regenerated by `clm rebase` and never merged (`regenerate-dont-merge`), even when the text would merge cleanly. Both report rule `gate/up-to-date` and tell the author to run `clm rebase`.

**Evaluating another commit** (`gate/evaluate-at`). `evaluate_at(repo, base, head, approved)` is the gate for `head` against `base` as `clm gate --base <base>` gives it with `head` checked out: it reads `head` only as a revision, through `git diff <merge-base>...<head>`, `git show <head>:<path>`, `git ls-tree <head>`, `git log <merge-base>..<head>` and `git merge-tree --write-tree <base> <head>`, never the working tree or the index, so it gives the same verdict whatever is checked out. A violation's path names the range as `<base>..<head>` with the revisions as given, so `clm gate`'s output stays `<base>..HEAD`. `clm gate` calls it with `HEAD`. `ledger`'s `verify --review` calls it for each signed land, with the land's first parent as `base` and the merge commit as `head`, to recompute the land's kind and whether it needed review without a checkout per land.

## Requirements

### Requirement: classify
The gate SHALL classify a PR as `spec` if every changed file is a spec path, or if `rename/replay` accepts it (`replays(repo, <merge-base>, HEAD)` is true), `code` if no changed file is a spec path, and `mixed` otherwise. It SHALL block `mixed` PRs, telling the author to split them, and, when the branch is `impl/<component>`, telling its build that a build edits no spec path: it leaves a requirement that needs a spec edit unbuilt and adds a `question` backlog item whose `blocks` names it (`orchestrator.run/stays-in-its-component`, `requirement-blocks`).

#### Scenario: mixed build branch
- GIVEN branch `impl/a` that changes `crates/a/lib.rs` and `specs/a/SPEC.md`
- WHEN the gate runs
- THEN it prints `mixed`, exits 1, and its line says the build leaves the requirement unbuilt and adds a `question` item

#### Scenario: mixed PR
- GIVEN a branch changing `specs/a/SPEC.md` and `crates/x/src/lib.rs`
- WHEN `clm gate --base main` runs
- THEN it prints `mixed` and exits 1

#### Scenario: rename branch
- GIVEN a branch from `main` holding only the committed result of `clm rename r`, which rewrote `specs/a/SPEC.md` and a marker in `crates/a/tests/t.rs`
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and exits 0

#### Scenario: rename branch with an extra code edit
- GIVEN that branch with one more commit changing `crates/a/src/lib.rs`
- WHEN `clm gate --base main` runs
- THEN it prints `mixed` and exits 1

#### Needs
- rename/replay

### Requirement: status-only-exemption
A change to a spec path whose diff lines are only `status:` frontmatter lines SHALL NOT count as a spec change, so code PRs can record component and change status.

#### Scenario: code PR flips status
- GIVEN a branch changing `crates/x/src/lib.rs` and only the `status:` line of `specs/x/SPEC.md`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0

#### Serves
- capability/land-safely
- architecture/fold-small-defer-large

### Requirement: approval
A `spec` branch that needs review (`needs-review`) SHALL be blocked with rule `gate/approval`, naming `clm approve`, unless `--approved` is given. A `spec` branch that needs no review, and a `code` branch, SHALL NOT require it.

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

### Requirement: needs-review
A `spec` branch SHALL need review exactly when its diff adds, modifies or deletes `ARCHITECTURE.md`, `CAPABILITIES.md` or `docs/allowed_signers` (the keys a signed repository trusts, `signed-ledger-records`); or `approval-setting` counts `clm.toml` as changed; or a change one of whose files it adds or modifies, a live change or a new archived one, has a `change.md` at `HEAD` that can't be read as UTF-8, whose frontmatter doesn't parse, or that declares a non-empty `human_burdens` (`yes-no-review`), so editing a change's deltas or assets alone doesn't leave its burdens unreviewed. A `spec` branch that `rename/replay` accepts SHALL instead need review exactly when its added change records a `term` rename, declares a non-empty `human_burdens`, or has a `change.md` that can't be read or parsed: its replayed edits to `ARCHITECTURE.md` and `CAPABILITIES.md` SHALL NOT count. Nothing else SHALL make a branch need review, and a `code` or `mixed` branch never does. `clm gate` SHALL print `spec review` for a `spec` branch that needs review and `spec` for one that doesn't.

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
- GIVEN a branch that changes the Contract of `specs/a/SPEC.md`, removes one of `a`'s requirements, adds `docs/decisions/d.md` and adds `changes/x/change.md`, `applied` with no `human_burdens`
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

#### Needs
- rename/replay
- model/parse-changes

### Requirement: undeclared-burden
A `spec` branch that adds a `backlog/` item whose `kind` at `HEAD` is `human-task`, while no change one of whose files it adds or modifies, a live change or a new archived one, declares a non-empty `human_burdens` in its `change.md` at `HEAD`, SHALL be blocked with rule `gate/undeclared-burden`, naming the item, its message naming the kind `human-task` and the key `human_burdens`, even with `--approved`. A `code` branch MAY add such an item freely: a build files `question` and `human-task` items, which block only their own requirements (`requirement-blocks`).

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

#### Needs
- model/parse-changes
- model/parse-backlog

### Requirement: up-to-date
The gate SHALL block, with rule `gate/up-to-date` and a message telling the author to run `clm rebase`, when `HEAD` doesn't merge cleanly into `<ref>`, or when a file under `specs/` changed both between the merge base and `<ref>` and between the merge base and `HEAD`. A branch that is merely behind `<ref>` SHALL NOT be blocked for that.

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

#### Needs
- clm/renamed-commands

#### Serves
- capability/land-safely
- capability/work-concurrently
- architecture/regenerate-dont-merge

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
The gate SHALL block any PR that modifies or deletes a file that exists under `changes/archive/` at the merge base.

#### Scenario: editing history
- GIVEN `changes/archive/x/change.md` on `main`
- WHEN a branch edits it and `clm gate --base main --approved` runs
- THEN it exits 1 naming the file

#### Serves
- architecture/spec-is-current
- architecture/humans-review-intent

### Requirement: backlog-additions
A file added under `backlog/` SHALL NOT count as a spec change, so code PRs can record deferred work (`fold-small-defer-large`). A change to an existing backlog file whose only effect is to add entries to its `blocks` list SHALL NOT count as a spec change either, so a build can record that a requirement waits on an item that already exists (`requirement-blocks`). Deleting `backlog/<slug>.md` SHALL NOT count as a spec change when the same branch archives a change whose `resolves` lists `<slug>`, or when a change outside the archive at the merge base lists `<slug>` in its `resolves`, so the build that fixes an item may delete it. Nor SHALL it count when a commit between the merge base and `HEAD` carries the trailer `Resolves: backlog/<slug>`, the item at the merge base has `kind: bug` or `kind: deferred`, and a file the PR adds, modifies or deletes outside the spec paths belongs to a component the item's `components` names: the file matches, as a `:(glob)` pathspec, that component's `paths` at the merge base, or its text at the merge base or at `HEAD` contains `req:<component>/` (`fixes-count-where-cited`). An item whose frontmatter doesn't parse SHALL NOT be exempt this way. So a fix that changes no spec may delete its bug in the same code PR (`code-fix-resolves-bug`), including a fix to a test kept under another component's `paths`, and a build that does a deferred item's work, such as removing code it kept for another component that no longer uses it, may delete that item (`deferred-work-resolves-in-code`). A `question`, `human-task` or `idea` item SHALL NOT be exempt this way: closing one is a decision. Any other modification or deletion of a backlog file SHALL count as a spec change.

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
- GIVEN the branch of *code fix to a test that cites the bug's component*, where the test's only marker cites `a.b/y` instead
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

#### Serves
- capability/keep-a-backlog
- architecture/fold-small-defer-large

#### Needs
- model/parse-backlog

### Requirement: resolves-named-items
Each `resolves` entry that a branch adds to a `change.md` outside the archive, comparing the parsed lists at the merge base and at `HEAD`, and including every entry of a `change.md` the branch adds, live or in a new archived change (`needs-review`), where the list at the merge base of a new archived change `changes/archive/<id>/` is that of `changes/<id>/change.md` there, if it exists, SHALL name a backlog item that exists at the merge base or at `HEAD`. Otherwise the gate SHALL report `gate/resolves-named-items` naming the change and the entry, and exit 1. Entries already there at the merge base SHALL NOT be tested, so a task may delete an item its change resolves (`task-deletes-fixed-items`). Backlog items are named by bare slug, so an entry that isn't one, such as `backlog/x` or `x.md`, names no item even when `backlog/x.md` exists, and is reported (`check/backlog-entry-shape` says what to write).

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

#### Serves
- capability/keep-a-backlog
- capability/catch-drift

### Requirement: git-through-layer
`clm-gate` SHALL list `clm-git` among its dependencies and SHALL run no `git` or `gh` process itself: every call goes through `clm-git`'s `git`, `gh`, `run` or repo queries (`git-plumbing-layer`), and its own wrapper is gone (`no-dead-code`).

#### Scenario: covered by the scan
- GIVEN the workspace
- WHEN a test in `clm-gate` reads `crates/clm-gate/Cargo.toml` and every file under `crates/clm-gate/src`
- THEN `clm-git` is a dependency and no file holds `Command::new("git")` or `Command::new("gh")`

#### Serves
- architecture/reuse-first

### Requirement: approval-setting
When the PR's diff lists `clm.toml` (the path at the top of the repository, matched as the gate matches `specs/` and `docs/`, and the file `ledger verify` audits lands by) and the change alters the value of its top-level `approval` key, that file SHALL count as a spec change that needs review (`needs-review`), so the policy deciding whose approval counts changes only with an approval (`approval-setting-is-spec`). It doesn't become a spec path: the list of spec paths stays as it is, and the file is reported as any other spec change is, with no rule of its own; only `files-outside-spec-paths` leaves it off the files it returns, so such a change can land on a `spec/*` branch. The gate SHALL compare the key's value as a parsed TOML value at the merge base and at `HEAD`, read with `git show <rev>:clm.toml`, a missing file or a missing key counting as unset: so editing a comment, the layout or another key changes nothing, setting `approval = "any"` where it was unset is a change, deleting a file that set it is a change, and a value `config` would reject, such as `approval = 1`, is compared like any other value. When the diff lists `clm.toml` and either side's text isn't valid UTF-8 or doesn't parse as TOML, the file SHALL count as a spec change that needs review, so the gate fails closed. Any other change to `clm.toml` SHALL NOT count as a spec change, and `clm.local.toml` is never compared.

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
`clm-gate` SHALL offer `evaluate_at(repo, base, head, approved)`, which SHALL return the kind, whether the branch needs review, and the violations that `clm gate --base <base>` gives with `head` checked out, reading `head` only as a revision, so that what is checked out, the working tree and the index change nothing. `clm gate` SHALL be `evaluate_at` with `HEAD`.

#### Scenario: another commit
- GIVEN a repository with `main`, a commit `s` on top of it that only adds a requirement to `specs/a/SPEC.md`, and `HEAD` checked out at `main` with an uncommitted edit to `ARCHITECTURE.md`
- WHEN `evaluate_at(repo, "main", s, false)` runs
- THEN it returns kind `spec` needing no review, as `clm gate --base main` prints with `s` checked out, and `HEAD`, the index and the working tree are as they were

#### Scenario: a merge commit against its first parent
- GIVEN a merge commit `m` of a branch that only changes `ARCHITECTURE.md` into `main`, with `main` as its first parent `p`
- WHEN `evaluate_at(repo, p, m, false)` runs
- THEN it returns kind `spec` needing review, with `gate/approval` among its violations

#### Serves
- capability/land-safely
- architecture/per-writer-ledgers

### Requirement: base-specs-as-checkout
The gate SHALL read `specs/` as committed at the merge base, for the component lookup of `code-fix-resolves-bug`, by loading with `clm_model::load` the temporary directory `clm-git`'s `extract_paths(root, <rev>, paths)` returns, where `paths` holds `specs` unless `git cat-file -e <rev>:specs` fails, and each path of the form `changes/<id>/change.md` that `git ls-tree -r --name-only <rev> -- changes` lists (`declared-keys-loaded`), and is empty when neither gives a path. Each file SHALL then hold the bytes a checkout of `<rev>` writes (`extract-as-checkout`): a path marked `export-ignore` and a file marked `export-subst` in `<rev>`'s `.gitattributes` SHALL load as a checkout writes them. `clm-gate` SHALL start no process itself, `tar` included: its `extract_specs` and the imports only it used are gone, and `tempfile`, which only its tests then use, moves to `[dev-dependencies]` (`no-dead-code`). Paths stay relative to the top of the repository, as the gate's other git calls are.

#### Scenario: specs marked export-ignore
- GIVEN `main` with component `a`, whose `paths` hold `src/a/**`, a `kind: bug` item `backlog/b.md` naming `a`, and a `.gitattributes` holding `specs/** export-ignore`, and a branch that changes `src/a/x.rs` and deletes `backlog/b.md` in a commit with the trailer `Resolves: backlog/b`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0, as it does without the attribute

#### Scenario: a spec marked export-subst
- GIVEN `main` with component `a`, whose `paths` hold `src/a/**` and whose Contract holds `$Format:%H$`, a `kind: bug` item `backlog/b.md` naming `a`, and a `.gitattributes` holding `specs/** export-subst`, and a branch that changes `src/a/x.rs` and deletes `backlog/b.md` in a commit with the trailer `Resolves: backlog/b`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0, since component `a` loads as a checkout writes it

#### Scenario: no process of its own
- GIVEN the workspace
- WHEN a test under `crates/clm-gate/tests/` (outside `src/`, so its own text isn't scanned) reads every file under `crates/clm-gate/src`
- THEN no file holds `Command::new(` or `fn extract_specs`

#### Serves
- capability/keep-a-backlog
- architecture/fold-small-defer-large

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
`clm-gate` SHALL offer `approval_changed(before, after)`, each the bytes of `clm.toml` at one side of a diff (`Option<&[u8]>`) or `None` where the file doesn't exist, returning whether `approval-setting` counts the file as a spec change: true exactly when the top-level `approval` values, compared as parsed TOML with a missing file or key counting as unset, differ, or either side's bytes aren't UTF-8 or don't parse as TOML. It SHALL run no process, SHALL build for `wasm32-unknown-unknown` (proven as `fingerprint_of`'s build is, by the wasm build of `clm-web` the checks run), and SHALL be what `evaluate_at` decides `approval-setting` by, so the gate and `web.worker` give one answer.

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
`clm-gate` SHALL offer `needs_review_of(paths, approval_changed, changes)`, where `paths` are the paths a branch's diff adds, modifies or deletes, `approval_changed` is whether `approval-setting` counts `clm.toml` as changed, and `changes` holds, for each change one of whose files the branch adds or modifies, its `change.md` frontmatter at `HEAD`, or `None` when that file is missing, isn't UTF-8 or its frontmatter doesn't parse. It SHALL return whether a `spec` branch with that diff needs review by `needs-review`'s rule: true exactly when a path is `ARCHITECTURE.md`, `CAPABILITIES.md` or `docs/allowed_signers`, or a path under one of them, or `approval_changed` is true, or an entry of `changes` is `None` or declares a non-empty `human_burdens`. It SHALL run no process, SHALL build for `wasm32-unknown-unknown` (proven as `approval_changed`'s build is, by the wasm build of `clm-web` the checks run), and SHALL be what `evaluate_at` decides a `spec` branch's review by, so the gate and `web.worker` give one answer, except for a branch `rename/replay` accepts, whose review `evaluate_at` decides by `needs-review`'s rule for such a branch: replaying needs the base's whole tree, which the Worker doesn't have, so the Worker's answer for a rename branch can say it needs review when the gate says it doesn't, and never the reverse. `change_needs_review` stays public only while a crate outside `clm-gate` calls it (`no-dead-code`).

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

#### Serves
- capability/approve-intent
- capability/follow-from-browser
- architecture/humans-review-intent

#### Needs
- model/parse-changes

### Requirement: fingerprint-reexported
`clm-gate` SHALL re-export `clm-fingerprint`'s `Entry`, `parse_ls_tree`, `fingerprint_of`, `adds_change`, `added_changes` and `fingerprint` under the same names, SHALL list `clm-fingerprint` among its dependencies, and SHALL define none of them itself: its own `fingerprint` module and its `sha2` dependency are gone (`no-dead-code`).

#### Scenario: same function through the gate
- GIVEN two tree listings parsed with `clm_gate::parse_ls_tree`, in which one blob's ID differs
- WHEN `clm_gate::fingerprint_of` and `clm_fingerprint::fingerprint_of` run on them
- THEN both return the same fingerprint

#### Scenario: no copy left
- GIVEN the workspace
- WHEN a test reads `crates/clm-gate/Cargo.toml` and every file under `crates/clm-gate/src`
- THEN `clm-fingerprint` is among its `[dependencies]`, `sha2` isn't, and no file defines `struct Entry`, `fn parse_ls_tree`, `fn fingerprint_of`, `fn adds_change`, `fn added_changes` or `fn fingerprint(`

#### Serves
- architecture/slim-components
- architecture/reuse-first

#### Needs
- gate.fingerprint/fingerprint

### Requirement: binary-tests-apart
Every test that proves a `gate` requirement by running the `clm` binary SHALL live in `crates/clm/tests/gate.rs`, and no other file under `crates/clm/tests/`, at any depth, SHALL hold a `req:` marker citing a `gate` requirement, so the root's other test files don't count toward `gate`'s size (`tests-count-where-cited`). The test that checks it SHALL build the marker prefix it searches for from pieces, so it cites nothing it doesn't mean to.

#### Scenario: one file cites the gate
- GIVEN the workspace
- WHEN a test reads every `.rs` file under `crates/clm/tests/`
- THEN `gate.rs` is the only one holding the text `req:gate/`, and `cli.rs` holds none

#### Serves
- architecture/slim-components

### Requirement: item-components
Where the gate reads a backlog item's components at a revision, for the code-fix exemption of `backlog-additions` (`code-fix-resolves-bug`), it SHALL read them from the item's `components`, or from its old key (`component-words`) when the item sets that key instead (`model/component-keys`), so an item written under either key is judged the same. An item at the merge base that sets both keys, which the model rejects, SHALL NOT be exempt, as an item whose frontmatter doesn't parse isn't.

#### Scenario: a bug filed under the new key
- GIVEN the repository of *code fix deletes its bug* (`backlog-additions`), with `backlog/x.md` naming its component as `components: [a]`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0

#### Needs
- model/component-keys

#### Serves
- architecture/plain-terms
- architecture/humans-review-intent

### Requirement: files-outside-spec-paths
`clm-gate` SHALL offer a library call returning the files a revision changes against a base, from the merge base as `classify` reads them, that aren't spec paths, sorted, and returning none when `rename/replay` accepts the revision. It SHALL leave out `clm.toml` when `approval-setting` counts it as a spec change, deciding that by `approval_changed` of the file's bytes at the merge base and at the revision, so a branch that changes whose approval counts, which needs review, can land as a `spec/*` branch, and when `upgrade-stamp-left-out` leaves it out, deciding that by `upgrade_stamp_only`; any other change to `clm.toml` it SHALL return. It SHALL read the revision only through git, never the working tree or the index, as `gate/evaluate-at` does.

#### Scenario: a code file among spec files
- GIVEN a revision changing `specs/a/SPEC.md` and `crates/x/src/lib.rs` against its base
- WHEN the call runs with something else checked out
- THEN it returns `crates/x/src/lib.rs` alone

#### Scenario: only a new backlog item
- GIVEN a revision that only adds `backlog/x.md`, for which `clm gate` prints `code`
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
When the diff lists `clm.toml` and its parsed TOML at the merge base and at `HEAD` differ only in the top-level keys `upgraded` and `declined_migrations`, a missing file or key counting as unset, the gate SHALL leave the file out of the files it classifies, and it SHALL NOT make the branch need review. A branch whose only changed file is such a `clm.toml` SHALL classify as `spec`. `clm-gate` SHALL decide it with `upgrade_stamp_only(before, after)`, of the two sides' bytes as `approval_changed` takes them, false when either side isn't UTF-8 or doesn't parse. The function SHALL run no process and SHALL build for `wasm32-unknown-unknown`, so `web.worker` can give the same answer. Any other change to `clm.toml` SHALL be judged as `approval-setting` says.

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
The gate SHALL count `AGENTS.md` at the top of the repository as a spec path, as it counts `CLAUDE.md` (`classify`, `files-outside-spec-paths`), since it holds the project's instructions for every agent harness (`platform`).

#### Scenario: instructions edit
- GIVEN a branch that edits only `AGENTS.md`
- WHEN `clm gate` judges it
- THEN its first line is `spec`

#### Serves
- capability/work-from-any-agent-harness
