---
title: Ledger
status: active
owners: ["@clunny"]
depends_on: [config, gate, vcs]
paths: ["crates/clm-ledger/**"]
successor: ""
serves: [capability/approve-intent, capability/land-safely, architecture/per-writer-ledgers]
---

## Contract
The records that say what was approved, checked and landed, as facts in git that any clone can replay (`per-writer-ledgers`). Each writer appends to its own branch, `clm/ledger/<person>/<clone>`, so appends never race, and the state is the fold over the union of every ledger branch on `origin` and the writer's own. Crate `clm-ledger` offers, as library calls: `writer(root)` (this clone's identity, creating it on first use), `append(root, record)` (commit the record to the writer's branch and push it), `fold(root)` (the state: approvals by fingerprint, checks by code key, lands by commit, and the live push subscriptions by `id`, each with its `writer`, `at`, `endpoint`, `p256dh` and `auth`, the open web proposals by `id`, the comments and the change requests by branch (`web-proposals-as-ledger-records`, `web-review-feedback-as-ledger-records`), the effort records by `id` the capability decisions by capability, and the verifications by commit), `append_all(root, entries)` (several records in one commit and one push), `effort_id(session, branch)` (an `effort` record's `id`), `code_key(root, rev)` (a commit's tree with the doc-only paths left out, hashed), `code_key_with(root, rev, path, from)` (`rev`'s code key with `path`'s entry as it is in `from`, which `land` reads to count a `clm.toml` whose settings are unchanged as checked), `doc_only(path)` (whether `code_key` leaves a path out, which `check.affected` reads so its selection and the code key agree), `person(writer)` (the person part of a `<person>/<clone>` writer), `slug(name)` (a name slugged as `writer` slugs git's `user.name`, which `land.approval` and `verify` slug commit authors with: `person-slug`), `Fold::cleared(request)` (whether a `changes_requested` record is cleared at its fingerprint, the one rule `land.approval` and `web.worker` hold by: `request-cleared`), and, for signed repositories (`signed-ledger-records`), `signed(root, rev)` (whether `rev`'s tree holds `docs/allowed_signers`), `sign(root, record, namespace)` (the record with its `id`, `writer`, `at` and `sig` filled in, signed with the person's key) and `signed_by(root, rev, record, namespace)` (whether the record's `sig` is a valid signature by a key `rev`'s `docs/allowed_signers` lists for the record's writer's person). `clm ledger compact` prunes the caller's own superseded records, `clm ledger verify` audits `main` against the records, approvals and signatures included, and `clm ledger show` prints the fold. Changing a record kind or a field name is a change to this Contract: `land`, `gate`, `view`, `web.worker` and the skills all read them.

Record kinds:
- `approved`: `fingerprint`, `branch`, `change` (a change ID, or null). A human's approval of a diff (`pinned-approval`).
- `checked`: `code_key`, `command`, and optionally `basis` (`head` and `base`: the code keys of a branch's head and of `main` whose checks it was composed from: `composed-check`). A tree whose code key passed the check command and `clm check`, written by `clm land` for its merged tree or by `clm land --check-only` for a clean, committed `HEAD` that contains `origin/main` (`land/check-only`, `worktree-check-counts`), or whose code key `code_key_with` maps, its `clm.toml` settings unchanged, to one that did, the land having run that tree's `clm check` and the tests that read `clm.toml` (`land/settings-unchanged-is-checked`). A record with `basis` is a *composed* check: `clm land` writes one for a merged tree it checked only where the branch and `main` overlap (`land/overlap-recheck`), and `clm ledger verify` audits that `checked` records name both of its keys.
- `landed`: `commit`, `branch`, `land_kind` (`spec` or `code`), `review` (whether the land needed a human's review, `gate/needs-review`; false for code), `fingerprint`, `code_key` (the landed commit's code key, which the land's `checked` record names), `recheck` (how the land checked its merged tree: `checked`, its code key already checked; `none`, the overlap selected nothing; `overlap`, the overlap ran; `selected`, the branch's own selection ran; absent on older records: `landed-recheck`), `recheck_files` (with `overlap` or `selected`, the number of `file` lines the check ran, or null when it ran every test), `changes` (the live change IDs a spec land added or modified), and, when the land ran `[land] check`, `check_seconds` (the command's wall-clock time in whole seconds, rounded down) and `steps` (each step it marked, in order, as `{name, seconds, selected_by}`, `selected_by` the paths its `::clm-selected::` lines named: `land/step-times`), which `Fold::timed_lands(n)` reads back as `TimedLand { commit, at, check_seconds, steps }` with `TimedStep { name, seconds, selected_by }` (`landed-times`). A `checked` record stays a fact about a code key and carries no times (`graph-test-selection`, `land-times-on-the-landed-record`). Older records lack `code_key` (`landed-record-names-its-code-key`). Records from before review was yes or no (`yes-no-review`) have `tier` instead of `review`: `HARD` reads as `review: true`, anything else as `false`.
- `subscribed`: `endpoint`, `p256dh`, `auth`. A browser's Web Push subscription for this repo, which `web.worker` notifies (`push-subscriptions-in-ledger`).
- `unsubscribed`: `subscription` (the `id` of a `subscribed` record). That subscription's end.
- `proposed`: `slug`, `title`, `body`, and the components at `/nodes`. A backlog item proposed from the web app, which `clm rebase` files as `backlog/<slug>.md` on the next `impl/*` branch (`web-proposals-as-ledger-records`).
- `withdrawn`: `proposal` (the `id` of a `proposed` record). That proposal's withdrawal by its proposer.
- `answered`: `item` (a backlog item's slug) and `answer`. A person's answer from the web app to a `question` item that lists it in `choices`, or `done` for a `human-task` item, which `clm triage` prints under the item (`web-answers-as-ledger-records`).
- `commented`: `branch`, `fingerprint`, `target` (always present: a requirement ID `<component>/<slug>`, or null for the change as a whole), `text`. A reviewer's comment on a spec branch's diff, made in the web app (`web-review-feedback-as-ledger-records`).
- `changes_requested`: `branch`, `fingerprint`, `text`. A reviewer's request, made in the web app, that the branch change before it lands; `clm land` holds the branch while its fingerprint is the one named (`land`).
- `effort`: `branch`, `session`, `models` (a map from a model's name to its `input`, `output`, `cache_read` and `cache_write` token counts), `active_seconds`. The work one Claude Code session spent on one branch, as `clm effort record` read it from this machine's transcripts (`effort`). Its `id` is `effort_id(session, branch)`, not random, so a later record of the same session and branch supersedes the earlier one (`effort-accounted`).
- `decided`: `capability` (the ID of the entry decided on, `capability/<slug>` or `architecture/<slug>`; the key keeps its name for both, so earlier records and earlier `clm`s read alike), `value` (`high`, `medium` or `low`), `decision` (`keep`, `cut`, `redesign` or `accept`), `settles` (the slugs of the workaround items it settles). A human's review decision on a capability or an architectural constraint (`review-capability-cost`, `docs/decisions/architecture-review.md`).
- `verified`: `commit` (a commit of `main`), `passed` (true or false), `since` (the last commit a `verified` record named before it, or null), `lands` (the first-parent commits of `main` it covers, oldest first), `culprit` (the land that broke it, or null), `components` (the components a red result holds, empty when it passed), `mode` (`local`, `pipeline` or `import`), `run` (the host pipeline run's URL, or null). The result of running a project's slow tests after the land on that commit (`verify-after-land`).

Every record also has `id` (random 128-bit hex, except an `effort` record's, which is `effort_id(session, branch)`), `kind` (one of the twelve above), `writer` (`<person>/<clone>`) and `at` (the time it was written, in UTC, always as `YYYY-MM-DDTHH:MM:SS.mmmZ`, as `clm` and `web.worker` write it, so two records' times order as strings: the fold sorts by `at` so, and `cleared` compares a request's and an approval's so for `land` and `web.worker` alike). A signed record also has `sig`: an `approved` record signed under the namespace `clm-approve`, or a `landed` record under `clm-land`; the other kinds never have it, and a record without it is unsigned. A record is one flat JSON object, so no record field shares a name with these five: the land's kind is `land_kind`, not `kind`.

## Design
**Writers.** The person part falls back to `git config user.name` slugged (`person-slug`) when no layer sets `writer`; an empty result is an error telling the human to set `writer`. The clone part is written to `.git/clm/writer` in the common git directory, so a clone's worktrees share it. Many sessions in one clone commit to one local branch, and git's ref lock orders them; two clones of one person never share a branch, so nothing ever rebases a ledger.

**Branches.** `clm/ledger/<person>/<clone>` is an orphan branch holding one file, `records.jsonl`, one JSON object per line, appended in order. The first append creates the branch with `git commit-tree` on a tree holding that one file, with no parent; later appends build the new tree from the previous one. The working tree and the index are never touched (`GIT_INDEX_FILE` or a tree built by hand). After each commit the branch is pushed without force, retrying a failed push a few times with a short wait, since a refusal can only be a network or lock failure: nobody else writes the branch. When `origin` can't be reached, `append` still commits and returns the record with a note that it isn't pushed; `clm reconnect` pushes it later (`land`). On every host, `none` included, `append` pushes and `compact` fetches first, `none`'s `origin` being the clone's own remote (`host/none-keeps-a-remote`).

**Fold.** `fold` reads `records.jsonl` at the tip of each `refs/remotes/origin/clm/ledger/*` ref and of the local writer's branch (which may be ahead), with `git cat-file`. Records are commutative facts, so the fold is the same in any order, and a duplicate `id` (the same record read from two refs) counts once. It doesn't fetch: the caller fetches first when it needs the latest (`land` does; `clm status` doesn't, as for claims). A line that doesn't parse is skipped with a warning, so one bad line never hides the rest. A `reviewed` line, a kind that recorded an after-merge review and that the ledger doesn't have, is skipped without a warning. The open proposals count any `withdrawn` record, whoever wrote it (who may write one is the Worker's rule). The comments and the change requests are maps by `branch` to the records in `at` order, so `land` and the Worker take a branch's requests and pick those naming a fingerprint. For `effort` records a repeated `id` is not one record read twice but a later count of the same session and branch, so the fold keeps the latest `at`, whatever its writer, in its records and its lines alike. Records with one `at` are ordered by writer and then by their lines' text, so every clone folds to the same one whatever order it reads the refs in. A repeated `id` on a record of another kind, or an `effort` record repeating another kind's `id`, never replaces what the fold holds. Whether a proposal has been filed or resolved is read from the tree, never from the ledger (`web-proposals-as-ledger-records`), and whether a change request is cleared is `cleared`'s rule over the fold's approvals (below), which `land` holds a land by and `web.worker` shows. Nothing compacts a web ledger, as nothing compacts its approvals, so the comments on a branch stay and a later branch of the same name shows them as made on an earlier version.

**Code keys.** `code_key(root, rev)` is a SHA-256 over `git ls-tree -r <rev>` with the lines under a doc-only path removed (`doc_only` lists them once: `light-ci-for-doc-prs`). Every path under `docs/` is doc-only, the guide's pages under `docs/guide/` and `docs/spec-format.md` included, though `clm guide` compiles them into the binary (`guide-pages-are-doc-only`): the tests that read them, from the tree or through `clm guide` or the site built from them, hold `clm:reads` markers (`doc-readers-marked`), so a land that edits a page runs those tests rather than every test, as a land that edits `specs/` does. Two trees that differ only in doc-only paths have one key, so a spec land on code that was already checked needs only `clm check` and the tests that read what it changed.

**A land's code key.** A `landed` record's *code key* is its `code_key` field, or, for a record without it, its *legacy key*, computed on demand (`landed-record-names-its-code-key`, `legacy-code-key`): the key over `git ls-tree -r <commit>` with the lines under the legacy doc-only paths removed. That list is frozen in the crate as `legacy_doc_only(path)`, a literal of its own beside `doc_only`'s that calls nothing live (`legacy-doc-only`); it never follows `doc_only`: those lands' `checked` records hold keys computed with it, and no change to the list can re-check a tree that has landed. `doc_only` and `code_key` move with the repository, for every land that records its key and for `check.affected`, which reads `doc_only` so a live land's selection and its key agree. `land` writes the field from the merged tree's key, which it has already computed to look up or append the `checked` record, so it costs nothing at land time; `verify` and `compact` then read it instead of listing a tree per land. `append` leaves the field out of a record that has no key, rather than writing `null`, so old and new lines read alike; the fold holds such a land with its key unknown (`None`). When compaction needs the computed key of a land whose commit isn't in the clone (another writer's land not yet fetched), it keeps every `checked` record and warns naming the commit, since dropping a check is the one mistake it can't undo.

**`clm ledger compact`.** Prunes what a land has made redundant, the caller's own records only (`compact`). A `checked` record that a land relied on stays, because `verify` reads it; when several name one key (a retried land, or two writers checking one tree), all of them stay, since compaction never decides between writers' facts. `landed` records stay: one line per land is `main`'s audit trail. `reviewed` lines go, since nothing reads them. The `commented` and `changes_requested` records whose fingerprint a `landed` record names go whatever their `branch`, as approvals do: the diff they were about is on `main`. `proposed`, `withdrawn` and subscription records stay: a proposal is open until a change on `main` resolves the item it became, which readers take from the tree. An `effort` record goes when the fold holds another record with its `id`, which is a later one (the order above), whoever wrote it; `decided` records stay, since they are the review's history. A signed land that needed review keeps its `approved` record, since `verify` needs the one that cleared it; where nothing is signed, nothing reads approvals after the land and they go.

**`clm ledger verify [--since <rev>]`.** Walks the first-parent chain of `main` from `--since` (default: the whole history after the first commit with a `landed` record). `clm land` always makes a merge commit, so a single-parent commit on that chain came from around it, such as `git push origin HEAD:main`. A commit whose message starts `Release v` is the release workflow's push (`host-ci-for-releases`): a merge commit when `main` moved during the release, or the version bump itself when its merge fast-forwards. A commit made by hand under a decision isn't reported either (`verify-by-hand-merge`): the trailer `Clm-By-Hand: <name>` is read with `git log`'s `%(trailers:key=Clm-By-Hand,valueonly,separator=%x1e)` through `clm-git`, and `<name>` is a kebab-case decision name whose `status: accepted` is read in its frontmatter (between a first line `---` and the next). In a signed repository the hatch is shut: anyone who can push could name a decision, and the commit would skip the signature audit. It is the one escape hatch from `clm land` (`refounding-edits-specs-directly`), and `git log --grep Clm-By-Hand` lists every use. `clm check` runs `verify` when the checked-out branch is `main` and the repository has a ledger branch, so a push to `main` that didn't go through `clm land` is a blocking error there (`no-silent-drift`). `clm wrap-up` runs it on `origin/main` too (`orchestrator.checkpoint`). Hosts with hooks can run it in `pre-receive`; this is the after-the-fact check for the rest.

**`clm ledger show`.** Prints the fold, one line each, sorted by `at`, for a human or a test to read: `approved <fingerprint> <branch> <writer> <at>` and `landed <commit> <branch> <land_kind> <review> <writer> <at>` (`<review>` being `review` or `-`); the other kinds print as their requirements say (`subscription-records`, `proposal-records`, `feedback-records`, `effort-records`, `decision-records`), with no quoting.

**Cleared requests** (`request-cleared`). Any approval counts, whatever the approval policy and whether or not it is signed, since a request withholds a land rather than granting one. Whether a request names the fingerprint a land or a review is at stays the caller's test. A record whose `at` isn't in the pinned form sorts by its string, for both callers alike: the helper doesn't refuse it.

**Signed records** (`signed-ledger-records`). A repository is *signed* at a commit when that commit's tree holds `docs/allowed_signers`, an OpenSSH allowed-signers file (`ssh-keygen(1)`, ALLOWED SIGNERS) whose principals are writer persons; `signed(root, rev)` looks for it with `git cat-file` through `clm-git`. A record's *signed message* is the record as a JSON object without `sig`, its keys sorted bytewise, with no whitespace, as `serde_json::to_string` prints a `serde_json::Value` built without the `preserve_order` feature, so the bytes don't depend on the field order of a struct. `sign` runs `ssh-keygen -Y sign -f <key> -n <namespace>` with the signed message on stdin and puts its armored output, the `-----BEGIN SSH SIGNATURE-----` block, in `sig`. The key is git's `user.signingkey` when the repository's `gpg.format` is `ssh` (`git config --get` through `clm-git`): a path is passed as is, and a value starting `key::` is a literal public key, written to a temporary file so that `ssh-keygen` signs through `ssh-agent`. With no such key it fails naming `user.signingkey` and `gpg.format`; when `ssh-keygen` fails or isn't on `PATH` it fails naming the command and its stderr. `append` keeps an `id`, `writer`, `at` and `sig` that the record already has, so a caller signs a record before an act it must not do unsigned and appends it after. `signed_by` writes `rev`'s `docs/allowed_signers` and the `sig` to temporary files and runs `ssh-keygen -Y verify -f <signers> -I <person> -n <namespace> -s <sig>` with the signed message on stdin, `<person>` being the person part of the record's `writer`; it is true exactly when that exits 0. `ssh-keygen` is neither `git` nor `gh`, so `clm-ledger` runs it itself. The fold keeps `sig` as it reads it and doesn't verify it: a signature is checked where an approval is counted (`land`) and by `verify`, and `fold_records` in wasm carries `sig` as data.

**Auditing signed lands.** For each landed commit on the chain whose first parent is signed, `verify` reports `ledger/unsigned-land` when `signed_by(root, <first parent>, <record>, "clm-land")` is false for every `landed` record naming it. For such a land whose signed record says it needed review, it recomputes the fingerprint as `clm land` computed it, with `clm-gate`'s `fingerprint_of` over `git ls-tree -r -z` of the merge base of the commit's two parents and of its second parent. An approval *counts* when `signed_by(root, <first parent>, <approved record>, "clm-approve")` is true and, when the `approval` that `<first parent>:clm.toml` sets is `other` (read with the `toml` crate; a missing file or key means `any`), its writer's person differs from the signed `landed` record's writer's person and from the slugged author name (`git log --format=%an`, slugged as the writer is) of every commit in `<first parent>..<second parent>`. A recomputed fingerprint that differs from the `landed` record's is reported as `ledger/unapproved-land` too, naming both, since any approval was given for another diff. A land whose first parent isn't signed is judged by its record alone, so history from before signing stays valid. Without `--review`, `verify` trusts a land's recorded kind and review, a false one being attributable through the record's signature. `clm-gate` is a dependency of the default feature only, so the wasm build doesn't see it. When a commit has several `landed` records, `verify` judges it by any one that is signed, and requires an approval when any signed one says it needed review.

**Signing details.** `sign` takes what `append` takes, the record's body and the injected environment, and returns a full record that `append` accepts unchanged; the exact Rust signatures are the builder's. The signed message is built explicitly with sorted keys (a `BTreeMap`), never by relying on `serde_json`'s `preserve_order` feature being off, and holds the fields exactly as the line writes them: a `code_key` left out of the line is left out of the message, a `null` stays `null`. `signed_by` checks the line as read, not a record re-serialised from the struct: the fold keeps each record's parsed JSON object beside it, so a field a newer `clm` adds still verifies on an older one. `signed_by` returns false for a record without `sig`, a `rev` without the file, or a signature `ssh-keygen` rejects, and an error when `ssh-keygen` can't be run, so a missing tool is never reported as a forged land. The key is read with `git config --get --type=path user.signingkey`, so `~/` expands, and a relative path is taken from the repository root; `gpg.ssh.defaultKeyCommand` is not used. The git config lookups go through `clm-git` with the injected environment (`HOME`, `GIT_CONFIG_GLOBAL`), so a test gives its own global config. `ssh-keygen` sets no time limit: a key that needs a passphrase or a touch waits for it. `docs/allowed_signers` and `clm.toml` are read at the top of the git repository, as `code_key`'s paths are. `toml` is a dependency of the default feature. `ssh-keygen` (OpenSSH 8.2 or later) must be on `PATH`.

**Recomputed review** (`verify-recomputes-review`). `clm ledger verify --review` recomputes the kind and review as `clm land` computed them in its step 6, the gate on the merge commit against the `origin/main` it merged into, which is the commit's first parent: `clm-gate`'s `evaluate_at(root, <first parent>, <commit>, false)`, read from git objects with no checkout (`gate/evaluate-at`). A land with no signed record is already `ledger/unsigned-land` and isn't recomputed. The recorded kind and review are those of the highest-ranked signed `landed` record naming the commit (the first in the fold's order among equals), and that same record supplies the lander and the fingerprint when a land recomputed as needing review is audited for approval. They rank `code` = `spec` < `spec review` < `mixed`, since a code land and a spec land that needs no review both land on checks alone, and the gate blocks a `mixed` diff outright. The message of `ledger/understated-land` is `recorded <kind> [review], the gate gives <kind> [review]` (as `clm land` prints them, so `recorded code, the gate gives spec review`). A change that needed review, recorded as not needing it, with no counted approval reports both rules, `ledger/understated-land` first, so the audit of one commit may give more than one finding. A recomputed rank at or below the recorded one is no finding: an over-stated land asked for more review than the diff needed. The gate's violations, `gate/up-to-date` among them, are ignored here; only its kind and review are read. A gate error on one land, such as a committed `change.md` the gate can't read, doesn't fail the run (`verify-unrated-land`): the message is `landed <sha>: the gate can't rate it: <the gate's message>`, so one old land the current gate can't rate neither hides the lands after it nor ends with an error every run that walks it, and the finding names the land for a human. A git failure of the walk itself, or of a read the audit makes outside the gate, still fails `verify` (exit 2).

`clm check` runs `verify` without `--review`, since recomputing runs the gate once per signed land, on every check of `main`, for the whole history, and nothing may remember what an earlier run verified (`git-is-truth`: no check reads a cache). `clm wrap-up` passes it, with `--since` at its review mark, so that it recomputes the lands since this clone's last checkpoint rather than the whole history (`orchestrator.checkpoint/checkpoint-review-mark`, `checkpoint-recomputes-new-lands`); a human can run it any time, and `clm ledger verify --review` with no `--since` is the full audit, which reads no mark. The flag covers every land, so `clm check`'s cost stays one walk of the chain. The library's `verify` takes the choice as an argument; its exact signature is the builder's.

## Requirements

### Requirement: writer-identity
`writer(root)` SHALL return `<person>/<clone>`, where `<person>` is `config`'s `writer` or, when unset, the repository's git `user.name` as a slug, and `<clone>` is the eight hex digits in `.git/clm/writer`, written on first use and read on every later one, so every worktree of the clone shares it. It SHALL fail, naming the `writer` setting, when the person part would be empty.

#### Scenario: first use
- GIVEN a clone whose git `user.name` is `Christopher Lunny`, no `writer` setting and no `.git/clm/writer`
- WHEN `writer(root)` runs
- THEN it returns `christopher-lunny/<8 hex digits>` and `.git/clm/writer` holds those digits

#### Scenario: shared by worktrees
- GIVEN that clone and a linked worktree of it
- WHEN `writer` runs in each
- THEN both return the same identity

#### Scenario: setting wins
- GIVEN `CLM_WRITER=clunny`
- WHEN `writer(root)` runs
- THEN its person part is `clunny`

### Requirement: append
`append(root, record)` SHALL add the record, with a fresh `id`, the writer and the time unless it already has them, as a record `sign` returned does (`record-signing`), as one JSON line at the end of `records.jsonl` on the writer's branch `clm/ledger/<person>/<clone>`, creating the orphan branch on first use, without touching the working tree or the index, and SHALL push the branch to `origin` without force, on every host, `none` included, whose `origin` is the clone's own remote (`host/none-keeps-a-remote`). When `origin` can't be reached it SHALL keep the commit and report the record as unpushed. It SHALL never rebase or force-push a ledger branch.

#### Scenario: first record
- GIVEN a clone with a bare `origin` and no ledger branch
- WHEN `append` runs with an `approved` record
- THEN `origin` has `clm/ledger/<writer>` whose `records.jsonl` holds one line with `kind` `approved`, the writer and an `at`, and the working tree is unchanged

#### Scenario: second record
- GIVEN that clone after the first record
- WHEN `append` runs with a `checked` record
- THEN `records.jsonl` on `origin` holds both lines in order, and the branch has two commits

#### Scenario: no host
- GIVEN a clone with `host = "none"` whose `origin` is a bare repository in its git directory, and no ledger branch
- WHEN `append` runs with an `approved` record
- THEN `origin` has the writer's ledger branch holding the record, and it reports the record as pushed

#### Scenario: offline
- GIVEN a clone whose `origin` URL points nowhere
- WHEN `append` runs
- THEN the local branch has the record, it reports the record as unpushed, and it doesn't fail

### Requirement: fold
`fold(root)` SHALL return the union of the records on every `origin/clm/ledger/*` ref and on the local writer's branch, counting a record once by `id`, indexed as approvals by fingerprint, checks by code key and lands by commit, without fetching. A line that doesn't parse SHALL be skipped with a warning naming the ref and line, not fail the fold. A `reviewed` line, a kind the ledger no longer has (`yes-no-review`), SHALL be skipped without a warning.

#### Scenario: two writers
- GIVEN `origin/clm/ledger/a/1` with an `approved` record for fingerprint `f` and `origin/clm/ledger/b/2` with a `landed` record for commit `c`
- WHEN `fold` runs
- THEN it has `f` among the approvals and `c` among the lands

#### Scenario: local ahead
- GIVEN the local writer's branch with an `approved` record not yet on `origin`
- WHEN `fold` runs
- THEN that approval is in the fold

#### Scenario: reviewed line
- GIVEN a ledger holding a `reviewed` line between two `approved` records
- WHEN `fold` runs
- THEN it returns both approvals and warns about nothing

#### Scenario: bad line
- GIVEN a ledger whose second line is not JSON
- WHEN `fold` runs
- THEN it returns the other records and warns naming the branch and line 2

### Requirement: code-key
`code_key(root, rev)` SHALL print the same key for two commits whose trees differ only under `specs/`, `changes/`, `backlog/` or `docs/`, the guide's pages under `docs/guide/` and `docs/spec-format.md` included, or in `ARCHITECTURE.md`, `CAPABILITIES.md`, `README.md`, `CLAUDE.md` or `AGENTS.md`, and a different key when any other entry differs, a new top-level file included.

#### Scenario: doc-only difference
- GIVEN commits `a`, `b` that changes only `docs/x.md` and `backlog/y.md`, `c` that also changes `crates/a/src/lib.rs`, `d` that adds `tool.toml` at the root, `e` that changes only `docs/guide/intro.md`, and `f` that changes only `docs/spec-format.md`
- WHEN `code_key` runs on each
- THEN `a`, `b`, `e` and `f` give the same key, and `c` and `d` each give a different one

#### Serves
- capability/land-safely
- architecture/deterministic-enforcement

### Requirement: compact
`clm ledger compact` SHALL rewrite the caller's own ledger without the `approved` records whose fingerprint any `landed` record names, except one whose fingerprint a signed land that needed review names, a land whose first parent's tree holds `docs/allowed_signers`, and without the `checked` records older than the newest `landed` record whose code key no land's code key matches, keep every `landed` record and every `checked` record a land's code key names, drop every `reviewed` line, push the result, and print the number removed. A land's code key is its record's `code_key`, or its legacy key, computed as `verify` computes it, when the record has none (`legacy-code-key`). It SHALL NOT touch another writer's branch.

#### Scenario: landed approval pruned
- GIVEN the caller's ledger with `approved f` and `checked k` (older), and another writer's `landed` record with fingerprint `f`, code key `m` and a later `at`
- WHEN `clm ledger compact` runs
- THEN the caller's ledger no longer holds either record, prints `removed 2`, and the other writer's branch is unchanged

#### Scenario: open approval kept
- GIVEN the caller's ledger with `approved g` that no `landed` record names
- WHEN `clm ledger compact` runs
- THEN the record is still there

#### Scenario: a land's check kept
- GIVEN the caller's ledger with `checked k` and then a `landed` record with code key `k`, and `clm ledger verify` printing nothing
- WHEN `clm ledger compact` runs
- THEN `checked k` is still there, and `clm ledger verify` still prints nothing

#### Scenario: an older land's check kept
- GIVEN the caller's ledger with `checked k`, a `landed` record for commit `c` with no `code_key` field, where `c`'s tree holds `docs/spec-format.md` and its legacy key is `k`, and a later `landed` record with code key `m`
- WHEN `clm ledger compact` runs
- THEN `checked k` is still there

#### Scenario: reviewed lines dropped
- GIVEN the caller's ledger with a `reviewed` line and an `approved` record that no `landed` record names
- WHEN `clm ledger compact` runs
- THEN it prints `removed 1`, and only the `approved` record is left

#### Scenario: a signed land's approval kept
- GIVEN the caller's ledger with `approved f`, and a `landed` record that needed review with fingerprint `f` for a merge commit whose first parent holds `docs/allowed_signers`
- WHEN `clm ledger compact` runs
- THEN `approved f` is still there, and `clm ledger verify` still accepts that land

### Requirement: verify
`clm ledger verify [--since <rev>]` SHALL report, in `check`'s format, `ledger/unrecorded-land` for each commit on `main`'s first-parent chain after `<rev>`, merge or not, that no `landed` record names, except one whose message starts `Release v`, and `ledger/unchecked-land` for each landed commit whose land's code key no `checked` record names, exiting 0 with no output when there are none and 1 otherwise. A land's code key SHALL be its record's `code_key`, or, only when the record has none, its legacy key: the commit's tree hashed as `code_key` does with `docs/spec-format.md` left out as well (`legacy-code-key`). `clm check` SHALL run it when the checked-out branch is `main` and the repository has a ledger branch.

#### Scenario: clean history
- GIVEN a `main` whose two merge commits each have a `landed` record and whose code keys have `checked` records
- WHEN `clm ledger verify` runs
- THEN it prints nothing and exits 0

#### Scenario: push behind clm's back
- GIVEN a `main` with a merge commit no `landed` record names
- WHEN `clm ledger verify` runs
- THEN it prints `ledger/unrecorded-land` naming that commit and exits 1

#### Scenario: direct push
- GIVEN a landed `main` and then a single-parent commit pushed straight onto it, which no `landed` record names
- WHEN `clm ledger verify` runs
- THEN it prints `ledger/unrecorded-land` naming that commit and exits 1

#### Scenario: release exempt
- GIVEN a `main` whose only unrecorded commits are a merge commit with the message `Release v0.9.0` and, later, a single-parent commit with the message `Release v0.10.0`
- WHEN `clm ledger verify` runs
- THEN it prints nothing and exits 0

#### Scenario: unchecked land
- GIVEN a `main` whose merge commit has a `landed` record with code key `k` and no `checked` record names `k`
- WHEN `clm ledger verify` runs
- THEN it prints `ledger/unchecked-land` naming that commit and exits 1

#### Scenario: recorded key read
- GIVEN a landed merge commit whose `landed` record's `code_key` is `k`, a `checked` record for `k`, and no `checked` record for the commit's own computed code key
- WHEN `clm ledger verify` runs
- THEN it prints nothing and exits 0

#### Scenario: legacy key for a land recorded without one
- GIVEN a landed merge commit whose tree holds `docs/spec-format.md`, whose `landed` record has no `code_key` field, a `checked` record for the commit's legacy key, and no `checked` record for `code_key(root, commit)`
- WHEN `clm ledger verify` runs
- THEN it prints nothing and exits 0

#### Scenario: check runs it on main
- GIVEN a repository on `main` with a ledger branch and the history of *push behind clm's back*
- WHEN `clm check` runs
- THEN it reports `ledger/unrecorded-land`

#### Serves
- capability/land-safely
- architecture/per-writer-ledgers
- architecture/git-is-truth

### Requirement: show
`clm ledger show` SHALL print the fold, one record per line sorted by `at`, each starting with its kind and naming its fingerprint, code key or commit, its branch where it has one, whether a land needed review, its writer and its time.

#### Scenario: listing
- GIVEN ledgers holding an `approved` and a `landed` record
- WHEN `clm ledger show` runs
- THEN it prints two lines, `approved ...` then `landed ...`, in time order

#### Serves
- capability/see-project-state

### Requirement: ledger-builds-for-wasm
`clm-ledger` SHALL build for `wasm32-unknown-unknown` with its default features disabled, exposing `fold_records` over records already read, so `web.worker` computes the same state.

#### Scenario: wasm build
- GIVEN the workspace
- WHEN `cargo build -p clm-ledger --target wasm32-unknown-unknown --no-default-features` runs
- THEN it succeeds

#### Serves
- capability/follow-from-browser
- architecture/git-is-truth

### Requirement: git-through-layer
`clm-ledger` SHALL list `clm-git` among its dependencies and SHALL run no `git` or `gh` process itself: every call goes through `clm-git`'s `git`, `run` or repo queries (`git-plumbing-layer`).

#### Scenario: covered by the scan
- GIVEN the workspace
- WHEN a test in `clm-ledger` reads `crates/clm-ledger/Cargo.toml` and every file under `crates/clm-ledger/src`
- THEN `clm-git` is a dependency and no file holds `Command::new("git")` or `Command::new("gh")`

#### Serves
- architecture/reuse-first

### Requirement: landed-record-fields
A `landed` record SHALL be written as one flat JSON object whose `kind` is `landed` and whose land kind is the field `land_kind` (`spec` or `code`), with `commit`, `branch`, `review` (a bool), `fingerprint`, `code_key` and `changes`, and never `tier`. A `landed` line without `code_key` SHALL still parse, with the code key unknown. A `landed` line with `tier` and no `review`, written before review became yes or no, SHALL parse with `review` true when `tier` is `HARD` and false otherwise.

#### Scenario: written
- GIVEN a `landed` record for commit `c`, land kind `code`, no review and code key `k`
- WHEN `append` writes it
- THEN its line in `records.jsonl` has `"kind":"landed"`, `"land_kind":"code"`, `"review":false` and `"code_key":"k"`, no `tier`, and no other field named `kind`

#### Scenario: written before the code key
- GIVEN a ledger line `{"id":"…","writer":"a/1","at":"…","kind":"landed","commit":"c","branch":"impl/x","land_kind":"code","tier":null,"fingerprint":"f","changes":[]}`
- WHEN `fold` reads it
- THEN `c` is among the lands with no code key and `review` false, and no warning is printed

#### Scenario: written with a tier
- GIVEN two ledger lines like the one above for commits `h` and `s`, with `"tier":"HARD"` and `"tier":"SOFT"`
- WHEN `fold` reads them
- THEN `h`'s land has `review` true and `s`'s false

### Requirement: append-refused-push
When `origin` refuses the push of the writer's ledger branch, `append` SHALL keep the commit on the local branch and report the record as unpushed, without failing, as it does when `origin` can't be reached (`append`).

#### Scenario: push refused
- GIVEN a clone with a bare `origin` whose `pre-receive` hook declines every push
- WHEN `append` runs
- THEN the local branch has the record, `origin` has no ledger branch, it reports the record as unpushed, and it doesn't fail

### Requirement: doc-only
`clm-ledger` SHALL offer `doc_only(path)`, true for a path relative to the repository root under `specs/`, `changes/`, `backlog/` or `docs/`, `docs/guide/` and `docs/spec-format.md` included, or equal to `ARCHITECTURE.md`, `CAPABILITIES.md`, `README.md`, `CLAUDE.md` or `AGENTS.md`, and false for any other path, with its default features disabled as well as enabled. `code_key` SHALL leave out exactly the entries `doc_only` names, so the list is written once.

#### Scenario: paths
- GIVEN the paths `docs/x.md`, `docs/guide/intro.md`, `docs/spec-format.md`, `backlog/y.md`, `CLAUDE.md`, `AGENTS.md`, `crates/a/src/lib.rs` and `docsy/z.md`
- WHEN `doc_only` runs on each
- THEN it returns true for `docs/x.md`, `docs/guide/intro.md`, `docs/spec-format.md`, `backlog/y.md`, `CLAUDE.md` and `AGENTS.md`, and false for `crates/a/src/lib.rs` and `docsy/z.md`

#### Serves
- capability/land-safely
- architecture/deterministic-enforcement

### Requirement: record-signing
`sign(root, record, namespace)` SHALL return the record with `id`, `writer` and `at` filled in and `sig` holding the output of `ssh-keygen -Y sign -n <namespace>` over its signed message (the record as a JSON object without `sig`, keys sorted, no whitespace), using git's `user.signingkey` when `gpg.format` is `ssh`, and SHALL fail naming `user.signingkey` when there is no such key. `append` SHALL write a record's existing `id`, `writer`, `at` and `sig` unchanged, and the fold SHALL read a line with or without `sig`.

#### Scenario: signed and appended
- GIVEN a clone whose git config sets `gpg.format = ssh` and `user.signingkey` to a fresh ed25519 key's path
- WHEN `sign` runs on an `approved` record with namespace `clm-approve` and `append` writes the result
- THEN its line in `records.jsonl` holds that `sig`, and `ssh-keygen -Y verify` with the key's public half, the namespace `clm-approve` and the line's object without `sig` (keys sorted, no whitespace) on stdin exits 0

#### Scenario: no key
- GIVEN a clone with no `user.signingkey`
- WHEN `sign` runs
- THEN it fails naming `user.signingkey` and appends nothing

#### Serves
- architecture/signed-ledger-records
- capability/approve-intent

### Requirement: signature-check
`signed(root, rev)` SHALL be true exactly when `rev`'s tree holds `docs/allowed_signers`. `signed_by(root, rev, record, namespace)` SHALL be true exactly when the record has a `sig` that `ssh-keygen -Y verify` accepts against `rev`'s `docs/allowed_signers` for the person part of the record's `writer`, the namespace and the record's signed message.

#### Scenario: listed signer
- GIVEN a commit whose `docs/allowed_signers` lists key `K` for `dana`, and an `approved` record by writer `dana/9f9f9f9f` signed with `K` under `clm-approve`
- WHEN `signed_by` runs with namespace `clm-approve`
- THEN it is true

#### Scenario: another person's name
- GIVEN the same file, and an `approved` record by writer `dana/1a2b3c4d` signed with a key the file lists only for `christopher-lunny`
- WHEN `signed_by` runs with namespace `clm-approve`
- THEN it is false

#### Scenario: edited after signing
- GIVEN the record of *listed signer* with its `fingerprint` changed after it was signed
- WHEN `signed_by` runs
- THEN it is false

#### Scenario: wrong namespace or unsigned
- GIVEN the record of *listed signer*, and the same record without `sig`
- WHEN `signed_by` runs on the first with namespace `clm-land`, and on the second with `clm-approve`
- THEN both are false

#### Serves
- architecture/signed-ledger-records
- capability/approve-intent

### Requirement: verify-signed-lands
`clm ledger verify` SHALL, for each landed commit on `main`'s first-parent chain whose first parent's tree holds `docs/allowed_signers`, report `ledger/unsigned-land` when no `landed` record naming it is signed under `clm-land` by its writer, and, when that record says the land needed review, report `ledger/unapproved-land` unless an `approved` record for the fingerprint recomputed from the commit's parents is signed under `clm-approve` by its writer and, when the first parent's `clm.toml` sets `approval = "other"`, its writer's person is neither the signed land's writer's person nor the slugged author of any commit the land brought in. It SHALL judge a land whose first parent has no `docs/allowed_signers` as it did before.

#### Scenario: signed and approved by another
- GIVEN `approval = "other"` and `docs/allowed_signers` listing `christopher-lunny` and `dana` on `main`, then a land that needed review of a branch authored by `Christopher Lunny`, its `landed` record signed by `christopher-lunny`, and an `approved` record for its fingerprint signed by `dana`
- WHEN `clm ledger verify` runs
- THEN it prints nothing about that land

#### Scenario: approval in another's name
- GIVEN the same, but the `approved` record names writer `dana/1a2b3c4d` and is signed with `christopher-lunny`'s key
- WHEN `clm ledger verify` runs
- THEN it prints `ledger/unapproved-land` naming the land's commit and exits 1

#### Scenario: author name changed
- GIVEN the same as *signed and approved by another*, but the branch's commits are authored as `Zed` and the only `approved` record is signed by `christopher-lunny`, who signed the land
- WHEN `clm ledger verify` runs
- THEN it prints `ledger/unapproved-land` naming the land's commit

#### Scenario: unsigned land
- GIVEN a land whose first parent holds `docs/allowed_signers` and whose `landed` record has no `sig`
- WHEN `clm ledger verify` runs
- THEN it prints `ledger/unsigned-land` naming that commit and exits 1

#### Scenario: approval for another diff
- GIVEN the land of *signed and approved by another*, whose `landed` record names the fingerprint `dana` signed but whose parents give a different one
- WHEN `clm ledger verify` runs
- THEN it prints `ledger/unapproved-land` naming the commit and both fingerprints

#### Scenario: before the switch
- GIVEN a `main` whose earlier lands, with unsigned `landed` and `approved` records, came before the land that added `docs/allowed_signers`
- WHEN `clm ledger verify` runs
- THEN it reports none of those earlier lands

#### Serves
- architecture/signed-ledger-records
- architecture/humans-review-intent
- capability/approve-intent

### Requirement: verify-recomputes-review
`clm ledger verify --review` SHALL, for each landed commit on `main`'s first-parent chain whose first parent's tree holds `docs/allowed_signers` and that a signed `landed` record names, recompute its kind and whether it needs review with `clm-gate`'s `evaluate_at` against its first parent, and report `ledger/understated-land`, naming the commit and the recorded and recomputed kind and review, when the recomputed one ranks above the recorded one in the order `code` = `spec` < `spec review` < `mixed`, the recorded one being the highest among the signed records naming the commit. It SHALL audit a land recomputed as needing review for a counted approval as `verify-signed-lands` audits a land recorded so. Without `--review`, and for a land whose first parent has no `docs/allowed_signers`, it SHALL judge the land by its recorded kind and review as before.

#### Serves
- architecture/signed-ledger-records
- architecture/humans-review-intent

#### Needs
- gate/evaluate-at

#### Scenario: review recorded as none
- GIVEN `docs/allowed_signers` on `main`, then a land of a branch that only edits `ARCHITECTURE.md`, whose signed `landed` record has land kind `spec` and `review` false, a `checked` record for its code key, and no `approved` record for its fingerprint
- WHEN `clm ledger verify --review` runs
- THEN it prints `ledger/understated-land` naming the commit, `spec` and `spec review`, and `ledger/unapproved-land` naming the commit, and exits 1

#### Scenario: spec recorded as code
- GIVEN `docs/allowed_signers` on `main`, then a land of a branch that only edits `ARCHITECTURE.md`, whose signed `landed` record has land kind `code`, and a `checked` record for its code key and an approval counted for its fingerprint
- WHEN `clm ledger verify --review` runs
- THEN it prints `ledger/understated-land` naming the commit, `code` and `spec review`, and exits 1

#### Scenario: honest lands
- GIVEN `docs/allowed_signers` on `main`, then a spec land of a branch that only adds a requirement and a code land of a branch that only changes `src/x.rs`, each signed, checked, and recorded with the kind and review `clm gate` gives its merge commit against its first parent
- WHEN `clm ledger verify --review` runs
- THEN it prints nothing and exits 0

#### Scenario: over-stated
- GIVEN the land of *honest lands* that only adds a requirement, recorded instead with `review` true and approved by a counted `approved` record
- WHEN `clm ledger verify --review` runs
- THEN it prints nothing about that land

#### Scenario: without the flag
- GIVEN the history of *review recorded as none*
- WHEN `clm ledger verify` runs without `--review`
- THEN it prints nothing about that land

#### Scenario: before the switch
- GIVEN a land recorded with `review` false, unsigned, of a branch that edits `ARCHITECTURE.md`, whose first parent has no `docs/allowed_signers`
- WHEN `clm ledger verify --review` runs
- THEN it reports nothing about that land

### Requirement: shared-plumbing
With its `git` feature on, `clm-ledger` SHALL take the remote's name from `clm-git`'s `REMOTE` wherever it names the remote to git (pushing a ledger branch, listing `refs/remotes/<remote>/clm/ledger/`), building each such argument from `REMOTE` rather than spelling it: no line of a file under `crates/clm-ledger/src`, apart from those whose first non-blank characters are `//`, SHALL hold `"origin"`, `"origin/` or `remotes/origin` (`remote-plumbing-scan`). Messages for a person that name `origin`, such as `the compacted ledger isn't on origin yet: git push origin <branch>`, SHALL keep their wording. Its record timestamps keep their own clock, since they need milliseconds and a formatted date that `clm-git`'s `now()` doesn't give. With default features off it SHALL still build for `wasm32-unknown-unknown` (`ledger-builds-for-wasm`).

#### Scenario: no copy of the remote's name
- GIVEN the workspace
- WHEN a test in `clm-ledger` reads every file under `crates/clm-ledger/src`, skipping each line whose first non-blank characters are `//`
- THEN no line holds `"origin"`, `"origin/` or `remotes/origin`

#### Serves
- architecture/reuse-first

### Requirement: subscription-records
The fold, `fold` and `fold_records` alike, SHALL parse `subscribed` and `unsubscribed` records and hold as its subscriptions every `subscribed` record whose `id` no `unsubscribed` record's `subscription` names, whatever order the records are read in. `clm ledger show` SHALL print a `subscribed` record as `subscribed <id> <the endpoint's origin> <writer> <at>` and an `unsubscribed` one as `unsubscribed <subscription> <writer> <at>`, never printing `p256dh` or `auth`. `clm ledger compact` SHALL keep both kinds when they are in the caller's own ledger.

#### Scenario: ended in another order
- GIVEN `clm/ledger/alice/web` holding `unsubscribed` naming `s1`, then `subscribed` `s1` for `https://push.example/s/1`, then `subscribed` `s2` for `https://push.example/s/2`
- WHEN `fold_records` runs over them
- THEN its subscriptions are exactly `s2`

#### Scenario: shown without keys
- GIVEN a ledger holding `subscribed` `s2` for `https://push.example/s/2` by `alice/web`
- WHEN `clm ledger show` runs
- THEN it prints a line starting `subscribed s2 https://push.example alice/web`, and the record's `p256dh` and `auth` appear nowhere in its output

#### Scenario: compaction keeps them
- GIVEN the caller's own ledger holding `subscribed` `s1`, `unsubscribed` naming `s1`, and an `approved` record whose fingerprint a `landed` record names
- WHEN `clm ledger compact` runs
- THEN it prints `removed 1`, and both subscription records are still there

#### Serves
- capability/follow-from-browser
- architecture/git-is-truth

### Requirement: verify-unrated-land
`clm ledger verify --review` SHALL, when the gate fails on a landed commit it recomputes, report `ledger/unrated-land` naming the commit and the gate's message, judge that land by its recorded kind and review, and go on to the lands after it, exiting 1 rather than failing. It SHALL still fail, as for a git failure, when git fails outside the gate.

#### Scenario: unrated land
- GIVEN `docs/allowed_signers` on `main`, then a signed and checked land recorded `spec` of a branch that adds `changes/x/change.md` as a gitlink, an entry whose object isn't in the repository, which the gate can't read, and after it the land of *review recorded as none*
- WHEN `clm ledger verify --review` runs
- THEN it prints `ledger/unrated-land` naming the first land's commit and, in the gate's message, the change's path, `ledger/understated-land` and `ledger/unapproved-land` naming the second's, and exits 1

#### Scenario: unrated land without the flag
- GIVEN the first land of *unrated land* alone
- WHEN `clm ledger verify` runs without `--review`
- THEN it prints nothing and exits 0

#### Serves
- architecture/signed-ledger-records
- architecture/humans-review-intent

### Requirement: legacy-doc-only
`clm-ledger` SHALL offer `legacy_doc_only(path)`, true for a path relative to the repository root under `specs/`, `changes/`, `backlog/` or `docs/` outside `docs/guide/`, or equal to `ARCHITECTURE.md`, `README.md` or `CLAUDE.md`, and false for any other path, the doc-only list as it was when the `landed` records without a `code_key` were written, so every reader keys such a land with that list (`legacy-code-key`). It SHALL be written as a literal of its own and SHALL NOT call `doc_only`, so a later change to the live list leaves it as it is.

#### Scenario: paths
- GIVEN the paths `docs/spec-format.md`, `docs/x.md`, `docs/guide/intro.md`, `CLAUDE.md` and `crates/a/src/lib.rs`
- WHEN `legacy_doc_only` runs on each
- THEN it returns true for `docs/spec-format.md`, `docs/x.md` and `CLAUDE.md`, and false for the others

#### Scenario: frozen
- GIVEN `crates/clm-ledger/src/doc_only.rs`
- WHEN a test reads it
- THEN `legacy_doc_only` names `docs/spec-format.md` nowhere and lists its paths in a constant of its own, and its body calls no `doc_only`

#### Serves
- capability/land-safely

### Requirement: proposal-records
The fold, `fold` and `fold_records` alike, SHALL parse `proposed` and `withdrawn` records and hold as its open proposals, by `id`, every `proposed` record whose `id` no `withdrawn` record's `proposal` names, whatever order the records are read in, each with its `writer`, `at`, `slug`, `title`, `body` and components (`/nodes`). `clm ledger show` SHALL print a `proposed` record as `proposed <id> <slug> <writer> <at>` and a `withdrawn` one as `withdrawn <proposal> <writer> <at>`. `clm ledger compact` SHALL keep both kinds when they are in the caller's own ledger.

#### Scenario: withdrawn in another order
- GIVEN `clm/ledger/alice/web` holding `withdrawn` naming `p1`, then `proposed` `p1` with slug `dark-mode`, then `proposed` `p2` with slug `export-csv`
- WHEN `fold_records` runs over them
- THEN its open proposals are exactly `p2`, with slug `export-csv` and writer `alice/web`

#### Scenario: shown
- GIVEN a ledger holding `proposed` `p2` with slug `export-csv` by `alice/web`
- WHEN `clm ledger show` runs
- THEN it prints a line starting `proposed p2 export-csv alice/web`

#### Scenario: compaction keeps them
- GIVEN the caller's own ledger holding `proposed` `p1`, `withdrawn` naming `p1`, and an `approved` record whose fingerprint a `landed` record names
- WHEN `clm ledger compact` runs
- THEN it prints `removed 1`, and both proposal records are still there

#### Serves
- capability/follow-from-browser
- architecture/git-is-truth

### Requirement: feedback-records
The fold, `fold` and `fold_records` alike, SHALL parse `commented` and `changes_requested` records and hold the comments and the change requests by `branch`, in `at` order, each with its `writer`, `at`, `branch`, `fingerprint` and `text`, and a comment's `target`. `clm ledger show` SHALL print a `commented` record as `commented <branch> <fingerprint> <target> <writer> <at> <text>`, `<target>` being `-` when null, and a `changes_requested` one as `changes_requested <branch> <fingerprint> <writer> <at> <text>`, the text trimmed and each run of whitespace holding a line break replaced by one space. `clm ledger compact` SHALL remove from the caller's own ledger the records of both kinds whose fingerprint any `landed` record names, whatever their branch, and keep the rest.

#### Scenario: folded
- GIVEN `clm/ledger/alice/web` holding `commented` on `spec/x` at fingerprint `f` with target `n/r` and `changes_requested` on `spec/x` at `f`, and `clm/ledger/bob/web` holding `changes_requested` on `spec/y` at `g`
- WHEN `fold_records` runs over them
- THEN the comments for `spec/x` are the one with target `n/r`, the change requests for `spec/x` are `alice/web`'s at `f` and those for `spec/y` are `bob/web`'s at `g`

#### Scenario: shown
- GIVEN a ledger holding `changes_requested` on `spec/x` at `f` by `alice/web` with the text "Split the" followed by a line break and "requirement"
- WHEN `clm ledger show` runs
- THEN it prints a line starting `changes_requested spec/x f alice/web` that ends `Split the requirement`

#### Scenario: landed feedback pruned
- GIVEN the caller's ledger with `commented` and `changes_requested` records at fingerprint `f` and a `commented` record at `g`, and another writer's `landed` record with fingerprint `f`
- WHEN `clm ledger compact` runs
- THEN it prints `removed 2`, and the comment at `g` is still there

#### Serves
- capability/follow-from-browser
- capability/approve-intent
- architecture/git-is-truth

### Requirement: effort-records
The fold, `fold` and `fold_records` alike, SHALL parse `effort` records and hold them by `id`, keeping for each `id` the record with the latest `at`, whatever its writer and whatever order the records are read in, in its records and its lines alike. `clm ledger show` SHALL print an `effort` record as `effort <branch> <session> <work tokens> <active_seconds>s <writer> <at>`, work tokens being `input` + `output` + `cache_write` summed over its models. `clm ledger compact` SHALL drop the caller's own `effort` records that a record with the same `id` and a later `at` supersedes.

#### Scenario: round trip
- GIVEN an `effort` line for `impl/a`, session `s1`, with two models
- WHEN the fold reads it
- THEN it holds that record with both models' counts

#### Scenario: latest wins
- GIVEN two ledgers, each with an `effort` line of one `id`, at t1 and t2 > t1
- WHEN the fold reads them, in either order
- THEN it holds the one at t2, once

#### Scenario: shown
- GIVEN an `effort` record for `impl/a`, session `s1`, with input 10, output 5, cache reads 100 and cache writes 20 over two models, and 120 active seconds
- WHEN `clm ledger show` runs
- THEN it prints `effort impl/a s1 35 120s <writer> <at>`

#### Scenario: superseded effort compacted
- GIVEN the caller's ledger with two `effort` records of one `id`, and a `decided` record
- WHEN `clm ledger compact` runs
- THEN the earlier `effort` record is gone, and the later one and the `decided` record are still there

#### Serves
- capability/review-capability-cost
- architecture/effort-accounted

### Requirement: decision-records
The fold, `fold` and `fold_records` alike, SHALL parse `decided` records and hold them by `capability`, the entry's ID, whether it is `capability/<slug>` or `architecture/<slug>`, in `at` order, skipping with a warning a line whose `value` or `decision` is none of its words. `clm ledger show` SHALL print a `decided` record as `decided <capability> <value> <decision> <writer> <at>`. `clm ledger compact` SHALL keep it.

#### Scenario: by capability
- GIVEN two `decided` records for `capability/x` and one for `capability/y`
- WHEN the fold reads them
- THEN it holds two for `capability/x`, in `at` order, and one for `capability/y`

#### Scenario: an architecture entry
- GIVEN a `decided` record whose `capability` is `architecture/x`, value `high`, decision `keep`
- WHEN the fold reads it and `clm ledger show` runs
- THEN the fold holds it under `architecture/x`, and `clm ledger show` prints `decided architecture/x high keep <writer> <at>`

#### Scenario: bad rating
- GIVEN a `decided` line whose `value` is `huge`
- WHEN the fold reads it
- THEN it skips the line with a warning naming it

#### Scenario: shown
- GIVEN a `decided` record for `capability/x`, value `low`, decision `cut`
- WHEN `clm ledger show` runs
- THEN it prints `decided capability/x low cut <writer> <at>`

#### Serves
- capability/review-capability-cost
- architecture/effort-accounted

### Requirement: append-all
`append_all(root, entries)` SHALL add the entries, each as `append` adds one, in one commit on the writer's branch and one push, and SHALL make no commit for no entries. An `effort` body SHALL get `effort_id(session, branch)` as its `id`, the first 32 hex digits of SHA-256 over `<session>\n<branch>`, where `append` would draw a random one. `append` SHALL be `append_all` with one entry.

#### Scenario: one commit
- GIVEN a clone with a bare `origin`
- WHEN `append_all` runs with three bodies
- THEN the writer's branch has one new commit holding three more lines, and each is reported pushed

#### Scenario: nothing to add
- GIVEN that clone
- WHEN `append_all` runs with no entries
- THEN the writer's branch is unchanged

#### Scenario: stable effort id
- GIVEN an `effort` body for session `s1` on `impl/a`
- WHEN it is appended twice
- THEN both records' `id` is `effort_id("s1", "impl/a")`

#### Serves
- capability/review-capability-cost
- architecture/effort-accounted

### Requirement: verify-by-hand-merge
`clm ledger verify` SHALL neither report as `ledger/unrecorded-land` nor judge as a land a commit on `main`'s first-parent chain whose message has the trailer `Clm-By-Hand: <name>`, read with `git log`'s `%(trailers:key=Clm-By-Hand,valueonly)` through `clm-git`, when `<commit>:docs/decisions/<name>.md` exists with `status: accepted` in its frontmatter and the commit's first parent's tree has no `docs/allowed_signers`. A commit whose trailer names no such decision, or whose first parent is signed, SHALL be reported as before.

#### Scenario: accepted decision
- GIVEN a landed `main`, then a merge commit no `landed` record names, whose message ends with `Clm-By-Hand: by-hand` and whose tree holds `docs/decisions/by-hand.md` with `status: accepted`
- WHEN `clm ledger verify` runs
- THEN it prints nothing and exits 0

#### Scenario: no such decision
- GIVEN the same, but the commit's tree has no `docs/decisions/by-hand.md`
- WHEN `clm ledger verify` runs
- THEN it prints `ledger/unrecorded-land` naming the commit and exits 1

#### Scenario: signed repository
- GIVEN *accepted decision*, but with `docs/allowed_signers` in the merge commit's first parent
- WHEN `clm ledger verify` runs
- THEN it prints `ledger/unrecorded-land` naming the commit

#### Serves
- capability/land-safely
- architecture/no-silent-drift

### Requirement: request-cleared
`clm-ledger` SHALL offer, in its wasm build as in its default one, `person(writer)`, the part of a `<person>/<clone>` writer before its first `/`, and `Fold::cleared(request)`, true exactly when the fold holds an `approved` record for the `changes_requested` request's `fingerprint` whose writer's person is the request's writer's person and whose `at` is greater than the request's as a string, whether or not it is signed, and false for a record of any other kind. It SHALL keep no other function that takes a writer's person apart.

#### Scenario: the requester approved later
- GIVEN a fold with a `changes_requested` record by `alice/web` at fingerprint `f` and `at` `2026-10-30T07:10:00.000Z`, and an unsigned `approved` record for `f` by `alice/1a2b3c4d` at `2026-10-30T07:11:00.000Z`
- WHEN `cleared` runs on the request
- THEN it is true

#### Scenario: not cleared
- GIVEN the same request, and `approved` records for `f` by `dana/9f9f9f9f` later, by `alice/1a2b3c4d` at the same `at` as the request, and for `g` by `alice/1a2b3c4d` later
- WHEN `cleared` runs on the request
- THEN it is false

#### Scenario: person
- GIVEN the writers `alice/1a2b3c4d` and `alice`
- WHEN `person` runs on each
- THEN both give `alice`

### Requirement: person-slug
`clm-ledger` SHALL offer `slug(name)` publicly: the name lowercased, each run of anything but letters and digits replaced by `-`, with none at either end, the slug `writer` makes of git's `user.name` and `verify` makes of a commit's author name.

#### Scenario: names slugged
- GIVEN the names `Christopher Lunny` and `  Dana_O'Neil `
- WHEN `slug` is called on each
- THEN it returns `christopher-lunny` and `dana-o-neil`

#### Serves
- architecture/reuse-first

### Requirement: verified-record
The fold, `fold` and `fold_records` alike, SHALL parse `verified` records and hold them by `commit`, the newest by `at` first, skipping with a warning a line whose `passed` isn't a boolean, whose `mode` isn't `local`, `pipeline` or `import`, or whose `commit`, `since`, `lands` or `culprit` holds anything but full 40-hex lowercase commit IDs, as git writes them (or null where null is allowed): the fold looks records up by exact string, so it would hold an uppercase ID but never match it. `append` SHALL write it like any record, unsigned in every repository: a `verified` record holds no land by itself (`land/verify-stop` reads the backlog item triage takes it in as), so it is a report, as the kinds other than `approved` and `landed` are. `clm ledger show` SHALL print a `verified` record as `verified <commit> passed|failed <writer> <at>`. `clm ledger compact` SHALL keep the caller's newest `verified` record for each commit and drop older ones for the same commit. `Fold::unresolved_reds(chain)`, given a first-parent chain of commits oldest first, SHALL return the newest record of each commit after the last one whose newest record passed (of every commit when none passed) whose newest record failed, in chain order, for `verify`, `land` and `orchestrator.triage` to read alike.

#### Scenario: by commit
- GIVEN two `verified` records for commit `c`, the second red with culprit `l`, and one for commit `d`
- WHEN the fold reads them
- THEN it holds both for `c`, the red one first, and one for `d`

#### Scenario: bad result
- GIVEN a `verified` line whose `passed` is `"yes"`
- WHEN the fold reads it
- THEN it skips the line with a warning naming it

#### Scenario: uppercase commit
- GIVEN a `verified` line whose `commit` is 40 uppercase hex digits
- WHEN the fold reads it
- THEN it skips the line with a warning naming it

#### Scenario: shown
- GIVEN a passing `verified` record for commit `c`
- WHEN `clm ledger show` runs
- THEN it prints `verified <c> passed <writer> <at>`

#### Scenario: unresolved reds
- GIVEN a chain `a`, `b`, `c`, `d`, a passing record for `a`, a red one for `b`, a red then a passing one for `c`, and a red one for `d`
- WHEN `unresolved_reds` reads the chain
- THEN it returns `d`'s record alone, since `c`'s newest record passed

#### Scenario: compacted
- GIVEN the caller's two `verified` records for commit `c`
- WHEN `clm ledger compact` runs
- THEN only the newer remains

#### Serves
- capability/slow-tests-after-land

### Requirement: code-key-with
`code_key_with(root, rev, path, from)` SHALL return the key `code_key(root, rev)` would return were `path`'s entry in `rev`'s tree the one `from`'s tree holds, or absent where `from`'s tree holds none, computed from the two trees' listings without writing a git object: `from`'s line for `path` replaces `rev`'s verbatim (mode and object included), or is inserted where `ls-tree -r` would list it, in byte order of paths, when `rev`'s tree holds none, and `path` names one file, matched exactly. For a path `doc_only` names it SHALL return `code_key(root, rev)`.

#### Scenario: one file taken from the base
- GIVEN commit `a`, and commit `b` on it that changes `clm.toml` and `docs/x.md`
- WHEN `code_key_with(root, b, "clm.toml", a)` runs
- THEN it returns `code_key(root, a)`, and `code_key(root, b)` differs from it

#### Scenario: absent in from
- GIVEN commit `a`, and commit `b` on it that adds `tool.toml`
- WHEN `code_key_with(root, b, "tool.toml", a)` runs
- THEN it returns `code_key(root, a)`

#### Scenario: absent in rev
- GIVEN commit `a` holding `tool.toml`, and commit `b` on it that removes it
- WHEN `code_key_with(root, b, "tool.toml", a)` runs
- THEN it returns `code_key(root, a)`

#### Scenario: another file differs too
- GIVEN commit `a`, and commit `b` on it that changes `clm.toml` and `crates/a/src/lib.rs`
- WHEN `code_key_with(root, b, "clm.toml", a)` runs
- THEN it differs from `code_key(root, a)`

### Requirement: landed-times
`append` SHALL write a `landed` record's `check_seconds` and `steps` when the record has them and leave both fields out when it has none, and `fold` SHALL keep them on each `landed` record it reads, reading a record without them as untimed and a step without `selected_by` as selected by no path. `Fold::timed_lands(n)` SHALL return, oldest first, the last `n` `landed` records by `at` that carry `check_seconds`, each as a `TimedLand` with its commit, `at`, `check_seconds` and `steps`.

#### Scenario: times round-trip
- GIVEN an empty ledger
- WHEN `append` runs with a `landed` record with `check_seconds` 30 and one step `web` of 25 seconds selected by `web/src/main.ts`, and then the ledger is folded
- THEN the fold's `landed` record has `check_seconds` 30 and that step, and its line in `records.jsonl` holds `"check_seconds":30`

#### Scenario: untimed records
- GIVEN a ledger whose `landed` record was written without `check_seconds` or `steps`
- WHEN it is folded
- THEN the record reads as untimed, and its line holds neither field

#### Scenario: the last timed lands
- GIVEN `landed` records `a`, `b` and `c` in that order of `at`, `a` and `c` with `check_seconds` and `b` without
- WHEN `Fold::timed_lands(2)` runs
- THEN it returns `a` and then `c`, each with its `check_seconds` and `steps`

### Requirement: composed-check
`append` and `fold` SHALL carry a `checked` record's optional `basis`, a map with `head` and `base`, each a code key, leaving it out of a record that has none, and SHALL hold a composed record among a code key's checks like any other, so every reader that asks whether a key is checked reads it unchanged: `clm land` writes one only after finding both basis keys checked (`land/overlap-recheck`). `clm ledger verify` SHALL report `ledger/unchecked-land` for a land whose only `checked` records for its code key are composed ones whose `head` or `base` key no `checked` record with the same command names, the audit of that rule. `clm ledger compact` SHALL keep every `checked` record whose code key the basis of a `checked` record it keeps names, as it keeps those a land's code key names.

#### Scenario: a composed check counts
- GIVEN `checked` records for keys `h` and `b` with command `x`, a composed `checked` record for `m` with command `x` and basis `h` and `b`, and a `landed` record with code key `m`
- WHEN `clm ledger verify` runs
- THEN it prints nothing about that land

#### Scenario: a composed check whose basis is missing
- GIVEN the same with no `checked` record for `b`
- WHEN `clm ledger verify` runs
- THEN it prints `ledger/unchecked-land` naming the landed commit

#### Scenario: compaction keeps the basis
- GIVEN the caller's ledger with `checked h` and `checked b`, older than a `landed` record with code key `m`, and a composed `checked m` naming them
- WHEN `clm ledger compact` runs
- THEN `checked h` and `checked b` are still there

### Requirement: landed-recheck
`append` and `fold` SHALL carry a `landed` record's optional `recheck`, one of `checked`, `none`, `overlap` and `selected`, and `recheck_files`, a count or null, leaving both out of a record that has neither, and `Fold::timed_lands(n)` SHALL give each land's `recheck` and `recheck_files` beside its times, so the share of lands that ran no check and the size of each overlap can be read back.

#### Scenario: a land that ran nothing
- GIVEN a `landed` record appended with `recheck: none` and no `recheck_files`
- WHEN the ledger is folded
- THEN the fold's land holds `recheck` `none` and `recheck_files` absent

#### Scenario: an older record
- GIVEN a `landed` record with neither field
- WHEN the ledger is folded
- THEN the land holds neither, and nothing fails

### Requirement: answer-records
The fold, `fold` and `fold_records` alike, SHALL parse `answered` records and hold as its open answers, for each item and writer, the one record of that writer naming that item with the latest `at`, ties broken by the greater `id`, whatever order the records are read in, each with its `id`, `writer`, `at`, `item` and `answer`. Whether its item is still in the backlog is the reader's to judge from the tree. `clm ledger show` SHALL print an `answered` record as `answered <id> <item> <writer> <at>`. `clm ledger compact` SHALL keep `answered` records when they are in the caller's own ledger.

#### Scenario: a later answer replaces an earlier one
- GIVEN `clm/ledger/alice/web` holding `answered` `a2` naming `q` with answer `no` at 10:00, then `answered` `a1` naming `q` with answer `yes` at 09:00, and `clm/ledger/bob/web` holding `answered` `b1` naming `q` with answer `yes` at 08:00
- WHEN `fold_records` runs over them
- THEN its open answers are `a2` (`alice/web`, `no`) and `b1` (`bob/web`, `yes`), and not `a1`

#### Scenario: shown
- GIVEN a ledger holding `answered` `a2` naming `q` by `alice/web`
- WHEN `clm ledger show` runs
- THEN it prints a line starting `answered a2 q alice/web`

#### Serves
- capability/follow-from-browser
- architecture/per-writer-ledgers
