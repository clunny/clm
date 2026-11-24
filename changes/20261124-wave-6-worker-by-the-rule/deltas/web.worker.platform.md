---
component: web.worker.platform
base: { frontmatter: "7b3ebf43c5388bd4", contract: "c83211e183b9b0dd", design: "727d65e766e7ff22" }
---

## MODIFIED Frontmatter
```yaml
depends_on: [model, orchestrator, ledger, gate, replica]
```

## MODIFIED Contract
Crate `clm-web-platform`, the layer the Worker's three crates stand on: `clm-web` (`web.worker`), `clm-web-review` (`web.worker.review`) and `clm-web-replica` (`web.worker.replica`). Built for `wasm32-unknown-unknown` and natively for tests, with no `worker` dependency: `clm-web`'s adapter implements its traits, and `clm-web-replica`'s object the parts it uses. It offers:
- `platform`: a platform's parts as traits, `Assets` (`fetch_asset`, and `asset` over it, `asset-responses`), `Cache`, `GitHub` (`send`, a `GhRequest` for a `GhResponse`), `Clock`, `Secrets`, `Live` and `Log`, and `Platform`, which every type offering all seven is (`platform-parts`); and `Auth`, who a GitHub call is made as, `User`, `App`, `Installation` or `None`. `clm-web` and `clm-web-replica` match `Auth` exhaustively and build `GhResponse` by struct literal, so a new variant or field is a contract change;
- `http`: `Request` (`method`, `path`, `query`, `headers`, `body`, `origin`, built by struct literal in three crates; `header`, `cookie`, `param`) and `Response` (`status`, `headers`, `body`, `upgrade`; `empty`, `json`, `html`, `redirect`, `with_header`, `header`), with `Upgrade`, a 101's socket only the platform makes and reads, and `http::percent_encode` and `http::percent_decode` (`percent-coding`);
- `access`: `Access`, which only a check makes (`owner`, `repo`, `full_name`, `auth`, `default_branch`, and `fresh`, whose rule `web.worker/fresh-live-state` states); `access::check` and `access::checked_direct`, with the name rule `access::valid_name` and `access::token_hash` (`access-check`); and `access::permission` and `access::permission_direct` (`permission`);
- `gh`: the GitHub client, `gh::API`, `gh::get`, `gh::get_ok`, `gh::post`, `gh::patch` and `gh::graphql`, and `gh::Failed`, a read GitHub didn't answer as asked;
- `files`: `files::listing`, `files::objects` and `files::objects_direct`, with `files::PER_QUERY` (`batched-fetches`); `files::blobs` and `TreeEntry`, a listing's blobs; and `files::spec_files` (`spec-files`);
- `keys`: `Listing` (`entries`, `truncated`; `code_key`, `entry`, `has`, `changed_paths`), `keys::listing_of`, `TreeLine` (built by struct literal in two crates' tests), `keys::code_key` (`code-key-doc-only`) and `keys::fingerprint` (`fingerprint-from-listings`);
- `install`: `install::installation_id`, `install::token` and `install::token_direct` (`installation-token`);
- `live`: `live::cached` (`live-state-cache`, `fresh-replaces-plain`), and `live::pulls_direct` and `live::pulls` with `Pull`, a repo's open pull requests, read in pages, the second kept with the live state (`open-pulls`);
- `ledger`: `ledger::read`, `ledger::sources` and `Sources`, every ledger branch's `records.jsonl`; `ledger::fold`, `ledger::web_folds` and `ledger::web_fold`, their folds by `clm_ledger`; `ledger::own`, the fold of a login's web ledger read at its tip, empty when GitHub says the branch isn't there; `ledger::branch_tip` and `ledger::tip` (`branch-tip`); and `ledger::web_writer`, `ledger::record`, `ledger::append` and `ledger::appended`, giving an `Appended { tip, text }` (`web-ledger-append`);
- `fake`, behind the cargo feature `fake`: the fake GitHub (`FakeGitHub`, `FakeRepo`, `FakePull`) and platform (`Fake`) the three crates' tests drive, each turning it on in its dev-dependencies. `FakePull` is built by struct literal outside this crate, and two crates import `fake::*` whole, so a field added to `FakePull` or a name added to `fake` is a contract change; `FakeRepo` and `Fake` are built only through their defaults. What the fake GitHub answers to the queries the crates make is stated where their tests need it (`fake-github`).

Withdrawn once their users have moved (`old-paths-removed`): `platform::Kv` and the fake's KV, `access::check_direct`, `files::is_spec_path` and `install::unix_time`.

It answers no route of its own, and decides nothing a route answers.

## MODIFIED Design
**The fake** (`nested-splits`). The fake GitHub and platform live in `src/fake.rs` behind the `fake` feature. The crates only the fake uses (`futures`' executor and the like) are optional dependencies enabled by that feature alone, so the Worker's wasm build compiles none of them. The fake calls nothing of `clm-web` and writes no checkout. Its knobs are what the three crates' tests set: a repo's trees, refs, pulls and events, a mode per path (`120000` for a symlink), and the reads and writes a test makes fail: a branch's ref lookup (`ref_fails`), a repo's or a login's permission lookup, a ref's creation or move, a token's mint, a blob's read (`blob_fails`), any REST path a test names (`fail_paths`), and a GraphQL query answered with a null `repository`. It answers the GraphQL queries as GitHub does where the crates' tests depend on it: pages of pull requests with their `pageInfo`, and a directory's entries with their modes in the blob query (`fake-github`). `src/fake.rs` counts toward this component's code: `check/node-size` reads the files under its `paths`, whatever their features.

**One way down.** Imports run one way, as `web.worker`'s Design says: what a module here needs from a route comes in as an argument. Code two of the three crates need goes here; code only one needs stays in it.

**Caches and endpoints.** Each cached call keeps its answer in the Cache API under a key that names the repo and never a token: `access:<token hash>:<o>/<r>` and `permission2:<repo>:<login>` for 5 minutes, `installation-token:<id>` until 5 minutes before it expires, `live:<o>/<r>:<what>` for 60 seconds, and `blob:<repo>:<sha>` and `files3:<repo>@<sha>` for a day, a commit never changing. The `files3` key is new because `spec-files` widened what a commit's spec files are, so no answer kept under the old set is read.

**A fresh read replaces the plain entry** (`fresh-replaces-plain`). Storing what a `fresh` read fetches under the plain key too, with two `cache_put` calls and no new `Platform` method, means the next read without `fresh` in that data centre (the 60-second refresh, a navigation, another tab) never goes back to the minute-old value, so an approval the app's reload showed doesn't vanish from a review's header. Two races are accepted: two fetches that finish together may leave the older value under the plain key for up to 60 seconds, as two tabs fetching at once may; and the Cache API is per data centre, so a reader in another data centre keeps its own minute-old copy until it expires or a notice's refresh replaces it.

**Assets as responses** (`asset-responses`). `fetch_asset` takes the whole request and returns the whole answer, so a platform answers as its `ASSETS` binding does (`web.worker/asset-adapter`); today the Worker calls it only through `asset`, for `/_clm/invited.json`. The fake answers a path it holds the same whatever the method.

**Reads without the cache** (`object-reads`). The `Replica` object (`web.worker.replica`) never reaches the Cache API (`web.worker/cache-is-disposable`), hence the uncached forms. Each cached form is built on its uncached one and keeps what it answers, so the two can't differ.

**A read that failed is not an absence** (`branch-tip`, `access-check`). Where GitHub's 404 means "not there", a call that its callers answer differently for the two gives `gh::Failed` for any other answer, so a route answers 502 rather than 404 or a refusal when GitHub didn't answer. `ledger::tip`, which folds the two, is kept for callers that tell them apart nowhere, as a test helper that only needs a tip.

**Shared rules come from their owners.** The files a commit's spec files are is the replica's rule and the gate's (`clm_replica::keeps_text`, `clm_gate::is_spec_path`); the code key's doc-only paths are `ledger`'s; record times, written and read, are `ledger`'s (`ledger/record-times`); percent coding is the `percent-encoding` crate's, as in `web.live`. This crate holds no copy of any of them. One rule is its own: which binary files `spec_files` fetches whole, those in an `assets/` directory under `specs/` or `changes/` (`files::is_asset`), since `replica`'s byte rule covers `specs/` alone.

**Withdrawn paths** (`expand-contract`). KV, the uncached repo check that folds a failure into a refusal, the model-files rule and the RFC 3339 parse go in steps, each land leaving `main` green: this crate's first build offers the new calls and takes KV out of `Platform`'s parts (`platform-parts`), keeping `platform::Kv`, the fake's KV, `access::check_direct`, `files::is_spec_path` and `install::unix_time` (by then calling `clm_ledger`'s parse) marked `clm:remove-by` this change; `web.worker`'s build drops its KV adapter, `web.worker.replica`'s moves its write check and its times, and `web.worker.review`'s moves its spec-path filter; then this crate's next build removes them (`old-paths-removed`), whose Needs order it after the last user's move. Once built, `old-paths-removed` states only an absence and is a cut to propose, as `land.approval/old-calls-gone` was.

## ADDED Requirements

### Requirement: platform-parts
A type that offers `Assets`, `Cache`, `GitHub`, `Clock`, `Secrets`, `Live` and `Log` SHALL be a `Platform`, and every call of this component that takes a `Platform` SHALL run on it: no platform SHALL have to offer a KV store.

#### Scenario: a platform with no KV
- GIVEN a test platform over the fake GitHub that offers those seven parts and no KV, its Cache API a map of its own
- WHEN `access::check` for `o/a` and `live::cached` for one item each run on it twice
- THEN the test compiles, each first call gives what the same call gives over the fake platform, and neither second call asks GitHub anything

#### Serves
- architecture/expand-contract
- architecture/no-dead-code

### Requirement: old-paths-removed
Once `web.worker`, `web.worker.replica` and `web.worker.review` have moved off them, this component SHALL offer none of the paths its newer calls replace: no `platform::Kv` and no KV store in the fake platform (`platform-parts`); no `access::check_direct`, which gives none alike for a refusal and a failed read (`access-check`); no `files::is_spec_path`, a copy of the model's half of the replica's rule (`spec-files`); and no `install::unix_time` (`ledger/record-times`). No `clm:remove-by` marker naming the change that adds this requirement SHALL be left in this component's files.

#### Scenario: the old paths are gone
- GIVEN this component's source files once the users' moves are built
- WHEN a test reads each of them
- THEN none declares a public `Kv` trait, a `kv` or `kv_writes` field, or a public `check_direct`, `is_spec_path` or `unix_time` function, and none holds a `clm:remove-by` marker naming the change

#### Needs
- web.worker/cache-is-disposable
- web.worker.replica/writes-through-object
- web.worker.replica/times-as-instants
- web.worker.review/pr-review

#### Serves
- architecture/expand-contract
- architecture/no-dead-code

### Requirement: branch-tip
`ledger::branch_tip(access, branch)` SHALL ask GitHub once for `GET /repos/{o}/{r}/git/ref/heads/{branch}`, with the access's token, and give the commit the ref names when GitHub answers 200, that the branch is missing when it answers 404, and `gh::Failed` for any other answer. `ledger::tip(access, branch)` SHALL give that commit, and none for a missing branch and a failed read alike.

#### Scenario: a tip, a missing branch and a failed read
- GIVEN the fake GitHub with `o/a`'s `spec/x` at `111`, no `spec/y`, and `spec/z` whose ref lookup fails with a 502
- WHEN `ledger::branch_tip` and `ledger::tip` read each of the three
- THEN `branch_tip` gives `111`, missing, and a failed read; `tip` gives `111`, none and none; and each call asked GitHub once

#### Serves
- capability/approve-intent
- architecture/git-is-truth

### Requirement: web-ledger-append
`ledger::record(login, body)` SHALL give a record of `body` by the writer `<login>/web` (`ledger::web_writer`), with an `id` of 32 lowercase hex digits from the platform's randomness and an `at` of the platform's now as `ledger/record-times` writes a time with no milliseconds. `ledger::append(access, record)` SHALL commit the record as the last line of `records.jsonl` on `clm/ledger/<writer>`, with the access's token and exactly four Git Data API writes: a blob, a tree holding that file alone, a commit whose parent is the branch's tip, or with no parent when `branch-tip` finds the branch missing, and the ref, created for a missing branch and otherwise moved with `force` false. A last line without its newline SHALL end before the new one. It SHALL give none and write nothing when the branch's tip is a failed read or its `records.jsonl` can't be read, and SHALL give none when GitHub refuses a write or the ref's creation or move. `ledger::appended(access, record)` SHALL make the same append and give, when it succeeds, the commit the ref now names and the whole `records.jsonl` text that commit holds, so a caller applies what it wrote without reading it again; `ledger::append` SHALL give unit exactly when `ledger::appended` gives an answer.

#### Scenario: the append
- GIVEN the fake GitHub, a repo whose `clm/ledger/alice/web` holds one record, and an `Access` to it
- WHEN `ledger::append` commits a record by `alice/web`
- THEN the fake saw exactly four writes, a blob, a tree, a commit whose parent is the old tip, and a ref update with `force` false, and the branch's `records.jsonl` holds the old line then the record's

#### Scenario: a first, an unended and a refused append
- GIVEN the fake GitHub and, in turn, a repo with no `clm/ledger/alice/web`; one whose `records.jsonl` there ends without a newline; one whose ref lookup for it fails with a 502; one whose `Ledgers` query GitHub answers with a null `repository`; and one whose ref GitHub refuses to move
- WHEN `ledger::append` commits a record by `alice/web` to each
- THEN the first creates the branch with one commit with no parent whose file holds the record's line alone; the second's file holds the old line, a newline and the record's line; the third and fourth give none with no write made; and the fifth gives none and leaves the ref where it was
- AND the record's `at` is `2027-01-15T08:00:00.000Z` for the fake's now, and its `id` 32 lowercase hex digits
- AND `ledger::appended`, making the first and second appends, gives the new tip and the file text the branch then holds at it, and none for the other three

#### Needs
- ledger/record-times

#### Serves
- architecture/git-is-truth
- architecture/reuse-first

### Requirement: access-check
`access::check(token, owner, repo)` SHALL give an `Access` when GitHub answers `GET /repos/{owner}/{repo}`, asked with the token, with 200 and a `full_name` whose owner and repo the name rule accepts; the `Access` SHALL carry that owner and repo as GitHub writes them, whatever casing was asked, the token, and GitHub's `default_branch`. It SHALL give none otherwise, asking GitHub nothing for an owner or repo the name rule refuses: `access::valid_name` SHALL accept a name of ASCII letters, digits, `-`, `_` and `.` that is not empty, `.` or `..`, and nothing else. `access::check` SHALL keep GitHub's answer to a 200, 403 or 404 for 5 minutes under a key holding `access::token_hash` of the token, its SHA-256 as 64 lowercase hex digits, and never the token, and SHALL keep no other answer. `access::checked_direct(token, owner, repo)` SHALL ask GitHub every time and keep nothing, giving the `Access` as `access::check` does, nothing for a name the rule refuses or a 403 or 404, and `gh::Failed` for any other answer, a 401 included, or a 200 whose `full_name` the rule refuses.

#### Scenario: who may read a repo
- GIVEN the fake GitHub, where token `u` reads `O/A`, whose default branch is `trunk`, and `q/c`'s lookup fails with a 502
- WHEN `access::check` and `access::checked_direct` are each asked, with `u`, for `o/a`, `r/none` (a 404), `o/..`, `o/a b`, an empty owner and `q/c`, and `access::check` for `o/a` and `q/c` again, the second time 4 minutes later and once more at 5 minutes
- THEN `o/a` gives an `Access` named `O/A` with default branch `trunk`; `r/none`, `o/..`, `o/a b` and the empty owner give none from both, the last three asking GitHub nothing; `q/c` gives none from `access::check` and a failed read from `access::checked_direct`; `access::check`'s second `o/a` asks nothing and its third asks again, while `q/c` is asked each time; and no Cache API key holds `u`

#### Serves
- architecture/invite-only-web
- architecture/git-is-truth

### Requirement: permission
`access::permission(auth, repo, login)` SHALL give the `permission` GitHub's `GET /repos/{repo}/collaborators/{login}/permission` answers with 200, asked as `auth`; `none` when it answers 404; and nothing for any other answer. It SHALL keep an answer for 5 minutes per repo and login and keep no failure. `access::permission_direct` SHALL give the same and keep nothing.

#### Scenario: a login's permission
- GIVEN the fake GitHub, where `alice` has `write` on `o/a`, `bob` is no collaborator, and the permission lookup for `carol` fails with a 502
- WHEN `access::permission` is asked for each, then for each again 4 minutes later and once more at 5 minutes, and `access::permission_direct` for each twice
- THEN `alice` gives `write`, `bob` `none` and `carol` nothing from both; `access::permission`'s second round asks GitHub only for `carol` and its third asks for all three; and `access::permission_direct` asks every time

#### Serves
- capability/approve-intent

### Requirement: installation-token
`install::installation_id(repo)` SHALL give the `id` GitHub's `GET /repos/{repo}/installation` answers with 200, and none otherwise. `install::token(id)` and `install::token_direct(id)` SHALL give the `token` GitHub's `POST /app/installations/{id}/access_tokens` answers with 200 or 201. Each of these SHALL ask as the App, by a JWT signed RS256 by the platform whose claims are `iss` the secret `GITHUB_APP_ID`, `iat` a minute before now and `exp` 9 minutes after it, and SHALL give none, asking GitHub nothing, when `GITHUB_APP_ID` is unset or the platform can't sign, and none when GitHub answers otherwise or names no `token`. `install::token` SHALL keep the token in the Cache API until 5 minutes before the `expires_at` GitHub gives, read as `ledger/record-times` reads a time, and keep nothing when `expires_at` is missing, unreadable or 5 minutes away or less; `install::token_direct` SHALL keep nothing.

#### Scenario: the Worker's form still caches
- GIVEN the fake platform
- WHEN `install::token` is called twice for one installation
- THEN the fake GitHub saw one token request

#### Scenario: a token near its expiry
- GIVEN the fake GitHub minting tokens, in turn, with `expires_at` an hour away, 4 minutes away, missing, and `soon`; then answering 401; then a platform with no `GITHUB_APP_ID`
- WHEN `install::token` is called twice for one installation with each, and `install::installation_id` for a repo the App is installed on and one it isn't
- THEN the first mints once and the next three twice; the 401 and the missing App ID give none, the last asking GitHub nothing; every JWT sent has `iss` `1`, `iat` now less 60 and `exp` now plus 540; and `installation_id` gives the installation's `id`, then none

#### Needs
- ledger/record-times

#### Serves
- capability/follow-from-browser

### Requirement: live-state-cache
`live::cached(access, what, fetch)` SHALL answer from the Cache API's `live:<o>/<r>:<what>`, or the delivery's key for an `Access` carrying `fresh`, while it holds a value that reads as the item, for 60 seconds from when it was stored. Otherwise it SHALL call `fetch` and store what it gives under that key; a fetch that gives nothing SHALL be stored nowhere and answer nothing, and a stored value that doesn't read as the item SHALL be fetched anew.

#### Scenario: a minute of live state
- GIVEN the fake platform and an `Access` to `o/a` without `fresh`
- WHEN a number is read through `live::cached` and read again 59 seconds later and then at 60 seconds; then an item whose fetch gives nothing is read twice; then an item whose key holds `"x"` where a number is meant is read
- THEN the read at 59 seconds calls no fetch and the one at 60 seconds does; the failing item is fetched both times and nothing is stored for it; and the unreadable value is fetched anew and replaced

#### Serves
- capability/follow-from-browser

### Requirement: batched-fetches
`files::listing(auth, repo, sha)` SHALL give the commit's tree as GitHub's `GET /repos/{repo}/git/trees/{sha}?recursive=1` lists it with 200, with its `truncated` flag, and none for any other answer. `files::objects(auth, repo, query, expressions)` and `files::objects_direct` SHALL give the text of each `<rev>:<path>` expression, in order, with none for a binary blob or one the commit lacks. They SHALL ask GitHub's GraphQL at most 100 expressions per query (`files::PER_QUERY`), read a text GraphQL cuts short whole from the blob API, `files::objects` keeping that blob a day, and give none at all when a query fails or GitHub answers a null `repository`.

#### Scenario: many files at once
- GIVEN the fake GitHub with a commit holding 150 text files, a binary file and a 400-record ledger longer than GraphQL's text limit, and a query name GitHub answers with a null `repository`
- WHEN `files::objects_direct` reads the 150 files, the binary file, a path the commit lacks and the ledger, and then the same expressions under the failing query name
- THEN the first read made two GraphQL queries and one blob API call and gives the 150 texts in order, none, none and the ledger's 400 lines whole; and the second gives none

#### Serves
- architecture/git-is-truth

### Requirement: spec-files
`files::spec_files(auth, repo, sha, tree)` SHALL give every file of the commit `sha`, its blobs as `tree` lists them or as `files::listing` lists them when `tree` is none, whose path `replica/files-it-fetches`' `keeps_text` or `gate/classify`'s `is_spec_path` accepts: each text file with its text from GitHub's GraphQL, and each binary file in an `assets/` directory under `specs/` or `changes/`, at any depth, with its bytes from the blob API. A binary file elsewhere SHALL be left out. It SHALL fetch at most 20 such blobs in one call and give none when the commit needs more, keeping each blob it fetched a day so a later call goes on from there, and SHALL give none when a fetch fails. It SHALL keep a complete answer a day by repo and commit.

#### Scenario: a commit's spec files
- GIVEN the fake GitHub with a commit holding `specs/a/SPEC.md`, `changes/c/change.md`, `docs/guide/start.md`, `CLAUDE.md`, `clm.toml`, `web/.gitattributes`, `src/lib.rs`, `README.md`, a binary `specs/a/assets/logo.png` and a binary `docs/img.png`; then a commit holding 21 binary assets under `specs/a/assets/`
- WHEN `files::spec_files` is called for the first commit twice, and for the second three times
- THEN the first call gives the first six files with their texts and `logo.png` with exactly its bytes, and nothing else, and the second asks GitHub nothing; for the second commit the first call gives none, the second gives all 21 with their bytes, and the third asks GitHub nothing

#### Needs
- replica/files-it-fetches
- gate/classify

#### Serves
- capability/approve-intent
- architecture/reuse-first

### Requirement: fingerprint-from-listings
`keys::fingerprint(base, head)` SHALL give, for the Trees API listings of two commits, what `clm fingerprint` prints for `head` against `base` in a repository whose CLM root is the top of its git repository (`gate.fingerprint/fingerprint`), whatever that repository's git settings for renames and abbreviation, and none when either listing is truncated.

#### Scenario: same as the CLI
- GIVEN a repository whose CLM root is the top of its git repository, whose spec branch renames a file with a one-line edit, changes a binary file and adds a file holding Latin-1 bytes, and whose config sets `core.abbrev=16` and `diff.renames=copies`
- WHEN `keys::fingerprint` runs on that repository's two tree listings, as the Trees API gives them
- THEN it equals what `clm fingerprint --base main` prints there

#### Scenario: either listing truncated
- GIVEN two listings of that repository, and each marked truncated in turn
- WHEN `keys::fingerprint` runs on each pair
- THEN it gives a fingerprint when neither is truncated and none when either is

#### Needs
- gate.fingerprint/fingerprint

#### Serves
- capability/approve-intent
- architecture/hash-based-staleness

### Requirement: percent-coding
`http::percent_encode(text)` SHALL keep ASCII letters, digits, `-`, `_`, `.` and `~` and write every other byte of the text's UTF-8 as `%` and two upper-case hex digits. `http::percent_decode(text)` SHALL turn each `%` followed by two hex digits into that byte, keep a `%` not followed by two hex digits as it is, keep `+` as it is, and give U+FFFD for bytes that aren't UTF-8. `Request::param(name)` SHALL give the first value of the query parameter `name`, with each `+` read as a space before it is decoded, and none when the query holds no `name=`.

#### Scenario: percent coding
- GIVEN the texts `a b/é`, `100%`, `%2`, `%zz`, `%ff`, `spec/a+b` and `spec%2Fa%2Bb`, and a request whose query is `q=a+b%2B&q=c&r`
- WHEN each text is encoded or decoded, and `param` reads `q` and `r`
- THEN `a b/é` encodes to `a%20b%2F%C3%A9`; `100%`, `%2` and `%zz` decode as written, `%ff` to U+FFFD, and `spec/a+b` and `spec%2Fa%2Bb` both to `spec/a+b`; `param("q")` is `a b+` and `param("r")` none

#### Serves
- capability/follow-from-browser

### Requirement: open-pulls
`live::pulls_direct(auth, repo)` SHALL give the repo's open pull requests, each with its `number`, `title`, `url`, head and base branch names, head commit and whether its head is a fork's, read from GitHub's GraphQL in pages of 100 in GitHub's order, at most 3 pages, with whether the listing was read to its end; and none when a page can't be read. `live::pulls(access)` SHALL give the same pull requests, kept with the live state (`live-state-cache`).

#### Scenario: many open pull requests
- GIVEN the fake GitHub with a repo holding 150 open pull requests and one closed, then one holding 301 open, then one whose `Pulls` query GitHub answers with a null `repository`
- WHEN `live::pulls_direct` reads each, and `live::pulls` reads the first twice
- THEN the first gives the 150 open ones in order, read to the end, in two GraphQL queries; the second gives the first 300, not read to the end, in three; the third gives none; and `live::pulls`' second read asks GitHub nothing

#### Serves
- capability/approve-intent
- architecture/git-is-truth

### Requirement: fake-github
The fake GitHub SHALL answer as GitHub does, where the three crates' tests depend on it, in four respects. It SHALL give a repo's open pull requests to the `Pulls` query (`FakeRepo.pulls` whose `state` is `open`, in their order) in pages of at most the `first` asked, from the `after` cursor, with `pageInfo`'s `hasNextPage` and `endCursor`. In a `Blobs` query it SHALL answer each variable `$e<i>` holding `<rev>:<path>` as the field `f<i>`, a blob's `text`, `isBinary`, `isTruncated` and `oid`, and each variable `$t<j>` holding `<rev>:<dir>`, or `<rev>:` for the top of the tree, as the field `t<j>`, that directory's `entries`, each with its `name` and its `mode` as GitHub's GraphQL gives it, an integer: the octal mode `FakeRepo.modes` holds for the path read as a number, 33188 (`100644`) when it holds none, and 16384 (`040000`) for a directory. `GET /repos/{o}/{r}/pulls?head=<owner>:<branch>` SHALL give each open pull request of `FakeRepo.pulls`, and each page of `FakeRepo.pages`, whose head is `<branch>` of the repository `<owner>` names, with its `number`, `html_url`, `title`, `head`'s `ref`, `sha` and `repo.full_name`, and `base`'s `ref` and `repo.full_name`: a pull request's head repository is the repo itself, or `<FakeRepo.fork_owner>/<repo>` when its `fork` is set (`fork` when `fork_owner` is empty), its base always the repo. And the fake SHALL answer 502 to `GET /repos/{o}/{r}/git/blobs/{sha}` for a blob `FakeRepo.blob_fails` names, and to any REST request whose path after `/repos/{o}/{r}/` starts with an entry of `FakeRepo.fail_paths` (as `pulls` or `git/matching-refs/heads/spec/`), leaving every other request as it was. `FakePull`'s fields are unchanged: the knobs are `FakeRepo`'s.

#### Scenario: pages, modes and a failed blob
- GIVEN a fake repo `o/a` with 150 open pull requests, among them #5 from `spec/c` and #6 from a fork's `spec/c2` beside #7 from `o`'s own `spec/c2`; a commit holding `a/link` at mode `120000`, `a/b.md`, `a/c/d.md` and `top.md`; the blob of `a/b.md` in `blob_fails`; and then `fail_paths` holding `pulls`, and then `git/matching-refs/heads/spec/`
- WHEN the `Pulls` query is asked with `first` 100 and then from the first page's `endCursor`; a `Blobs` query asks for `$e0` `<commit>:a/b.md`, `$t0` `<commit>:a` and `$t1` `<commit>:`; the blob API is asked for `a/b.md`'s blob; REST `pulls?head=o:spec/c2` and `pulls?head=o:spec/c` are asked; and, under each `fail_paths`, REST `pulls?head=o:spec/c`, `git/matching-refs/heads/spec/` and the `Pulls` query are asked
- THEN the first page holds 100 with `hasNextPage` true and the second 50 with it false; `f0` is `a/b.md`'s text, `t0`'s entries are `b.md` at 33188, `c` at 16384 and `link` at 40960, and `t1`'s are `a` at 16384 and `top.md` at 33188; the blob API answers 502; `head=o:spec/c2` gives #7 alone, its head and base repository `o/a`, and `head=o:spec/c` gives #5; with `pulls` failing the REST pulls answers 502 while `matching-refs` and the `Pulls` query answer as before; and with `git/matching-refs/heads/spec/` failing that listing answers 502 while both pulls reads answer as before

#### Serves
- architecture/git-is-truth

## MODIFIED Requirements

### Requirement: code-key-doc-only <!-- base: 99e1fff620b471ee -->
`keys::code_key` SHALL give, for a commit's tree as the Trees API lists it (its entries in any order, `tree` entries among them), the code key `ledger/code-key` gives for that commit, leaving out exactly the paths `ledger/doc-only` counts as doc-only, so a change to `ledger`'s list changes the Worker's code keys with no edit here. `Listing::code_key` SHALL give that key for a listing, and none when the Trees API marked the listing truncated.

#### Scenario: doc-only paths from the ledger
- GIVEN a tree listing of `blob` entries (any mode and sha) at `docs/x.md`, `docs/guide/intro.md`, `backlog/y.md`, `CLAUDE.md`, `AGENTS.md`, `web/README.md`, `crates/a/src/lib.rs` and `docsy/z.md`
- WHEN `keys::code_key` runs on it
- THEN it equals `keys::code_key` over only `web/README.md`, `crates/a/src/lib.rs` and `docsy/z.md`, the entries `ledger/doc-only` doesn't count, and differs from the key of the listing with any one of those three removed

#### Scenario: the guide's files are doc-only
- GIVEN a repository whose commits `a`, `b` changing only `docs/x.md`, `c` changing only `docs/guide/start.md`, `d` changing only `docs/spec-format.md` and `e` changing only `crates/a/src/lib.rs`, each listed as the Trees API lists it, and a commit holding `src.rs`, `src/a.rs` and `src/a-b.rs`, whose order `.`, `-` and `/` decide
- WHEN `Listing::code_key` runs on each listing
- THEN each key equals the key `ledger/code-key` gives for the same commit, `a`, `b`, `c` and `d` share one, and `e`'s differs from it

#### Scenario: a truncated listing has no code key
- GIVEN the Trees API's listing of a commit, once as given and once marked `truncated`
- WHEN `Listing::code_key` runs on each
- THEN the first gives the commit's code key and the second none

#### Serves
- architecture/reuse-first
- architecture/no-silent-drift

#### Needs
- ledger/doc-only
- ledger/code-key

### Requirement: fresh-replaces-plain <!-- base: 2297901a2ff7d5af -->
When `live::cached` fetches a live-state item for an `Access` that carries a `fresh` delivery, it SHALL store the value under the plain key `live:<o>/<r>:<what>` as well as under the delivery's key, each for 60 seconds, so a later read of that item without `fresh` answers from it; a value it reads from the delivery's key SHALL be stored under no other key.

#### Serves
- capability/follow-from-browser

#### Scenario: the next plain read sees it
- GIVEN the fake platform, `o/a`'s ledgers stored under the plain key 10 seconds ago without `clm/ledger/alice/web`, and that branch created since with one record
- WHEN `ledger::fold` reads with an `Access` to `o/a` carrying `fresh` `d1`, then with one carrying no `fresh`, and a plain read of an item a fresh read stored is made 59 and then 60 seconds after it
- THEN both folds hold `alice/web`'s record, and the second read made no request to GitHub; the plain read at 59 seconds calls no fetch and the one at 60 seconds does

#### Scenario: a cached fresh read
- GIVEN the fake platform and an item already stored under `fresh` `d1`'s key
- WHEN `live::cached` reads it again with `fresh` `d1`
- THEN it makes no request to GitHub and stores nothing

### Requirement: asset-responses <!-- base: 6084735c40b9752b -->
A platform's assets SHALL be given by `fetch_asset(request)`, which answers the request with the status, headers and bytes the assets give. `asset(path)` SHALL, on every platform, give the text of the answer to a `GET` of `path` with no query, headers or body when it is a 200 whose body is UTF-8, and nothing otherwise. The fake platform SHALL answer `fetch_asset` from the assets it was given, a content type and bytes per path: 200 with that `Content-Type` and those bytes for a path it holds, and 404 with no body for one it lacks.

#### Serves
- architecture/invite-only-web

#### Scenario: a binary asset
- GIVEN the fake platform given `/fonts/inter.woff2` as `font/woff2` with bytes that aren't UTF-8, and no `/guide/nothing-here`
- WHEN `fetch_asset` answers a GET of each
- THEN the first is 200 with `Content-Type` `font/woff2` and exactly those bytes, the second is 404 with an empty body, and `asset` of either path gives nothing

#### Scenario: text through the provided method
- GIVEN the fake platform with its default assets, `/_clm/invited.json` as `application/json` holding `["clunny"]`
- WHEN `asset("/_clm/invited.json")` runs
- THEN it gives `["clunny"]`

### Requirement: object-reads <!-- base: e46c07e574f40df2 -->
`install::installation_id`, `install::token_direct`, `access::checked_direct`, `access::permission_direct`, `files::listing`, `files::objects_direct`, `live::pulls_direct`, `ledger::read`, `ledger::own`, `ledger::branch_tip`, `ledger::tip`, `ledger::append` and `ledger::appended` SHALL each run on a platform that offers GitHub, the clock, secrets, `Live` and the log and no Cache API or assets, and SHALL ask GitHub every time they are called, keeping nothing. Where there is a cached form, each SHALL give what it gives over the same GitHub: `install::token_direct` as `install::token`, `access::permission_direct` as `access::permission`, `files::objects_direct` as `files::objects`, and `access::checked_direct` an `Access` exactly when `access::check` gives one, its failed reads being `access-check`'s.

#### Scenario: a platform without storage
- GIVEN a test platform that offers only `GitHub`, `Clock`, `Secrets`, `Live` and `Log`, over the fake GitHub
- WHEN each of those calls is made twice with the same arguments, and each cached form once over the fake platform with the same GitHub, GitHub answering every request as asked
- THEN the test compiles, each call gives what its cached form gives, the fake GitHub saw each request twice, and nothing was written to the fake's Cache API

#### Serves
- architecture/git-is-truth
- architecture/reuse-first

## REMOVED Requirements

### Requirement: shared-layer <!-- base: 33f47bc2affe5833 -->

### Requirement: unix-time <!-- base: add5963f4d9d4ca1 -->
