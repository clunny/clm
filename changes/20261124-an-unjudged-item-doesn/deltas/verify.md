---
component: verify
base: { design: "9f5e9218bff09cc6" }
---

## MODIFIED Design
**Settings** (`settings-from-main`). `clm verify` verifies `origin/main`, so it reads how to verify it from `origin/main`: the commit it checks out is the one whose settings it follows. It fetches `origin` first (every host has one: `none-keeps-its-own-remote`; the fetch needs no setting), and then loads the settings once, through `config/committed-settings`, the repository layer being the text of `origin/main:<prefix>clm.toml`, `<prefix>` the CLM root's place in its git repository (none when `origin/main` or the file is missing). `host`, which names the host adapter `import` reads runs through, `[verify]`, `[affected] all`, `[land] target_cap_gb`, the cap on verify's build place (`verify-build-cap`), and `max_builds`, after which the pass takes its slots, come from those; the clone's `clm.local.toml` still sets any of them for this clone, as everywhere (`local-settings`), since it belongs to the clone and is never behind. Whether `[verify]` is set is decided only after the fetch, so a project with no `[verify]` pays one fetch per call. The sessions that run `clm verify --due --event work-done` at their end run it in their own checkout, which they leave as it is and which may be far behind `main`: reading the checkout's file there would skip every run once `[verify]` had landed after the checkout was made. `host` is read from there too, not from the root as `land` reads it (`land-policy-from-base`): verify uses it only to read the runs of a pipeline that ran `main`'s commits, nothing needs it before the fetch, and a clone that reaches its host differently says so in its `clm.local.toml`. So `clm verify` loads no settings from the working tree's `clm.toml`, and a checkout whose file an older or newer `clm` can't load (a table whose keys have changed since, say) still has its slow tests run by `main`'s settings. The ledger's writer identity, which every record it appends carries, reads no repository `clm.toml` either (`ledger/writer-identity`), since `writer` is never set there. The one exception is the filing (`files-unproven`, below): its branch lands through `clm land`, which reads the root's settings as for any land (`land-policy-from-base`), so a checkout whose `clm.toml` can't be loaded leaves the items unfiled with `unproven: not filed: <why>`, and a later call files them; the verification's record and outcome are already made by then. This is the decision `verify-settings-from-main`.

**Since when.** The *last verified commit* is the newest commit on `origin/main`'s first-parent chain that a `verified` record names, whatever its result; the *last green commit* the newest that one with `passed: true` names. The *unverified lands* are the first-parent commits of `origin/main` after the last verified commit, up to its head, or all of them when no record exists. A run's tests are selected by `check.affected`'s library call with the last green commit as its base, as `clm affected --base <last green commit>` selects them (`graph-test-selection`), written to a file in `TMPDIR` and handed to the check as `CLM_SELECTION` as `[land] check` gets it, so a run covers everything changed since the tests last passed, the lands of an earlier red run included. With no green record, with an `[env]` that differs from the green commit's, or with a selection that can't be made, there is none and every test runs.

**Triggers** (`verify-due`). Read from `[verify] triggers` and `[verify] every` over the unverified lands, tried in a fixed order and the first that holds named: a red last record (`red-again`), since a fix may have landed; `change-done` when one of them adds a `changes/archive/<id>/change.md` (`git diff --diff-filter=A` against its first parent); `queue-drained` when every requirement `check.coverage`'s `pending` returns for `origin/main`'s head, checked out into the verify tree, is blocked, its `blocked_by` or its `waits_on` holding something, so also when it returns none (computed only when `queue-drained` is among the triggers and none before it holds): a blocked requirement waits on a human's answer or on other work, which no build is about to do, and one of a component under a live claim is still pending and unblocked in `origin/main` until its build lands, so the trigger reads no claim and waits for that land. A spec graph with violations leaves the queue unknown, and the trigger doesn't hold; `work-done` when the caller passed `--event work-done`; `every.lands` when there are at least that many; `every.days` when the oldest of them was committed at least that many days ago. No unverified land means nothing is due, whatever else holds.

**The run** (`verify-run`). It checks `origin/main`'s head out into `<home>/.clm/verify-tree` as `land` keeps its land tree (`fixed-land-tree`), `<home>` found as `land` finds it, from the common git directory rather than by listing worktrees, so a run waiting for the lock never reads the entry of a worktree another run is still adding. A tree git doesn't list, or that isn't a worktree of its own, is removed and added afresh. It runs `[verify] check` there through `land`'s shared check runner (`land/shared-check-runner`), so under the clone's check lock, `<home>/.clm/check.lock`, like every other check of the clone (`check-runs-one-at-a-time`): CLM can't know what a project's tests share, so a slow run and a land's check never overlap. A land on the same clone waits while a slow run's check runs; triggers make that rare, and a project that can't accept the wait runs its slow tests in `pipeline` mode or from another clone. Each bisect probe takes the lock afresh, so lands can go between probes. **Build place** (`verify-build-dir`, `verify-build-cap`). Every run of `[verify] check`, a bisect probe's included, gets `CLM_LAND_DIR` set to `<home>/.clm/verify-build`, as a land's check gets `.clm/land` (`land/land-dir-env`): a place that lasts from run to run outside the verify tree, so a check script that points its toolchain there for lands (`init/starter-check-land-dir`) does so for verify too, with incremental compilation off. Build output kept inside the verify tree grew without bound, most of it incremental data no run between whole commits reuses, so the tree keeps nothing past a `clm verify` (`verify-tree-cleared`, below). Verify sets no toolchain's variables itself. Before each run of the check, and before the mutation pass's command, which gets the same `CLM_LAND_DIR` (`mutation-pass`), verify creates that place when it is missing, so a command may write into it before building anything there, as the pass's copy of its coverage file does (`ci/mutate-script`): a fresh clone has none, and the cap below removes it whole. Creating it in verify rather than in each script that writes there keeps the place a promise of the environment, as `.clm/land` is a land's by the time its check builds, whatever the project's toolchain (`reuse-first`); nothing about the place may lose a run's result. After each run of the check, still holding the verify tree's lock (`verify.lock`), so no other verify run is building there, it applies `land`'s cap to that place through `land/shared-target-cap`, named `.clm/verify-build` and capped at `[land] target_cap_gb` from the settings it follows. Cargo prunes no unit no run builds again, so the cap trades one cold run for a bounded place, as the land's does. The check's output goes to stderr and to `<home>/.clm/verify-logs/<sha>.log`, which a passing run removes and a failing one keeps. `local` and `pipeline` differ only in where the project calls it: `pipeline` is a host pipeline running `clm verify --due` on each push to `main`, with `CLM_WRITER` naming it and the right to push `clm/ledger/*`. A run killed before it records leaves its trigger due. **Output.** Once the check has exited, nothing `clm verify` writes can lose its result: every write to stderr before the record ignores a failure, as the runner's copy of the check's output already does, and nothing goes to stdout before it, since the outcome and the notes are announced only after the record is appended. A failed write before the check exits ends nothing either, since ending then would leave the check running unwatched in the verify tree. The check's output is still copied to stderr, so a person watching a run sees it progress; the log holds it whole. `clm verify --commit <sha>` for a commit other than the head takes the last verified commit before it as its `since` and checks that commit out.

**A clean tree between runs** (`verify-tree-cleared`). Once a `clm verify` is done with the verify tree, after the mutation pass when one ran and before it files anything, it removes from the tree every file git doesn't track, those git ignores included (`git clean -ffdx` at the tree's top), under the verify tree's lock. Each checkout within one `clm verify` still removes only untracked files and keeps ignored ones, so what the check installs there (`node_modules/`, which the pass's StrykerJS runs from: `ci/mutate-script`) serves the pass after it; what any command leaves in the tree past the end of the call is not kept. A project's scripts can build caches under the tree, such as a mutation tool's per-job targets, that grow without bound beside the capped `.clm/verify-build` and starve the build loop, which counts free room before each launch (`machine/disk-room`). The place a project keeps from run to run is `CLM_LAND_DIR`, which the cap bounds; the tree is no second, unbounded one, whatever a project's scripts write there, so the bound lies in verify, for every project, rather than in one project's script (`redesign-before-workaround`). It costs the pass one cold build per job each pass; a pass's peak, its jobs' targets while it runs, is room the work uses and stays. The clean runs only when git run in the tree finds its top there, as the checkout's test of a tree it reuses does: git run in a tree with no `.git` of its own finds the main worktree around it, and a clean there would go through the main worktree's repository rather than the tree's. Every call that checked the tree out cleans, however it ends; the pass's tree has dropped the lock by then, so the clean takes it afresh. A `clm verify` killed with `SIGKILL` cleans nothing; the next one that checks the tree out cleans what it left, since the clean takes the whole tree.

**Import** (`verify-import`). `host`'s `runs` for `[verify] workflow` (`host/list-runs`), each with its head commit, whether it completed and its conclusion, newest first as `host/list-runs` gives them, so the first that is a result is the newest. A completed run whose conclusion is `success` or `failure` on an unverified land is a result; the newest such run's commit is the one recorded, covering the unverified lands up to it, `passed` from its conclusion and `run` its URL. Any other conclusion (cancelled, skipped) is no result.

**Culprits and components** (`verify-bisect`). A red result over one unverified land names that land as the culprit. Over several, `local` and `pipeline` runs bisect at once: a search of the first-parent lands between the last green commit (exclusive) and the red one, each probe running the same selection in the verify tree, the first red land being the culprit; `import` records `culprit: null` and leaves the search to `clm verify --bisect <sha>` on a developer's machine, which appends a fresh red record for the same commit with what the search found. **Untestable lands.** The codes `verify-bisect` counts as untestable are `sh`'s own for a command it found but could not run and one it could not find, as on a commit older than the project's slow-test script, and the code `git bisect run` already gives a script for "this commit can't be tested", so a project's script can say so without CLM inventing a convention (`reuse-first`). A land whose `[env]` can't be used is set aside the same way: the check can't be given its variables there, and refusing would lose the red result already found. Such a land is neither green nor red. The search keeps a range of lands `lo..hi`, `hi` the red one (at first the red commit itself) and every land before `lo` in the range known green; it probes the land in `lo..hi` (`hi` excluded) not set aside that is nearest `lo + (hi - lo) / 2` (rounded down), the later of two equally near, moving `lo` past a green probe and `hi` to a red one, until no such land is left. When `lo` is `hi`, that land is the culprit. Otherwise every land from `lo` to `hi` but the last was set aside and any of them may hold the break, so the culprit is unknown. The search assumes, as `git bisect` does, that a land after a red one stays red; to conclude it is unknown it must probe every land it set aside, each a checkout and a check that ends at once. The record's components are those whose `paths` hold a file the culprit's first-parent diff changes, or, with no culprit, a file any of the lands that may hold it change: those from `lo` to `hi` when the search left it unknown, and in `import` mode, which searches nothing, the covered lands. A land with no first parent (the repository's first commit) diffs against the empty tree. When no component's `paths` hold such a file (a land that changed only doc-only paths), the root component.

**What it stops.** A red record is a report: `clm intake` lists it for triage (`orchestrator.triage/intake-verify-failures`) and `clm land` warns a branch touching its components. Once the human takes it in as a `kind: bug` item with `verify_failures`, `clm land` holds code lands on the item's components until a branch whose commit carries `Resolves: backlog/<slug>` for it lands, letting through meanwhile a branch that resolves another item holding the same component (`land/verify-stop`). A host's run never holds a land by itself (`land-locally`).

**The mutation pass** (`mutation-pass`, `mutation-after-land`). A marker says which test claims a requirement, and the land's check says the test passed; neither says the test would fail if the code were wrong. Mutation says that, and it is slow, so it runs here, after the land. CLM names no tool: the project's `[verify] mutate` command gets a plan and writes verdicts, as `[land] check` gets a selection, and it is unset unless the project sets it. A requirement is *due* when one of its citing tests was added, or changed in more than its markers, since the last pass, so a requirement that was only re-cited isn't judged: its tests are the ones an earlier pass saw, or none did. A sample of mutants drawn from a whole component can't judge it either, since such a sample is noticed by the tests of some of its requirements often and by those of others almost never, all of them sound. The plan therefore hands the command the code the lands changed first, and a limit of 20 further mutants from the rest of the component, a constant (`yagni`). Of both, the command tries only mutants of lines the requirement's tests run (`ci/mutate-script`), since a mutant of code they never reach is one no citing test could notice: a component's first mutants are often code such as `Display` impls and getters that the tests of a requirement whose rule is data never run, and the pass would file those sound tests as unproven. A requirement whose tests run lines of its code but none a mutant changes, as when its rule is data, is not judged; one whose tests run none of its code, as an empty test, still reads as unproven, unless a file holding one of its citing tests carries a `clm:reads` marker (`check.affected`). Such a test reads the repository's own files, and the rule it proves can be data there that no `code` line holds and no mutation tool changes: `clm/doc-readers-marked`'s rule is the markers themselves, in test files of half a dozen crates, and its tests run `clm-affected`'s `readers`, none of the root's code, so the pass filed those sound tests as unproven whenever they changed. The plan names each such marker's glob on a `reads` line, so the command can tell a test that reads data from an empty one, and leaves the requirement not judged (`ci/mutate-script`). A layer entry, a capability or an architectural constraint a test cites, has no component code to mutate, so it is never due. A Rust requirement whose rule is code in its test's own file, as `vcs/one-git-layer`'s scan of every crate is, is judged by mutants of that file whenever its tests run no mutant of its code and no other component's code, reads lines or not (`ci/mutate-script`), since `cargo mutants` never mutates a test target and would otherwise leave sound tests filed unproven or never judged. The markers are read per file, as `check.affected`'s readers are: a marker anywhere in the file holding a citing test counts, and one in another file of the same test target does not. So an empty test escapes when it sits in a file whose other tests read files, as the shared `crates/clm/tests/cli.rs` and `crates/clm-check/tests/cli.rs` do; that was accepted over reading only the markers inside the test's own span, since the markers are written once at a file's head and none sits inside a test. Tests that predate the pass are never judged (`backlog/tests-written-before-the-mutation-pass-were-never-judged`). The pass runs after the `verified` record is appended and the outcome announced, and outside the check lock, so it can lose no verdict and hold no land; it keeps the verify tree's lock, so two passes never share the tree, and it gets slots of its own, from one past the build places up to `mutate_jobs` of them, one for each requirement it judges at once, since its tests overlap lands' checks and builds' (`check-runs-one-at-a-time` covers the runs that share slot 0, which this one doesn't). It judges up to `mutate_jobs` requirements at once, cut down to one per 2 GiB of available memory, since each concurrent build of a crate's tests takes one to two gigabytes and the build loop often runs builds beside a verify. A requirement an item named is not judged again when the item goes: the build that mended its test changed the test, which makes it due, and the build that found the test sound and deleted the item has settled it. A pass keeps what it reached however it ends: the command writes each verdict as it is reached, and when it fails, reaches `mutate_minutes`, or is stopped, `clm verify` records the verdicts written and names the due requirements it didn't reach under `left`, which the next pass judges first, with the diff from where they were first due, since a pass that recorded nothing would restart its requirements from the first, and a long pass would have no limit. A pass killed with `clm verify` itself, by `SIGKILL`, still records nothing, and the next pass starts from the same commit. The call's memory file, minute, grace and stop request are its caller's (`Call`), so the crate's tests pass a file, short lengths and a set flag; a stop or the limit can end the command before `coverage.sh` runs, which leaves `[health] lcov` as the last whole pass wrote it.

**Filing** (`files-unproven`). Verify pushes nothing to `main`, so a verdict reaches the tree as a backlog item, landed through `clm land` like any branch: adding a new item lands on checks. What is still to file is found each time from the ledger and from `main`'s history, not remembered: an item is named after the commit of the record that found it, so a filing that failed, or that a refused call or a dry run never made, is made by the next `clm verify --due` on a clone in `local` mode, and an item a build has resolved is not filed twice. The item's name holds the commit as `git rev-parse --short` abbreviates it, which git lengthens as a repository grows, so the history is searched for a name holding any prefix of the commit at least 7 characters long, as the kept log is found (`kept-log`): looking for the name the commit would get today would file again, after git lengthened its abbreviations, every item a build had deleted. Verify deletes its branch on `origin` itself, so none is left when the land fails. A pipeline has the right to push only the ledger, so `pipeline` mode records and leaves the filing to such a clone. A stop request keeps the filing from starting its writes, its push or its land, since an interrupted `clm verify` should end soon and a land runs the whole `[land] check`; a land already started runs to its end, as `clm land` has no stop request of its own. The filing's `git` commands and its land run in `clm verify`'s process group, unlike the mutation pass's command, so a Ctrl-C at a terminal ends whichever of them is running: running them in a group of their own would let no `git push` ask for a password at the terminal, and would make a Ctrl-C wait out a land's check of many minutes. A command ended so fails its step, the filing reports `unproven: not filed: <why>` and deletes its branch as after any failure, and the next call files the items; a land ended part-way is what a Ctrl-C leaves of a `clm land` run by hand.

**Requirements a pass can't reach** (`files-unproven`). A requirement under `left` goes first in the next pass, so a pass that leaves it again was stopped, reached its limit, or met a tool that fails on it: a command that fails on one block leaves that block and every block not started, pass after pass, while the outcome line still says `verify passed` and the reason is a line on stderr after it, which no one reads. So a requirement two passes in a row left is filed as a `bug` item on its component, beside the `unproven` ones, which the build loop drafts like any bug (`mutation-after-land`). Two, not three: the second pass tried it first. The item is named after the commit of the second record of the run, so the passes that leave it again file nothing more.

**Why a pass left it** (`files-unproven`, `mutation-pass`). The item says why, and the `mutated` record is where that is kept: the filing reads records two passes apart, and `clm verify` keeps its stderr nowhere, so a reason only printed is gone by the time an item is filed. It was first left out, on the view that a draft could read the reason on verify's output. The pass after `88470edf` showed otherwise: one StrykerJS failure on five `web` blocks filed items on seven components, each pointing at output nobody had kept, and six drafts hunted a cause that lay in `mutate.sh` (`backlog/an-unjudged-item-doesn-t-say-which-failure-ended-the-pass-that`). So the command writes `<id> failed <why>` for a block whose tool failed (`ci/mutate-script`), and the record keeps those requirements under `failed` with their text, and how the command ended under `ended` (`ledger/mutated-record`). The filing counts a requirement against a record only when that record's pass could have judged it: not when the stop request ended the pass, a person's act and no fault of the requirement, and not when the pass left it only because another block failed first, since that block goes first again in the next pass and keeps the rest from starting until it is mended. Such a pass says nothing about the requirement either way, so it neither adds to the run nor ends it. The item then goes to the component whose block failed, and the starved requirements are judged once it is mended, or filed then if they fail on their own. A pass that reaches `mutate_minutes` still counts every requirement it left: two passes in a row that can't get through what is due within the limit are worth a person's look. A tool that can't be installed or fails on its fixture, before any block, names no requirement, so its pass counts every requirement it left, as before: rare, and the item's body names the command's status.

**The kept log** (`kept-log`). Matching a log's name by prefix rather than by the abbreviation `git rev-parse --short` gives now keeps a log found after the repository's abbreviations have grown longer than when the run named it.

## MODIFIED Requirements

### Requirement: mutation-pass <!-- base: 66d709f6ec597141 -->
When the settings it follows (`settings-from-main`) set `[verify] mutate`, a `clm verify --due` or `clm verify --commit` in `local` or `pipeline` mode whose check passed SHALL, after appending the `verified` record and printing the outcome line, and holding the verify tree's lock but not `.clm/check.lock`, run the mutation pass on the commit `C` it verified. `P` is the commit the newest `mutated` record names among the commits before `C` on its first-parent chain, or, when there is none, the first parent of the oldest commit of the unbroken run of commits ending at `C` on that chain whose `clm.toml` sets `[verify] mutate`, which is the last commit before the project switched the pass on; a commit sets it when its `clm.toml` parses as TOML and holds a non-empty string there, and any other commit ends the run. A requirement is *due* when, between the trees of `P` and `C`, a test citing it at its current version in `C` (`check.coverage/test-spans`, groups left out) was added, or its lines changed in more than their markers; with no `P`, as when the chain's first commit already sets `mutate`, every requirement a test of `C` cites at its current version is due. An entry of the `capability` or `architecture` layer is never due, whatever cites it. A requirement under `left` in the newest `mutated` record on that chain, `C` included and the newest by `at` among a commit's records, is due as well when a test of `C` cites it at its current version; its `diff` runs from the `since` of the oldest record of the unbroken run of newest records whose `left` names it, so a requirement carried over keeps the change that made it due. So a pass that ends before it judges a requirement loses nothing: the next one judges it, and a pass that records nothing starts from the same commit. For the due requirements it SHALL write a plan file holding, for each, first those under that `left` in the list's order and then the others in ID order, the line `requirement <id>`, then one line `code <path>` for each file its component owns in `C`, then `diff <path>` naming a file that holds `git diff <P> <C>` over those files when that is not empty, then one line `test <file>`, a tab and the test's name for each test citing it in `C`, then one line `reads <glob>` for each distinct glob, in byte order, of the `clm:reads` markers in `C`'s text of the files holding those tests, each the text after `clm:reads ` up to the next whitespace, as `check.affected/affected-readers` reads a reader's glob, then `limit 20`. It SHALL run the command through `sh -c` in the verify tree, with `CLM_MUTATE` naming the plan, `CLM_MUTATE_RESULT` naming an empty file, `CLM_SLOT` set to one more than the clone's `max_builds`, a number that, with the `mutate_jobs` less one numbers after it, no build place and no check holds, so the tests it runs may overlap any other run (`tests-run-concurrently`), `CLM_MUTATE_JOBS` set to the whole number of 2 GiB (2097152 kB) in the `MemAvailable` line of the memory file its call is given, at least 1 and at most `[verify] mutate_jobs`, or to 1 when `mutate_jobs` is unset or that file or line can't be read, `CLM_LAND_DIR` and the `[env]` variables as the check gets them, `CLM_SELECTION` unset whatever the caller's environment holds, and standard input empty, its output copied to stderr, in a process group of its own. `Call::standard(stop)` SHALL give the memory file `/proc/meminfo`, a minute of 60 seconds and a grace of 10 seconds, with `stop` as its stop request. With `[verify] mutate_minutes` set, when the command has run that many of its call's minutes it SHALL send `SIGTERM` to that group, and `SIGKILL` when the command hasn't ended after its call's grace, and print `mutation: stopped at its limit of <m> minutes` on stderr; when its call's stop request is set while the command runs, it SHALL end the command the same way and print `mutation: stopped` on stderr. However the command ends, it SHALL then read from the result file the lines that end in a newline, one per requirement, the last for a requirement holding: `<id> killed`, the tests failed for at least one mutant; `<id> survived <n>`, `<n>` mutants were tried and the tests passed for every one; `<id> not-judged`; or `<id> failed`, alone or followed by a space and text to the line's end, the command's block for the requirement having failed (`ci/mutate-script`), which leaves the requirement, as no line does, and gives it that text as its *why*, empty when there is none; a line of `survived 0` is not judged, a line that doesn't parse and one naming a requirement that isn't due are ignored, and a due requirement with no line is left. It SHALL then append one `mutated` record naming `C`, `P` as `since`, the killed and not-judged IDs, the survived ones as `unproven` entries at their versions in `C`, the left ones under `left` in the plan's order, each left one whose line was `failed` under `failed` with its why, and, when the command didn't exit 0, how it ended under `ended`: `stopped` when the stop request ended it, `limit <m>` when `mutate_minutes`, `<m>`, did, `status <n>` when it exited with status `<n>` without being ended so, and `signal <n>` when a signal `<n>` the pass didn't send ended it (`ledger/mutated-record`), and print `mutation: <k> killed, <u> unproven, <n> not judged` on stderr, followed by `, <l> left` when any is left. With nothing due it SHALL run no command and append the record with empty lists and no `ended`. When the command exits non-zero without being ended so, it SHALL print `mutation: <command> failed with status <n>` on stderr before the record's line, and when a signal the pass didn't send ends it, `mutation: <command> was ended by signal <n>`, followed by ` (<name>)` when it is one of `SIGHUP`, `SIGINT`, `SIGQUIT`, `SIGILL`, `SIGTRAP`, `SIGABRT`, `SIGBUS`, `SIGFPE`, `SIGKILL`, `SIGUSR1`, `SIGSEGV`, `SIGUSR2`, `SIGPIPE`, `SIGALRM`, `SIGTERM`, `SIGCHLD`, `SIGCONT`, `SIGSTOP`, `SIGTSTP`, `SIGTTIN`, `SIGTTOU`, `SIGURG`, `SIGXCPU`, `SIGXFSZ`, `SIGVTALRM`, `SIGPROF` and `SIGSYS`, `<name>` that name, and with nothing after the number for any other signal; either way it SHALL still append the record, so the verdicts it reached are kept and the rest are left. Any other failure of the pass, such as a scratch directory it can't make in `TMPDIR`, a git command or file operation that fails, or the record's append failing, SHALL end it with `mutation: <why>; nothing recorded` on stderr and no `mutated` record. The pass SHALL NOT change the outcome line, anything on stdout or the exit status, and SHALL NOT run when the check failed, in `import` mode, for a result answered from the ledger, for a bisect, or with `mutate` unset.

#### Scenario: a new test that notices
- GIVEN `mutate` set to a fake that copies its plan and writes `a/x killed`, a passing check, a `mutated` record for the last verified commit, and one land since that added `tests/x.rs` with a test `proves_x` citing `a/x` and changed `src/a.rs`, both `a`'s
- WHEN `clm verify --due` runs
- THEN the plan holds `requirement a/x`, `code src/a.rs`, `code tests/x.rs`, a `diff` line whose file names `src/a.rs`, `test tests/x.rs` with `proves_x` and `limit 20`; the ledger gains a `mutated` record with `a/x` under `killed`; stdout is the one line `verify passed <sha> 1 lands`; and it exits 0

#### Scenario: a test that notices nothing
- GIVEN the same with a fake that writes `a/x survived 12`
- WHEN `clm verify --due` runs
- THEN the `mutated` record holds `a/x@<version>` under `unproven`, stderr holds `mutation: 0 killed, 1 unproven, 0 not judged`, and it still prints `verify passed` and exits 0

#### Scenario: the first pass fails, and the next loses nothing
- GIVEN no `mutated` record, a land `S` that set `mutate`, a land `L` after it that added a test citing `a/x` and changed `a`'s code, a `mutate` command that writes nothing and exits 3, and `clm verify --due` run once at `L`, which records a `mutated` record for `L` whose `since` is the first parent of `S` and whose `left` holds `a/x`
- WHEN a further land adds a test citing `a/y`, the command is replaced by one that copies its plan, and `clm verify --due` runs again
- THEN the plan holds the block for `a/x`, whose `diff` file holds `L`'s change to `a`'s code, and then the one for `a/y`, and the new `mutated` record's `since` is `L`

#### Scenario: a test that reads the repository's files
- GIVEN the land of the first scenario, with `tests/x.rs` also holding the markers `clm:reads specs/**` and `clm:reads ARCHITECTURE.md`, and another file of `a`'s holding `clm:reads docs/**` but no test citing `a/x`
- WHEN `clm verify --due` runs with a fake that copies its plan
- THEN the block for `a/x` holds `reads ARCHITECTURE.md` and then `reads specs/**`, after its `test` line and before `limit 20`, and no `reads docs/**`

#### Scenario: the pass switched on from the first commit
- GIVEN no `mutated` record and a chain whose first commit sets `mutate` and holds a test citing `a/x` at its current version
- WHEN `clm verify --due` runs with a passing check
- THEN the plan holds a block for `a/x`, with no `diff` line

#### Scenario: only re-cited
- GIVEN a land since the last pass that changed only the version in the marker of the test citing `a/x`
- WHEN `clm verify --due` runs
- THEN the command isn't run, and the `mutated` record's lists are empty

#### Scenario: an item resolved with its test unchanged
- GIVEN a backlog item naming `a/x` under `unproven` at the last pass's commit, deleted by a land since, with the tests unchanged
- WHEN `clm verify --due` runs
- THEN the command isn't run for `a/x`, so a build that found the test sound settles it in one look

#### Scenario: its own slot
- GIVEN `max_builds` 4 and a fake `mutate` that records `CLM_SLOT`
- WHEN the pass runs it
- THEN it recorded `5`

#### Scenario: the command fails part-way
- GIVEN `a/x`, `a/y` and `a/z` due, and a `mutate` command that writes `a/x killed` and exits 3
- WHEN `clm verify --due` runs with a passing check
- THEN the `mutated` record holds `a/x` under `killed` and `a/y` and `a/z` under `left`, stderr names the command and status 3 and then holds `mutation: 1 killed, 0 unproven, 0 not judged, 2 left`, and it prints `verify passed` and exits 0

#### Scenario: a line cut short
- GIVEN `a/x` and `a/y` due, and a `mutate` command that writes `a/x killed`, a newline and `a/y survived 1` with no newline, and exits 3
- WHEN `clm verify --due` runs
- THEN the record holds `a/x` under `killed` and `a/y` under `left`

#### Scenario: the time limit
- GIVEN `mutate_minutes = 1`, a call whose minute is one second and whose grace is one second, `a/x` and `a/y` due, and a `mutate` command that writes `a/x killed`, starts a child that sleeps, and then ignores `SIGTERM` and sleeps
- WHEN the pass runs
- THEN within a few seconds the command and its child no longer run, the record holds `a/x` under `killed` and `a/y` under `left`, stderr holds `mutation: stopped at its limit of 1 minutes`, and the outcome is unchanged

#### Scenario: a stop request
- GIVEN `a/x` and `a/y` due, and a `mutate` command that writes `a/x killed` and then waits until a file appears
- WHEN the call's stop request is set once the line is written
- THEN the command no longer runs, the record holds `a/x` under `killed` and `a/y` under `left`, and stderr holds `mutation: stopped`

#### Scenario: what is left goes first
- GIVEN the newest `mutated` record's `left` holding `a/z`, and a land since that added a test citing `a/a`
- WHEN `clm verify --due` runs with a fake that copies its plan
- THEN the plan's block for `a/z` comes before the one for `a/a`

#### Scenario: jobs follow memory
- GIVEN `mutate_jobs = 6` and a fake `mutate` that records `CLM_MUTATE_JOBS`
- WHEN the pass runs with a memory file whose `MemAvailable` reads 5242880 kB, then one whose reads 41943040 kB, then one with no such line, then with `mutate_jobs` unset
- THEN it recorded `2`, then `6`, then `1`, then `1`

#### Scenario: unset
- GIVEN a `[verify]` with no `mutate`
- WHEN `clm verify --due` runs with a passing check
- THEN no `mutated` record is appended and stderr holds no `mutation:` line

#### Scenario: a red check
- GIVEN `mutate` set and a check that fails
- WHEN `clm verify --due` runs
- THEN the command isn't run

#### Scenario: lands don't wait
- GIVEN a `mutate` command that waits for a file to appear
- WHEN `clm verify --due` is in its pass and a land of the clone runs its check
- THEN the land's check starts before the file appears

#### Scenario: the standard call
- GIVEN a stop flag
- WHEN `Call::standard` is called with it
- THEN the call's memory file is `/proc/meminfo`, its minute 60 seconds, its grace 10 seconds, and its stop request that flag

#### Scenario: a command killed by a signal
- GIVEN `a/x` and `a/y` due, and `mutate` set to a command line that writes `a/x killed` to its result file and then runs `kill -9 $$`, so the shell `sh -c` runs it in is killed
- WHEN `clm verify --due` runs with a passing check
- THEN stderr holds `mutation: <command> was ended by signal 9 (SIGKILL)` and then `mutation: 1 killed, 0 unproven, 0 not judged, 1 left`, the record holds `a/x` under `killed` and `a/y` under `left`, and it prints `verify passed` and exits 0

#### Scenario: lines that aren't verdicts
- GIVEN `a/x` due and `a/q` not, and a `mutate` command that writes `a/q killed`, `a/x maybe` and `a/x survived many`, each ending in a newline
- WHEN `clm verify --due` runs with a passing check
- THEN the record names no `a/q` and holds `a/x` under `left`

#### Scenario: no selection for the pass
- GIVEN `a/x` due, `CLM_SELECTION=/x` in the caller's environment, and a fake `mutate` that records `${CLM_SELECTION-unset}` and what it reads from standard input
- WHEN the pass runs it
- THEN it recorded `unset` and read nothing

#### Scenario: a capability citation
- GIVEN a `mutated` record for the last verified commit, and one land since that adds a test citing only `capability/c` at its current version
- WHEN `clm verify --due` runs with a passing check and a fake `mutate` that records that it ran
- THEN the command isn't run, and the new `mutated` record's lists are empty

#### Scenario: a pass that can't run
- GIVEN `a/x` due and `TMPDIR` naming a directory that doesn't exist
- WHEN `clm verify --due` runs with a passing check
- THEN stderr holds a line starting `mutation: ` and ending `; nothing recorded`, no `mutated` record is appended, and it prints `verify passed` and exits 0

#### Scenario: a block's tool fails
- GIVEN `a/x` and `a/y` due, and a `mutate` command that writes `a/x failed StrykerJS ran no test` and exits 1
- WHEN `clm verify --due` runs with a passing check
- THEN the `mutated` record holds `a/x` and `a/y` under `left`, `a/x` alone under `failed` with the why `StrykerJS ran no test`, and `ended` `status 1`, and it prints `verify passed` and exits 0

#### Scenario: how the pass ended
- GIVEN the commands and calls of *the time limit*, *a stop request* and *a command killed by a signal*, and a command that writes `a/x killed` and `a/y killed` and exits 0
- WHEN the pass runs each
- THEN the four records' `ended` are `limit 1`, `stopped`, `signal 9` and none, and none holds anything under `failed`

#### Needs
- config/verify-setting
- ledger/mutated-record
- check.coverage/test-spans
- model/parse-backlog

#### Serves
- architecture/proven-by-tests
- capability/prove-by-tests

### Requirement: files-unproven <!-- base: d8c36be490c3f887 -->
At the end of every `clm verify --due` without `--dry-run`, in `local` mode, whose settings set `[verify] mutate`, whether or not a run was due, and after the mutation pass when one ran, unless the verification ended in a refusal or an error, it SHALL, for each `mutated` record that names a commit `R` on `origin/main`'s first-parent chain and holds `unproven` entries, and for each component those entries name, file the item `backlog/unproven-<component>-<short sha of R>.md` unless a commit after `R` on that chain added a file `backlog/unproven-<component>-<p>.md`, `<p>` a prefix of `R`'s full commit ID at least 7 characters long, so an item a build has since deleted is never filed again, whatever length `git rev-parse --short` gave `R` when the item was filed, or none of the component's entries names a requirement at its current version in `origin/main`. It SHALL also, taking for each commit on that chain that a `mutated` record names the newest such record by `at`, a record *passes over* a requirement when its `left` names it, its `failed` doesn't, and either its `ended` is `stopped` or its `failed` names another requirement, and *counts it against* the requirement when its `left` names it and it doesn't pass over it; for each requirement of `origin/main` that the newest of those records not passing over it counts against, find the run of those records, newest commit first, that count it against it, leaving out those that pass over it and ending at the first record that does neither, so two passes on one commit count as one, and neither a pass the stop request ended nor one that left the requirement only because another requirement's block failed adds to the run or ends it, and the item goes to the component whose block failed; when the run holds two or more records, `R'` being the commit of the second oldest of the run, it SHALL file, for each component and `R'`, the item `backlog/unjudged-<component>-<short sha of R'>.md` unless a commit after `R'` on that chain added a file `backlog/unjudged-<component>-<p>.md`, `<p>` a prefix of `R'`'s full commit ID at least 7 characters long. It SHALL write the items to file in a temporary worktree on a branch `unproven/<short sha of origin/main>` cut from `origin/main`, each with `title` `Tests of <component> notice no change to its code`, `kind: bug`, `components` that component, `unproven` the entries still at their current version, and a body naming each entry's citing tests and `R`, and each `unjudged` item with `title` `The mutation pass can't judge requirements of <component>`, `kind: bug`, `components` that component, and a body naming each of its requirements and, for each record of its run, its commit and how that pass left it: `its block's tool failed`, followed by `: <why>` when its why isn't empty, when the record's `failed` names the requirement, and otherwise `the pass reached its limit of <m> minutes` for `ended` `limit <m>`, `the command failed with status <n>` for `status <n>`, `the command was ended by signal <n>` for `signal <n>`, and `no reason recorded` when the record has no `ended`; commit them; push the branch and land it with `clm land`, as any branch lands; and delete the branch, here and on `origin`, whether or not it landed. It SHALL print `unproven: filed <n> items for <m> requirements` on stderr when it landed `unproven` items, and `unjudged: filed <n> items for <m> requirements` when it landed `unjudged` ones, each counting its own kind, and `unproven: not filed: <reason>` when any step of the filing failed, leaving the items for the next call. When the call's stop request (`mutation-pass`) is set once it has found an entry still at its current version or an `unjudged` item to file, it SHALL start none of the steps that write and commit the items, push the branch or land it that it hasn't started yet, judging the request before each, delete the branch here and on `origin` when it made one, and print `unproven: not filed: stopped`, leaving the items for the next call. It SHALL NOT put its `git` commands or its land in a process group of their own, as the mutation pass puts its command, so a signal sent to `clm verify`'s process group, as a Ctrl-C at a terminal is, reaches the one running then; a step whose command a signal ends fails as any failed step does, and the branch is deleted here and on `origin` as after any failure. It SHALL NOT change the outcome line, stdout or the exit status. `clm verify --commit`, `--import` and `--bisect`, and `pipeline` and `import` mode, file nothing.

#### Scenario: filed and read as not built
- GIVEN a `mutated` record for `origin/main`'s head `R` with `unproven` `a/x@<current version>`, and `a/x` current
- WHEN `clm verify --due` runs with nothing due, and then `clm coverage` runs on the new `origin/main`
- THEN `origin/main` holds `backlog/unproven-a-<short sha of R>.md`, a `bug` item on `a` with `unproven: [a/x@<version>]`, no branch `unproven/<short sha>` is left on `origin`, and coverage prints `a/x unbuilt`

#### Scenario: already filed
- GIVEN the same once the item is on `origin/main`
- WHEN `clm verify --due` runs again
- THEN it lands nothing

#### Scenario: reworded since
- GIVEN a `mutated` record with `unproven` `a/x@11111111`, and `a/x` at another version on `origin/main`
- WHEN `clm verify --due` runs
- THEN it lands nothing

#### Scenario: resolved by a build
- GIVEN that item filed, and then deleted by a land
- WHEN `clm verify --due` runs again
- THEN it lands nothing

#### Scenario: resolved under a shorter name
- GIVEN one `mutated` record for a commit `R` holding `a/x@<current version>` under `unproven` and `a/y` under `left`, and one for an earlier commit `Q` holding `a/y` under `left`, with `a/y` existing in `origin/main`; a land after `R` that added `backlog/unproven-a-<first 7 of R>.md` and `backlog/unjudged-a-<first 7 of R>.md`, and a later land that deleted both; and the clone's `core.abbrev` set to 12
- WHEN `clm verify --due` runs with nothing due
- THEN it lands nothing

#### Scenario: the land fails
- GIVEN an entry to file and an `origin` that refuses the push
- WHEN `clm verify --due` runs
- THEN stderr holds `unproven: not filed:` with the reason, the exit status is what the verification gave, and a later call files it

#### Scenario: left by two passes in a row
- GIVEN `mutated` records for two commits `Q` and then `R` of `origin/main`'s first-parent chain, each holding `a/x` under `left`, `a/x` existing in `origin/main`
- WHEN `clm verify --due` runs with nothing due, and again once a third record, for a later commit `S`, holds `a/x` under `left` too
- THEN after the first, `origin/main` holds `backlog/unjudged-a-<short sha of R>.md`, a `bug` item on `a` whose body names `a/x`, and stderr holds `unjudged: filed 1 items for 1 requirements` and no line starting `unproven: filed`; the second lands nothing

#### Scenario: left by one pass
- GIVEN the same with `a/x` under `killed` in the record for `Q`
- WHEN `clm verify --due` runs with nothing due
- THEN it lands nothing

#### Scenario: pipeline mode
- GIVEN `mode = "pipeline"` and an entry to file
- WHEN `clm verify --due` runs
- THEN it lands nothing

#### Scenario: a dry run files nothing
- GIVEN an entry to file for `origin/main`'s commit `R`
- WHEN `clm verify --due --dry-run` runs; and, after one more land whose `clm.toml` adds `[env]` with `CLM_X = "1"`, `clm verify --due` runs with a trigger holding
- THEN the first exits as `verify-due` says and the second exits 1 naming `env`, neither prints an `unproven:` line, and `origin/main` gains no item under `backlog/`

#### Scenario: stopped before it files
- GIVEN an entry to file for `origin/main`'s commit `R`, and the call's stop request set
- WHEN `clm verify --due` runs with nothing due, and then again with the stop request unset
- THEN the first prints its outcome and exits as `verify-due` says, stderr holds `unproven: not filed: stopped`, `origin/main` gains no item and `origin` holds no branch `unproven/<short sha>`; after the second, `origin/main` holds `backlog/unproven-a-<short sha of R>.md`

#### Scenario: a filing command ended by a signal
- GIVEN an entry to file for `origin/main`'s commit `R`, and a root `clm.toml` whose `[land] check` sends `SIGINT` to its own shell (`kill -INT $$`)
- WHEN `clm verify --due` runs with nothing due, and then again after one more land whose `clm.toml` sets a `[land] check` that exits 0
- THEN after the first, stderr holds `unproven: not filed:` with the reason, the exit status is what the verification gave, `origin/main` gains no item and neither the clone nor `origin` holds a branch `unproven/<short sha>`; after the second, `origin/main` holds `backlog/unproven-a-<short sha of R>.md`

#### Scenario: a block that fails starves the rest
- GIVEN `mutated` records for two commits `Q` and then `R` of `origin/main`'s first-parent chain, each holding `a/x` and `b/y` under `left`, `a/x` under `failed` with the why `StrykerJS ran no test`, and `ended` `status 1`, `a/x` and `b/y` existing in `origin/main`
- WHEN `clm verify --due` runs with nothing due
- THEN `origin/main` holds `backlog/unjudged-a-<short sha of R>.md`, whose body names `a/x`, `Q`, `R` and `its block's tool failed: StrykerJS ran no test`, and no file whose name starts `backlog/unjudged-b-`

#### Scenario: a limit reached twice
- GIVEN `mutated` records for two commits `Q` and then `R` of `origin/main`'s first-parent chain, each holding `a/x` under `left` and `ended` `limit 60`, `a/x` existing in `origin/main`
- WHEN `clm verify --due` runs with nothing due
- THEN `origin/main` holds `backlog/unjudged-a-<short sha of R>.md`, whose body names `a/x` and `the pass reached its limit of 60 minutes`

#### Scenario: stopped twice
- GIVEN `mutated` records for two commits `Q` and then `R` of `origin/main`'s first-parent chain, each holding `a/x` under `left` and `ended` `stopped`, `a/x` existing in `origin/main`
- WHEN `clm verify --due` runs with nothing due, and again once a third record, for a later commit `S`, holds `a/x` under `left` with `ended` `status 1` and nothing under `failed`
- THEN the first lands nothing; after the second, `origin/main` holds no item for `a`, since the run holds `S` alone

#### Scenario: a stop between two failures
- GIVEN `mutated` records for three commits `Q`, `R` and `S` of `origin/main`'s first-parent chain, each holding `a/x` under `left`, `Q` and `S` with `ended` `status 1` and `R` with `ended` `stopped`, `a/x` existing in `origin/main`
- WHEN `clm verify --due` runs with nothing due
- THEN `origin/main` holds `backlog/unjudged-a-<short sha of S>.md`, whose body names `Q` and `S` and not `R`

#### Needs
- ci/mutate-script
- ledger/mutated-record
- model/parse-backlog
- check.coverage/coverage-report
- orchestrator.triage/triage

#### Serves
- architecture/proven-by-tests
- capability/prove-by-tests
