---
component: ledger
base: { contract: "c90a35cc11716869", design: "8cd04d5545e6972a" }
---

## MODIFIED Contract
The records that say what was approved, checked and landed, as facts in git that any clone can replay (`per-writer-ledgers`). Each writer appends to its own branch, `clm/ledger/<person>/<clone>`, so appends never race, and the state is the fold over the union of every ledger branch on `origin` and this clone's own. Crate `clm-ledger` offers, as library calls, `env` being the environment as a lookup, which `config` and git's config are read with:
- `writer(root, env)`: this clone's identity, creating it on first use.
- `append(root, entry, env)`: commit the entry to the writer's branch and push it, returning `Appended { record, pushed }`. An `entry` (`Entry`) is a record's `Body`, given a fresh `id`, the writer and the time, or a `Signed` record `sign` returned. `append_all(root, entries, env)`: several entries in one commit and one push, returning one `Appended` per entry.
- `fold(root)`: the state, a `Fold` holding `records` (every record once, by `at` and then `id`), `approvals` by fingerprint, `checks` by code key, `lands` by commit, `subscriptions` (the live push subscriptions by `id`, each with its `writer`, `at`, `endpoint`, `p256dh` and `auth`), `proposals` (the open web proposals by `id`), `answers` (the open answers by item), `comments` and `change_requests` by branch (`web-proposals-as-ledger-records`, `web-review-feedback-as-ledger-records`), `efforts` (the effort records by `id`), `decisions` (the capability decisions by capability), `verifications` and `mutations` (the slow-test results and the mutation passes by commit, newest first), `lines` (each record's JSON object as read, by `id`) and `warnings` (each line or ledger it skipped). `fold_records(sources)`: the same fold over `(ledger, records.jsonl text)` pairs already read, which the build with default features off offers too.
- On a `Fold`: `cleared(request)` (whether a `changes_requested` record is cleared at its fingerprint, the one rule `land.approval` and `web.worker` hold by: `request-cleared`), `timed_lands(n)` (the last `n` timed lands as `TimedLand { commit, at, check_seconds, steps, recheck, recheck_files }`, each step a `TimedStep { name, seconds, selected_by }`: `landed-times`, `landed-recheck`) and `unresolved_reds(chain)` (the red `verified` results nothing has since cleared: `verified-record`).
- `effort_id(session, branch)` (an `effort` record's `id`), `code_key(root, rev)` (a commit's tree with the doc-only paths left out, hashed), `code_key_with(root, rev, path, from)` (`rev`'s code key with `path`'s entry as it is in `from`, which `land` reads to count a `clm.toml` whose settings are unchanged as checked), `doc_only(path)` (whether `code_key` leaves a path out, which `check.affected` reads so its selection and the code key agree), `legacy_doc_only(path)` (the doc-only list as it was when the `landed` records without a `code_key` were written: `legacy-doc-only`), `person(writer)` (the person part of a `<person>/<clone>` writer) and `slug(name)` (a name slugged as `writer` slugs git's `user.name`, which `land.approval` and `verify` slug commit authors with: `person-slug`).
- `verify(root, tip, since, review)`: the audit `clm ledger verify` makes, of `tip`'s first-parent chain, returning its findings, each a `Finding { rule, commit, message }`. `on_main(root)`: those findings for `main` as violations, for `clm check` to report. `compact(root, env)`: what `clm ledger compact` does, returning `Compacted { removed, warnings }`. `show(fold)`: the lines `clm ledger show` prints.
- For signed repositories (`signed-ledger-records`): `signed(root, rev)` (whether `rev`'s tree holds `docs/allowed_signers`), `sign(root, body, namespace, env)` (the body as a record with its `id`, `writer` and `at` filled in, signed with the person's key, as `Signed { record, sig }`, whose `line()` is the JSON object `append` writes), `signed_by(root, rev, line, namespace)` (whether a record's line as read, a JSON object such as `Fold::lines` holds, has a `sig` that is a valid signature by a key `rev`'s `docs/allowed_signers` lists for the line's writer's person) and `signed_message(line)` (the bytes a signature is over).

A call that fails returns `Error`: `Git` (`clm-git`'s error), `Config` (`config`'s), `Io`, `Writer` (the writer identity can't be made, or a signed record is another writer's), `Signing` (a record can't be signed or a signature checked: `record-signing`, `signature-check`; `land.approval` reads its text) or `Settings` (a committed `clm.toml` the audit reads doesn't parse). The record types are `Record { id, writer, at, body }`, `body` a `Body` with one variant per kind below, holding the kind's fields under their names in the line, but for `Body::Landed`'s `kind`, written `land_kind`; and `LandKind`, `Basis { head, base }`, `Recheck`, `TimedStep`, `Usage { input, output, cache_read, cache_write }`, `Rating`, `Decision` and `VerifyMode`, the values their fields hold. `Body::kind()` gives a body's kind as `kind` holds it; `Usage::work()` gives `input` + `output` + `cache_write`, saturating rather than overflowing; and `Rating::as_str()` and `Decision::as_str()` give the value as the line writes it. `Body::mutated(...)` and `Body::with_left(left)` build a `mutated` body without naming the variant's fields (`mutated-constructor`, `mutated-record`). `clm ledger compact` prunes the caller's own superseded records, `clm ledger verify` audits `main` against the records, approvals and signatures included, and `clm ledger show` prints the fold. Each prints a warning on standard error as `warning: <text>`, and a failure as `clm ledger: <why>` with exit 2; `clm ledger verify` exits 1 when it finds anything, and otherwise each exits 0 (`commands-in-crate`). Changing a record kind or a field name is a change to this Contract: `land`, `gate`, `view`, `web.worker` and the skills all read them.

Record kinds:
- `approved`: `fingerprint`, `branch`, `change` (a change ID, or null). A human's approval of a diff (`approve-pins-the-review`).
- `checked`: `code_key`, `command`, and optionally `basis` (`head` and `base`: the code keys of a branch's head and of `main` whose checks it was composed from: `composed-check`). A tree whose code key passed the check command and `clm check`, written by `clm land` for its merged tree or by `clm land --check-only` for a clean, committed `HEAD` that contains `origin/main` (`land/check-only`, `worktree-check-counts`), or whose code key `code_key_with` maps, its `clm.toml` settings unchanged, to one that did, the land having run that tree's `clm check` and the tests that read `clm.toml` (`land/settings-unchanged-is-checked`). A record with `basis` is a *composed* check: `clm land` writes one for a merged tree it checked only where the branch and `main` overlap (`land/overlap-recheck`), and `clm ledger verify` audits that `checked` records name both of its keys.
- `landed`: `commit`, `branch`, `land_kind` (`spec` or `code`), `review` (whether the land needed a human's review, `gate/needs-review`; false for code), `fingerprint`, `code_key` (the code key the land's `checked` record names: the landed commit's, or, for a land checked together with others (`land/batched-lands`), that of the commit their check ran on), `recheck` (how the land checked its merged tree: `checked`, its code key already checked; `none`, the overlap selected nothing; `overlap`, the overlap ran; `selected`, the branch's own selection ran; absent on older records: `landed-recheck`), `recheck_files` (with `overlap` or `selected`, the number of `file` lines the check ran, or null when it ran every test), `changes` (the live change IDs a spec land added or modified), and, when the land ran `[land] check`, `check_seconds` (the command's wall-clock time in whole seconds, rounded down) and `steps` (each step it marked, in order, as `{name, seconds, selected_by}`, `selected_by` the paths its `::clm-selected::` lines named: `land/step-times`), which `Fold::timed_lands(n)` reads back (`landed-times`). A `checked` record stays a fact about a code key and carries no times (`graph-test-selection`, `land-times-on-the-landed-record`). Older records lack `code_key` (`landed-record-names-its-code-key`). Records from before review was yes or no (`yes-no-review`) have `tier` instead of `review`: `HARD` reads as `review: true`, anything else as `false`.
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
- `mutated`: `commit` (a commit of `main`), `since` (the commit the pass started from, or null), `killed` and `not_judged` (requirement IDs), `unproven` (entries `<component>/<slug>@<version>`), `left` (requirement IDs the pass didn't reach; absent in older records, read as empty). What a mutation pass found on that commit (`mutation-after-land`).

Every record also has `id` (random 128-bit hex, except an `effort` record's, which is `effort_id(session, branch)`), `kind` (one of the fourteen above), `writer` (`<person>/<clone>`) and `at` (the time it was written, in UTC, always as `YYYY-MM-DDTHH:MM:SS.mmmZ`, as `clm` and `web.worker` write it, so two records' times order as strings: the fold sorts by `at` so, and `cleared` compares a request's and an approval's so for `land` and `web.worker` alike). A signed record also has `sig`: an `approved` record signed under the namespace `clm-approve`, or a `landed` record under `clm-land`; the other kinds never have it, and a record without it is unsigned. A record is one flat JSON object, so no record field shares a name with these five: the land's kind is `land_kind`, not `kind`.

`clm-ledger` offers `clm ledger` as a call, `ledger_command(args, root, env, out, err)` (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. Its arguments are `LedgerArgs`, which `crates/clm`'s parser reads, holding the subcommand as a `Ledger` (`Compact`, `Verify` or `Show`), and whose `root()` is the `--root` the subcommand was given. `crates/clm` routes the command to it and adds nothing to what it writes.

## MODIFIED Design
**Writers.** The person part falls back to `git config user.name` slugged (`person-slug`) when no layer sets `writer` (`writer-identity`). The clone part is written to `.git/clm/writer` in the common git directory, so a clone's worktrees share it. Many sessions in one clone commit to one local branch: each moves the branch with `git update-ref` only from the tip it read, and reads the new tip and appends again when another session moved it first or holds its lock. Two clones of one person never share a branch, so nothing ever rebases a ledger.

**Branches.** `clm/ledger/<person>/<clone>` is an orphan branch holding one file, `records.jsonl`, one JSON object per line, appended in order. The first append creates the branch with `git commit-tree` on a tree holding that one file, with no parent; later appends build the new tree from the previous one. The working tree and the index are never touched (`GIT_INDEX_FILE` or a tree built by hand). After each commit the branch is pushed without force, retrying a failed push a few times with a short wait, since a refusal can only be a network or lock failure or a hook: nobody else writes the branch. A record committed but not pushed waits for a later push, which `clm reconnect` makes (`land`). On every host, `none` included, `append` pushes and `compact` fetches first, `none`'s `origin` being the clone's own remote (`host/none-keeps-a-remote`).

**Fold.** `fold` reads `records.jsonl` at the tip of each `refs/remotes/origin/clm/ledger/*` ref and of every local ledger branch whose clone part is this clone's, with `git cat-file`: the writer's own, which may be ahead, and any the clone wrote under an earlier person part, whose records aren't all pushed yet. Records are commutative facts, so the fold is the same in any order, and a duplicate `id` (the same record read from two refs) counts once. It doesn't fetch: the caller fetches first when it needs the latest (`land` does; `clm status` doesn't, as for claims). Lines are parsed one at a time, so one bad line never hides the rest (`fold`). The fold orders records by `at` as written and then by `id`: it trusts each writer's clock, as nothing else orders records across clones. The open proposals count any `withdrawn` record, whoever wrote it (who may write one is the Worker's rule). The comments and the change requests are maps by `branch` to the records in `at` order, so `land` and the Worker take a branch's requests and pick those naming a fingerprint. For `effort` records a repeated `id` is not one record read twice but a later count of the same session and branch, so the fold keeps the latest `at`, whatever its writer, in its records and its lines alike. Of `effort` records with one `at`, the fold keeps the greater writer and then the greater text of the line as `signed_message` writes it, keys sorted at every depth, so every clone, and the Worker's wasm build, folds to the same one whatever order it reads the refs in and however `serde_json` orders a map's keys (`effort-records`). `serde_json::to_string` of a map follows the order the keys were read in where `serde_json`'s `preserve_order` feature is on, which `clm-init` turns on in the workspace's native build and nothing turns on in the wasm build or in `clm-ledger` built alone; so the test of this tie compares two records whose lines are written with their keys in an order other than sorted (`models` before `active_seconds`, and two records that differ first in the later key), and asserts the same winner as for the same two records written with sorted keys. Without the sorted writing that test fails wherever `preserve_order` is on, as in the workspace run `clm land`'s checks make; with the feature off, `serde_json` sorts every map already and the unmended code gives the right winner, so no test can fail there, and a mutation spot-check of the tie runs in the workspace build. Two lines that share an `id` but differ, unless both are `effort` records, are not one record read twice: the fold keeps the earliest by `at`, then writer, then sorted-key text, so a later line reusing an `id` never displaces the record that first had it, and every read order folds alike. Whether a proposal has been filed or resolved is read from the tree, never from the ledger (`web-proposals-as-ledger-records`), and whether a change request is cleared is `cleared`'s rule over the fold's approvals (below), which `land` holds a land by and `web.worker` shows. Nothing compacts a web ledger, as nothing compacts its approvals, so the comments on a branch stay and a later branch of the same name shows them as made on an earlier version.

**Code keys.** `code_key(root, rev)` is a SHA-256 over the entries `git ls-tree -r -z <rev>` lists, each followed by its NUL, without those under a doc-only path (`doc_only` lists them once: `land-locally`). Every path under `docs/` is doc-only, the guide's pages under `docs/guide/` and `docs/spec-format.md` included, though `clm guide` compiles them into the binary (`guide-pages-are-doc-only`): the tests that read them, from the tree or through `clm guide` or the site built from them, hold `clm:reads` markers (`doc-readers-marked`), so a land that edits a page runs those tests rather than every test, as a land that edits `specs/` does. Two trees that differ only in doc-only paths have one key, so a spec land on code that was already checked needs only `clm check` and the tests that read what it changed.

**A land's code key.** A `landed` record's *code key* is its `code_key` field, or, for a record without it, its *legacy key*, computed on demand (`landed-record-names-its-code-key`, `legacy-code-key`): the key over `git ls-tree -r -z <commit>` without the entries under the legacy doc-only paths. That list is frozen in the crate as `legacy_doc_only(path)`, a literal of its own beside `doc_only`'s that calls nothing live (`legacy-doc-only`); it never follows `doc_only`: those lands' `checked` records hold keys computed with it, and no change to the list can re-check a tree that has landed. `doc_only` and `code_key` move with the repository, for every land that records its key and for `check.affected`, which reads `doc_only` so a live land's selection and its key agree. `land` writes the field from the merged tree's key, which it has already computed to look up or append the `checked` record, so it costs nothing at land time; `verify` and `compact` then read it instead of listing a tree per land. `append` leaves the field out of a record that has no key, rather than writing `null`, so old and new lines read alike; the fold holds such a land with its key unknown (`None`). Compaction keeps every check when it can't compute such a key (`compact`), since dropping a check is the one mistake it can't undo.

**`clm ledger compact`.** Prunes what a land has made redundant, the caller's own records only (`compact`). A `checked` record that a land relied on stays, because `verify` reads it; when several name one key (a retried land, or two writers checking one tree), all of them stay, since compaction never decides between writers' facts. `landed` records stay: one line per land is `main`'s audit trail. `reviewed` lines go, since nothing reads them. The `commented` and `changes_requested` records whose fingerprint a `landed` record names go whatever their `branch`, as approvals do: the diff they were about is on `main`. `proposed`, `withdrawn` and subscription records stay: a proposal is open until a change on `main` resolves the item it became, which readers take from the tree. An `effort` record goes when the fold holds another record with its `id`, which is a later one (the order above), whoever wrote it; `decided` records stay, since they are the review's history. A signed land that needed review keeps its `approved` record, since `verify` needs the one that cleared it; where nothing is signed, nothing reads approvals after the land and they go.

**`clm ledger verify [--since <rev>]`.** Walks the first-parent chain of `main` after `--since` (default: from the oldest commit a `landed` record names, that commit included, since it is a land to audit and what came before it predates the ledger). `clm land` always makes a merge commit, so a single-parent commit on that chain came from around it, such as `git push origin HEAD:main`. A commit whose message starts `Release v` is the release workflow's push (`host-ci-for-releases`): a merge commit when `main` moved during the release, or the version bump itself when its merge fast-forwards. A commit made by hand under a decision isn't reported either (`verify-by-hand-merge`): the trailer `Clm-By-Hand: <name>` is read with `git log`'s `%(trailers:key=Clm-By-Hand,valueonly,separator=%x1e)` through `clm-git`, and `<name>` is a kebab-case decision name whose `status: accepted` is read in its frontmatter (between a first line `---` and the next). In a signed repository the hatch is shut: anyone who can push could name a decision, and the commit would skip the signature audit. It is the one escape hatch from `clm land` (`refounding-edits-specs-directly`), and `git log --grep Clm-By-Hand` lists every use. `clm check` runs `verify` when the checked-out branch is `main` and the repository has a ledger branch, so a push to `main` that didn't go through `clm land` is a blocking error there (`no-silent-drift`). `clm wrap-up` runs it on `origin/main` too (`orchestrator.checkpoint`). Hosts with hooks can run it in `pre-receive`; this is the after-the-fact check for the rest.

**`clm ledger show`.** Prints the fold, one line per record in the fold's order, for a human or a test to read (`show`).

**Cleared requests** (`request-cleared`). Any approval counts, whatever the approval policy and whether or not it is signed, since a request withholds a land rather than granting one. Whether a request names the fingerprint a land or a review is at stays the caller's test. A record whose `at` isn't in the pinned form sorts by its string, for both callers alike: the helper doesn't refuse it.

**Signed records** (`signed-ledger-records`). A repository is *signed* at a commit when that commit's tree holds `docs/allowed_signers`, an OpenSSH allowed-signers file (`ssh-keygen(1)`, ALLOWED SIGNERS) whose principals are writer persons; `signed(root, rev)` looks for it with `git ls-tree` through `clm-git`. A record's *signed message* is the record as a JSON object without `sig`, its keys sorted bytewise, with no whitespace, as `serde_json::to_string` prints a `serde_json::Value` built without the `preserve_order` feature, so the bytes don't depend on the field order of a struct. `sign` runs `ssh-keygen -Y sign -f <key> -n <namespace>` with the signed message on stdin and puts its armored output, the `-----BEGIN SSH SIGNATURE-----` block, in `sig`. The key is git's `user.signingkey` when the repository's `gpg.format` is `ssh` (`git config --get` through `clm-git`): a path is passed as is, and a value starting `key::` is a literal public key, written to a temporary file so that `ssh-keygen` signs through `ssh-agent` (`record-signing`). `append` keeps an `id`, `writer`, `at` and `sig` that the record already has, so a caller signs a record before an act it must not do unsigned and appends it after. `signed_by` writes `rev`'s `docs/allowed_signers` and the `sig` to temporary files and runs `ssh-keygen -Y verify -f <signers> -I <person> -n <namespace> -s <sig>` with the signed message on stdin, `<person>` being the person part of the record's `writer`; it is true exactly when `ssh-keygen` accepts the signature (`signature-check`). `clm-ledger` isn't one of the crates that start programs themselves, so it runs `ssh-keygen` through `clm-git`'s `run`, as it runs `git` (`vcs/one-git-layer`). The fold keeps `sig` as it reads it and doesn't verify it: a signature is checked where an approval is counted (`land`) and by `verify`, and `fold_records` in wasm carries `sig` as data.

**Auditing signed lands.** For each landed commit on the chain whose first parent is signed, `verify` reports `ledger/unsigned-land` when `signed_by(root, <first parent>, <record's line>, "clm-land")` is false for every `landed` record naming it. For such a land whose signed record says it needed review, it recomputes the fingerprint as `clm land` computed it, with `clm-gate`'s `fingerprint_of` over `git ls-tree -r -z` of the merge base of the commit's two parents and of its second parent. An approval *counts* when `signed_by(root, <first parent>, <approved record's line>, "clm-approve")` is true and, when the `approval` that `<first parent>:clm.toml` sets is `other` (read with the `toml` crate; a missing file or key means `any`), its writer's person differs from the signed `landed` record's writer's person and from the slugged author name (`git log --format=%an`, slugged as the writer is) of every commit in `<first parent>..<second parent>`. A recomputed fingerprint that differs from the `landed` record's is reported as `ledger/unapproved-land` too, since any approval was given for another diff (`verify-signed-lands`). A land whose first parent isn't signed is judged by its record alone, so history from before signing stays valid. Without `--review`, `verify` trusts a land's recorded kind and review, a false one being attributable through the record's signature. `clm-gate` is a dependency of the default feature only, so the wasm build doesn't see it. When a commit has several `landed` records, `verify` judges it by any one that is signed, and requires an approval when any signed one says it needed review.

**Signing details.** `sign` takes what `append` takes, the record's body and the injected environment, and returns a full record that `append` accepts unchanged. The signed message is built explicitly with sorted keys (a `BTreeMap`), never by relying on `serde_json`'s `preserve_order` feature being off, and holds the fields exactly as the line writes them: a `code_key` left out of the line is left out of the message, a `null` stays `null`. `signed_by` checks the line as read, not a record re-serialised from the struct: the fold keeps each record's parsed JSON object beside it, so a field a newer `clm` adds still verifies on an older one. The key is read with `git config --get --type=path user.signingkey`, so `~/` expands, and a relative path is taken from the top of the git repository; `gpg.ssh.defaultKeyCommand` is not used. The git config lookups go through `clm-git` with the injected environment (`HOME`, `XDG_CONFIG_HOME`, `GIT_CONFIG_GLOBAL`), so a test gives its own global config. `ssh-keygen` sets no time limit: a key that needs a passphrase or a touch waits for it. `docs/allowed_signers` and `clm.toml` are read at the top of the git repository, as `code_key`'s paths are. `toml` is a dependency of the default feature. `ssh-keygen` (OpenSSH 8.2 or later) must be on `PATH`.

**Recomputed review** (`verify-recomputes-review`). `clm ledger verify --review` recomputes the kind and review as `clm land` computed them in its step 6, the gate on the merge commit against the `origin/main` it merged into, which is the commit's first parent, under the rules that gate applied: `clm-gate`'s `evaluate_by(root, <first parent>, <commit>, false, <rules>)`, read from git objects with no checkout (`gate/evaluate-at`, `gate/rules-version`). A land's *rules in force* are the higher of its own and its *floor*. Its own is the highest value among the `Gate-Rules` trailers of its merge commit (`land/gate-rules-trailer`) that, trimmed, is a whole number of at least 1, read with `git log`'s `%(trailers:key=Gate-Rules,valueonly,separator=%x1e)` through `clm-git` as `Clm-By-Hand` is, or 1 when there is none: lands from before the trailer were judged by the rules before version 2, and several trailers can only raise the version. Its floor is the highest own version, capped at the auditing `clm`'s `RULES`, of any earlier commit on the chain whose first parent is signed and that a `landed` record signed by a key of that first parent's `docs/allowed_signers` names (as `ledger/unsigned-land` judges it), read from the whole chain whatever `--since` says, so a lander can't take older, laxer rules by writing a lower trailer than one `main` has already landed under (`audit-by-the-rules-in-force`); a land judged by an older `clm` after a newer one landed is judged by the newer rules, and a finding then names a land that got less than `main`'s rules asked. A land's own version above the auditing `clm`'s `RULES` makes `evaluate_by` fail, so that land is reported as one the gate can't rate, naming the version; the cap keeps it from making every later land unratable too, so one land written by a newer `clm`, or with a forged high trailer, is one finding and the lands after it are still recomputed. A land with no signed record is already `ledger/unsigned-land` and isn't recomputed. The recorded kind and review are those of the highest-ranked signed `landed` record naming the commit (the first in the fold's order among equals), and that same record supplies the lander and the fingerprint when a land recomputed as needing review is audited for approval. They rank `code` = `spec` < `spec review` < `mixed`, since a code land and a spec land that needs no review both land on checks alone, and the gate blocks a `mixed` diff outright. A change that needed review, recorded as not needing it, with no counted approval reports both rules, `ledger/understated-land` first, so the audit of one commit may give more than one finding. A recomputed rank at or below the recorded one is no finding: an over-stated land asked for more review than the diff needed. The gate's violations, `gate/up-to-date` among them, are ignored here; only its kind and review are read. A gate error on one land, such as a committed `change.md` the gate can't read, doesn't fail the run (`verify-unrated-land`), so one old land the current gate can't rate neither hides the lands after it nor ends with an error every run that walks it, and the finding names the land for a human. A git failure of the walk itself, or of a read the audit makes outside the gate, still fails `verify` (`commands-in-crate`).

`clm check` runs `verify` without `--review`, since recomputing runs the gate once per signed land, on every check of `main`, for the whole history, and nothing may remember what an earlier run verified (`git-is-truth`: no check reads a cache). `clm wrap-up` passes it, with `--since` at its review mark, so that it recomputes the lands since this clone's last checkpoint rather than the whole history (`orchestrator.checkpoint/checkpoint-review-mark`, `checkpoint-recomputes-new-lands`); a human can run it any time, and `clm ledger verify --review` with no `--since` is the full audit, which reads no mark. The flag covers every land, so `clm check`'s cost stays one walk of the chain. The library's `verify` takes the choice as its `review` argument.

## MODIFIED Requirements

### Requirement: writer-identity <!-- base: 6a854d4d4009684b -->
`writer(root, env)` SHALL return `<person>/<clone>`, where `<person>` is `config`'s `writer` or, when unset, the repository's git `user.name` as a slug, and `<clone>` is the eight hex digits in `.git/clm/writer`, written on first use and read on every later one, so every worktree of the clone shares it. A `.git/clm/writer` that holds nothing but white space SHALL be taken over, fresh digits written to it. It SHALL fail, naming the `writer` setting, when the person part would be empty, and it SHALL refuse a `writer` setting that `slug` would change, one `config` accepts such as `ann--lee`, as ``` `writer` is `<setting>`, not a slug: use lowercase letters, digits and `-`, such as `<its slug>` ```.

#### Scenario: first use
- GIVEN a clone whose git `user.name` is `Christopher Lunny`, no `writer` setting and no `.git/clm/writer`
- WHEN `writer(root, env)` runs
- THEN it returns `christopher-lunny/<8 hex digits>` and `.git/clm/writer` holds those digits

#### Scenario: shared by worktrees
- GIVEN that clone and a linked worktree of it
- WHEN `writer` runs in each
- THEN both return the same identity

#### Scenario: setting wins
- GIVEN `CLM_WRITER=clunny`
- WHEN `writer(root, env)` runs
- THEN its person part is `clunny`

#### Scenario: odd identities
- GIVEN, in turn, `CLM_WRITER=ann--lee`, `CLM_WRITER=Christopher/Lunny`, and a clone whose git `user.name` is `T` and whose `.git/clm/writer` is empty
- WHEN `writer(root, env)` runs on each
- THEN the first fails with ``` `writer` is `ann--lee`, not a slug: use lowercase letters, digits and `-`, such as `ann-lee` ```, the second fails naming `` `writer` ``, and the third returns `t/<8 hex digits>` and `.git/clm/writer` then holds those digits

### Requirement: append <!-- base: d894aff04a31a819 -->
`append(root, entry, env)` SHALL add the record, with a fresh `id`, the writer and the time unless it already has them, as a record `sign` returned does (`record-signing`), as one JSON line at the end of `records.jsonl` on the writer's branch `clm/ledger/<person>/<clone>`, creating the orphan branch on first use, without touching the working tree or the index, and SHALL push the branch to `origin` without force, on every host, `none` included, whose `origin` is the clone's own remote (`host/none-keeps-a-remote`). The time it writes SHALL be the UTC time as `YYYY-MM-DDTHH:MM:SS.mmmZ`. When `origin` can't be reached it SHALL keep the commit and report the record as unpushed. It SHALL never rebase or force-push a ledger branch. It SHALL refuse a signed record whose `writer` isn't this clone's writer, as `the record is signed as writer <its writer>, but this clone writes as <this clone's writer>`, committing nothing. Appends made at once in one clone SHALL each keep their record. Each commit SHALL name the writer's person as its author and its committer, with the email git gives the clone, or an empty one when git can't find or guess one.

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

#### Scenario: odd appends
- GIVEN a clone whose git `user.name` is `Ann`, with a bare `origin`
- WHEN `append` runs with a record `sign` returned for writer `dana/<clone>`; on four threads at once, each with a `checked` record; once with git's `user.email` set to `t@example.com`; and once in a clone with no `user.email` and `user.useConfigOnly` set
- THEN the first fails naming `dana/` and leaves no `clm/*` branch; the branch then holds all four threads' records; the third commit's author and committer are `ann <t@example.com>`, and the fourth's `ann <>`; and each record's `at` is 24 characters of the form `YYYY-MM-DDTHH:MM:SS.mmmZ`

### Requirement: fold <!-- base: 134ea7aa4a7628d1 -->
`fold(root)` SHALL return the union of the records on every `origin/clm/ledger/*` ref and on every local `clm/ledger/<person>/<clone>` branch whose `<clone>` is this clone's, whatever its `<person>` (none while the clone has no `.git/clm/writer`), counting a record once by `id`, indexed as approvals by fingerprint, checks by code key and lands by commit, without fetching, and `fold_records(sources)` SHALL return the same fold over `(ledger, text)` pairs already read. The fold's `records` SHALL hold every record once, ordered by `at` compared as strings and then by `id`, whatever order the ledgers are read in and their lines written in, and the approvals, checks and lands SHALL list a key's records in that order. A line that doesn't parse, among them one whose `kind` the ledger doesn't have, one without a field its kind requires (a `commented` line without `target` included) and one whose JSON isn't an object, SHALL be skipped with the warning `<ledger> line <n>: skipped, not a record: <why>`, `<n>` counting the file's lines from 1, blank ones included, not fail the fold. A ledger ref whose tree has no `records.jsonl` SHALL be skipped with the warning `<ledger>: skipped, no records.jsonl: <why>`. `<ledger>` is the ref without `refs/remotes/` or `refs/heads/`. A blank line, and a `reviewed` line, a kind the ledger no longer has (`yes-no-review`), SHALL be skipped without a warning, and a field a record's kind doesn't have SHALL be read without a warning and kept in the record's line.

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

#### Scenario: odd lines
- GIVEN `origin/clm/ledger/x/1` whose lines are, in order, a line of `kind` `nope`, a `commented` record without `target`, `[1,2]`, a blank line and an `approved` record for `f` with the extra field `"x":1`; `origin/clm/ledger/y/2` whose tree has no `records.jsonl`; and a local branch `clm/ledger/old/<this clone's digits>` holding an `approved` record for `g`
- WHEN `fold` runs
- THEN its approvals are `f` and `g`, `f`'s line holds `"x":1`, and its warnings are `origin/clm/ledger/y/2: skipped, no records.jsonl: ` with git's message, and lines starting `origin/clm/ledger/x/1 line 1: skipped, not a record: `, `origin/clm/ledger/x/1 line 2: ` and `origin/clm/ledger/x/1 line 3: `, and no other

#### Scenario: times out of order or equal
- GIVEN ledger `a` holding a record of `id` `2` at `2026-01-02T00:00:00.000Z` and after it one of `id` `3` at `2026-01-01T00:00:00.000Z`, and ledger `b` holding one of `id` `1` at `2026-01-02T00:00:00.000Z`
- WHEN `fold_records` runs over them, `a` first and again `b` first
- THEN its `records` are `3`, `1` and `2` in that order, both times

### Requirement: code-key <!-- base: 9217c035af7f193d -->
`code_key(root, rev)` SHALL return the same key for two commits whose trees differ only under `specs/`, `changes/`, `backlog/` or `docs/`, the guide's pages under `docs/guide/` and `docs/spec-format.md` included, or in `ARCHITECTURE.md`, `CAPABILITIES.md`, `README.md`, `CLAUDE.md` or `AGENTS.md`, and a different key when any other entry differs, a new top-level file included.

#### Scenario: doc-only difference
- GIVEN commits `a`, `b` that changes only `docs/x.md` and `backlog/y.md`, `c` that also changes `crates/a/src/lib.rs`, `d` that adds `tool.toml` at the root, `e` that changes only `docs/guide/intro.md`, and `f` that changes only `docs/spec-format.md`
- WHEN `code_key` runs on each
- THEN `a`, `b`, `e` and `f` give the same key, and `c` and `d` each give a different one

#### Serves
- capability/land-safely
- architecture/deterministic-enforcement

### Requirement: compact <!-- base: cf9d661fdc1b02e4 -->
`clm ledger compact` SHALL fetch first, compacting against what was last fetched when the fetch fails, and SHALL rewrite the caller's own ledger without the `approved` records whose fingerprint any `landed` record names, except one whose fingerprint a signed land that needed review names, a land whose first parent's tree holds `docs/allowed_signers`, and without the `checked` records older than the newest `landed` record whose code key no land's code key matches, keep every `landed` record and every `checked` record a land's code key names, drop every `reviewed` line, push the result, and print `removed <n>`, `<n>` the number removed. A land's code key is its record's `code_key`, or its legacy key, computed as `verify` computes it, when the record has none (`legacy-code-key`). It SHALL NOT touch another writer's branch. When a `landed` record without `code_key` names a commit not in the clone and one of the caller's `checked` records is older than the newest `landed` record and names a key no `landed` record's `code_key` names, it SHALL keep every `checked` record and warn ``kept every `checked` record: landed <commit> isn't in this clone, so its code key can't be computed; fetch it and compact again``. When a `landed` record that needed review names the fingerprint of one of the caller's `approved` records and a commit not in the clone, it SHALL keep that approval and warn ``kept the approval of <fingerprint>: landed <commit> isn't in this clone, so whether `clm ledger verify` reads it can't be told; fetch it and compact again``. It SHALL keep a line it can't parse, and SHALL make no commit when it removes nothing. When `origin` refuses the push, it SHALL keep the local commit, warn `the compacted ledger isn't on origin yet: git push origin clm/ledger/<writer>`, print `removed <n>` and exit 0.

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
- GIVEN the caller's ledger with `checked k`, a `landed` record for commit `c` with no `code_key` field, where `c`'s tree holds a page under `docs/guide/`, so its legacy key `k` differs from its code key, and a later `landed` record with code key `m`
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

#### Scenario: what compaction can't see
- GIVEN, in turn: the caller's `approved f` and `checked old`, then a `landed` record with fingerprint `f` and no `code_key` for a commit not in the clone, and a later `landed` record with code key `m`; the caller's `approved f` and a `landed` record that needed review with fingerprint `f` for a commit not in the clone; the caller's ledger holding the line `not json` and an `approved` record a `landed` record names; the caller's ledger holding only `approved g`, which no `landed` record names; and the history of *landed approval pruned*, with `origin`'s `pre-receive` hook declining every push
- WHEN `clm ledger compact` runs on each
- THEN the first prints `removed 1`, keeps `checked old` and warns on standard error ``warning: kept every `checked` record: landed <commit> isn't in this clone, so its code key can't be computed; fetch it and compact again``; the second prints `removed 0`, keeps `approved f` and warns ``warning: kept the approval of f: landed <commit> isn't in this clone, so whether `clm ledger verify` reads it can't be told; fetch it and compact again``; the third prints `removed 1` and keeps `not json`; the fourth prints `removed 0` and leaves the branch's tip where it was; and the fifth prints `removed 2`, warns `warning: the compacted ledger isn't on origin yet: git push origin clm/ledger/<writer>`, exits 0, and leaves the local branch without the two records

### Requirement: verify <!-- base: 0a969f83b1ffd9df -->
`clm ledger verify [--since <rev>]` SHALL report, in `check`'s format, `ledger/unrecorded-land` for each commit on `main`'s first-parent chain after `<rev>`, merge or not, that no `landed` record names, except one whose message starts `Release v`, and `ledger/unchecked-land` for each landed commit whose land's code key no `checked` record names, in chain order, oldest first, exiting 0 with no output when there are none and 1 otherwise. Without `--since` it SHALL audit the chain from the oldest commit a `landed` record names, that commit included, and report nothing when no `landed` record names a commit on it. The message of `ledger/unrecorded-land` SHALL be ``commit <sha> (<subject>) has no `landed` record: it is on main without `clm land` ``, and that of `ledger/unchecked-land` ``landed <sha>: no `checked` record names its code key <keys>``, `<keys>` each code key its records give, sorted and joined by `, `; `verify` called with another tip names that tip in place of `main`. A land's code key SHALL be its record's `code_key`, or, only when the record has none, its legacy key: the key over the commit's tree as `code_key` computes it, but leaving out exactly the paths `legacy-doc-only` names instead of today's doc-only paths, so a page under `docs/guide/`, `CAPABILITIES.md` and `AGENTS.md` are code in it (`legacy-code-key`). `clm-ledger` SHALL offer the same findings, as violations against `main`, for `clm check` to report: `on_main(root)` SHALL return those of `main` when the checked-out branch is `main` and the repository has a ledger branch, local or on `origin`, and none otherwise; when the audit itself fails it SHALL return one violation of rule `ledger/verify` whose message is `can't audit main against the ledger: <why>`, never none. That `clm check` reports them is `clm/check-across-components`'s.

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
- GIVEN a landed merge commit whose tree holds a page under `docs/guide/`, so its legacy key and its code key differ, whose `landed` record has no `code_key` field, a `checked` record for the commit's legacy key, and no `checked` record for `code_key(root, commit)`
- WHEN `clm ledger verify` runs
- THEN it prints nothing and exits 0

#### Scenario: offered to the check on main only
- GIVEN a repository on `main` with a ledger branch and the history of *push behind clm's back*
- WHEN `on_main(root)` runs, and again with another branch checked out
- THEN the first hold `ledger/unrecorded-land`, and the second are empty

#### Scenario: where the audit starts
- GIVEN a `main` whose merge commits are `m0` and then `m1`, a `landed` record naming `m1` with code key `k`, and no `checked` record naming `k`
- WHEN `clm ledger verify` runs without `--since`; again once a `checked` record names `k` and a single-parent commit `p` with the subject `hotfix` is pushed onto `main`; and again on a `main` whose ledger's only `landed` record names a commit not on its chain
- THEN the first prints exactly ``main: ledger/unchecked-land: landed <m1>: no `checked` record names its code key k``, names `m0` nowhere and exits 1; the second prints exactly ``main: ledger/unrecorded-land: commit <p> (hotfix) has no `landed` record: it is on main without `clm land` `` and exits 1; and the third prints nothing and exits 0

#### Scenario: an audit that can't run
- GIVEN a repository on `main` with a ledger branch whose audit fails, such as one whose signed `landed` record names a merge commit whose first parent holds `docs/allowed_signers`, with no `ssh-keygen` on `PATH`
- WHEN `on_main(root)` runs
- THEN it returns one violation against `main`, of rule `ledger/verify`, whose message starts `can't audit main against the ledger: `

#### Serves
- capability/land-safely
- architecture/git-is-truth

### Requirement: show <!-- base: 803f9c27fd165e04 -->
`clm ledger show` SHALL print the fold, one record per line in the order of the fold's `records` (`fold`): an `approved` record as `approved <fingerprint> <branch> <writer> <at>`, a `checked` one as `checked <code_key> <writer> <at>` and a `landed` one as `landed <commit> <branch> <land_kind> <review> <writer> <at>`, `<review>` being `review` or `-`, with no quoting, and each other kind as its requirement says. `show(fold)` SHALL return those lines.

#### Scenario: listing
- GIVEN ledgers holding an `approved` and a `landed` record
- WHEN `clm ledger show` runs
- THEN it prints two lines, `approved ...` then `landed ...`, in time order

#### Scenario: a check
- GIVEN a ledger holding a `checked` record for code key `k` by writer `x/1` at `2026-01-01T00:00:00.000Z`
- WHEN `clm ledger show` runs
- THEN it prints `checked k x/1 2026-01-01T00:00:00.000Z`

#### Serves
- capability/see-project-state

### Requirement: record-signing <!-- base: 12908fe7d59d06cc -->
`sign(root, body, namespace, env)` SHALL return the record with `id`, `writer` and `at` filled in as `append` fills them and `sig` holding the output of `ssh-keygen -Y sign -n <namespace>` over its signed message (the record as a JSON object without `sig`, keys sorted, no whitespace), using git's `user.signingkey` when `gpg.format` is `ssh`: a path, a relative one taken from the top of the git repository, or a `key::<public key>` value, whose key `ssh-keygen` signs with through `ssh-agent`. When `gpg.format` isn't `ssh`, or `user.signingkey` is unset or empty, it SHALL fail with `Error::Signing` holding `no SSH signing key: set git's user.signingkey to your SSH key, and gpg.format to ssh (git config gpg.format ssh)`, and when `ssh-keygen` fails or can't be started, with `Error::Signing` holding `can't sign the record: ssh-keygen -Y sign -f <key> -n <namespace>: ` followed by `ssh-keygen`'s standard error, or, when that is empty, its exit status, or why it couldn't start. `append` SHALL write a record's existing `id`, `writer`, `at` and `sig` unchanged, and the fold SHALL read a line with or without `sig`.

#### Scenario: signed and appended
- GIVEN a clone whose git config sets `gpg.format = ssh` and `user.signingkey` to a fresh ed25519 key's path
- WHEN `sign` runs on an `approved` record with namespace `clm-approve` and `append` writes the result
- THEN its line in `records.jsonl` holds that `sig`, and `ssh-keygen -Y verify` with the key's public half, the namespace `clm-approve` and the line's object without `sig` (keys sorted, no whitespace) on stdin exits 0

#### Scenario: no key
- GIVEN a clone with no `user.signingkey`
- WHEN `sign` runs
- THEN it fails naming `user.signingkey` and appends nothing

#### Scenario: signing refused
- GIVEN, in turn, a clone at `<home>/a` whose `user.signingkey` names a key and whose `gpg.format` is unset; one whose `gpg.format` is `ssh` and `user.signingkey` is `../ann`, a key at `<home>/ann`; one whose `user.signingkey` is `key::` and the public half of a key an `ssh-agent` holds; and the second with no `ssh-keygen` on `PATH`
- WHEN `sign` runs on each with namespace `clm-approve`
- THEN the first fails with `Error::Signing` holding `no SSH signing key: set git's user.signingkey to your SSH key, and gpg.format to ssh (git config gpg.format ssh)`; the second's and the third's `sig` verify against the key's public half; and the fourth fails with `Error::Signing` holding a message that starts `can't sign the record: ssh-keygen -Y sign -f `

#### Serves
- architecture/signed-ledger-records
- capability/approve-intent

### Requirement: signature-check <!-- base: 8b219b3b0f7ed90c -->
`signed(root, rev)` SHALL be true exactly when `rev`'s tree holds `docs/allowed_signers`. `signed_by(root, rev, line, namespace)`, `line` a record's JSON object as read, SHALL be true exactly when the line has a `sig` that `ssh-keygen -Y verify` accepts against `rev`'s `docs/allowed_signers` for the person part of the line's `writer`, the namespace and the line's signed message. It SHALL be false for a line without a `sig` or a `writer` string, or whose writer's person part is empty, and for a `rev` whose tree has no `docs/allowed_signers`. When `ssh-keygen` can't be run it SHALL fail with `Error::Signing` holding `can't check a signature: ` and why, never return false, so a missing tool is never taken for a forged record.

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

#### Scenario: no tool
- GIVEN the record of *listed signer*, the same record's line without `writer`, and no `ssh-keygen` on `PATH`
- WHEN `signed_by` runs on each with namespace `clm-approve`
- THEN the first fails with `Error::Signing` holding a message that starts `can't check a signature: `, and the second is false

#### Serves
- architecture/signed-ledger-records
- capability/approve-intent

### Requirement: verify-signed-lands <!-- base: 09eefb149e9911f0 -->
`clm ledger verify` SHALL, for each landed commit on `main`'s first-parent chain whose first parent's tree holds `docs/allowed_signers`, report `ledger/unsigned-land` when no `landed` record naming it is signed under `clm-land` by its writer, and, when that record says the land needed review, report `ledger/unapproved-land` unless an `approved` record for the fingerprint recomputed from the commit's parents is signed under `clm-approve` by its writer and, when the first parent's `clm.toml` sets `approval = "other"`, its writer's person is neither the signed land's writer's person nor the slugged author of any commit the land brought in. It SHALL judge a land whose first parent has no `docs/allowed_signers` as it did before. A `code` land whose signed record says `review` true SHALL be audited for its approval as one that needed review. A signed land that needed review SHALL be reported `ledger/unapproved-land` when its commit has one parent, as `landed <sha>: it needed review, but isn't a merge, so no diff was approved`, and when its parents share no history, as `landed <sha>: its parents share no history, so no diff was approved`. The message of `ledger/unsigned-land` SHALL be ``landed <sha>: no `landed` record naming it is signed by its writer``, and that of `ledger/unapproved-land`, when the fingerprints differ, ``landed <sha>: its `landed` record names fingerprint <recorded>, but its parents give <recomputed>, so any approval was for another diff``, and otherwise ``landed <sha>: it needed review, but no `approved` record for its fingerprint <fingerprint> is signed by its writer``, followed, under `approval = "other"`, by ` and by someone other than the lander and the authors`. When the first parent's `clm.toml` that such an audit reads doesn't parse, `clm ledger verify` SHALL fail naming `<first parent>:clm.toml`.

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

#### Scenario: lands no approval can clear
- GIVEN `docs/allowed_signers` on `main`, then, each with a `checked` record for its code key: a single-parent commit `p` named by a signed `landed` record that needed review; a merge commit `u` of a branch that shares no history with `main`, named by such a record; and a merge commit `c` whose signed `landed` record has land kind `code` and `review` true, with no `approved` record for its fingerprint
- WHEN `clm ledger verify` runs, and again with a `clm.toml` that doesn't parse in `c`'s first parent and a counted approval of `c`'s fingerprint
- THEN the first prints `main: ledger/unapproved-land: landed <p>: it needed review, but isn't a merge, so no diff was approved`, `main: ledger/unapproved-land: landed <u>: its parents share no history, so no diff was approved` and `ledger/unapproved-land` naming `c`, and exits 1; the second prints on standard error `clm ledger: <c's first parent>:clm.toml: ` and the parser's message, and exits 2

#### Scenario: what the findings say
- GIVEN the histories of *unsigned land*, *approval for another diff* and *approval in another's name*
- WHEN `clm ledger verify` runs on each
- THEN it prints ``main: ledger/unsigned-land: landed <sha>: no `landed` record naming it is signed by its writer``, ``main: ledger/unapproved-land: landed <sha>: its `landed` record names fingerprint <recorded>, but its parents give <recomputed>, so any approval was for another diff`` and ``main: ledger/unapproved-land: landed <sha>: it needed review, but no `approved` record for its fingerprint <fingerprint> is signed by its writer and by someone other than the lander and the authors``, in turn

#### Serves
- architecture/signed-ledger-records
- architecture/humans-review-intent
- capability/approve-intent

### Requirement: verify-recomputes-review <!-- base: 42a503d90e447077 -->
`clm ledger verify --review` SHALL, for each landed commit on `main`'s first-parent chain whose first parent's tree holds `docs/allowed_signers` and that a signed `landed` record names, recompute its kind and whether it needs review as the gate rates its merge commit against its first parent (`gate/evaluate-at`) under the rules in force when it landed (`gate/rules-version`), and report `ledger/understated-land` when the recomputed one ranks above the recorded one in the order `code` = `spec` < `spec review` < `mixed`, the recorded one being the highest among the signed records naming the commit, as `landed <sha>: recorded <kind>, the gate gives <kind>`, each kind written `code`, `spec`, `spec review` or `mixed`. The rules in force SHALL be the higher of the commit's own version, the highest value among its message's `Gate-Rules` trailers that is a whole number of at least 1, or 1 when there is none, and its floor, the highest own version, capped at the gate's `RULES` (`gate/rules-version`), of an earlier commit on the chain whose first parent holds `docs/allowed_signers` and that a `landed` record signed by a key listed there names, `--since` or not. It SHALL audit a land recomputed as needing review for a counted approval as `verify-signed-lands` audits a land recorded so. Without `--review`, and for a land whose first parent has no `docs/allowed_signers`, it SHALL judge the land by its recorded kind and review as before.

#### Serves
- architecture/signed-ledger-records
- architecture/humans-review-intent

#### Needs
- gate/rules-version

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

#### Scenario: a removal before the rule
- GIVEN `docs/allowed_signers` on `main`, then a land of a branch that only removes a requirement, whose merge commit has no `Gate-Rules` trailer and whose signed `landed` record has land kind `spec` and `review` false, with a `checked` record for its code key and no `approved` record
- WHEN `clm ledger verify --review` runs
- THEN it prints nothing about that land

#### Scenario: a removal under the rule
- GIVEN the history of *a removal before the rule*, its merge commit ending with `Gate-Rules: 2`
- WHEN `clm ledger verify --review` runs
- THEN it prints `ledger/understated-land` naming the commit, `spec` and `spec review`, and `ledger/unapproved-land` naming the commit, and exits 1

#### Scenario: a lower trailer after a higher one
- GIVEN `docs/allowed_signers` on `main`, then a signed, honest land whose merge commit ends with `Gate-Rules: 2`, then the land of *a removal before the rule*, whose merge commit ends with `Gate-Rules: 1`
- WHEN `clm ledger verify --review --since` the first land's commit runs
- THEN it prints `ledger/understated-land` and `ledger/unapproved-land` naming the second land's commit

#### Scenario: rules newer than this clm
- GIVEN `docs/allowed_signers` on `main`, then a signed, checked land whose merge commit ends with `Gate-Rules: ` and a number above `clm-gate`'s `RULES`
- WHEN `clm ledger verify --review` runs
- THEN it prints `ledger/unrated-land` naming the commit, and in the gate's message the land's version and `RULES`, and exits 1

#### Scenario: a land after rules newer than this clm
- GIVEN the history of *rules newer than this clm*, then the land of *review recorded as none*, whose merge commit has no `Gate-Rules` trailer
- WHEN `clm ledger verify --review` runs
- THEN it prints `ledger/unrated-land` naming the first land's commit, and `ledger/understated-land` and `ledger/unapproved-land` naming the second's

### Requirement: subscription-records <!-- base: e4cb2df28bcc4638 -->
The fold, `fold` and `fold_records` alike, SHALL parse `subscribed` and `unsubscribed` records and hold as its subscriptions every `subscribed` record whose `id` no `unsubscribed` record's `subscription` names, whatever order the records are read in. `clm ledger show` SHALL print a `subscribed` record as `subscribed <id> <origin> <writer> <at>`, `<origin>` being the endpoint's `<scheme>://<host>[:<port>]`, without user information, path, query or fragment, or `-` when the endpoint doesn't start with a scheme followed by `://`, and an `unsubscribed` one as `unsubscribed <subscription> <writer> <at>`, never printing `p256dh` or `auth`. `clm ledger compact` SHALL keep both kinds when they are in the caller's own ledger.

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

#### Scenario: odd endpoints
- GIVEN `subscribed` records for the endpoints `https://push.example:8443/s?x=1`, `https://user:token@push.example/s/1`, `not a url` and `foo?x=https://a/b`
- WHEN `clm ledger show` runs
- THEN their origins print as `https://push.example:8443`, `https://push.example`, `-` and `-`

#### Serves
- capability/follow-from-browser
- architecture/git-is-truth

### Requirement: verify-unrated-land <!-- base: 48a0dc741b8c7270 -->
`clm ledger verify --review` SHALL, when the gate fails on a landed commit it recomputes, report `ledger/unrated-land` as `landed <sha>: the gate can't rate it: <the gate's message>`, judge that land by its recorded kind and review, and go on to the lands after it, exiting 1 rather than failing. It SHALL still fail, as for a git failure, when git fails outside the gate.

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

### Requirement: legacy-doc-only <!-- base: ddac0ce8f6ea4e02 -->
`clm-ledger` SHALL offer `legacy_doc_only(path)`, true for a path relative to the repository root under `specs/`, `changes/`, `backlog/` or `docs/` outside `docs/guide/`, or equal to `ARCHITECTURE.md`, `README.md` or `CLAUDE.md`, and false for any other path, the doc-only list as it was when the `landed` records without a `code_key` were written, so every reader keys such a land with that list (`legacy-code-key`).

#### Scenario: paths
- GIVEN the paths `docs/spec-format.md`, `docs/x.md`, `docs/guide/intro.md`, `specs/SPEC.md`, `changes/x/change.md`, `backlog/y.md`, `ARCHITECTURE.md`, `README.md`, `CLAUDE.md`, `CAPABILITIES.md`, `AGENTS.md`, `crates/a/README.md` and `crates/a/src/lib.rs`
- WHEN `legacy_doc_only` runs on each
- THEN it returns true for `docs/spec-format.md`, `docs/x.md`, `specs/SPEC.md`, `changes/x/change.md`, `backlog/y.md`, `ARCHITECTURE.md`, `README.md` and `CLAUDE.md`, and false for the others

#### Serves
- capability/land-safely

### Requirement: effort-records <!-- base: 2bee0a30a6c6b463 -->
The fold, `fold` and `fold_records` alike, SHALL parse `effort` records and hold them by `id`, keeping for each `id` the record with the latest `at`, and of two with one `at` the one with the greater writer and then the one whose line, written as `signed_message` writes a line (every object's keys sorted bytewise at every depth, `models` and each model's counts included, and no white space), is the greater text, the same in the build with default features off as in the default one, whatever its writer and whatever order the records are read in, in its records and its lines alike. Of two lines that share an `id` but whose JSON objects differ, unless both are `effort` records, the fold SHALL keep, whatever order it reads them in, the one with the earlier `at`, then the lesser writer, then the lesser text written as `signed_message` writes a line, in its records and its lines alike: a later line reusing a record's `id` never displaces it. `clm ledger show` SHALL print an `effort` record as `effort <branch> <session> <work tokens> <active_seconds>s <writer> <at>`, work tokens being `input` + `output` + `cache_write` summed over its models, saturating at 18446744073709551615 rather than overflowing. `clm ledger compact` SHALL drop the caller's own `effort` records other than the one the fold keeps for their `id`.

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

#### Scenario: ties and collisions
- GIVEN two `effort` lines of `id` `e` at one `at`, by `a/1` and `b/2`; two of `id` `e` at one `at` by one writer, one with `active_seconds` 20 and `models` `{"m":{…"output":1…}}` and the other with `active_seconds` 10 and `models` `{"m":{…"output":9…}}`, each written with `models` before `active_seconds`; an `approved` line of `id` `e` at `2026-01-01T00:00:00.000Z` and an `effort` line of `id` `e` at `2026-01-02T00:00:00.000Z`; an `effort` record whose output is 18446744073709551615 and whose active seconds are 120; and the caller's own two `effort` records of one `id` and one `at`
- WHEN the fold reads each pair in either order, `clm ledger show` prints the fourth, and `clm ledger compact` runs on the fifth
- THEN the fold keeps `b/2`'s of the first pair and the one with `active_seconds` 20 of the second, the approval of the third, with no `effort` record held and the approval's line in its `lines`; the line holds ` 18446744073709551615 120s `; and compaction prints `removed 1` and leaves one of the two

#### Serves
- capability/review-capability-cost
- architecture/effort-accounted

### Requirement: append-all <!-- base: 86b61f219f125182 -->
`append_all(root, entries, env)` SHALL add the entries, each as `append` adds one, in one commit on the writer's branch and one push, and SHALL make no commit for no entries. An `effort` body SHALL get `effort_id(session, branch)` as its `id`, the first 32 hex digits of SHA-256 over `<session>\n<branch>`, where `append` would draw a random one. `append` SHALL be `append_all` with one entry.

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

### Requirement: request-cleared <!-- base: 0fe654c619744570 -->
`clm-ledger` SHALL offer, in its wasm build as in its default one, `person(writer)`, the part of a `<person>/<clone>` writer before its first `/`, or the whole writer when it has none, and `Fold::cleared(request)`, true exactly when the fold holds an `approved` record for the `changes_requested` request's `fingerprint` whose writer's person is the request's writer's person and whose `at` is greater than the request's as a string, whether or not it is signed, and false for a record of any other kind. Wherever its calls and commands read a writer's person part (`cleared`, the person `signed_by` checks a signature for, and the lander and the approvers `clm ledger verify` compares under `approval = "other"`), it SHALL be the part `person` gives.

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

#### Scenario: one person part
- GIVEN a commit whose `docs/allowed_signers` lists key `K` for `dana`, an `approved` record for `f` by the writer `dana`, with no `/`, signed with `K` under `clm-approve`, and a `changes_requested` record at `f` by `dana/web` with an earlier `at`
- WHEN `signed_by` runs on the approval's line at that commit with namespace `clm-approve`, and `cleared` on the request in a fold holding both
- THEN both are true

### Requirement: commands-in-crate <!-- base: b9042545a47793b6 -->
`clm-ledger` SHALL offer `clm ledger` as a call, `ledger_command(args, root, env, out, err)`, that takes the command's arguments as the command line gives them after `ledger`, the repository root the caller found, which the subcommand's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints and SHALL return the command's exit status, never ending the process itself. Each warning `compact` or `show` gives SHALL go to standard error as `warning: <text>`, and the lines they print to standard output. A subcommand that fails, for git, the settings, the writer identity or a signature, SHALL write `clm ledger: <why>` to standard error and return 2. For arguments it can't parse it SHALL write the usage error to the error stream and return 2. Given `--help`, after the command or after one of its subcommands, it SHALL write that command's help to the output stream, write nothing to the error stream, and return 0. The help's first line SHALL be the command's description, then an empty line, then the `Usage:` line naming the command in full: for `clm ledger`, ``The ledger of approvals, checks and lands: print it, audit `main` against it, or prune your own superseded records`` and `Usage: clm ledger <COMMAND>`; for `compact`, `Rewrite your own ledger without approvals of landed diffs and checks older than the newest land, push it, and print how many records went` and `Usage: clm ledger compact [OPTIONS]`; for `verify`, ``Report each commit on `main` no `landed` record names, merge or not, and each land no `checked` record covers; exit 1 when there is one. Doesn't fetch`` and `Usage: clm ledger verify [OPTIONS]`; and for `show`, `Print every record, one per line in time order. Doesn't fetch` and `Usage: clm ledger show [OPTIONS]`. It SHALL print a violation as the line `clm-model` gives for it (`model/violation-line`), since this crate does not depend on `check`.

#### Scenario: a command called in a test
- GIVEN a fixture repository whose `main` has a merge commit no `landed` record names
- WHEN `ledger_command` is called with the arguments of `clm ledger verify` and two buffers
- THEN the buffers hold what `clm ledger` prints to standard output and to standard error there, and the call returns the status `clm ledger` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN it is passed an option `clm ledger` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the caller's process goes on

#### Scenario: what it prints where
- GIVEN a ledger whose second line is not JSON
- WHEN `ledger_command` is called with `show`, and with `verify --since nowhere`
- THEN the first writes the records' lines to the output buffer and `warning: ` and the skipped line's warning to the error buffer, and returns 0; and the second writes `clm ledger: ` and git's message to the error buffer and nothing to the output buffer, and returns 2

#### Scenario: help
- GIVEN the arguments `--help`, `compact --help`, `verify --help` and `show --help`
- WHEN `ledger_command` is called with each
- THEN for each it writes to the output buffer the command's description as its first line, an empty line, and the `Usage:` line, as `commands-in-crate` quotes them, writes nothing to the error buffer, and returns 0

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

#### Needs
- model/violation-line

## REMOVED Requirements

### Requirement: git-through-layer <!-- base: c3b2a0b8159e78f0 -->

### Requirement: shared-plumbing <!-- base: 10a161a97571094a -->
