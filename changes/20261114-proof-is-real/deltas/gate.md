---
component: gate
base: { frontmatter: "18968b77529938e5", contract: "41c4e97ec6aa91a0", design: "a69a9e02fab8dd4a" }
---

## MODIFIED Frontmatter

```yaml
depends_on: [model, context, vcs, gate.fingerprint, rename, check.coverage]
```

## MODIFIED Contract

`clm gate --base <ref> [--approved]` decides whether `HEAD` may merge into `<ref>`. Its first line is the branch's kind, `spec review` for a spec branch that needs a human's review, `spec` for one that doesn't, `code` or `mixed`, then any violations in `check`'s format, and it exits 0 to allow or 1 to block. `--approved` says the ledger holds an approval of this diff that the repository's policy accepts; `clm land` computes it from the fold (`land-locally`), and the gate itself reads no ledger and no host.

`clm fingerprint`, the fingerprint an approval is pinned to, is the child `gate.fingerprint`'s (crate `clm-fingerprint`). `clm-gate` re-exports its `Entry`, `parse_ls_tree`, `fingerprint_of`, `adds_change`, `added_changes` and `fingerprint`, so `land`, `ledger`, `web` and `crates/clm` call them as `clm_gate::...` (`fingerprint-reexported`).

`clm-gate` also offers `settings_unchanged(before, after)`: given `clm.toml`'s bytes on each side of a diff (`Option<&[u8]>`, `None` where the file doesn't exist), whether its settings are the same once the upgrade notes `upgraded` and `declined_migrations`, comments and formatting are set aside, false when either side isn't UTF-8 TOML. `land` reads it to count a land whose `clm.toml` changed only so as checked (`land/settings-unchanged-is-checked`).

`clm-gate` also offers `foreign_files(repo, base, head, component)`: the code files `head` changes against `base` that aren't `component`'s build's to change, each with its owner, none when the root at the merge base sets no `wiring` (`own-files`). The gate itself reads no branch name, so `clm gate` doesn't report them: `clm land`, which knows the branch, applies the rule (`land/build-holds-own-files`), as it applies `files-outside-spec-paths` to a spec branch.

## MODIFIED Design

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

**A build's own files** (`own-files`, `own-files-at-the-gate`). `path-ownership` was kept by the build prompt alone; a program now refuses what the prompt asked for. The gate stays a function of two trees and one flag and reads no branch name (`spec-branches-hold-spec-paths`), so `foreign_files` takes the component as an argument and `clm land`, which knows the branch, calls it for a branch named `impl/<component>`: no other branch has a component, and a small change on a branch of another name is not judged. The specs it reads are the merge base's, loaded as for the code-fix check; a file's owner is `check.coverage`'s `owner`, which the gate depends on for it. Three kinds of file are the build's to change beside its own. The root's `wiring` names the shared ones. A file that cites the component, the gate's existing reading of where a test belongs (`fixes-count-where-cited`), covers the tests Cargo keeps in another package's directory, added, edited or deleted, until each crate proves its requirements through its own entry point (`backlog/clm-s-cli-tests-prove-other-components-through-the-binary`); it opens the whole file, since a new test's imports and helpers lie outside any test. A capability test may be edited, never added or deleted, by a build whose component serves the capability it cites, since a behaviour and the test that observes it at the boundary can't land apart (`capability-tests-follow-rules`). That a wiring edit adds only the build's own entry, and that a capability-test edit keeps what its scenarios state, stay judgements no program makes. The rule is off until the root sets `wiring`, as `check`'s path ownership is off until it sets `outside`, so a project adopts it when it has named its wiring.

## ADDED Requirements

### Requirement: own-files
`clm-gate` SHALL offer `foreign_files(repo, base, head, component)`, returning, when the root component at the merge base of `base` and `head` sets `wiring` (`model/wiring-globs`) and the specs there hold `component`, each code file (one outside the spec paths) that `head` adds, modifies or deletes against the merge base and that is none of:
- a file `component` owns, by `check.coverage/owner` over the specs at the merge base;
- a file no component owns, which `check/unowned-files` judges;
- a file one of the root's `wiring` globs matches;
- a file whose text at the merge base or at `head` holds `req:<component>/`, searched as `code-fix-resolves-bug` searches it;
- a file `head` modifies, without adding or deleting it, whose text at the merge base holds `req:capability/<slug>` for a capability that the effective `serves` of one of `component`'s requirements names (its own `#### Serves` block when it holds one, its component's `serves` otherwise);

each with the component that owns it, sorted by path. It SHALL return nothing when the root sets no `wiring` or the specs at the merge base hold no such component. It SHALL read `head` only as a revision, as `evaluate-at` does, and `clm gate`'s output and exit status SHALL NOT change with it.

#### Scenario: another component's source
- GIVEN a root with `wiring: []`, components `a` with `paths: ["crates/a/**"]` and `b` with `paths: ["crates/b/**"]`, and a branch changing `crates/a/src/lib.rs` and `crates/b/src/lib.rs`
- WHEN `foreign_files` runs for `a`, and `clm gate --base main` runs
- THEN the first returns `crates/b/src/lib.rs` with owner `b`, and the gate prints `code` and exits 0

#### Scenario: wiring
- GIVEN the same with the root's `wiring: ["Cargo.lock"]` and a branch changing `crates/a/src/lib.rs` and `Cargo.lock`, which the root owns
- WHEN `foreign_files` runs for `a`
- THEN it returns nothing

#### Scenario: a test that cites the component
- GIVEN the same with `impl/a` adding a test citing `a/x` to `crates/b/tests/cli.rs`, and deleting `crates/b/tests/old.rs`, which cited `a/y` at the merge base
- WHEN `foreign_files` runs for `a`
- THEN it returns nothing

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

#### Serves
- architecture/path-ownership
- capability/land-safely

#### Needs
- model/wiring-globs
