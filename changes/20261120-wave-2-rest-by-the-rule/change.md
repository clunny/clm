---
status: applied
resolves: [the-proof-is-real-upgrade-note-doesn-t-say-wiring-switches-on-a-check-too]
---

## Intent

Wave 2 of phase 5 of `backlog/a-passing-check-proves-less-than-the-spec-claims`, its second branch of two: `effort`, `ledger`, `upgrade` and `init` are each brought, in one pass and one delta, to the rule for a requirement (`requirements-state-what-the-contract-shows`, as amended on 2026-11-20). The wave is for a spec that is true and proven, not a smaller one. `20261120-wave-2-check-by-the-rule` holds the check family; no clause moves between the two. This branch also records the wave in the plan item, and the human's answer of 2026-11-20 that a wave's build run designs fixes for what its builds find.

## Problem

Read against the rule and the code on 2026-11-20 and 2026-11-21, by one reader per component and then by one drafter per component that checked each entry against the code:

- **Every Contract was wrong or incomplete.** `ledger`'s gave wrong argument lists for `writer`, `append`, `append_all`, `sign` and `signed_by`, left out five fields of the fold other crates read, and named neither `verify` nor the record types. `effort`'s offered two calls that are private to the crate. `upgrade`'s left out `supersedes()`, the real shape of a migration and what its calls return. `init`'s left out `clm survey --root`, `own_migrations()`, the rules it seeds and every exit-2 path.
- **About 55 clauses the code shows had no requirement**: the exact lines the commands print, what `--apply` does when a migration fails, how the fold orders records whose times are equal or out of order, which transcript lines are read under a branch, what the calls do with `--help` and a closed stream.
- **Four Design sentences of `ledger` and one of `init` contradicted the code**, among them that `clm ledger verify` audits from "after" the first landed commit, where it audits that commit too, and that `clm survey` reads the old `nodes` key, which it refuses.
- **Bugs on `main`, each mended by its component's build:**
  - `upgrade`: `clm upgrade --apply` and `--decline` rewrite `clm.toml` from the key's line to wherever a count of `[` and `]` returns to zero, comments and strings included. A stamp line that opens a `[` it never closes, or carries one in a comment, loses every line after it, exit 0; a quoted or dotted key gains a duplicate, leaving a file that doesn't parse; and when `clm.toml` can't be written the message loses the `applied` lines. `--apply` stops at the first note whose file it can't write, and `check_lock` at the first difference.
  - `ledger`: two effort records of one ID, time and writer are told apart by their JSON text, whose key order differs between the native build and the Worker's, so `clm` and the web app can keep different records; and of two records of different kinds that share an ID the fold keeps whichever line it reads first, so two readers can fold one ledger to different states. `verify`'s definition of a land's legacy key was that of today's key, where the code uses the frozen legacy list.
  - `effort`: `clm architecture` gives a rule its component doesn't hold yet (one a live change adds) a share on top of the whole of the component's cost, and `inf` when the component holds no rule; a transcript entry whose timestamp carries an offset, or is missing, loses its tokens, while a timestamp with a month 13 or unpadded fields is read as a time; a line that isn't UTF-8 is counted in a branch's warning whether or not it could be the branch's; an approval is counted for the component a rule's ID names though the component doesn't hold the rule yet.
  - `init`: the `clm.toml` it writes lists the hosts as `none`, `git` or `github`, leaving out `azure`.
  - `effort`, `ledger`, `init`: `--help` through the crate's own call prints a code comment as its first line, and three of `effort`'s subcommands a `Usage:` line without the command's name. Each `commands-in-crate` now states the help its own call writes; that `clm <command> --help` prints the same is the root's to state, in wave 7 (`commands-live-in-their-crates`).
- **`backlog/the-proof-is-real-upgrade-note-doesn-t-say-wiring-switches-on-a-check-too`** asks for a sentence in a note whose text a requirement lists, so it is a clause of `upgrade/migration-markers-in-tests` and no code-only fix. The migration has not shipped, so its note can still change; the next release freezes it.

## Outcome

- Each of the four has one delta: Contract and Design made right against the code, requirements reworded to what someone outside the component observes, the unstated clauses stated with one scenario each, and the cuts below.
- Where the code falls short of a requirement's plain meaning the requirement keeps its meaning, gains a scenario for the failing case, and its build mends the code.
- `clm upgrade --apply` refuses a `clm.toml` that doesn't parse or can't be read, as `--decline` does, running no migration; and both replace only their key's own lines.
- The comment `clm init` writes above `host` names `azure`, and carries no migration: the human waived `upgrades-carry-migrations` for a comment's text alone on 2026-11-21, and `template-change-ships-its-migration` records it. An advised note was drafted and withdrawn; it showed that the first migration in `init`'s own list fails a root test, which `backlog/the-first-migration-in-init-s-own-list-fails-the-root-s-test-of-clm-upgrade-lock` holds.
- Of the lines that share an ID the fold holds one, chosen over the whole set whatever its size and the order it is read in, by `at`, then writer, then the line's text with its keys sorted at every depth: the greatest when all are effort records, the latest count, and the least when any is of another kind.
- `upgrade`'s note requirements name the command that prints each note, `clm upgrade --note <id>` and the first line `clm upgrade` lists, so each is a command's output and none a rule over a template's text.
- `init`'s template requirements speak of the files `clm init` writes, proven by running it.
- `upgrade/shipped-migrations-frozen` takes the scenario "no lock yet" from `init/own-migrations-frozen`: the lock check is `upgrade`'s code.
- The fold's order is stated as it is: by `at` as written, then `id`. `backlog/a-request-for-changes-made-after-an-approval-is-read-as-cleared-when-the-approver-s-clock-runs-ahead` records what that costs one person acting from two machines; its mend spans `land.approval` and is not this wave's.
- `backlog/help-through-a-crate-s-own-command-call-is-not-the-help-the-binary-prints` records the crates outside the wave that were not measured.

## Constraints

- A build changes only its own component's files (`own-files-at-the-gate`). No build of this branch needs a file outside its component.
- No call's return that another component's test asserts changes. `check_lock` returns every difference where it returned the first; `init`'s one test of it has one difference.
- `plugin-panel/test/agents.test.ts` parses `crates/clm-effort/src/prices.rs` as text. `effort`'s build can't edit that test, so its Design tells it to keep the rows' shape and the fallback constant's name.
- `upgrade/migration-markers-in-tests` is built before the next release, which freezes the note it changes. No build and no release script reads this sentence: `backlog/a-release-locks-a-migration-whose-requirement-is-still-pending` holds it, and until that is mended the human holds the release.
- `init` is built twice: `all-migrations`, `own-migrations-frozen` and `init-stamps-upgraded` need requirements of `upgrade` this change modifies, and five of `upgrade`'s need `init`'s. The needs hold no cycle.
- `crates/clm-init/tests/says_component.rs` keeps its `clm:reads ARCHITECTURE.md` marker though the scan that read the file goes: the root's test of `clm/doc-readers-marked` names the file, and `init`'s build can't edit it. `init`'s Design says so; wave 7 drops both.
- `ledger`'s test of the effort tie can fail only where `serde_json` keeps keys in written order, which is the workspace's build, the one `clm land` checks; built alone the crate already sorts them. Its Design says so.
- All of this change's components were over the spec budget before it and each grows, as "Settled on 2026-11-20" in the plan item expects of every wave: stating what was unstated costs more text than the rule's cuts save. No seam is proposed here; budgets and splits are `backlog/the-brakes-on-the-spec-s-growth-only-advise`, for after the plan (`slim-components`).
- The names other components and the human layers use stay: every requirement of `init` that another component needs or `ARCHITECTURE.md` names, `upgrade/own-list-closed`, `ledger/request-cleared`, and the `effort` and `ledger` requirements `effort`, `replica` and the web crates name.

## What the requirements no longer state

Under the yes given in wave 1 (`requirements-state-what-the-contract-shows`):
- `git-through-layer`, whole, in `ledger` and `init`, each with its scenario "covered by the scan" and its scan test. `vcs/one-git-layer` reads every crate for every call these looked for.
- `effort/says-component`, whole, with its two scan tests. Its sentence "The text `clm capabilities` and `clm architecture` print SHALL say component and components, never the old words" goes with it, and the gate does not list it, since the requirement's one scenario, "a capability with no rules", moves under its name to `effort/capability-page`. The printed words stay stated in whole lines by `capabilities-list`, `capability-page`, `architecture-list` and `architecture-page`.
- `init/says-component`: the scan sentence and its test. What the messages and the written files say stays stated and tested.

Sentences that only say where a test sits, cut with no separate yes (`cuts-are-what-a-program-lists`):
- `commands-in-crate` in `effort`, `ledger` and `init`: "Every test of `<component>`'s requirements that sat in the top-level test files of `crates/clm/tests/` SHALL sit in `<component>`'s own files…", and in `init` the sentence that the tests of `clm guide` SHALL sit in `crates/clm-init/tests/guide.rs`.
- Without `SHALL`: `init/guide-constraints-page`'s "Its tests hold a `clm:reads` marker naming `ARCHITECTURE.md`", `init/init-root-ownership`'s "Its tests assert the frontmatter written", and the sentences of three `upgrade` requirements that name `crates/clm/tests/hooks.rs` and what a past build gave its fixtures.

New proposals, each read by the human:
- `ledger/shared-plumbing` and `init/shared-plumbing`, whole, each with its scenario "no copy of the remote's name" and its scan test. Each scans its crate's source for `"origin"`, `"origin/` and `remotes/origin`. No entry of `ARCHITECTURE.md` names either under `#### Enforced by`. Lost: nothing stops a build from writing the remote's name into a git argument in these two crates where `clm-git`'s `REMOTE` should be used; nothing a user observes changes while `REMOTE` is `origin`, which CLM lets no one change. `ledger`'s sentence on the push-refused message moves into `ledger/compact`. The family's other five copies (`host`, `land`, `sync`, `orchestrator`, `view.repo`) are their waves'.
- `upgrade/forms-without-a-list-gone`, whole, with its scenario "no caller is left" and both tests of `forms_gone.rs`. It states that seven old calls are absent, read from every crate's source; the mutation pass judged it unprovable. `upgrade/pending-migrations` keeps "It SHALL read no list but the one it is given", and the Contract says every call works over its caller's list.
- `init/survey-tracked-through-git`, whole, with its scenario "no copy of its own": a scan of `clm-init`'s source for `ls-files`. The behaviour it guarded stays stated in `init/survey-map`, which gains "each once, a file the index holds at several merge stages included" and the need of `vcs/tracked-files`, proven through `clm survey`.
- `init/survey-agents-md`, whole, as a duplicate: `init-root-ownership` and `survey-components` name `AGENTS.md` among the files left out, and its scenario "an existing AGENTS.md" moves under its name to `init-root-ownership`, which takes its `#### Serves`.
- `init/guide-table-from-files`: the clause that no `include_str!` under `crates/clm-init/src/` names a `docs/guide/` path, and its scenario "no page named in source". That a page added to `docs/guide/` is in the next build stays stated and proven.
- `init/survey-components`: "No `clm:remove-by 20261102-rename-node-to-component-contract` marker SHALL be left." `clm/node-names-retired` holds the same sentence.
- `init/all-migrations`: "Every migration written after `20261114-proof-is-real` SHALL be this crate's, whatever it carries…". Where code goes; the Design keeps it, and `upgrade/own-list-closed` holds `upgrade`'s list at thirteen.
- `init/own-migrations-frozen`: the scenario "a shipped migration edited", which `upgrade`'s own test of the lock check proves with one more assertion.
- `ledger/request-cleared`: "It SHALL keep no other function that takes a writer's person apart." In its place: wherever a call or command reads a writer's person part it SHALL be the part `person` gives, with the scenario "one person part".
- `ledger/legacy-doc-only`: "It SHALL be written as a literal of its own and SHALL NOT call `doc_only`…", and its scenario "frozen". The scenario "paths" now gives every element of the list, eight true and five false. Lost: a rewrite of the legacy list in terms of the live one that gives today's answers would pass, and a later addition to the live list would then reach it.
- `upgrade/own-list-closed`: "`crates/clm-upgrade/src/migrations/` SHALL hold one directory for each and no other", and its scenario "a directory added". That `migrations()` holds exactly its thirteen in order stays.
- Five sentences of `upgrade` that tell a build how to make a test's fixture or what a test expects: one each in `migration-tests-overlap` and `migration-land-dir`, two in `migration-build-setup`, and `migration-list`'s "The forms without a list … SHALL stay until `forms-without-a-list-gone` is built", false since they went.
- `upgrade/migration-markers-in-tests`: the scenario "a new project", which `migration-list`'s "a fresh project is done" shows.
- `effort`'s Contract no longer offers `decide_architecture` and `architecture_review`, private to the crate.

Narrowed or widened to what is meant, each a judgement the human reads:
- `effort/transcripts`: with a branch given, the warning counts the lines that can't be read among those that hold the branch's name or a worktree path that holds it, where it said "warning once with the count". A broken line that can't be the branch's says nothing of the branch's record.
- `effort/history`: an item counts as a `question`, `human-task` or `workaround` by its kind as added; one that began as another kind and was edited to one of these starts no life.
- `ledger/fold`: it reads every local ledger branch whose clone part is this clone's, whatever its person part, where it said the local writer's branch. Records a clone wrote under an earlier name, and hasn't pushed, are facts it wrote.
- `init/init-keeps-a-remote` goes by the host set in `clm.toml` or `clm.local.toml`, else the host `clm init` writes or would write, where it said the host the kept `clm.toml` sets.
- `init/guide-command` orders the index by each page's first link in `README.md`, as `guide-table-from-files` and `guide-table-at-build-time` have it, where it said the numbered list.

Scenario outcomes that change under their names, none losing a case:
- `effort`: `capabilities-list` "outside a CLM repository", `commands-in-crate` "arguments it can't parse", `prices` "unknown model" (the fallback's $3.00, where it named a row that holds $2.00).
- `ledger`: `legacy-doc-only` "paths", `commands-in-crate` "arguments it can't parse".
- `upgrade`: `migration-list` "the forms that take a list are public" and "a fresh project is done" (every `done` is true but that of `20261105-clm-upgrade-2`, which a later migration supersedes), `own-list-closed` "the list as it stands", `upgrade-note-and-decline` "declining advised drift", `migration-agents-md` "CLAUDE.md alone", `migration-markers-in-tests` "the note", `shipped-migrations-frozen` "locking a release".
- `init`: `init-next-steps` "azure next steps", `init-check-permissions` "no settings" (every rule `init-seeds-permissions` names, where it said twenty of what are twenty-six), `guide-command` "page left out", `guide-table-from-files` "this repository", `template-says-pull-request` "the written architecture", `template-says-architectural-constraint` "the written layers", `templates-say-new-words` "a new repository", `survey-components` "entries the model refuses", `all-migrations` "a duplicate ID", `commands-in-crate` "arguments it can't parse".

## What no test asserts today

Each delta's drafter read, for every newly stated clause, the assertion that fails if the behaviour breaks. The list is a floor. None today: every `--help` clause; `upgrade`'s "an odd stamp line", "an odd declined line", "two differences" and the note file that can't be written; `effort`'s "a rule not built yet", the offset and missing timestamps and the branch cases of "lines that count nothing"; `ledger`'s tie between effort records under sorted keys; `init`'s "the host comment". Some of `ledger`'s newly stated clauses are asserted today by a test that cites a requirement which didn't say it, which its build re-cites; about fourteen cases have no test anywhere, among them the exact messages of `verify` for a commit that isn't a merge or shares no history, the `ssh-keygen` failure messages, the fold's warnings and "one person part".

## Open questions

None that holds the build. Five things the wave found and leaves, each a backlog item of this branch: one person's request for changes read as cleared when it is written by a clock behind the approval's; the help of command calls in crates outside the wave; the status word `clm archive` prints; a release that locks a migration whose requirement is still pending; and the root's test of `clm upgrade --lock`, which the first migration in `init`'s own list fails.
