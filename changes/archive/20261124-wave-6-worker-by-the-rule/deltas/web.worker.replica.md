---
component: web.worker.replica
base: { contract: "cd8bafbf62762b88", design: "e6b4011a249772ed" }
---

## MODIFIED Contract
Crate `clm-web-replica`: the Durable Object class `Replica`, one per repo, named `<o>/<r>` (`id_from_name`), which runs `replica`'s core on Cloudflare with a GitHub source and the object's own storage (`web-repo-replica`). `clm-web` re-exports the class and reaches each object through a `Replicas` platform trait, which tests fake. Its routes, all internal (the Worker is the only caller, through the `REPLICA` binding):

- `POST /event`: a verified webhook's event name, delivery and body, plus the installation's id. Answers 202.
- `GET /view/<name>?fresh=<0|1>`: 200 `{"sha", "computed_at", "view"}`, `view` being the view `replica`'s `view` gives for `<name>`; 503 `{"preparing": <share>}` while it has none; 404 with no body for a name that is no view's (`answers-views`).
- `POST /write`: a web write that needs the replica to validate (`decide`, `propose`, `withdraw`, `answer`), with the requester's `login` and `token`, in the body `web.worker/writes-to-object` gives. It answers the status and body the Worker's route answers: 200 with what was appended, a refusal (`write-refusals`), 404 for a token that can't read the repo, 502 when GitHub fails, or 503 `{"preparing": <share>}` while it has no views (`writes-through-object`).

The same entry points are offered to a host's own code and tests as `Object::load(name, store, alarm)` over a `clm_replica::Store` and an `Alarm` (`get`, `set`, `clear` in Unix seconds), with `event(platform, request)`, `view(platform, name, fresh)`, `write(platform, request)` and `on_alarm(platform)`, each but the last answering an `Answer { status, body }` whose `body` is the JSON text, empty for none, and `removed()`, whether an uninstall emptied the object.

A read also tells the object the repo has open pages; the object keeps that for 5 minutes.

## MODIFIED Design
**Source.** The GitHub source, built on `web.worker.platform`'s uncached forms (`object-reads`): installation tokens, `gh` calls, `files::objects_direct` and `ledger::read` for ledgers, `files::listing` for code keys. The object's own queries, the refs under a prefix (`source::tips`) and GitHub's events listing, are the only copies: `web.worker.platform`'s `live::tips` and `live::events` go with its build. The open pull requests are read through platform's paged `live::pulls_direct`, which says whether the listing was read to its end; the object holds no copy of that query. Which refs the views read, and the prefixes they sit under, is `replica`'s rule (`clm_replica::reads_ref`, `clm_replica::REF_PREFIXES`, `replica/refs-the-views-read`): the object holds no copy. Each alarm or request keeps within 50 subrequests, the Free plan's limit per invocation: a first build or a recovery fetches in slices and re-arms the alarm for the next. It fetches by `replica`'s own rule, `clm_replica::keeps_text`, `clm_replica::wanted` and `clm_replica::keeps_hash` (`replica/files-it-fetches`, `replica/asset-hashes`), builds every `File` it gives the replica with `replica`'s constructors (`replica/files-a-source-gives`), never by literal, and leaves the NUL probe to the replica, which makes it; it reads every time GitHub gives with `clm_ledger::unix_time` (`ledger/record-times`). A compare names no file modes, so the blob query asks each listed file's parent tree for its entry's mode in the same GraphQL query (fields `t<j>`, one per distinct parent directory, beside the blobs' `f<i>`), and a file is given at the mode its tree gives, GraphQL's integer (33188, 33261, 40960, 57344) turned back into the octal text the tree listing gives (`100644`, `100755`, `120000`, `160000`): a symlink's blob is fetched with it, and the replica, given it at its mode, holds it as no file. A ref whose change files or code key couldn't be read is held as applied to the replica and, apart, as still to read, so a resync reads it again without taking it for a new ref.

**Events.**
- A `push` to the default branch: `compare` the replica's tip with the push's `after`. A fast-forward that lists fewer than 300 files is fetched by path; anything else lists the tree and refetches every blob whose SHA changed.
- A `push` to `clm/ledger/*`: that branch's `records.jsonl` at its new tip.
- A `push` that creates, moves or deletes an `impl/*`, `spec/*`, `autosync/spec/*` or `propose/*` ref: the ref, its tip's commit time, and the spec ref's change files or the `impl/*` head's code key.
- A `pull_request`: the pull request's number, title, head, base, head SHA, page and whether it is a fork's.
- `installation_repositories` with `removed`, or `installation` with `deleted`: delete the storage of each named repo's object.

Each event is applied, recorded for the activity feed when it is a push or a branch creation, and saved, and the alarm is set for one minute later unless one fires sooner.

**Alarm.** It recomputes, saves, and then:
- posts `{"delivery": <the latest delivery applied>}` to the repo's `Live` object's `/notify` (`web.live/hub`), so open pages refresh. The notice names no branch, record or item, only the latest delivery the new views hold, so a socket opened by someone who has since lost access learns only that the repo changed, and the page's refetch goes through the routes that check access;
- sends the Web Push notifications the recompute's changes call for (`push-waiting`, `push-review`), posting them to the `Live` object's `/send`, at most `clm_web_live::MAX_MESSAGES` per send, and keeps the ids of the subscriptions whose endpoint `/send` answered gone in its own storage beside the replica, since no ledger record it may write ends another person's subscription. That list is a disposable cache of the object, as `git-is-truth` and `web.worker/cache-is-disposable` allow: a rebuild from git forgets it, and it is re-learned at the cost of one push per gone endpoint;
- re-arms for the time `recompute` returned while the repo has open pages, and, while it has, about every 10 minutes for a resync that lists the refs and open pull requests and applies any difference.

**First build.** Every ref the first build lists is one the object didn't hold because it hadn't looked, not because it was created, so the first build records the branch creations GitHub's events listing gives and none for the refs it lists; a later resync records a creation for a ref it didn't hold, since its webhook was lost.

**Storage.** `replica`'s chunks are values of the object's SQLite storage, each at most 1 MiB, under Cloudflare's 2 MB per value. The object loads them once when it wakes and keeps the replica in memory while it is awake, so reads and recomputes cost no rows read, and saves the whole replica, in chunks, after each change it applies. Beside them it keeps its own record of what the source told it, with the version of the object that wrote it: storage written by an object that didn't hash binary assets (its record names no version) is dropped on waking and built again (`fetches-binary-assets`). A write applies the ledger text the append committed, so it shows at once without reading it back.

**Platform** (`object-class`). The class implements, over `worker`'s `Env`, `Fetch` and clock, only the platform traits the object uses (`GitHub`, `Clock`, `Secrets`, `Live`, `Log`), as `clm-web-live`'s `durable.rs` implements its own `Host`, and calls `web.worker.platform`'s uncached forms (`object-reads`). What the Worker keeps in the Cache API the object keeps in memory while it is awake: an installation token until five minutes before it expires, and each login's permission for five minutes, as `access::permission` does. It reaches no KV or Cache API binding (`web.worker/cache-is-disposable`). Ledger records are built with `clm-ledger`'s types (`depends_on` lists `ledger`).

**Budget.** An object's request or alarm has 30 s of CPU. The account's daily allowance on the Free plan (100,000 requests, 5 million rows read, 100,000 rows written, 13,000 GB-s, 5 GB stored) is what binds; `web-on-free-plan` says when to move to Paid.

## MODIFIED Requirements

### Requirement: applies-events <!-- base: f75e1a7c901a727f -->
On a verified `push` to the default branch, the object SHALL fetch only the files the compare between its tip and the push's `after` lists as added or modified, and only when the compare is a fast-forward that lists fewer than 300 files, since a list of 300 may be cut short, and otherwise SHALL list the tree and fetch the blobs whose SHAs it doesn't hold. On a push to a ledger branch it SHALL fetch only that branch's records, and on a push that moves or creates a ref the views read, only that ref's change files or code key; a push that deletes such a ref or a ledger branch SHALL fetch nothing and drop it from the views. Each push to a branch other than a `clm/ledger/*` branch SHALL reach the activity feed as a push by its pusher, or its sender when it names no pusher, at its head commit's time, and a push that creates an `impl/*` branch the views read also as its claim; a push of a tag SHALL be applied to nothing.

#### Needs
- replica/apply-what-changed
- replica/stores-in-chunks
- replica/refs-the-views-read

#### Scenario: a push to main
- GIVEN a replica at `111` and a push to `222` whose compare lists `t.rs` modified and `u.rs` removed
- WHEN the event is applied
- THEN the only file fetched is `t.rs` at `222`, and no tree was listed

#### Scenario: a ledger append
- GIVEN a replica holding `clm/ledger/alice/web`
- WHEN a push moving that branch is applied
- THEN the only fetch is that branch's `records.jsonl` at its new tip

#### Scenario: truncated compare
- GIVEN a replica at `111` and a push to `222` whose compare lists 300 files of the 301 that changed
- WHEN the event is applied
- THEN the tree at `222` is listed, and only blobs whose SHAs the replica doesn't hold are fetched

#### Scenario: a claim pushed
- GIVEN a built replica
- WHEN a push creating `impl/a` at `c1` is applied, then a push deleting it, and the views are recomputed after each
- THEN the first made one request to GitHub beyond the token, the tree at `c1`, and the implementation view lists `a` claimed; the second made no request, and the view lists no claim on `a`

#### Scenario: a push in the feed
- GIVEN a built replica
- WHEN `alice` pushes `impl/a`, creating it, then pushes `spec/x`, creating it, then `clm/ledger/alice/web`, then the tag `v1`, and the views are recomputed
- THEN `activity` holds alice's claim of `a` and her pushes to `impl/a` and `spec/x`, each at its head commit's time, and no claim of `x` and nothing for the ledger branch or `v1`

### Requirement: alarm-batches <!-- base: 7c4c8c7aae957501 -->
Applying an event SHALL set the alarm to one minute later unless one is set to fire within that minute, and SHALL NOT recompute. The alarm SHALL recompute once, save, then post `{"delivery"}` naming the latest delivery applied to the repo's `Live` object's `/notify`. While the repo has open pages, the alarm SHALL also fire at the time the last recompute said the implementation view next changes, recompute, and post `{"delivery"}` to `/notify`, naming 32 lowercase hex digits no event has when no event was applied since the last notice.

#### Needs
- replica/batched-recompute

#### Scenario: a burst of pushes
- GIVEN a replica whose views describe `111`
- WHEN pushes to `222`, `333` and `444` are applied within 30 seconds, and the alarm then fires
- THEN the views were recomputed once, describe `444`, and the `Live` object was posted one notice, naming the third push's delivery

#### Scenario: an idle alarm pulled in
- GIVEN a replica read a minute ago, whose alarm is set ten minutes ahead for a resync
- WHEN a push is applied
- THEN the alarm is set one minute after it

#### Scenario: a claim lapses while a page is open
- GIVEN a replica holding a claim on `a` that lapses at 14:00, read at 13:58
- WHEN the alarms run until 14:00
- THEN an alarm fired at 14:00, the implementation view no longer lists the claim, and one notice was posted, whose `delivery` is 32 hex digits naming no event

### Requirement: first-build <!-- base: d07421b038280704 -->
A read of a repo whose object holds nothing SHALL answer 503 `{"preparing": 0}` and start the build, and an event reaching such an object SHALL answer 202, start the build and apply nothing of the event. No request or alarm of the build SHALL make more than 50 subrequests. The views SHALL be computed, with coverage indexing, as soon as the spec files are in, before the non-Markdown files are scanned. When the build can't get an installation token, or can't read the default branch, its tip or its tree, the alarm SHALL re-arm a minute later and reads SHALL go on answering 503 `{"preparing": 0}`. The build SHALL put each branch creation GitHub's events listing gives in the activity feed once, by its actor, and no creation for a ref it lists that the listing doesn't give.

#### Needs
- replica/preparing
- replica/batched-recompute

#### Scenario: a repo of 1,000 files
- GIVEN a repo with 10 spec files and 990 non-Markdown files, and an object that holds nothing; and, in a second run, a repo of 6,000 non-Markdown files
- WHEN the snapshot is read, and the object's alarms run until it re-arms no more
- THEN the first read answered 503 `{"preparing": 0}`, no invocation made more than 50 subrequests, a read after the first alarm answered a snapshot with coverage indexing, and a read after the last a snapshot with coverage complete; the second run completes too, no invocation over 50

#### Scenario: GitHub not ready
- GIVEN an object that holds nothing
- WHEN a push event is applied, the alarm fires while GitHub fails the tree listing, and then fires again once GitHub answers
- THEN the event answered 202 and set the alarm to now; the first alarm re-armed 60 s on, and a read then answered 503 `{"preparing": 0}`; after the second alarm, the snapshot describes the tip GitHub gave

#### Scenario: a claim made before the first build
- GIVEN a repo holding `impl/a` and `impl/b`, and GitHub's events listing `alice`'s creation of `impl/a` and none of `impl/b`
- WHEN the object's first build runs and the views are recomputed
- THEN `activity` holds exactly one claim, `a` by `alice`

### Requirement: resync <!-- base: 69cbabef83dfe188 -->
While a repo has open pages, the object SHALL list its refs and open pull requests about every 10 minutes and apply any difference from what it holds, and SHALL list nothing for a repo nobody has read for 5 minutes. A held ref GitHub no longer lists SHALL be dropped; when a prefix holds more than 300 refs, the resync SHALL change nothing under that prefix. It SHALL read the open pull requests in pages of 100, at most 300; a held pull request the listing doesn't name SHALL be closed only when the listing was read to its end, and with more than 300 open the resync SHALL change no pull request. A ref whose change files or code key the object couldn't read SHALL be read again at the next resync and SHALL put no further claim in the feed, and once GitHub no longer lists it, it SHALL be dropped as any held ref is.

#### Needs
- replica/apply-what-changed
- replica/refs-the-views-read
- web.worker.platform/object-reads
- web.worker.platform/open-pulls
- web.worker.platform/fake-github

#### Scenario: a lost webhook
- GIVEN a repo read a minute ago, and an `impl/x` created on GitHub whose webhook never arrived
- WHEN the resync runs
- THEN the next recompute's implementation view lists the claim on `x`

#### Scenario: nobody looking
- GIVEN a repo last read an hour ago
- WHEN 20 minutes pass with no event
- THEN the object made no request to GitHub

#### Scenario: what a resync can't list
- GIVEN a repo read a minute ago
- WHEN a resync runs after `impl/x` is deleted with its webhook lost; after `impl/` comes to hold 301 refs; after 151 pull requests are open, all known from webhooks; after #3 of them is closed with its webhook lost; and after 301 are open and #4 is closed with its webhook lost
- THEN `x`'s claim goes; nothing under `impl/` changes; all 151 pull requests stay open; #3 is closed while the other 150 stay open; and with 301 open, #4 stays open

#### Scenario: a code key that can't be read
- GIVEN a repo read a minute ago, and `impl/y` created by `bob` with its webhook lost, whose code key GitHub can't give
- WHEN two resyncs run, and then `impl/y` is deleted with its webhook lost and a third runs
- THEN after the second `activity` holds one claim of `y` and the implementation view lists the claim; after the third it lists no claim on `y`

### Requirement: writes-through-object <!-- base: 43885d86010e94ae -->
A write SHALL be validated against the replica after the object has brought itself up to the tip, by reading the default branch's ref and applying any push it hasn't seen, and SHALL be appended to `clm/ledger/<login>/web` with the requester's token and applied to the replica before the answer, with the ledger text the append committed, whether or not a read of that branch afterwards succeeds: a decision as a `decided` record with the entry's `capability` id, `value`, `decision` and the `settles` validation gives, answered 200 with the record; a proposal as a `proposed` record with the slug validation gives, the `title`, the `body` and the `nodes`, answered 200 `{"id"}`; a withdrawal as a `withdrawn` record naming the `proposal`, answered 200 with no body; and an answer as an `answered` record with `item` and `answer`, answered 200 `{"id"}`. A decision, a proposal or an answer SHALL recompute before the answer, so a read right after it shows the record. An answer SHALL be validated by `replica/validate-answers`: an item it doesn't know answers 404 `{"error": "unknown"}`, and an answer its item doesn't offer 422 `{"error": "answer"}`. While the replica has no views to answer, a write SHALL answer 503 `{"preparing": <share>}` as a read does, before it reads the ref, start the build when none is under way, and append nothing. A body without a string `login` and `token`, or with another `kind`, SHALL answer 422; a token GitHub answers 403 or 404 for SHALL answer 404; and when GitHub fails that check (a 401 included), the ref read, the append or a withdrawal's read of the requester's own ledger, or the object's tip isn't the ref's commit after applying the push, a write SHALL answer 502, all appending nothing more, so nothing is judged against a tip it knows to be behind; a tip that moved with files still to fetch has moved.

#### Needs
- replica/validate-answers
- replica/open-answers
- replica/validate-writes
- replica/apply-what-changed
- web.worker.platform/object-reads
- web.worker.platform/access-check
- web.worker.platform/web-ledger-append

#### Scenario: an answer read back
- GIVEN a replica whose backlog holds question `q` with choices `yes` and `no`
- WHEN an answer `yes` to `q` is written through the object, and the implementation view is read right after
- THEN the view's `answers` holds it

#### Scenario: refused
- GIVEN the same
- WHEN the answer `maybe` to `q` is written, then `yes` to `nope`
- THEN the first answers 422 `{"error": "answer"}` and the second 404 `{"error": "unknown"}`, and nothing is appended

#### Scenario: a decision shows at once
- GIVEN a replica whose capabilities view shows `x` undecided
- WHEN a decision on `x` is written, and the capabilities view is read
- THEN the view shows the decision

#### Scenario: a proposal shows at once
- GIVEN a built replica
- WHEN a proposal is written
- THEN it answers 200 with an `id`, and the implementation view read right after has `computed_at` no earlier than the write and lists the proposal

#### Scenario: the tip can't be read
- GIVEN a replica holding `x`, and a GitHub that fails the read of the default branch's ref
- WHEN a decision on `x` naming its version, a proposal and a withdrawal are written
- THEN each answers 502, and nothing is appended

#### Scenario: GitHub fails the write
- GIVEN a built replica
- WHEN a proposal and an answer are written while GitHub fails the append, a withdrawal while it fails the read of the requester's ledger, a proposal while it answers the access check 500, one while it answers it 401, and a proposal with a token it answers 404 for; and a proposal whose append succeeds
- THEN the first five answer 502 and the sixth 404, appending nothing; the last answers 200, the implementation view read right after lists the proposal, and the fake's calls hold no read of `clm/ledger/<login>/web` after the ref's move

#### Scenario: still preparing
- GIVEN an object that holds nothing
- WHEN a proposal is written
- THEN it answers 503 `{"preparing": 0}`, the alarm is set, and nothing is appended

#### Scenario: reworded since the page was drawn
- GIVEN a replica that hasn't applied a push rewording `x`, which GitHub's default branch already holds
- WHEN a decision on `x` naming its old version is written
- THEN the object applies the push, refuses the decision as stale, and appends nothing

### Requirement: push-waiting <!-- base: 8e710ff839c3143e -->
After a recompute whose tip moved, when the notice `web.live/notices` gives for the previous and new backlog items is a payload, the object SHALL post that payload, one message per recipient and at most as many as `web.live/push-send` takes in one send (10), to the repo's `Live` object's `/send`, the recipients being, for each branch `clm/ledger/<login>/web` whose `<login>` GitHub says has a permission other than `none` on the repo, that branch's web subscriptions: the live subscriptions of its records folded alone, whatever `writer` they claim, which no record of another branch ends (`web-subscriptions-by-branch`). A `subscribed` record outside such a branch SHALL receive nothing. It SHALL post no send when no VAPID key is set, there was no previous recompute, or `web.live/notices` gives none. While the object is awake, GitHub SHALL be asked a login's permission at most once in 5 minutes; and for at most 10 logins per notification, any further login counting as having none. A subscription whose endpoint a send it was sent in answered as gone SHALL receive no later message; a later `subscribed` record is a subscription of its own, whatever its endpoint.

#### Scenario: a question lands
- GIVEN a replica at `111` with no `question` items, a VAPID key, live subscriptions in `clm/ledger/alice/web` and `clm/ledger/bob/web`, and GitHub answering `read` for `alice` and `none` for `bob`
- WHEN a push moving `main` to `222`, whose backlog adds the `question` item "Which region?", is applied and the alarm fires
- THEN one send was posted to the object named `o/a`, holding one message, for `alice`'s subscription, whose payload's body is "Which region?"

#### Scenario: nothing new waits
- GIVEN the same, with `222` adding only an `idea` item
- WHEN the push is applied and the alarm fires
- THEN no send was posted

#### Scenario: a claimed writer in another branch
- GIVEN the same as *a question lands*, with `alice`'s subscription `s1` in her branch, a `subscribed` record with writer `alice/web` and another endpoint in `clm/ledger/mallory/web`, an `unsubscribed` record naming `s1` in that branch too, and GitHub answering `none` for `mallory`
- WHEN the push is applied and the alarm fires
- THEN one send was posted, holding one message, for `s1`'s endpoint only

#### Scenario: nothing to send
- GIVEN the same as *a question lands*, with no VAPID key set; and, in a second run, with a VAPID key and no recompute before `222`'s
- WHEN the push is applied and the alarm fires
- THEN no send was posted in either run

#### Scenario: at most ten
- GIVEN the same as *a question lands*, with 12 live subscriptions in `alice`'s branch
- WHEN the push is applied and the alarm fires
- THEN one send was posted, holding 10 messages

#### Scenario: permissions remembered
- GIVEN an awake object, two notifications within 5 minutes, and 11 web ledger branches each holding a live subscription
- WHEN each is sent
- THEN GitHub was asked each of the first 10 logins' permission once, and the 11th branch's subscription received nothing

#### Scenario: a gone endpoint
- GIVEN the same as *a question lands*, with `alice` holding subscriptions `s1` and `s2`, and a `/send` that answers `{"gone": [<s1's endpoint>]}`
- WHEN two pushes each adding a `question` item are applied, the alarm firing after each, and `alice` then subscribes again as `s3` with `s1`'s endpoint and a third such push is applied
- THEN the first send held messages for `s1` and `s2`, the second for `s2` only, and the third for `s2` and `s3`

#### Needs
- ledger/subscription-records
- web.live/push-send
- web.live/notices

### Requirement: push-review <!-- base: 0b5b98e15d5d797e -->
On a `pull_request` event for which `web.live/notices` gives a review notice, the object SHALL post it to the repo's `Live` object's `/send` for the recipients and within the limits `push-waiting` gives, and SHALL post no send when it gives none or no VAPID key is set.

#### Scenario: opened for review
- GIVEN a VAPID key, a live subscription by `alice/web`, and GitHub answering `write` for `alice` on `o/a`
- WHEN a `pull_request` event with action `opened` for head `spec/x` of `o/a`, titled "Add x", is applied
- THEN one send was posted holding one message for `alice`'s subscription, whose payload's body is "spec/x: Add x"

#### Scenario: not for review
- GIVEN the same
- WHEN the event's action is `closed`, and, in a second run, the event of *opened for review* with no VAPID key set
- THEN no send was posted either time

#### Needs
- ledger/subscription-records
- web.live/push-send
- web.live/notices

### Requirement: uninstall-deletes <!-- base: b0ac190fcfdc7acb -->
An `installation_repositories` event with action `removed`, or an `installation` event with action `deleted`, SHALL delete the storage of each named repo's object, and the object SHALL then answer reads as one that holds nothing.

#### Scenario: a repo removed
- GIVEN objects holding `o/a` and `o/b`
- WHEN an `installation_repositories` event removes `o/a`
- THEN `o/a`'s object holds nothing and `o/b`'s is unchanged

#### Scenario: an installation deleted
- GIVEN an object holding `o/a`
- WHEN an `installation` event with action `deleted` naming `o/a` is applied, and, in a second run, one with action `suspend` naming it
- THEN after the first the object holds nothing and a read answers 503 `{"preparing": 0}`; after the second it holds what it held

#### Serves
- architecture/git-is-truth

### Requirement: object-class <!-- base: 2ce37c075e02cbfc -->
`clm-web-replica` SHALL build for `wasm32-unknown-unknown` on its own, and its Durable Object class `Replica` SHALL answer `POST /event`, `GET /view/<name>` and `POST /write` and run the alarm over the object's own SQLite storage. While it is awake, the object SHALL mint one installation token for every request and alarm in the 55 minutes after minting it.

#### Scenario: the class builds
- GIVEN the workspace
- WHEN `cargo build -p clm-web-replica --target wasm32-unknown-unknown` runs
- THEN it succeeds

#### Scenario: a token minted once
- GIVEN an object that holds nothing, over a repo whose first build takes several alarms
- WHEN the alarms run until it re-arms no more
- THEN GitHub minted one installation token

#### Needs
- web.worker.platform/object-reads

#### Serves
- architecture/git-is-truth

### Requirement: times-as-instants <!-- base: 5331269f9780ff65 -->
The times the object gives the replica, a ref's tip commit time from a push, a resync or a first build, a push's head commit time and an event's time, SHALL be the instants GitHub's RFC 3339 times name, their offsets counted, so a claim lapses and the activity feed orders by the instant a commit was made, whatever offset its committer's clock had. A push whose head commit time is missing or can't be read SHALL be recorded at the time it is applied; an event of GitHub's events listing whose time can't be read SHALL be left out of the feed; and a listed ref whose commit time can't be read SHALL still be held, at the time it is listed.

#### Scenario: a commit made two hours east
- GIVEN a built replica, and a push creating `impl/a` whose head commit's `timestamp` is `2026-10-01T12:00:00.5+02:00`; and a second built replica whose resync's refs listing gives `impl/b`, which it doesn't hold, a `committedDate` of `2026-10-01T12:00:00+02:00`
- WHEN the push is applied to the first and the resync runs on the second, and the replica each object saved is loaded and recomputed at `2026-10-01T11:00:00Z`
- THEN the first's `activity` holds the claim of `a` at 10:00 UTC, and each `recompute` returns 14:00 UTC as the time the implementation view next changes

#### Scenario: a time GitHub gives that can't be read
- GIVEN a built replica read a minute ago, holding `impl/b`, and an object that holds nothing
- WHEN a push creating `impl/a` whose head commit's `timestamp` is `yesterday` is applied at 10:00; the object that holds nothing makes its first build over an events listing holding a push whose `created_at` is `soon`; and a resync's refs listing gives `impl/b` a `committedDate` of `soon`
- THEN the claim of `a` is at 10:00; the feed holds nothing for that push; and `impl/b` is still held

#### Needs
- ledger/record-times

### Requirement: fetches-what-the-replica-wants <!-- base: bb23e1f0b8dfeca6 -->
Of the files a compare or a tree listing names, the object SHALL fetch only those the replica reads (`replica/files-it-fetches`): every path that doesn't end `.md`, and the Markdown files whose text the replica keeps; and SHALL give each to the replica at the mode GitHub's tree gives it, so the replica holds no symlink or submodule as a file; GraphQL gives a mode as an integer, and the object SHALL give the replica its octal text, as a tree listing writes it. In a first build or a recovery it SHALL fetch those whose text the replica keeps before any other, so every file `pending_fetch` lists is fetched while the budget lasts.

#### Scenario: a push listing every kind of file
- GIVEN a replica at `111`, and a push to `222` whose compare lists `web/.gitattributes`, `clm.toml`, `docs/decisions/d.md`, `src/b.rs`, `README.md` and `docs/guide.md` as modified
- WHEN the event is applied
- THEN the files fetched are `web/.gitattributes`, `clm.toml`, `docs/decisions/d.md` and `src/b.rs` at `222`, and neither `README.md` nor `docs/guide.md`

#### Scenario: a first build and a recovery
- GIVEN a repo of spec files and non-Markdown files more than one query holds; and a built replica whose push is a force-push
- WHEN the first build runs, and the force-push is applied
- THEN each time the first blob query asks only for files whose text the replica keeps, every wanted file is fetched, and none other is fetched before the last of those

#### Scenario: a link changed by a push
- GIVEN a replica at `111` holding `src/a.rs`, and a push to `222` whose compare lists `src/a.rs`, now a symlink (mode `120000`) to `src/b.rs`, and `src/b.rs` modified
- WHEN the event is applied and the views are recomputed
- THEN GitHub's GraphQL gave `src/a.rs`'s mode as 40960 and the replica was given it at `120000`; `src/b.rs` is scanned, `src/a.rs` is neither held nor waited for, and coverage is complete

#### Needs
- web.worker.platform/fake-github

## ADDED Requirements

### Requirement: answers-views
`GET /view/<name>` SHALL answer 200 `{"sha", "computed_at", "view"}` for a view the replica holds, `sha` the commit it describes, `computed_at` the Unix seconds of its recompute and `view` the view `replica`'s `view` gives; 503 `{"preparing": <share>}`, setting the alarm to now when none is set, while the replica is preparing it; and, once the object holds a tip, 404 with no body for a name that is no view's; an object that holds nothing SHALL answer 503 `{"preparing": 0}` for any name. With `fresh=1` it SHALL first read the default branch's ref, apply a push it hasn't seen and recompute if anything is stale; when GitHub fails that read, it SHALL answer the view it holds.

#### Needs
- replica/preparing

#### Scenario: a view read
- GIVEN a built replica; a replica whose spec files are half fetched; an object that holds nothing; and a name `nope`
- WHEN `snapshot` is read from the first, `snapshot` from the second, and `nope` from the first and the third
- THEN the first answers 200 with `sha` the replica's tip, `computed_at` its last recompute and `view` its snapshot; the second 503 `{"preparing": 0.5}` with the alarm set; `nope` 404 with no body from the first and 503 `{"preparing": 0}` from the third

#### Scenario: a fresh read
- GIVEN a replica that hasn't applied a push GitHub's default branch holds
- WHEN the snapshot is read with `fresh=1`, then again with `fresh=1` while GitHub fails the ref read
- THEN the first answer describes the new tip, and the second answers 200 with the view it held

### Requirement: write-refusals
The object SHALL refuse a write, appending nothing, with:
- a decision: 422 with no body for a body without a string `id`, `version`, `value` and `decision`, a `value` not `high`, `medium` or `low`, or a `decision` not `keep`, `cut`, `redesign` or `accept`; then, as `replica/validate-writes` judges it, 422 with no body for a tip whose specs don't load or an entry the tip's `CAPABILITIES.md` or `ARCHITECTURE.md` doesn't hold; 422 `{"error": "signed", "command": "clm <layer> decide <slug> --value <value> --decision <decision>"}`, `<layer>` `architecture` for an `architecture/<slug>` id and `capabilities` otherwise, for a tip that holds `docs/allowed_signers`; and 409 with no body when `version` isn't the entry's version now; in that order;
- a proposal: 422 with no body for a body without a string `title` and `body`; 422 `{"error": "nodes"}` for one `replica/validate-writes` refuses for its components or its slug; 409 `{"error": "taken"}` for one whose slug is taken; and 422 with no body for a tip whose specs don't load;
- a withdrawal: 422 with no body for a body without a string `id`, and 404 with no body for an `id` the requester's own web ledger, read at its tip, holds no open `proposed` record of.

#### Needs
- replica/validate-writes
- ledger/proposal-records

#### Scenario: signed
- GIVEN a default branch whose tip holds `docs/allowed_signers`, a capability `x` and an architectural constraint `e`, each named with a stale `version`
- WHEN a decision on `capability/x` with `value` `high` and `decision` `keep` is written, and one on `architecture/e` with the same
- THEN they answer 422, not 409, with `error` `signed` and `command` `clm capabilities decide x --value high --decision keep` and `clm architecture decide e --value high --decision keep`, and nothing is appended

#### Scenario: reworded since
- GIVEN a replica holding capability `x` at version `v2`
- WHEN a decision on `x` naming `v1` is written
- THEN it answers 409, and nothing is appended

#### Scenario: unknown capability
- GIVEN a replica that doesn't hold `capability/nope`, and, in a second run, a replica whose `specs/c/SPEC.md` doesn't parse
- WHEN a decision on `nope` is written, then one on `x` in the second run, and a decision on `x` with `value` `huge`
- THEN each answers 422 with no body, and nothing is appended

#### Scenario: not an entry
- GIVEN a replica holding architectural constraint `e` at `v2`
- WHEN a decision on `architecture/nope` is written, then one on `e` naming `v1`
- THEN the first answers 422 and the second 409, and nothing is appended

#### Scenario: refused
- GIVEN a replica whose backlog holds `export-csv` and whose graph holds component `a`
- WHEN a proposal titled "Export CSV" is written, then "Dark mode" naming component `nope`
- THEN the first answers 409 `{"error": "taken"}` and the second 422 `{"error": "nodes"}`, and nothing is appended

#### Scenario: taken by a change
- GIVEN a replica whose `resolved` holds `export-csv`, and, in a second run, whose fold holds an open proposal with slug `export-csv`
- WHEN a proposal titled "Export CSV" is written
- THEN it answers 409 `{"error": "taken"}` both times, and nothing is appended

#### Scenario: withdrawn
- GIVEN `clm/ledger/alice/web` holding `proposed` `p1` and `clm/ledger/bob/web` holding `proposed` `p2`
- WHEN `alice` withdraws `p2`, then `p1`, then `p1` again
- THEN the first answers 404 appending nothing, the second appends `withdrawn` naming `p1` to her ledger and answers 200, and the third answers 404 appending nothing

### Requirement: fetches-binary-assets
A binary file whose hash the replica keeps (`replica/asset-hashes`) SHALL be fetched whole through GitHub's blob API, one subrequest each within the budget, and given to the replica with its bytes; a blob the budget can't afford SHALL stay to fetch. A blob the blob API fails to give, or whose tree size is over 10 MiB, SHALL be given as a binary file without its bytes, so it is held, left out of `assets`, and holds no view back. An object whose storage was written by an object that didn't hash binary assets SHALL, on waking, drop what it holds and build again as an object that holds nothing.

#### Needs
- replica/asset-hashes
- web.worker.platform/fake-github

#### Scenario: a logo
- GIVEN a repo holding `specs/b/assets/logo.png` and `specs/b/assets/big.png`, binary files, the second of 11 MiB, `specs/b/assets/lost.png`, whose blob the blob API fails, and `src/blob.bin`, another binary file; and a second object whose storage names no version, holding the same repo
- WHEN the first build runs on an object that holds nothing, then the snapshot is read; and the second object is read, then read again after its alarms
- THEN the blob API was asked for `logo.png` and `lost.png` only; the snapshot's `b` lists `logo.png` in `assets` with the hash `clm` gives and neither `big.png` nor `lost.png`, and the snapshot is ready; the second object first answers 503 `{"preparing": 0}`, then the same snapshot
