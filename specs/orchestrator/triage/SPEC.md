---
title: Triage
status: active
owners: ["@clunny"]
depends_on: [orchestrator, model, check, vcs, land, host, ledger, verify]
paths: ["crates/clm-triage/**"]
successor: ""
serves: [capability/triage-unattended]
---

## Contract
`clm triage` groups the backlog's untaken items into units of design work and prints them with the proposed spec work already on `origin` and the items waiting on the human, each line with its priority and the number of requirements it would release, sorted most urgent first. `clm draft <slug>...` claims those items through `orchestrator`'s backlog claim and writes the prompt for a session that designs one change from them unattended to a file, printing a short launcher that names it (`build-prompt-file`); the session lands the change with `clm land` when it needs no review, and leaves one that needs review on its branch with no pull request. `clm draft --resume <change-id>` writes, the same way, the prompt for a session that continues a draft whose session ended before it returned, on the spec branch it pushed (`draft-resume`), for the build loop (`orchestrator.loop`). `clm draft --fold <id>...` writes, the same way, the prompt for one session that folds the changes needing review that a triage run's drafts left into one spec branch and opens one pull request for it, so the human approves them once (`backlog-triage`, `yes-no-review`). The `triage` skill gives each launcher to a background subagent of the human's session in its own worktree, as `build` gives `clm run`'s. Each draft holds one of the clone's `max_builds` places (`orchestrator`'s `build-places`), named by the change ID `clm draft` chooses for it, from when `clm draft` starts it until it lands, is finished or its claims lapse; `clm draft --finish <change>`, which the `triage` skill runs whenever a draft's subagent returns, frees it, and the skill launches drafts only while a place is free. A fold takes no place. `clm triage` prints every group whatever the places. `clm intake` matches the host's inbox to the backlog: for each open issue it says whether a backlog item takes it in, whether a landed change or fix has resolved the item that did, or whether it is new, it lists the open pull requests from forks, and it matches the host's open dependency alerts to the backlog the same way, by an item's `alerts`, so the `triage` skill can take the inbox in before it drafts (`issues-enter-through-the-backlog`). After the host's lines it lists the red `verified` results on `main` that no green one has followed, each with its culprit land and components, matched to the backlog by an item's `verify_failures`, and, after each such result no item has taken in, the path of the log its run kept when this clone holds one (`verify`'s `kept_log`), so a slow test that failed after the land reaches triage as a bug on the components it broke, with the failure's own output for the human to judge it by (`verify-after-land`). All of them fetch `origin` first and read git and the clone's places only, `clm intake` also reading the clone's kept verify logs and `host`'s list of open issues and pull requests and its open dependency alerts, a failed read of the alerts printing `alerts unavailable` rather than failing the command, and exit 0 on success, 1 when the work isn't possible (with the reason), and 2 when git fails; `clm draft` also exits 3 when the clone has no free place.

## Design
Crate `clm-triage`. The queue, the claims and what holds an item are `orchestrator`'s (`taken_items`, `claim`); the items and their `blocks` are `model`'s; the tree is `origin/main`'s, extracted as `clm next` extracts it; the landing the prompt describes is `land`'s, so this component waits for `land` to settle. This component owns the grouping rule, the draft budget, and the draft and fold prompts' text.

**`clm triage`.** After fetching, it reads `origin/main`'s backlog and the taken items. It prints, sorted, one line per unit of work, in eight forms. A change `taken_items` reports as *proposed* (on `origin/spec/<id>` or `origin/autosync/spec/<id>`) or *on main* takes one of three, each `<form> <id> [<components>] <P> <n> <slug>...` with the items it names that are still in `origin/main`'s `backlog/`, sorted by slug, and its components as `clm proposals` gives them; a named item stays on the line while a claim also holds it, as it does in `clm proposals`:
- `designed` for a proposed change whose `change.md`, read from `origin/spec/<id>` (else `origin/autosync/spec/<id>`), has `status: applied`, or, when that ref holds no `changes/<id>/change.md`, whose `changes/archive/<id>/change.md` there has `status: done` (`designed-archived`): its design is finished and it waits to land, on the human's approval when it needs review. The archived case is a rename change, which `clm rename` applies and archives on its own branch (`rename-by-replay`), and which `orchestrator` reports as proposed from that file (`orchestrator/archived-spec-work`); a term swap always needs review, so its `designed` line is how the triage skill comes to offer it for approval and fold it alone. `clm-triage` reads the status itself, with `git show <ref>:<path>` through `clm-git`, as `--fold` reads a change, so `orchestrator`'s `taken_items` needs no extra field;
- `open` for any other proposed change, one whose `change.md` (live, else archived) has another status or doesn't parse: still being designed;
- `main` for a change on main: applied, waiting to be built, and archived by the `clm rebase` after its build;
- `group <kind>:<components> [<components>] <P> <n> <slug>...` for each group of untaken items of kind `bug`; and the items with the same sorted `components` are one group, keyed `bug:<component>+<component>...` (`bug:` for items with no `components`). The kind is read with `matches!` on the kinds it groups and lists, never with an exhaustive `match`, so a kind the model adds later neither breaks this crate's build nor is drafted by mistake. The items are ordered by priority then slug. A group with more items than the draft budget, a constant in `clm-triage` (6), is cut in that order into groups of at most the budget, each cut keeping the key;
- `human <slug> [<components>] <P> <n>` for each untaken `question` or `human-task` item, which the `triage` skill drafts only once the human has answered it in its session (`--answers`, below);
- `idea <slug> [<components>] <P> <n>` for each untaken `idea` item, which no draft takes: the `triage` skill brainstorms it with the human in its own session, which ends by proposing it, deferring it or dropping it (`ideas-brainstormed`). An idea is design work whose worth and shape are the human's call, so a draft, which can't ask, would only guess at it.
- `deferred <slug> [<components>] <P> <n>` for each untaken `deferred` item, whose take-up waits on a condition its body names that the human judges, so a draft would only stop on it: the `triage` skill shows it to the human, who judges the condition and has it drafted, kept or dropped (`triage-owns-backlog-work`);
- `workaround <slug> [<components>] <P> <n>` for each untaken `workaround` item, a build's flag against a requirement (`workaround-flags`): evidence that `clm health` counts, never drafted unless the human asks, so the `triage` skill shows it to the human, who has it drafted, kept or dropped.

An untaken item of a kind no form takes appears on no line.

`<components>` are comma-separated and sorted, `[]` when empty. `<P>` is the highest priority among the line's items (`priority-caps`), `P5` when none sets one. `<n>` *releases*: the number of distinct requirement IDs named in the `blocks` of the line's items, whether or not the spec still holds them, which deleting the items would free (`requirement-blocks`). Lines sort by form (`group` first, so bugs lead the list; then `designed`, `open`, `main`, `human`, `idea`, `deferred` and `workaround` last), then `<P>` (`P1` first), then `<n>` descending, then the text after the form word, as strings. An item held only by a claim (`origin/propose/<slug>` within the expiry, named by no change) is left out of every form: a draft already has it. It takes `--expiry <hours>` (default 4).

**`clm draft <slug>...`** (`draft`, `draft-takes-place`). It creates `.clm/prompts/draft/` beside the exclude before taking a place or claiming anything (`draft-prompt-file`). The place is taken first under a provisional name unique to the invocation, `draft-<pid>-<first slug given>` with the process ID, since the change ID depends on which items it claims; that name is never reused, so a draft that claims nothing frees only its own place. It claims each item whatever its kind (the skill passes only `group` lines). The change ID is `<YYYYMMDD>-<words>`, `<words>` being the first claimed slug's first four `-`-separated words (all of them when it has fewer). The date comes from the clock and time zone the command is given (the injected environment, as `TMPDIR` is for `land`), so tests fix it and never flake at midnight. The rename to the final ID is `rename_place`, under the places lock. It keeps `.clm/` out of git through `.git/info/exclude`, as `clm run` does, and prints `draft <id>` as the last line on stderr, which the skill reads to finish the draft later. `clm draft` chooses the ID, rather than the session, so the place has its final name before the session starts and the skill knows what to finish whatever the draft's closing line says.

**Answers** (`answers-reach-drafts`, `draft-answers`). `clm draft --answers <file> <slug>...` carries what the human said in the triage session into the prompt: answers to `question` and `human-task` items, a policy for a set of items, or an instruction for every draft. A relative path is read against the directory it runs in, as the skill passes `.clm/answers.md` from the checkout it runs `clm` in. The read fails for any reason (missing, a directory, unreadable, not UTF-8) before anything is fetched or claimed; a file holding only whitespace adds nothing. The text goes in as read, trimmed at both ends, with nothing escaped, so the prompt file holds a snapshot that a later edit of the file doesn't change for a draft already running. An item whose answer would break `ARCHITECTURE.md`, a decision or a requirement is left, with `left <slug>: <reason>` naming the rule. Every answer the session used is quoted in `change.md`, so the answer is kept in the change and reaches the reviewer and the archive, never only the session; a fact a settled `question` or `human-task` item waited on goes into the specs as a requirement or decision, never left only in the quote. An answer that settles a `question` item but leaves only code to change, a requirement or scenario as written already stating what that code must do (`code-fix-needs-a-written-rule`) and no requirement the item blocks waiting to be built for it, takes one route that ends in the same draft: the spec branch turns the item into a `kind: bug` item that carries the answer, and once that branch has landed, the session fixes the bug by component as it fixes any code-only bug (`code-fix-resolves-bug`). The item is turned into a bug rather than resolved and filed again, so it keeps its slug, its claim and its `issues`, and no other session drafts it between the land and the fix. `clm claim --fix` keeps refusing `question` items: a build never closes a question, and only the human's answer, carried by a spec change, makes one fixable in code (`apply-deletes-answered-questions`, `deferred-work-resolves-in-code`, `settled-question-becomes-bug`). A branch that needs review stops the draft as any does, and the bug reaches the next `clm triage` once the fold has landed it. A question whose remaining work is building the requirements it blocks needs no such route: it stays in `resolves`, `clm apply` deletes it, and the builds of the requirements it released do the work. Without `--answers` the prompt has neither the section nor the steps' words about it. `--answers` is rejected as a usage error with `--finish` and `--fold`, as `--fold` with `--finish` is: a fold merges changes that already quote what they used.

**Prompt files** (`build-prompt-file`). A draft's prompt holds the items rather than a context bundle, but passing it inline would have the orchestrating session read and re-emit every group's prompt. So `clm draft` writes its prompt to `<root>/.clm/prompts/draft/<id>.md` and `clm draft --fold` to `<root>/.clm/prompts/fold/<first folded ID>.md`, `<root>` being the checkout it runs in; a component ID never contains `/`, so neither collides with `clm run`'s `.clm/prompts/<component>.md`. On stdout each prints only the launcher, a fixed template of at most five lines: read `<absolute path>` with the Read tool, all of it (in parts when it is too large to read at once), before anything else, and follow it as your instructions; it is the prompt for drafting `<id>` (or folding `<ids>`), and its first step is `git branch --show-current`. The skill passes the launcher unchanged as the Agent tool's prompt and the subagent, whose worktree is elsewhere, reads the file by that absolute path. The file stays in the orchestrator's checkout until a later run of the same ID replaces it; with no launcher printed nothing reads it.

**Cargo.** Neither prompt names `CARGO_TARGET_DIR`. A draft that takes the code-fix path compiles into its own worktree's `target/` (`build-places`): its place bounds how many such directories exist at once, and `clm land` deletes a linked worktree's `target/` after landing an `impl/*` branch from it. A fold changes only spec text and runs no cargo.

**`clm draft --finish <change>`** (`draft-finish`) frees the place `<change>`, printing `freed <change>` when there was one. A branch has landed by two tests: the place's (`origin/main`'s tree holds `changes/<change>/` or `changes/archive/<change>/`), or the local branch `spec/<change>` is landed as `clm wrap-up` judges a branch (its tip is an ancestor of `origin/main` and isn't on `origin/main`'s first-parent chain). The removal is forced and prints `removed <path>`, `kept <path>: uncommitted changes` or `removed <branch>`, as `clm run --finish` does for a build. A draft that landed itself had its `spec/<change>` deleted by `clm land`; a folded draft's branch is deleted here, since its change reached `main` on the fold's branch. A change that needs review, waiting for its fold or for approval, or a draft that stopped, keeps its branches and its worktree: only the place and the claims below go, so a waiting approval holds no place. A folded change reaches `origin/main` with the fold's branch, so running `--finish` on each folded draft after that land removes its worktree and branches. The fold's own branch `spec/<batch>` names no change, so the first test never finds it; the second does once `clm land` has merged it, and `clm draft --finish <batch>`, which the skill runs after the folded drafts', removes the fold's worktree and deletes its local branch (`clm land` already deleted it on `origin`), with `autosync/spec/<batch>` when there is one. The ancestry test is the rule `clm wrap-up` uses, built in `clm-triage` from `clm-git`'s `is_ancestor` and a first-parent `rev-list` through `clm_git::git`, as checkpoint builds its own; the rule is short and `clm-triage` doesn't depend on `clm-checkpoint`, so the two agree by rule, not by shared code. Under it a branch with a commit `origin/main` lacks, or one cut from `origin/main` with no commit of its own (its tip is on the first-parent chain), isn't landed, and keeps its worktree. The second test is `--finish`'s only: `clm draft` judges a change ID held by the tree test alone. When only the second test holds, `origin`'s branches are judged by their own tips, since the local tip is what passed; `autosync/spec/<change>` is deleted only when no worktree was kept, since a kept worktree's uncommitted work is what that snapshot holds. `clm wrap-up` removes only `impl/*` worktrees, so `--finish` is what removes a fold's worktree, and its `target/` if it has one. With one change the fold's branch is the draft's, `spec/<id>`, which the first test already finds. **Nothing outside history.** A worktree holds nothing that isn't in history when it has no unstaged change (`git diff --quiet`), no untracked file that isn't ignored, and an index whose tree (`git write-tree`) is the tree of a commit reachable from its `HEAD` (`git log --format=%T HEAD`); an unmerged index, on which `git write-tree` fails, keeps it. The test is not `git status --porcelain` being empty, because the draft's branch can move on while the draft's worktree still has it checked out: a single-change fold that checked out the draft's branch elsewhere, or a sync or land run in another worktree, commits to `spec/<change>`, and the draft's worktree, whose index is still its last commit's tree, then shows staged changes against the new `HEAD` though every byte of them is in history. Removing such a worktree loses nothing, so `--finish` removes every worktree that passes the test with `git worktree remove --force`, since git judges a moved one dirty, its ignored files (such as `target/`) going with it; the history is walked whole, which costs milliseconds on a clone's history. The fold is not barred from checking out a branch another worktree holds instead: the branch moves under the draft's worktree whichever command moves it, so the test belongs where the worktree is judged.

**Claims released** (`propose-claims`). A draft's `propose/<slug>` claims hold its items only until its spec branch holds them, and the session has returned by the time the skill runs `--finish`: each item the change resolves is held by its branch or by its change on `main`, an item fixed on a code branch is deleted, and an item the draft left is meant to be free again. Releasing the claims at once means a left item can be drafted again in the same triage session once the human has answered what it waited on, and a bug the draft moved to other components appears in the first `clm triage` after the draft's change landed, where a claim left to lapse would hide each for up to the expiry. So, reading the place before freeing it, `--finish` releases on `origin` each claim the place names (its `branch propose/<slug>` lines), reading each claim's tip and committer time from `origin/propose/<slug>` after the fetch through `clm_git::git`, as `orchestrator` reads claims (its claim reading is crate-private, and the read is one `for-each-ref`, so `orchestrator`'s API is unchanged), deleting each with `git push --force-with-lease=refs/heads/propose/<slug>:<sha> origin --delete propose/<slug>` on the SHA that read gave and printing `released propose/<slug>`, before `freed <change>`. Any git failure other than a claim already gone or a lease refusal exits 2. It keeps two:
- a claim whose tip's committer time is at or after the place's `taken` time plus the expiry (`taken + expiry_hours * 3600`, in Unix seconds): that claim is another session's, taken after the draft's own lapsed, since no session can claim an item while the draft's claim on it is live and the draft claims just after taking its place;
- while the draft's change waits, `origin/spec/<change>` existing with neither landed test holding, the claim of an item the change's `resolves` doesn't name whose `backlog/<slug>.md` that branch changes since its merge base with `origin/main` (`git diff --name-only origin/main...origin/spec/<change> -- backlog/`), such as a bug whose `components` the draft corrected: the edit reaches `main` only with the change, so an item drafted again before then would be drafted on its old components. That claim lapses at its expiry, since the place that names it is freed now and a waiting approval must hold no place; a draft that corrects components and lands on checks, the usual case, has landed by the time `--finish` runs and releases it at once.

The place's lines that start `propose/` are its claims; `spec/<change>` is not one. With no place, or one whose file doesn't parse, it releases nothing, since nothing else names the draft's claims; they lapse at their expiry. Releasing a claim on an item the change resolves changes nothing for it, since the change holds it (`taken_items`).

It makes no worktree, creates no branch and starts no session: the session cuts its own branch from `origin/main`, in the worktree Claude Code gives it (`builds-in-subagents`). It takes `--expiry <hours>` (default 4).

**`clm draft --fold <id>...`** (`fold`, `yes-no-review`). The skill passes the IDs in the order of the `group` lines they were drafted from. An ID whose `change.md` isn't `applied` is left since `clm rebase` re-applies only `applied` changes. A rename change is the exception (`fold-rename-alone`, `rename-by-replay`): `clm rename` applies and archives it on its own branch, so `origin/spec/<id>` holds `changes/archive/<id>/change.md`, `status: done` with its `renames`, and no live `change.md`. `--fold` reads it there when the live one is missing, testing `renames` from the frontmatter text with a small `serde_yaml` helper of its own, as `gate`'s `renames_of` does, since `model` keeps the raw key private and the test is one line; it folds it only alone, since the gate classifies a rename branch by replaying it (a branch that adds exactly one change recording `renames`, its tree what `clm rename` produces from the base), and any other change's commit on the branch would break the replay and make it a code branch. Alone, there is nothing to merge: the one-change prompt reads the change from the archive path and opens its pull request, and the triage session approves and lands it as it does a fold. Beside other changes it is left with a reason that says to fold it alone, which the skill does with a second `clm draft --fold <id>`. `--fold` is a flag over the positional arguments, which it reads as change IDs; it excludes `--finish` and ignores `--expiry`. The prompt file goes under the CLM repository it runs for (`--root`, as for `clm draft`). It takes no place, since the skill folds only after every draft has returned: the folded drafts' branches keep holding their items (*proposed*) until the fold's branch lands with their changes, which are then *on main*, and `clm draft --finish` then deletes the drafts' branches. No change is named after the fold's branch: each folded change keeps its ID, its `change.md` and its deltas, so one branch carries several changes, each of whose `change.md` the gate reads for `human_burdens`, `clm rebase` re-applies in ID order and `clm land` records together.

**Fold prompt.** It names the folded changes, each with its branch and its `resolves`, then the steps. With one change there is nothing to merge: check `git branch --show-current`; read the change from its branch (`git show origin/spec/<id>:changes/<id>/change.md`), report CLM defects with `clm defect` (`defect-reports`), write the pull request body and open the pull request, and end with `proposed <id> review <url>`. With several:
- **First step.** Check `git branch --show-current`; run `git switch -c spec/<batch> origin/main` with the fold branch's name, which the prompt gives in full; push it with `clm branch push` (`prompts-push-through-clm`).
- **Unattended preamble.** As the draft's, for changes, except that the fold never lands: the triage session lands the fold's branch after the human's approval, so the preamble says the session never pushes `main` without naming the command that does. A change that can't be folded without a decision only the human can make (two changes that contradict each other, say) is left out before anything of it is merged, and the final message carries `left <id>: <reason>` for it before its closing line; its branch stays as its draft left it, for `/clm:propose`.
- **The job.** Bring each folded change onto the branch as one commit, in the order given, keeping the branch linear: `clm rebase` rebases without keeping merge commits, so a merge's conflict resolutions would be lost at the next rebase. For each, write the draft's diff outside `specs/` to a file under `.clm/` with `git diff origin/main...origin/spec/<id> --output=<file> -- . ':(exclude)specs'`, apply it with `git apply --3way <file>`, and commit it as `Fold <id>`. A conflict (`backlog/`, `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`) keeps both changes' edits, merging the text where they overlap. `specs/` is never brought over: `clm rebase` regenerates it from `origin/main` and every change on the branch. Then `clm rebase` until its last line is `rebase ok`: it applies the changes in ID order, so a stale delta it reports is a later change modifying an element an earlier change on the branch also modifies, and the session merges the earlier change's edit, which sync prints, into the later change's delta and sets that delta's base from `clm hashes <component>` on `specs/` as it stands with the earlier changes applied: it restores `specs/` from `origin/main`, sets each earlier change to `status: proposed` and runs `clm apply` on it in ID order, reads the hash, then restores `specs/` from `origin/main` again and sets every folded change back to `status: applied` before committing, so sync regenerates `specs/` from the committed deltas. Then `clm check` and `clm gate --base origin/main`, whose first line is `spec review`, since the folded changes need review; its `gate/approval` is expected, as in the draft, and a `human-task` item the fold adds gets a burden in the change it concerns. Then the builder check (`blocked-by-backlog`) on the elements where two changes met, settled as a draft settles it. The session never pushes to, rebases or deletes a folded draft's branch.
- **Rebase points.** As the draft's.
- **Pull request** (`own-pr-body`). Before the pull request it reports CLM defects with `clm defect`. The body, written with `Write` to `.clm/pr-body.md`: `## Intent`, a line naming the folded changes followed by each change's own intent under its ID; `## Decisions` and `## Open questions`, each change's under its ID, with the elements where two changes met and how they were merged; `## CLM defects`, the `kind: bug` items the folded branches add and its own reports; and that the branch needs review, the human approving every folded change at once. The body also names the drafts' branches and says their changes reach `main` with this one. It opens the pull request with `clm pr open spec/<batch> --title <title> --body-file .clm/pr-body.md` and stops: the triage session asks the human for the one approval and lands the branch.
- **Final message.** One closing line: `proposed <batch> review <url>`, or `stopped: <reason>`. Any `left <id>:` lines come before it.

The fold prompt never names a superpowers skill, `gh pr create`, `--body `, `gh pr edit`, `clm watch`, `clm approve` or `clm land`.

**Prompt.** The claimed items, each with its kind, priority, components, `blocks` and body, then the steps:
- **First step.** Check `git branch --show-current`; run `git switch -c spec/<id> origin/main` with the `<id>` `clm draft` chose, which the prompt gives in full; write `changes/<id>/change.md` with `status: proposed`, `resolves` naming every claimed item, and `priority` the highest among them, left out when all are `P5`; commit and push it with `clm branch push`. The spec work then holds the items past the claims' expiry (`propose-claims`). The ID is never renamed after that.
- **Already fixed.** Before the job, the session checks each item against `origin/main`, since an item filed while another change was fixing the same thing outlives that change, which resolves only its own copy, and drafting it costs a whole session (context, a builder check, a pull request and a land) that ends by deleting it. It takes the date the item was filed from the author date, which `clm rebase`'s rebases keep, as `YYYY-MM-DD` (a change ID's date is `YYYYMMDD`). It reads the `## Intent` of each change under `changes/` and `changes/archive/` whose ID is dated no earlier than a week before that and that has a delta for one of the item's components or no delta at all (every change so dated when the item names no components); lists the code fixes landed since a week before that, `git log --since=<date> --grep='Resolves: backlog/' --format='%h %s' origin/main`; and checks whether the decisions, requirements and commands the item names already exist (`docs/decisions/`, `clm hashes <component>`). An item that one of those changes or fixes already fixes stays in `resolves`: the session deletes its `backlog/<slug>.md` on `spec/<id>`, names in `## Intent` what fixed it, and runs no `clm context` and designs nothing for it, as `propose` does for an item it finds already fixed. When the change has no delta to write and the branch edits `backlog/`, the session takes the backlog-only route (`draft-backlog-only-change`): it writes `## Intent`, one line of which says what the change does to the backlog; runs `clm check` until it passes and `clm apply <id>`; commits; runs `clm rebase` as the rebase points say; runs `clm gate --base origin/main`; and goes to the pull request and the land as for a change that needs no review (or stops as for one that does, should the gate's first line be `spec review`). The check is the session's judgment, not a gate: `clm draft` lists no candidates itself, since matching an item's text to its fix is judgment, and the few `git` reads cost the session little beside a full draft.
- **Unattended preamble.** No questions in the conversation. An item that can't be designed without a fact only the human knows or a step only they can take is left out of `resolves`, and the final message carries `left <slug>: <reason>` for it before its closing line; its claim is released when the draft is finished. Claude Code may ask the human to confirm a tool call, which the session waits for. When a write is refused, the session writes nothing another way and reports the item as left.
- **The job.** Read each item and `clm context <component>` for its components. Each item is judged by the principles as they stand, not as it was filed (`draft-restraint`); for a bug the first question is whether the design is wrong. The bugs whose fix is code-only (`draft-code-fix-written-rule`) and lies in one of the bug's `components` are fixed by component (`code-fix-resolves-bug`): `clm claim <component> --fix backlog/<slug>` for the component's first such bug, then on that `impl/<component>` branch each of the component's code-only bugs fixed test first and its `backlog/<slug>.md` deleted in a commit whose trailer block holds `Resolves: backlog/<slug>`, the repository's checks run as `CLAUDE.md` asks, `clm gate --base origin/main` printing `code`, the pull request opened and the branch landed with `clm land`; those items leave `resolves`, and the code-only bugs of a component whose fix claim is refused are left, each with `left <slug>: <reason>` naming the claim's reason, since a spec change has nothing to say about them. A claimed `deferred` item whose work is code-only and lies in one of its `components` takes the same route, since `clm claim --fix` takes a `deferred` item as it takes a bug (`draft-workaround-deferred`, `deferred-work-resolves-in-code`); the rest of the deferred work is designed. A code-only bug whose fix lies in no component the item names is filed on the wrong component (`draft-corrects-bug-components`); a plan may file one beside a sibling. When, after this, no item is left to design, the session takes the backlog-only route above. The rest become one change: the design written into `change.md` (`## Intent`, the problem, `## Approaches`, the outcome, constraints, and in its frontmatter `human_burdens`, one short line each for what the change spends of the human's money or time or can't undo, a `kind: human-task` item it adds among them, since a spec branch adding one with no burden is blocked by `gate/undeclared-burden`), one `deltas/<component>.md` per component with bases from `clm hashes <component>`, decisions as `docs/decisions/<name>.md`, backlog edits, `clm check` until it passes, `clm apply <id>`, commit, `clm rebase` until its last line is `rebase ok`, and `clm gate --base origin/main`, whose first line says whether the change needs review (`spec review`) or lands on checks (`spec`). A change that needs review also gets `gate/approval`, the gate exiting 1, until the human approves it; that is expected, and a burden is never removed to make the gate pass. Another first line (`code`, `mixed`) means a file outside the spec paths is on the branch, which the session takes off before running the gate again. A `check/priority-caps` report is an item left for the human, never a demotion. Then the builder check (`blocked-by-backlog`, `builder-check-model`): a fresh subagent reads the change as its builder, without this conversation, and lists its questions; the session settles what the repo answers in the change, gives a requirement that waits on another component's pending requirement a `#### Needs` block (`requirement-needs`), adds, for work a requirement relies on from another component that no requirement of that component states, that component's requirement, since a Contract or Design edit builds nothing, and names it in the relying requirement's `#### Needs` (`dependency-work-is-a-requirement`), turns facts and steps only a human can supply into `question` or `human-task` items whose `blocks` name the requirements that wait (`requirement-blocks`), and lists the rest under `## Open questions`, declaring a burden for each `human-task` item it adds. When the builder check changed the change (any edit to `changes/<id>/` or `backlog/`), the session runs `clm check`, commits, and runs `clm rebase` until its last line is `rebase ok`, which re-applies the edited deltas, since the change is `applied` by then and `clm apply` refuses it; then it runs `clm gate --base origin/main` again, whose first line the pull request and the land go by, and pushes as the rebase points say, as `propose` does after its builder check. The builder check stays after `clm apply`, so the builder reads the applied spec through `clm context`. The prompt doesn't mention superpowers at all: brainstorming is a conversation, and the session has none.
- **Rebase points.** After each commit the session runs `clm rebase`, resolves what it reports until `rebase ok`, and pushes with `clm branch push`. It reads `clm rebase`'s outcome from its last line, never its exit code.
- **Pull request and landing** (`own-pr-body`, `land-locally`). Before the pull request, or before stopping on a change that needs review, it reports CLM defects with `clm defect` (`defect-reports`), then goes by the first line of its last `clm gate` run. When it is `spec`, the session writes the body with `Write` to `.clm/pr-body.md`: `## Intent`, `## Decisions`, `## Open questions` and `## CLM defects`. It opens the pull request, corrects the body with `clm pr body .clm/pr-body.md`, and runs `clm land spec/<id>`, going by its last line: `landed <sha> ...` ends the draft; `gate/up-to-date` or a conflict means `clm rebase`, resolve, push and land again; a check failure is fixed and landed again. When it is `spec review` it opens no pull request and stops after its last push, its branch synced: the triage session folds every change needing review that its drafts left into one branch with one pull request (`yes-no-review`). The prompt never names `gh pr create`, `--body `, `gh pr edit`, `clm watch` or `clm approve`.
- **Nothing left to design.** Before deciding there is no spec change, the session reports the CLM defects it met with `clm defect` on `spec/<id>` (`defect-reports`): `clm defect` writes to the current branch, and once the branch is deleted a report has nowhere to go. A report is a backlog edit, so the branch then takes the backlog-only route above and lands it. When every item was fixed on a code branch or left and `spec/<id>` changes nothing but its `change.md`, there is no spec change: the session detaches at `origin/main`, deletes `spec/<id>` locally, and deletes `spec/<id>` and `autosync/spec/<id>` on `origin`, each with its own `clm branch delete <ref>` (`host/branch-delete`), so no empty change holds the items. A ref the clone never fetched fails only its own call, leaving the other to be deleted.
- **Final message.** One closing line: `landed <sha>`, the spec change's land, or the last fix branch's when there was no spec change; `proposed <id> review` for a change that needs review, left on its branch for the fold; or `stopped: <reason>`, which a draft whose items were all left ends with. Any `left` lines come before it. The same paragraph carries the rules on background work (`background-work-stopped`, here and in `orchestrator.run`), phrased for drafts: other drafts' and builds' processes are what a name match would also find; a turn that ends while the draft only waits on a command it started, such as its own `clm land spec/<id>`, ends with `waiting: <command>` and no closing line, since the triage session receives it as the draft's result (`plugin.triage/triage-waiting-drafts`); and the builder check is among the subagents to wait for.

**`clm intake`.** After fetching, it takes `host`'s `list` and, for each open issue, reads `origin/main`'s backlog with the model: an item whose `issues` names it makes the issue `imported`. Otherwise it walks `origin/main`'s history for deletions under `backlog/` (`git log --diff-filter=D --name-only` through `clm-git`), newest first, and reads each deleted file at its deleting commit's first parent with `git show`, parsing it with `model`'s `parse_backlog_item`, which checks nothing against a graph, since a deleted item may name components that are not in the graph; the first whose frontmatter `issues` names the issue makes it `resolved`, by the change on `origin/main`, live or archived, whose `resolves` or `from` names its slug, else by the deleting commit, as a code fix's `Resolves:` trailer deletes an item with no change (`code-fix-resolves-bug`). A deleted file that does not parse is skipped. Then it takes `host`'s `alerts` and matches each open alert the same way, by `alerts` instead of `issues`, both in the backlog and in the deleted items, so an issue and an alert of the same number never match each other's items. The lines carry numbers, slugs, change IDs, commits and logins only, never a title or body, so no stranger's text reaches the lines a session parses (`untrusted-text-read-only-verifier`); a new alert's line adds its severity, ecosystem, package and manifest path, each one word as `host` returns it (`dependency-alerts-through-intake`).

**Unattended steps run `clm` commands** (`prompts-push-through-clm`, `unattended-steps-run-clm`). A draft is a subagent under the human's permission mode. In auto mode Claude Code's classifier denied a draft's `clm land spec/<id>` as a merge without review, and denies raw force-pushes and remote deletes as destructive, so the human had to run them. Every prompt `clm draft` writes, the draft's and the fold's, pushes with `clm branch push`, deletes a remote branch with `clm branch delete`, and lands with `clm land`, and the project's allow rules, which `clm init` seeds (`init/init-seeds-permissions`), let them run. `clm land` is safe to allow because it is the gate: it refuses a `spec review` branch without the human's approval (`land.approval/approval-policy`). Handing the land to the triage session instead was rejected: that session meets the same classifier, so the step only moves. The `triage` skill checks the project's rules before the first draft (`plugin/unattended-permissions-check`).

## Requirements

### Requirement: triage
`clm triage` SHALL fetch `origin` and print, sorted as the Design says (`group` lines first, `deferred` lines next to last and `workaround` lines last), one `group` line per group of untaken `bug` items with the same sorted `components`, cut into groups of at most the draft budget; for each proposed or on-main change that names backlog items one `designed` line when it is proposed and its `change.md` has `status: applied`, one `open` line when it is proposed otherwise, and one `main` line when it is on main; one `human` line per untaken `question` or `human-task` item; one `idea` line per untaken `idea` item; one `deferred` line per untaken `deferred` item; and one `workaround` line per untaken `workaround` item (`workaround-flags`); each as `<form> <name> [<components>] <P> <n> <slug>...`, with `<P>` the highest priority of its items and `<n>` the number of distinct requirements in their `blocks`. An item held only by a claim, and an untaken item of a kind no form takes, SHALL appear on no line. It SHALL exit 0, and 2 when git fails.

#### Scenario: groups by kind and nodes
- GIVEN untaken items `a` and `b` of kind `bug` with `components: [check]`, `c` of kind `idea` with `components: [check]`, `d` of kind `deferred` with `components: [check]`, and `e` of kind `bug` with no `components`
- WHEN `clm triage` runs
- THEN it prints, in this order, `group bug: [] P5 0 e`, `group bug:check [check] P5 0 a b`, `idea c [check] P5 0` and `deferred d [check] P5 0`, and no other line

#### Scenario: budget cuts a group
- GIVEN eight untaken `bug` items `a` to `h` with `components: [check]`, where `g` has `priority: P2`
- WHEN `clm triage` runs
- THEN it prints two `group bug:check` lines, the first `P2` with `g a b c d e` and the second `P5` with `f h`

#### Scenario: releases and order
- GIVEN untaken `idea` item `q` with `components: [gate]` and `blocks: [gate/x, gate/y]`, untaken `idea` item `r` with `components: [check]` and no `blocks`, untaken `question` item `s` with `components: [model]` and `blocks: [model/z]`, and untaken `bug` item `t` with `components: [view]`
- WHEN `clm triage` runs
- THEN it prints, in this order, `group bug:view [view] P5 0 t`, `human s [model] P5 1`, `idea q [gate] P5 2` and `idea r [check] P5 0`

#### Scenario: open and taken work
- GIVEN `origin/spec/20260101-c` whose `change.md` has `status: proposed` and `resolves: [q]` with a delta for `check`, where `q` is an `idea` with `components: [model]` and `blocks: [model/z]`; `origin/propose/t` committed 1 hour ago, where `t` is a `bug`; and untaken `bug` item `u`
- WHEN `clm triage` runs
- THEN it prints a `group` line for `u` first, then `open 20260101-c [check,model] P5 1 q`, and no line names `t`

#### Scenario: designed and on main
- GIVEN `origin/spec/20260101-d` whose `change.md` has `status: applied` and `resolves: [x]` with a delta for `check`, `origin/main` holding `changes/20260101-m/change.md` with `status: applied` and `resolves: [y]` and a delta for `gate`, where `x` and `y` are untaken `bug` items with no `components`, and untaken `bug` item `u`
- WHEN `clm triage` runs
- THEN it prints, in this order, a `group` line for `u`, `designed 20260101-d [check] P5 0 x` and `main 20260101-m [gate] P5 0 y`

#### Scenario: deferred and workaround come last
- GIVEN `origin/main` whose component `check` has a requirement `w`, an untaken item `f` of kind `workaround` with `components: [check]` and `requirement: check/w@<version>`, where `<version>` is `w`'s current version, an untaken `deferred` item `k` with `components: [gate]`, an untaken `idea` item `i` with no `components`, and an untaken `bug` item `u`
- WHEN `clm triage` runs
- THEN it prints, in this order, a `group` line for `u`, `idea i [] P5 0`, `deferred k [gate] P5 0` and `workaround f [check] P5 0`

#### Needs
- model/parse-backlog

### Requirement: draft
`clm draft <slug>...` SHALL fetch `origin` and claim each item with the backlog claim, printing `left <slug>: <reason>` on stderr for each refused one, and SHALL exit 1 printing nothing on stdout when no item was claimed. Otherwise it SHALL choose the change ID `<YYYYMMDD>-<words>`, the local date and the first claimed slug's first four `-`-separated words, with `-2`, `-3` and so on added while `origin/main`, `origin` or one of the clone's places already holds that ID (`draft-takes-place`), the date taken from the clock and time zone it is given, keep `.clm/` out of git through `.git/info/exclude`, write the prompt to its file and print the launcher (`draft-prompt-file`), print `draft <id>` as the last line on stderr and exit 0. The prompt SHALL list each claimed item with its kind, priority, components, `blocks` and body; SHALL say first to check `git branch --show-current`, cut `spec/<id>` from `origin/main` with the ID it chose, write `change.md` with `status: proposed`, `resolves` naming the claimed items and `priority` the highest among them unless all are `P5`, commit and push it; SHALL have the unattended preamble with `left <slug>: <reason>`; SHALL say, before the job, to check each item against `origin/main`, with the date it was filed from `git log -1 --diff-filter=A --format=%as origin/main -- backlog/<slug>.md`, the `## Intent` of the changes under `changes/` and `changes/archive/` with a delta for one of its components or none, and an ID dated no earlier than a week before that, the commits since then with a `Resolves: backlog/` trailer, and the decisions, requirements and commands it names, and, for an item already fixed, to keep it in `resolves`, delete its `backlog/<slug>.md` on `spec/<id>`, name in `## Intent` what fixed it and design nothing for it, skipping the design and the builder check when no item is left to design; SHALL say to fix code-only bugs by component with `clm claim <component> --fix backlog/<slug>` and a `Resolves: backlog/<slug>` trailer, and to design the rest as one change, declaring in `human_burdens` what it spends of the human's money or time or can't undo, a `human-task` item it adds among them, giving, once either human layer holds an entry, every component it adds a `serves` list and a rule whose purpose differs from its component's a `#### Serves` block, since `clm check` fails a rule that serves nothing (`check/untraced`), with `clm hashes`, `clm check`, `clm apply`, `clm rebase` read by its last line `rebase ok`, `clm gate --base origin/main` and the builder check, which adds the requirement for work a requirement relies on from another component that no requirement of that component states, since a Contract or Design edit builds nothing, and names it in a `#### Needs`, and after which, when it changed the change, `clm check`, a commit, `clm rebase` until `rebase ok`, which re-applies the edited deltas, and `clm gate --base origin/main` again; SHALL say that a change needing review also gets `gate/approval` until the human approves it, which is expected, and never to remove a burden to make the gate pass; SHALL say, when the first line of its last gate run is `spec`, to write the body to `.clm/pr-body.md`, open the pull request with `clm pr open spec/<id> --title <title> --body-file .clm/pr-body.md` and land with `clm land spec/<id>`, and, when it is `spec review`, to open no pull request and stop after its last push; SHALL say to report CLM defects with `clm defect` before the pull request or the stop; SHALL say to delete `spec/<id>` and `autosync/spec/<id>` on `origin` when nothing is left to design, each with its own `clm branch delete <ref>` (`draft-backlog-only-change`); SHALL say to end with one line, `landed <sha>`, `proposed <id> review` or `stopped: <reason>`; and SHALL NOT name a superpowers skill, `gh pr create`, `--body `, `gh pr edit`, `clm watch`, `clm approve`, `clm significant`, `## Significant`, `gate: `, `HARD`, `SOFT`, `AUTO`, `CARGO_TARGET_DIR` or `exits 0` outside the items' own text. It SHALL make no worktree, create no branch and start no `claude` process. The prompt it writes SHALL name every command by the name `clm/renamed-commands` gives it, never by an old name that requirement keeps as an alias, and call a branch's pull request a pull request.

#### Scenario: items claimed and prompt written
- GIVEN untaken `bug` items `a` and `b` in `origin/main`, `a` with `components: [check]`, `priority: P3` and `blocks: [check/x]`, `CARGO_TARGET_DIR` unset, and a fake `claude` on PATH that records whether it ran
- WHEN `clm draft a b` runs
- THEN it exits 0, the last line on stderr is `draft <date>-a` with the local date as `YYYYMMDD`, `origin/propose/a` and `origin/propose/b` are claim commits, no local branch was created, `.git/info/exclude` holds `.clm/`, the fake never ran, and the prompt file lists `a` with `bug`, `P3`, `check`, `check/x` and its body, and contains `git branch --show-current`, `git switch -c spec/<date>-a`, `resolves`, `left <slug>`, `--diff-filter=A`, `changes/archive/`, `already fixes`, `clm claim <component> --fix backlog/<slug>`, `Resolves: backlog/<slug>`, `clm hashes`, `clm apply`, `rebase ok`, `clm gate --base origin/main`, `human_burdens`, `gate/undeclared-burden`, `serves`, `#### Serves`, `check/untraced`, `holds an entry`, `spec review`, `gate/approval`, `a burden is never removed`, `builds nothing`, `re-applies the edited deltas`, `.clm/pr-body.md`, `clm pr open spec/<id>`, `clm land spec/<id>`, `clm defect`, `clm branch delete <ref>`, `landed <sha>`, `proposed <id> review`, `stopped:`, and doesn't contain `spec/<id> autosync/spec/<id>`, `superpowers`, `gh pr create`, `--body `, `clm watch`, `clm approve`, `clm significant`, `## Significant`, `gate: `, `HARD`, `SOFT`, `AUTO`, `CARGO_TARGET_DIR` or `exits 0`

#### Scenario: one item refused
- GIVEN untaken item `a` and item `b` claimed by `origin/propose/b` committed 1 hour ago
- WHEN `clm draft a b` runs
- THEN it exits 0, stderr has `left b: ` naming `propose/b`, and the prompt file lists `a` and not `b`

#### Scenario: nothing claimed
- GIVEN item `b` claimed by `origin/propose/b` committed 1 hour ago
- WHEN `clm draft b` runs
- THEN it exits 1, prints nothing on stdout, and stderr names `propose/b`

#### Scenario: ID taken
- GIVEN untaken item `long-item-slug-with-many-words`, and `origin/main` holding `changes/archive/<date>-long-item-slug-with/` for the local date
- WHEN `clm draft long-item-slug-with-many-words` runs
- THEN it exits 0 and the last line on stderr is `draft <date>-long-item-slug-with-2`

#### Scenario: no old command names
- GIVEN untaken item `a` in `origin/main`
- WHEN `clm draft a` runs
- THEN the prompt file names no `clm` followed by one of the old subcommands `sync`, `checkpoint` or `page`, nor the word `page`

#### Needs
- clm/renamed-commands
- model/parse-changes
- model/parse-backlog
- host/branch-delete

### Requirement: draft-takes-place
`clm draft <slug>...` SHALL take a place with `take_place`, using the `max_builds` it is given and its `--expiry`, before claiming any item, and when none is free SHALL exit 3, print nothing on stdout, print on stderr a message naming `max_builds`, its value and the number of places held, and claim nothing. When it claims nothing it SHALL free the place. It SHALL take the place under a provisional name unique to the invocation and never reused, and when it claims nothing SHALL free only that place. It SHALL choose an ID that no change on `origin/main`, no `spec/*`, `autosync/spec/*` or `propose/*` ref on `origin` and no place of the clone holds, renaming the provisional place under the places lock and taking the next suffix when the name is taken meanwhile. When it exits 0, the clone's place SHALL be `<id>` for the change ID it printed, holding `kind draft`, `branch spec/<id>` and one `branch propose/<slug>` per claimed item, and no other place SHALL be left by it.

#### Scenario: cap reached
- GIVEN untaken item `a`, `max_builds` 1, and one place that counts
- WHEN `clm draft a` runs
- THEN it exits 3, stdout is empty, stderr names `max_builds` and `1`, and `origin` has no `propose/a`

#### Scenario: place named by the change
- GIVEN untaken items `a` and `b`, and no places
- WHEN `clm draft a b` runs
- THEN the clone has one place, named for the ID on stderr's last line, with `kind draft`, `branch spec/<id>`, `branch propose/a` and `branch propose/b`

#### Scenario: nothing claimed frees the place
- GIVEN item `b` claimed by `origin/propose/b` committed 1 hour ago, and no places
- WHEN `clm draft b` runs
- THEN it exits 1 and the clone has no place

#### Scenario: ID held by a place
- GIVEN untaken item `q`, and a place `<date>-q` of the clone for the local date
- WHEN `clm draft q` runs
- THEN the last line on stderr is `draft <date>-q-2`, and the place `<date>-q` is unchanged

#### Scenario: refused draft frees only its own place
- GIVEN item `b` claimed by `origin/propose/b` committed 1 hour ago, and a place `draft-1-b` left by another invocation that counts
- WHEN `clm draft b` runs
- THEN it exits 1 and the place `draft-1-b` is still there

#### Serves
- capability/triage-unattended
- capability/work-concurrently

#### Needs
- config/max-builds-setting

### Requirement: draft-finish
`clm draft --finish <change>` SHALL fetch `origin`, release the draft's claims as below, free the place `<change>`, and, when `origin/main`'s tree holds `changes/<change>/` or `changes/archive/<change>/`, or the local branch `spec/<change>` exists and its tip is an ancestor of `origin/main` that isn't on `origin/main`'s first-parent chain (as for a fold's branch `spec/<batch>` once it has landed), remove each worktree under the main worktree's `.claude/worktrees/` that has `spec/<change>` checked out and holds nothing outside history, unlocking it first and removing it with `--force`, printing `removed <path>`, and print `kept <path>: uncommitted changes` for one that holds something else; it SHALL then delete the local `spec/<change>` when no worktree has it checked out, and `spec/<change>` and `autosync/spec/<change>` on `origin`, each leased on the tip it read, printing `removed <branch>` for each and skipping one that doesn't exist. A worktree SHALL count as holding nothing outside history when it has no unstaged change, no untracked file that isn't ignored, and an index whose tree is the tree of a commit reachable from its `HEAD`, even when that commit isn't `HEAD`. When only the second test holds, it SHALL delete `spec/<change>` on `origin` only when that branch's tip is an ancestor of `origin/main`, and `autosync/spec/<change>` only when it kept no worktree. When neither holds it SHALL remove no worktree and push or delete no branch. The second test SHALL NOT change how `clm draft` judges a change ID held. When the place `<change>` exists and parses, it SHALL delete on `origin` each `propose/<slug>` the place names, leased on the tip it read, printing `released propose/<slug>` before `freed <change>`, except a claim whose tip's committer time is at or after the place's `taken` time plus the expiry, and, while `origin/spec/<change>` exists and neither landed test holds, the claim of an item the change's `resolves` doesn't name whose `backlog/<slug>.md` that branch changes since its merge base with `origin/main`; a claim already gone, or one whose delete the lease refuses, SHALL be skipped without failing. When there is no place it SHALL release no claim. It SHALL exit 0, also when there was no place, and 2 when git fails.

#### Scenario: landed draft tidied
- GIVEN a place `20261027-x`, `changes/20261027-x/` in `origin/main`, and a clean, locked worktree at `.claude/worktrees/agent-2` with `spec/20261027-x` checked out
- WHEN `clm draft --finish 20261027-x` runs
- THEN it exits 0, the place is gone, and so is the worktree

#### Scenario: folded draft's branches deleted
- GIVEN `changes/20261027-f/` in `origin/main`, `origin/spec/20261027-f` and `origin/autosync/spec/20261027-f` whose tips aren't in `origin/main`, and a clean worktree under `.claude/worktrees/` with `spec/20261027-f` checked out
- WHEN `clm draft --finish 20261027-f` runs
- THEN it exits 0, the worktree is gone, the local `spec/20261027-f` is gone, and `origin` has neither `spec/20261027-f` nor `autosync/spec/20261027-f`

#### Scenario: landed fold tidied
- GIVEN no place and no `changes/20261027-a-fold/` in `origin/main`, a local `spec/20261027-a-fold` whose tip is the second parent of `origin/main`'s last merge commit, no `origin/spec/20261027-a-fold`, an `origin/autosync/spec/20261027-a-fold`, and a clean, locked worktree under `.claude/worktrees/` with `spec/20261027-a-fold` checked out
- WHEN `clm draft --finish 20261027-a-fold` runs
- THEN it exits 0, the worktree is gone, the local `spec/20261027-a-fold` is gone, `origin` has no `autosync/spec/20261027-a-fold`, and it printed `removed` with the worktree's path

#### Scenario: fold not landed
- GIVEN no `changes/20261027-b-fold/` or `changes/20261027-c-fold/` in `origin/main`, and two clean worktrees under `.claude/worktrees/`, one with `spec/20261027-b-fold` checked out at a commit `origin/main` lacks, the other with `spec/20261027-c-fold` checked out at `origin/main`'s tip
- WHEN `clm draft --finish 20261027-b-fold` and `clm draft --finish 20261027-c-fold` run
- THEN each exits 0, and both worktrees and both local branches are unchanged

#### Scenario: landed fold with uncommitted work
- GIVEN no `changes/20261027-d-fold/` in `origin/main`, a local `spec/20261027-d-fold` whose tip is the second parent of `origin/main`'s last merge commit, checked out in a worktree under `.claude/worktrees/` with an uncommitted change, `origin/autosync/spec/20261027-d-fold`, and `origin/spec/20261027-d-fold` with a commit `origin/main` lacks
- WHEN `clm draft --finish 20261027-d-fold` runs
- THEN it exits 0, prints `kept` with the worktree's path, and the worktree, the local branch, `origin/spec/20261027-d-fold` and `origin/autosync/spec/20261027-d-fold` are unchanged

#### Scenario: worktree left behind by its moved branch
- GIVEN `changes/20261027-m/` in `origin/main`, and a locked worktree under `.claude/worktrees/` with `spec/20261027-m` checked out whose index and files are the tree of the branch's previous commit `P`, the branch having since moved on to a child of `P` made in another worktree, with no unstaged change and no untracked file
- WHEN `clm draft --finish 20261027-m` runs
- THEN it exits 0, prints `removed` with the worktree's path, and the worktree and the local `spec/20261027-m` are gone

#### Scenario: staged work not in history kept
- GIVEN `changes/20261027-n/` in `origin/main`, and a worktree under `.claude/worktrees/` with `spec/20261027-n` checked out and a staged change whose index tree no commit reachable from its `HEAD` has
- WHEN `clm draft --finish 20261027-n` runs
- THEN it exits 0, prints `kept` with the worktree's path, and the worktree is unchanged

#### Scenario: draft waiting for review
- GIVEN a place `20261027-y`, `origin/spec/20261027-y` with its change, no `changes/20261027-y/` in `origin/main`, and a clean worktree with `spec/20261027-y` checked out
- WHEN `clm draft --finish 20261027-y` runs
- THEN it exits 0, the place is gone, and the worktree and `origin/spec/20261027-y` are unchanged

#### Scenario: landed draft's claims released
- GIVEN a place `20261027-r` taken 1 hour ago naming `propose/a` and `propose/b`, both claims committed 1 hour ago, and `changes/20261027-r/` in `origin/main` resolving `a` only
- WHEN `clm draft --finish 20261027-r` runs
- THEN it exits 0, prints `released propose/a` and `released propose/b`, and `origin` has neither `propose/a` nor `propose/b`

#### Scenario: moved item's claim kept while the change waits
- GIVEN a place `20261027-w` taken 1 hour ago naming `propose/a` and `propose/b`, both claims committed 1 hour ago, no `changes/20261027-w/` in `origin/main`, and `origin/spec/20261027-w` whose change resolves `a` and which changes `backlog/b.md`
- WHEN `clm draft --finish 20261027-w` runs
- THEN it exits 0, `origin` has `propose/b` unchanged and no `propose/a`

#### Scenario: another session's later claim kept
- GIVEN a place `20261027-s` taken 6 hours ago naming `propose/a`, with `--expiry 4`, and `origin/propose/a` committed 1 hour ago
- WHEN `clm draft --finish 20261027-s --expiry 4` runs
- THEN it exits 0, the place is gone, and `origin/propose/a` is unchanged

#### Scenario: no place, no release
- GIVEN no place `20261027-q`, `changes/20261027-q/` in `origin/main`, and `origin/propose/a` committed 1 hour ago
- WHEN `clm draft --finish 20261027-q` runs
- THEN it exits 0 and `origin/propose/a` is unchanged

#### Serves
- capability/triage-unattended
- capability/work-concurrently

### Requirement: draft-prompt-file
`clm draft <slug>...` SHALL create `.clm/prompts/draft/` under the directory it runs in before taking a place or claiming any item, and exit 1 naming the path, with nothing on stdout and nothing claimed, when it can't. When it would exit 0, it SHALL write the prompt to `.clm/prompts/draft/<id>.md` there, `<id>` the change ID it chose, replacing any earlier file, and print on stdout only a launcher of at most five lines that names that file by its absolute path, says to read all of it before anything else and follow it, and names `git branch --show-current` as the file's first step. When writing the file fails after the claims, it SHALL free its place, exit 1 naming the path and print nothing on stdout. When it exits 1, 2 or 3 it SHALL print nothing on stdout.

#### Scenario: launcher
- GIVEN untaken `bug` item `a` whose body holds the text `body-of-a`
- WHEN `clm draft a` runs
- THEN it exits 0, stdout is at most five lines, names the absolute path of `.clm/prompts/draft/<date>-a.md` and `git branch --show-current`, and doesn't contain `body-of-a`, and that file contains `body-of-a`

#### Scenario: earlier prompt replaced
- GIVEN untaken item `a`, and `.clm/prompts/draft/<date>-a.md` holding the text `stale`
- WHEN `clm draft a` runs
- THEN it exits 0 and that file doesn't contain `stale`

#### Scenario: no prompts directory
- GIVEN untaken item `a`, and a file at `.clm/prompts`
- WHEN `clm draft a` runs
- THEN it exits 1, stdout is empty, stderr names `.clm/prompts`, `origin` has no `propose/a`, and the clone has no place

### Requirement: fold
`clm draft --fold <id>...` SHALL fetch `origin` and, for each `<id>` in the order given, fold it when `origin/spec/<id>` holds `changes/<id>/change.md` with `status: applied`, or when it holds a rename change that `fold-rename-alone` lets it fold, printing `left <id>: <reason>` on stderr for each other one, and SHALL exit 1 printing nothing on stdout when none is left to fold. Otherwise it SHALL keep `.clm/` out of git through `.git/info/exclude`, write the fold prompt to `.clm/prompts/fold/<first folded id>.md` under the directory it runs in, replacing any earlier file, print on stdout only a launcher of at most five lines that names that file by its absolute path, says to read all of it before anything else and follow it, and names `git branch --show-current` as its first step, and exit 0. It SHALL take no place, claim nothing, push nothing, create no branch or worktree and start no `claude` process. The prompt SHALL name each folded change with its branch and its `resolves`, and SHALL have the unattended preamble. With one folded change, the prompt SHALL say to write the pull request body from that change to `.clm/pr-body.md`, open the pull request with `clm pr open spec/<id> --title <title> --body-file .clm/pr-body.md`, and end with `proposed <id> review <url>`. With several, it SHALL name the fold's branch `spec/<first folded id>-fold`, with `-2`, `-3` and so on added while `origin` has that `spec/` or `autosync/spec/` branch or the clone has that local branch, and SHALL say to cut it from `origin/main` and push it; to bring each folded change over in the order given as one commit of its diff outside `specs/`, taken with `git diff origin/main...origin/spec/<id>` and applied with `git apply --3way`, keeping both changes' edits in a conflict, and never to use `git merge`; to run `clm rebase` until its last line is `rebase ok`, merging an earlier change's edit into a later change's stale delta, setting its base from `clm hashes`, and restoring `specs/` from `origin/main` and every folded change to `status: applied` before committing; to run `clm check` and `clm gate --base origin/main`, its `gate/approval` being expected and a `human-task` item the fold adds getting a burden, and the builder check on the elements where two changes met; to leave out, with `left <id>: <reason>`, a change it can't fold without the human; never to push to or delete a folded change's branch; to report CLM defects with `clm defect`; to write the body to `.clm/pr-body.md` with `## Intent`, `## Decisions`, `## Open questions` and `## CLM defects` holding each folded change's under its ID and saying that the branch needs review, and open the pull request with `clm pr open spec/<batch> --title <title> --body-file .clm/pr-body.md`; and to end with one line, `proposed <batch> review <url>` or `stopped: <reason>`. No fold prompt SHALL name a superpowers skill, `gh pr create`, `--body `, `gh pr edit`, `clm watch`, `clm approve`, `clm land`, `clm significant`, `## Significant`, `gate: `, `HARD`, `SOFT`, `AUTO` or `CARGO_TARGET_DIR`. The prompt it writes SHALL name every command by the name `clm/renamed-commands` gives it, never by an old name that requirement keeps as an alias, and call a branch's pull request a pull request.

#### Scenario: several changes folded
- GIVEN `origin/spec/20261027-a` and `origin/spec/20261027-b`, each holding its `changes/<id>/change.md` with `status: applied`, `a`'s with `resolves: [x]`, and a fake `claude` on PATH that records whether it ran
- WHEN `clm draft --fold 20261027-a 20261027-b` runs
- THEN it exits 0, stdout is at most five lines and names the absolute path of `.clm/prompts/fold/20261027-a.md`, no local branch or place was created, `origin` has no new branch, the fake never ran, and that file names both changes and `x`, and contains `git branch --show-current`, `git switch -c spec/20261027-a-fold`, `git diff origin/main...origin/spec/20261027-a`, `git diff origin/main...origin/spec/20261027-b`, `git apply --3way`, `rebase ok`, `clm hashes`, `clm gate --base origin/main`, `spec review`, `gate/approval`, `human_burdens`, `left <id>`, `clm defect`, `.clm/pr-body.md`, `clm pr open spec/20261027-a-fold`, `proposed 20261027-a-fold review <url>` and `stopped:`, and doesn't contain `git merge`, `superpowers`, `gh pr create`, `--body `, `clm watch`, `clm approve`, `clm land`, `clm significant`, `## Significant`, `gate: `, `HARD`, `SOFT`, `AUTO` or `CARGO_TARGET_DIR`

#### Scenario: fold branch name taken
- GIVEN `origin/spec/20261027-a`, `origin/spec/20261027-b`, each with its change, and `origin/spec/20261027-a-fold`
- WHEN `clm draft --fold 20261027-a 20261027-b` runs
- THEN it exits 0 and the prompt file contains `git switch -c spec/20261027-a-fold-2`

#### Scenario: one change
- GIVEN `origin/spec/20261027-a` holding `changes/20261027-a/change.md`
- WHEN `clm draft --fold 20261027-a` runs
- THEN it exits 0, and the prompt file contains `clm pr open spec/20261027-a` and `proposed 20261027-a review <url>`, and doesn't contain `git merge`, `git switch -c` or `HARD`

#### Scenario: missing or unapplied change left
- GIVEN `origin/spec/20261027-a` with its change `applied`, `origin/spec/20261027-p` whose change is `proposed`, and no `origin/spec/20261027-z`
- WHEN `clm draft --fold 20261027-a 20261027-p 20261027-z` runs
- THEN it exits 0, stderr has `left 20261027-p: ` and `left 20261027-z: `, and the prompt file names `20261027-a` and neither of the others

#### Scenario: nothing to fold
- GIVEN no `origin/spec/20261027-z`
- WHEN `clm draft --fold 20261027-z` runs
- THEN it exits 1, stdout is empty, and stderr has `left 20261027-z: `

#### Scenario: no old command names
- GIVEN `origin/spec/20261027-a` and `origin/spec/20261027-b`, each holding its `changes/<id>/change.md` with `status: applied`
- WHEN `clm draft --fold 20261027-a 20261027-b` runs
- THEN the prompt file names no `clm` followed by one of the old subcommands `sync`, `checkpoint` or `page`, nor the word `page`

#### Needs
- clm/renamed-commands
- model/parse-changes
- model/parse-backlog

#### Serves
- capability/triage-unattended
- architecture/humans-review-intent

### Requirement: background-work-stopped
The `clm draft` prompt SHALL tell the session to wait for a command it runs in the background, such as `clm land spec/<id>` or the checks of the code-fix path, through that command's own completion notice, and never through a loop, watch or monitor that matches processes by name (such as `pgrep -f`), since such a match also finds other drafts' and builds' processes and outlives the command it waits for; running a command in the background is a setting of the tool call, not a change to the command. It SHALL tell the session that a `clm land` it started is waited for until its completion notice arrives, however long that takes, since lands queue behind other drafts' and builds' lands and can take many minutes, and that it never writes its final message while one is still running, since the triage session reads the final line as the draft's end and then frees its place and removes its worktree. It SHALL tell the session that when its turn ends while a command it started in the background still runs, because waiting for that command's completion notice is all that is left, the message ending that turn is not its final message, since the triage session may receive it as the draft's result: it carries no closing line, neither `landed <sha>`, `proposed <id> review` nor `stopped: <reason>`, and ends with the line `waiting: <command>`, naming the command it waits for; the final message comes in a later turn, once the command has ended. It SHALL tell the session that it hands back one report, its final message (`one-report-per-run`): a tool that delivers its report to the triage session and accepts one report per run, such as Claude Code's `SubagentHandback`, is never called for the message ending such a turn, which the session ends as plain text, and is called once, with the final message, since a report sent while waiting uses up the run's one report and the final message would then never reach the triage session. It SHALL tell the session that before it writes its final message, landed, proposed or stopped, it stops every background task it started that is still running (background shells, monitors, watches) and waits for every subagent it launched, the builder check among them, to return, so that no task of a finished draft wakes the triage session later. It SHALL name no tool for stopping a task. These instructions SHALL be in every prompt `clm draft <slug>...` writes, in the paragraph that tells the session to end its final message with `landed <sha>`, `proposed <id> review` or `stopped: <reason>`, and the text SHALL keep to the wording `draft` allows (it names no `clm watch`, `exits 0` or `CARGO_TARGET_DIR`).

#### Scenario: prompt
- GIVEN untaken `bug` item `a`, once alone and once with item `b` claimed by `origin/propose/b` committed 1 hour ago
- WHEN `clm draft a` and `clm draft a b` run
- THEN in each prompt file it writes, the paragraph that contains `stopped: <reason>` also contains `completion notice`, `pgrep` and `background task`, and says to stop those tasks before the final message

#### Scenario: a turn ends while waiting
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN in the prompt file it writes, the paragraph that contains `stopped: <reason>` also contains `many minutes`, `waiting: <command>` and `not your final message`, and says that the message ending such a turn carries none of `landed <sha>`, `proposed <id> review` and `stopped: <reason>`

#### Scenario: a waiting turn sends no report
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN in the prompt file it writes, the paragraph that contains `stopped: <reason>` also contains `one report`, `SubagentHandback` and `one-report-per-run`, and says that the hand-back tool is never called for the message ending a turn that waits, and is called once, with the final message

#### Needs
- plugin.triage/triage-waiting-drafts

### Requirement: draft-code-fix-written-rule
The prompt `clm draft` writes SHALL tell the session that a `bug` is code-only, and fixed by component on `impl/<component>`, only when a requirement or scenario as written already states the behaviour the fix restores and the fix's test cites it; a bug whose behaviour no requirement states goes into the change, as a requirement or scenario the change adds, and its code is fixed when the component is built (`code-fix-needs-a-written-rule`). It SHALL say that an `ARCHITECTURE.md` entry, carried or the project's own, is not such a requirement, since a test never cites an entry (`human-layer-proof`): a bug whose behaviour only an architecture entry states is a bug no requirement states, unless its work is test plumbing. It SHALL tell the session too that a `bug` or `deferred` item whose work is test plumbing only is never a requirement of any component: test plumbing is harness-only work, changing only the plumbing of the capability suite and nothing a capability test observes, or the plumbing of a component's own test suite (its runner settings, the ports and servers its tests start, the paths they write), changing nothing any of its tests asserts. When the Design that holds that plumbing, if any, stays true as written, the session fixes it in the draft on a branch of its own from `origin/main`, named outside `impl/`, `spec/`, `propose/` and `autosync/`, with no claim and any test it adds citing no requirement, deleting the item in a commit whose trailer block holds `Resolves: backlog/<slug>`, lands it with `clm land` once `clm gate --base origin/main` prints `code`, deletes the pushed branch with `clm branch delete`, and takes the item out of `resolves` (`small-change-no-claim`); when that Design must change, the change edits the Design, keeps the item in `resolves` and adds a `kind: deferred` item for the code. The prompt says it in the paragraph that already describes code-only bugs, not in a new one.

#### Scenario: prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that contains `code-fix-needs-a-written-rule` also contains `as written`, `harness-only`, `test plumbing`, `own test suite` and `only an architecture entry states`

#### Serves
- capability/triage-unattended
- architecture/fold-small-defer-large

### Requirement: draft-restraint
The prompt `clm draft` writes SHALL tell the session to judge each item by `redesign-before-workaround`, `yagni` and `reuse-first` rather than as it was filed; that a bug's answer is a redesign, an acceptance, or a small workaround for a genuine edge case; that an accepted item stays in `resolves`, its `backlog/<slug>.md` is deleted on `spec/<id>` and `## Intent` says why the behaviour is accepted; and that `change.md` lists the edge cases and workarounds the change already knows it needs. It says so in the paragraph that tells the session to read each item and `clm context <component>`.

#### Scenario: prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that contains `clm context <component>` also contains `redesign-before-workaround`, `yagni`, `reuse-first`, `accept` and `workarounds`

#### Serves
- capability/triage-unattended
- architecture/redesign-before-workaround
- architecture/yagni
- architecture/reuse-first

### Requirement: draft-backlog-only-change
The prompt `clm draft` writes SHALL tell the session that when the change has no delta to write and the branch edits `backlog/` (an item deleted as already fixed or accepted, an item deferred, a bug's `components` corrected, or a CLM defect reported), it skips the design and the builder check, writes an `## Intent` saying what the change does to the backlog, and applies, syncs, gates and lands it as any other change, by the first line of its gate; that before deciding there is no spec change it reports its CLM defects with `clm defect` on `spec/<id>`; and that only when `spec/<id>` changes nothing but its `change.md` does it delete the branch, deleting `spec/<id>` and `autosync/spec/<id>` on `origin` each with its own `clm branch delete <ref>`, so a ref the clone never fetched fails only its own call.

#### Scenario: prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN the prompt file it writes contains `` edits `backlog/` ``, `` `clm defect` on `spec/<id>` `` and `clm branch delete <ref>`, and doesn't contain `--delete spec/<id> autosync/spec/<id>`

#### Needs
- host/branch-delete

### Requirement: builder-check-model
The prompts `clm draft` and `clm draft --fold` write SHALL tell the session to launch the builder check's subagent with the Agent tool's `model` set to `sonnet`, the model builds run on (`model-per-launch`), and never to launch a subagent without naming its model.

#### Scenario: draft prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that names the builder check contains `model: "sonnet"`

#### Scenario: fold prompt
- GIVEN `origin/spec/20261027-a` holding `changes/20261027-a/change.md` with `status: applied`
- WHEN `clm draft --fold 20261027-a` runs
- THEN the prompt file contains `model: "sonnet"`

#### Serves
- capability/right-model-per-task

### Requirement: draft-answers
`clm draft --answers <file> <slug>...` SHALL read the file, a relative path against the directory it runs in, before fetching, taking a place or claiming any item, and when the read fails SHALL exit 1 naming the path, claiming nothing and leaving no place. When the file holds more than whitespace, the prompt SHALL hold its text, verbatim and trimmed, under `## The human's answers`, after the items and before the steps, and the steps SHALL say that the answers are the human's, given in the triage session; that the session designs from those that bear on its items, an answer outranking the text of the item it answers but never `ARCHITECTURE.md`, a decision or a requirement; that an item the answers make designable is designed, not left; that every answer it used is quoted in `change.md` under `## The human's answers`; that a `question` or `human-task` item the answers settle stays in `resolves`, the fact it waited on going into the specs the change edits; and that an item the answers only edit, leaving it open and still waiting on the human (such as a `human-task` item whose done steps the human has trimmed out), is left out of `resolves`, since a change deletes the items it resolves when it is archived: the session edits its `backlog/<slug>.md` on `spec/<id>` to what the answers leave of it, says in `## Intent` what it changed and that the item stays open, and takes the backlog-only route (`draft-backlog-only-change`) when nothing else is left to design; and that a `question` item the answers settle while leaving only code to change, a requirement or scenario as written already stating what that code must do (`draft-code-fix-written-rule`) and no requirement the item blocks waiting to be built for it, is the exception to the settled items that stay in `resolves`: it is left out of `resolves` (taken out of the list first written) and turned into a bug: the session sets its `kind` to `bug` on `spec/<id>`, removes its `blocks`, sets its `components` to the components whose `paths` hold the fix and rewrites its body to the code work left, naming the requirement the fix restores and quoting the answer, keeping its slug and its other fields, such as `priority` and `issues`, says in `## Intent` that the answer turns it into a bug, and takes the backlog-only route when nothing else is left to design; once `clm land spec/<id>` has landed the branch, the session fixes that bug in the same draft by component, as it fixes any code-only bug, with `clm claim <component> --fix backlog/<slug>` and a `Resolves: backlog/<slug>` trailer, leaving it with `left <slug>: <reason>` when the claim is refused, and ends with the spec change's `landed <sha>`, while a branch that needs review stops the draft as any does and leaves the bug to the next triage. Without `--answers`, or when the file holds only whitespace, the prompt SHALL NOT contain `The human's answers`. `--answers` SHALL conflict with `--finish` and `--fold`.

#### Scenario: answers in the prompt
- GIVEN untaken `question` item `q` and a file `answers.md` holding `q: use the default`
- WHEN `clm draft --answers answers.md q` runs
- THEN it exits 0, and the prompt file contains `## The human's answers`, then `q: use the default`, then `## Steps`, in that order, and the text after `## Steps` contains `given in the triage session`, `outranking`, `designed, not left`, `quoted in` and `stays in`

#### Scenario: an answer that only edits an item
- GIVEN untaken `human-task` item `h` and a file `answers.md` holding `h: trim the done steps and keep the rest open`
- WHEN `clm draft --answers answers.md h` runs
- THEN it exits 0, and the text after `## Steps` contains `only edit`, `left out of`, `stays open` and `draft-backlog-only-change`

#### Scenario: an answer that leaves only code
- GIVEN untaken `question` item `q` and a file `answers.md` holding `q: the rule already says so, only the code is left`
- WHEN `clm draft --answers answers.md q` runs
- THEN it exits 0, and the text after `## Steps` contains `only code to change`, `turned into a bug`, `has landed the branch` and `the next triage`

#### Scenario: unreadable answers
- GIVEN untaken item `a`, no places, and no file `missing.md`
- WHEN `clm draft --answers missing.md a` runs
- THEN it exits 1, stderr names `missing.md`, `origin` has no `propose/a`, and the clone has no place

#### Scenario: no answers
- GIVEN untaken item `a`
- WHEN `clm draft a` runs
- THEN the prompt file doesn't contain `The human's answers`

#### Scenario: whitespace-only answers
- GIVEN untaken item `a` and a file `answers.md` holding only blank lines
- WHEN `clm draft --answers answers.md a` runs
- THEN it exits 0 and the prompt file doesn't contain `The human's answers`

#### Needs
- model/parse-backlog

### Requirement: draft-scratch-files
Every prompt `clm draft` writes, `clm draft --fold`'s included, SHALL tell the session to keep the scratch files it writes while working (context excerpts, parts of a delta, notes) under its own worktree's `.clm/`, which git ignores, and never in a scratchpad or temporary directory its system prompt names or in any other directory shared with other sessions, since the drafts and builds launched beside it are given the same directory and would overwrite one another's files. It says so in the paragraph that tells the session it is running unattended.

#### Scenario: draft prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that contains `running unattended` also contains `.clm/`, `scratch` and `scratchpad`

#### Scenario: fold prompt
- GIVEN `origin/spec/20261027-a` holding `changes/20261027-a/change.md` with `status: applied`, once alone and once with `origin/spec/20261027-b` likewise
- WHEN `clm draft --fold 20261027-a` and `clm draft --fold 20261027-a 20261027-b` run
- THEN in each prompt file it writes, the paragraph that contains `running unattended` also contains `.clm/`, `scratch` and `scratchpad`

### Requirement: builder-check-order
The prompts `clm draft` and `clm draft --fold` write SHALL tell the session, in the paragraph that names the builder check, to ask its subagent, for each requirement the change adds or modifies (for the fold, each one on the elements where two changes met, not every requirement of the folded changes), which requirements not yet current it relies on, of any component, among the change's own and those `clm pending` lists, answering `none` for a requirement that relies on none (`builder-check-asks-order`); to give each such requirement of another component a `#### Needs` line, as the paragraph already asks; and that a requirement relying on a pending requirement of its own component that has a `#### Needs` block carries that block's needs too, since a need may not name its own component.

#### Scenario: draft prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that names the builder check contains `builder-check-asks-order`, `clm pending` and `its own component`

#### Scenario: fold prompt
- GIVEN `origin/spec/20261027-a` and `origin/spec/20261027-b`, each holding its `changes/<id>/change.md` with `status: applied`
- WHEN `clm draft --fold 20261027-a 20261027-b` runs
- THEN the paragraph of the prompt file that names the builder check contains `builder-check-asks-order`, `clm pending` and `its own component`

### Requirement: draft-contract-step-deferred
The prompt `clm draft` writes SHALL tell the session, in the paragraph that names the builder check, that a requirement removing or renaming part of a component's Contract that another component's code uses needs the requirements that move each such component off it (`contract-removal-needs`); that a rename is written as two requirements of the provider in the same change (`rename-adds-then-removes`): an adding one that offers the new name beside the old, needs nothing and leaves every user's code building as it stands (a second name, never a widened type a user matches exhaustively), and a removing one, stated as an added or modified requirement, that needs each user's move, each move needing the adding one, so the needs form no cycle, the adding one's build marking the old name `clm:remove-by` the change, which stays live until the removal is built (`expand-contract`), and that the builder check searches each user's code for exhaustive uses (a `match`, a `switch` with no default, a map keyed by the type) the adding one must not break; that no build edits another component's code or tests to follow a change (`path-ownership`), so a requirement that widens a type another component matches exhaustively (an added enum variant) needs that component's requirement that first handles a value it doesn't know with a default arm marked `clm:remove-by` the change, that component's requirement that handles the new value, removing the arm, needs the widening, and a requirement that adds a field another component builds by struct literal needs that component's requirement that builds it so the field leaves it compiling (`coupled-changes-sequence`), the builder check searching each user's code for exhaustive uses of a type the change widens too; that when the removal can't carry that `#### Needs` (the removing component is required and the component that must move is optional, or the removed text is a whole requirement under `REMOVED Requirements`), the contract step goes in a second change on the same branch, left `proposed`, and the branch also adds a `kind: deferred` backlog item for it whose `components` are the contract change's components and whose body names the contract change and each requirement it waits on with the version it must be current at, which the contract change's `resolves` doesn't name, so `clm triage` prints it as a `deferred` line until the human judges it due (`contract-step-deferred`); and that a claimed `deferred` item for a contract step is designed by applying the change it names with `clm apply`, rebasing its stale deltas on `clm hashes <component>` first, the session's own change resolving the item.

#### Scenario: prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that names the builder check contains `contract-removal-needs`, `rename-adds-then-removes`, `exhaustive`, `widens`, `coupled-changes-sequence`, `contract-step-deferred`, `optional` and `kind: deferred`

### Requirement: fold-rename-alone
`clm draft --fold` SHALL take as a rename change an `<id>` whose `origin/spec/<id>` holds no `changes/<id>/change.md` but holds `changes/archive/<id>/change.md` whose frontmatter parses with `status: done` and a `renames` key holding a YAML list of at least one entry, the entries' shape unchecked, as `clm rename` leaves a rename change on its own branch (`rename-by-replay`), reading its `resolves` from that file; a live `changes/<id>/change.md`, when the branch holds one, SHALL be judged as `fold` says whatever the archive holds. It SHALL fold a rename change only when every other distinct `<id>` given is left for a reason `fold` gives (missing, unreadable or not applied); when any other distinct `<id>` given is an applied change or a rename change, it SHALL print `left <id>: a rename change folds alone (rename-by-replay): run clm draft --fold <id>` on stderr for each rename change and fold the rest, exiting 1 when none is left to fold, as `fold` says. The one-change prompt for a rename change SHALL say to read the change with `git show origin/spec/<id>:changes/archive/<id>/change.md`, and SHALL NOT name `changes/<id>/change.md`. A `changes/archive/<id>/change.md` with another status or no `renames` SHALL be left as `fold` leaves a missing change.

#### Scenario: rename change folded alone
- GIVEN `origin/spec/20261027-r` holding `changes/archive/20261027-r/change.md` with `status: done`, `resolves: [x]` and one `term` rename, and no `changes/20261027-r/change.md`
- WHEN `clm draft --fold 20261027-r` runs
- THEN it exits 0, and the prompt file names `x` and contains `git show origin/spec/20261027-r:changes/archive/20261027-r/change.md`, `clm pr open spec/20261027-r` and `proposed 20261027-r review <url>`, and doesn't contain `changes/20261027-r/change.md` or `git switch -c`

#### Scenario: rename change beside others left
- GIVEN `origin/spec/20261027-a` holding `changes/20261027-a/change.md` with `status: applied`, and `origin/spec/20261027-r` holding the rename change above
- WHEN `clm draft --fold 20261027-a 20261027-r` runs
- THEN it exits 0, stderr has `left 20261027-r: a rename change folds alone (rename-by-replay): run clm draft --fold 20261027-r`, and the prompt file names `20261027-a` and not `20261027-r`

#### Scenario: rename change beside an unapplied one folded
- GIVEN `origin/spec/20261027-p` holding `changes/20261027-p/change.md` with `status: proposed`, and `origin/spec/20261027-r` holding the rename change above
- WHEN `clm draft --fold 20261027-p 20261027-r` runs
- THEN it exits 0, stderr has `left 20261027-p: ` and not `left 20261027-r`, and the prompt file contains `proposed 20261027-r review <url>`

#### Scenario: two rename changes left
- GIVEN `origin/spec/20261027-r` and `origin/spec/20261027-s`, each holding a rename change as above under its own ID
- WHEN `clm draft --fold 20261027-r 20261027-s` runs
- THEN it exits 1, stdout is empty, and stderr has `left 20261027-r: a rename change folds alone` and `left 20261027-s: a rename change folds alone`

#### Scenario: archived change without renames left
- GIVEN `origin/spec/20261027-q` holding `changes/archive/20261027-q/change.md` with `status: done` and no `renames`, and no `changes/20261027-q/change.md`
- WHEN `clm draft --fold 20261027-q` runs
- THEN it exits 1, stdout is empty, and stderr has `left 20261027-q: ` and not `folds alone`

#### Serves
- capability/triage-unattended
- architecture/humans-review-intent

### Requirement: draft-workaround-deferred
The prompt `clm draft` writes SHALL show each claimed `workaround` item's `requirement` beside its kind, and SHALL tell the session: that a claimed `workaround` item is resolved by a change to the requirement it names (rewording or re-scoping it, adding the `#### Needs` or Design it lacks), deleting the flag, or re-recording its `requirement` at the new version when the workaround still applies to the new text (`workaround-flags`); that a claimed `deferred` item's condition was judged met by the human, so the session does the work it names; that when that work is code-only, changing nothing under `specs/`, `changes/`, `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md` or `CLAUDE.md` and leaving every requirement and scenario as written true, and lies in one of the item's `components`, the session does it by component as it fixes a code-only bug, with `clm claim <component> --fix backlog/<slug>` and a commit whose trailer block holds `Resolves: backlog/<slug>` (`deferred-work-resolves-in-code`), the item leaving `resolves`, and leaves it with `left <slug>: <reason>` naming the claim's reason when the claim is refused; that otherwise the session designs the work into the change; and that an item the human's answers drop stays in `resolves`, the session deleting its `backlog/<slug>.md` and saying in `## Intent` that the human dropped it and why, the backlog-only route (`draft-backlog-only-change`). What it says of `deferred` items SHALL be in the paragraph that describes code-only bugs.

#### Scenario: a workaround item's prompt
- GIVEN an untaken `workaround` item `f` with `requirement: check/w@<version>`
- WHEN `clm draft f` runs
- THEN the prompt file holds `check/w@<version>`, and says the change rewords or re-scopes the requirement and deletes or re-records the flag

#### Scenario: a deferred item's code route
- GIVEN an untaken `deferred` item `k` with `components: [check]`
- WHEN `clm draft k` runs
- THEN the paragraph of the prompt file that contains `clm claim <component> --fix backlog/<slug>` also contains `deferred`, `deferred-work-resolves-in-code`, `judged met` and `designs the work`

#### Scenario: a dropped item
- GIVEN an untaken `deferred` item `k` and an answers file saying the human drops `k`
- WHEN `clm draft --answers <file> k` runs
- THEN the prompt says an item the answers drop stays in `resolves` and its `backlog/<slug>.md` is deleted

#### Needs
- model/parse-backlog
- orchestrator/claim-fix

### Requirement: designed-archived
`clm triage` SHALL print a proposed change as `designed` when `origin/spec/<id>` (else `origin/autosync/spec/<id>`) holds no `changes/<id>/change.md` and its `changes/archive/<id>/change.md` has `status: done`, and as `open` when that archived file has another status or doesn't parse. A ref that holds `changes/<id>/change.md` SHALL be judged by that file alone.

#### Scenario: rename branch waiting for review
- GIVEN `origin/spec/20260101-r` whose tip holds `changes/archive/20260101-r/change.md` with `status: done`, `renames` and `resolves: [x]`, and no `changes/20260101-r/change.md`, no `changes/20260101-r/` or `changes/archive/20260101-r/` in `origin/main`, and `x` an untaken `bug` item with no `components` in `origin/main`'s `backlog/`
- WHEN `clm triage` runs
- THEN it prints `designed 20260101-r [] P5 0 x`, and no `group` line names `x`

#### Serves
- capability/triage-unattended

#### Needs
- orchestrator/archived-spec-work

### Requirement: says-component
The prompts `clm draft` and `clm draft --fold` write, and the lines `clm triage` prints, SHALL say component and components, never the old words, as `component-words` defines them: the prompts name a component ID `<component>` (`clm context <component>`, `clm hashes <component>`, `clm claim <component> --fix backlog/<slug>`, `deltas/<component>.md`), the backlog key `components`, and the closing line `left <slug>: components corrected to <components>`; each item a draft prompt shows is headed with its `components`. The names of decisions and architectural constraints the prompts cite keep their spelling. A test reads the string literals in `crates/clm-triage/src/` and fails naming the file, the line and the word for each old word `component-words` doesn't keep.

#### Scenario: a draft prompt
- GIVEN untaken `bug` item `a` with `components: [check]`
- WHEN `clm draft a` runs
- THEN the prompt file it writes shows `a` with ``components `[check]` ``, and holds no old word but in a decision's or constraint's name

#### Serves
- architecture/plain-terms

### Requirement: draft-corrects-bug-components
The prompt `clm draft` writes SHALL tell the session that a code-only bug whose fix lies in no component the item names has its `components` set on `spec/<id>` to the components whose `paths` hold the fix, is left out of `resolves` with `left <slug>: components corrected to <components>`, and is not fixed by that draft, since `clm claim --fix` takes only a component the item on `origin/main` names. It says so in the paragraph that describes code-only bugs.

#### Scenario: prompt
- GIVEN untaken `bug` item `a` with `components: [check]`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that contains `clm claim <component> --fix backlog/<slug>` also contains `components corrected to <components>`

### Requirement: draft-hand-edit-check
The prompt `clm draft` writes SHALL tell the session that when its spec branch changes a file outside `specs/`, `changes/` and `backlog/` (a hand edit, such as one to `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `README.md` or `docs/`), which a code test may read and which neither `clm check` nor the gate tests, it runs `clm land --check-only` after its last `clm gate` run, before it opens the pull request or stops for review (`draft-checks-hand-edits`). It SHALL tell the session that a failing test that reads a file the branch changes is the change's: the session names each such test, its component and the hand edit that breaks it under `## Open questions` in `change.md`, commits, rebases and pushes as after any commit, and doesn't land; that when the gate's first line is `spec review` it then stops as for any change that needs review, and when it is `spec` it opens no pull request and ends with `stopped: land check fails: <tests>`; and that a failing test that reads no file the branch changes isn't the change's and doesn't stop it. It says so in the paragraph that tells the session to go by the first line of its last `clm gate` run, and keeps to the wording `draft` allows.

#### Scenario: prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that contains `go by the first line` also contains `clm land --check-only`, `` outside `specs/`, `changes/` and `backlog/` ``, `## Open questions`, `stopped: land check fails: <tests>` and `draft-checks-hand-edits`, and the prompt file doesn't contain `CARGO_TARGET_DIR` or `exits 0`

### Requirement: draft-adds-glossary-terms
The prompt `clm draft` writes SHALL tell the session that when a requirement its change adds or modifies introduces a term `clm/glossary-covers-terms` covers (a top-level `clm` command, a requirement state, a component status, a change status, a backlog kind or a key of `change.md`'s frontmatter), it adds the term's entry to `docs/guide/glossary.md` on `spec/<id>`, since the code branch that builds the term can't carry a spec path and its land fails without the entry. It says so in the paragraph that tells the session to design the rest as one change, and keeps to the wording `draft-prompt-file` allows.

#### Scenario: prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that contains `Design the rest as one change` also contains `docs/guide/glossary.md` and `clm/glossary-covers-terms`

#### Serves
- architecture/plain-terms

### Requirement: draft-architecture-audit
The prompts `clm draft` and `clm draft --fold` write SHALL tell the session, in the paragraph that names the builder check, to launch the `architecture-auditor` subagent beside it, in the same message, with the Agent tool's `model` set to the alias of its own model, in the words `your own model`, since the audit is a design review (`model-per-launch`), giving it only the change ID and the path of its checkout (for the fold, asking it about the elements where two changes met, as the builder check is asked); to fix in the change each finding it agrees with and to write each one it leaves under `## Open questions` in `change.md`, with why (in the fold, in the `change.md` of the folded change whose element it concerns), since the pull request body carries each change's open questions to the human approving it; that the audit advises and never gates (`spec-architecture-audit`); and that when the builder check or the audit changed the change, the session runs the round the paragraph already names after the builder check.

#### Scenario: draft prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that names the builder check contains `architecture-auditor`, `your own model`, `## Open questions` and `spec-architecture-audit`

#### Scenario: fold prompt
- GIVEN `origin/spec/20261027-a` and `origin/spec/20261027-b`, each holding its `changes/<id>/change.md` with `status: applied`
- WHEN `clm draft --fold 20261027-a 20261027-b` runs
- THEN the paragraph of the prompt file that names the builder check contains `architecture-auditor`, `your own model`, `## Open questions` and `spec-architecture-audit`

#### Serves
- capability/triage-unattended
- architecture/right-over-convenient

#### Needs
- plugin.agents/architecture-auditor

### Requirement: intake
`clm intake` SHALL fetch `origin`, take `host`'s list of open issues and pull requests, and print one line for each, sorted by number: `pull <n> <author>` for a pull request; for an issue, `issue <n> imported <slug>...` when items on `origin/main`'s backlog name it in their `issues`, sorted by slug; else `issue <n> resolved <slug> <by>` when an item whose `issues` named it was deleted from `origin/main`'s backlog, the most recent such deletion, `<by>` being the greatest ID among the changes on `origin/main`, live or archived, whose `resolves` or `from` names the slug, or, when none does, the deleting commit's hash abbreviated as `git rev-parse --short` abbreviates it; and `issue <n> new` otherwise. After those lines it SHALL take `host`'s open dependency alerts (`host/list-alerts`) and print one line for each, sorted by number, matched to the backlog as an issue is, with `alerts` in place of `issues`: `alert <n> imported <slug>...`, `alert <n> resolved <slug> <by>`, or `alert <n> new <severity> <ecosystem> <package> <manifest>` with the fields `host` returned. When the host's alerts fail, for any reason, it SHALL print `alerts unavailable` as the last line, in their place, and go on as when there are none, since a login that may read issues may still be refused the alerts. It SHALL print no title, body or other text from the host. It SHALL exit 0, printing no `issue`, `pull` or `alert` line when there is no host or no open issue, pull request or alert (`intake-verify-failures` adds its own lines); 1 with the reason when the host's list of issues and pull requests fails; and 2 when git fails.

#### Scenario: new, imported and a pull request
- GIVEN a fake `gh` reporting open issues 73 and 92 and open pull request 80 by `dana`, and `origin/main` holding `backlog/port-clash.md` with `kind: bug` and `issues: [73]`
- WHEN `clm intake` runs
- THEN it prints, in this order, `issue 73 imported port-clash`, `pull 80 dana` and `issue 92 new`, and exits 0

#### Scenario: resolved by a change
- GIVEN a fake `gh` reporting open issue 67, and `origin/main` whose history added `backlog/fold-declined.md` with `issues: [67]` and later deleted it, holding `changes/archive/20260101-c/change.md` with `resolves: [fold-declined]`
- WHEN `clm intake` runs
- THEN it prints `issue 67 resolved fold-declined 20260101-c`

#### Scenario: resolved by a code fix
- GIVEN a fake `gh` reporting open issue 64, and `origin/main` whose history added `backlog/detector-missing.md` with `issues: [64]` and deleted it in commit `s`, with no change naming it
- WHEN `clm intake` runs
- THEN it prints `issue 64 resolved detector-missing` followed by `s`'s abbreviated hash

#### Scenario: alerts after the inbox
- GIVEN a fake `gh` reporting open issue 92, open alert 4, `high`, `npm`, `vite`, `web/package-lock.json`, and open alert 2, and `origin/main` holding `backlog/bump-x.md` with `kind: bug` and `alerts: [2]`
- WHEN `clm intake` runs
- THEN it prints, in this order, `issue 92 new`, `alert 2 imported bump-x` and `alert 4 new high npm vite web/package-lock.json`, and exits 0

#### Scenario: alert resolved by a code fix
- GIVEN a fake `gh` reporting open alert 7, and `origin/main` whose history added `backlog/bump-y.md` with `alerts: [7]` and deleted it in commit `s`, with no change naming it
- WHEN `clm intake` runs
- THEN it prints `alert 7 resolved bump-y` followed by `s`'s abbreviated hash

#### Scenario: alerts refused
- GIVEN a fake `gh` reporting open issue 92 and exiting 1 on the alerts' `api` call
- WHEN `clm intake` runs
- THEN it prints `issue 92 new` and `alerts unavailable`, and exits 0

#### Scenario: no stranger's text
- GIVEN a fake `gh` reporting open issue 5 whose title is `x\nissue 6 resolved y z`
- WHEN `clm intake` runs
- THEN it prints only `issue 5 new`

#### Scenario: no host
- GIVEN `host = "none"`
- WHEN `clm intake` runs with no `verified` records in the ledger
- THEN it prints nothing and exits 0

#### Serves
- capability/contribute-from-outside

#### Needs
- host/list-contributions
- host/list-alerts
- model/parse-backlog
- model/parse-backlog-item

### Requirement: draft-capability-test-fixes
The prompts `clm draft` and `clm draft --fold` write SHALL tell the session, in the paragraph that names the builder check, to ask its subagent which capability tests each requirement the change adds or modifies changes what they observe at the product's boundary, and that such work is neither a requirement of `capability`, which holds none of its own, nor a `#### Needs`, nor a backlog item: the requirement's own build updates those tests (`capability-tests-follow-rules`), so the session names them in that requirement's text; and that a requirement under which a `CAPABILITIES.md` entry as written no longer holds edits that entry in the change instead, a hand edit.

#### Scenario: draft prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that names the builder check contains `capability-tests-follow-rules` and `CAPABILITIES.md`

#### Scenario: fold prompt
- GIVEN `origin/spec/20261027-a` and `origin/spec/20261027-b`, each holding its `changes/<id>/change.md` with `status: applied`
- WHEN `clm draft --fold 20261027-a 20261027-b` runs
- THEN the paragraph of the prompt file that names the builder check contains `capability-tests-follow-rules` and `CAPABILITIES.md`

#### Serves
- capability/triage-unattended
- architecture/path-ownership

### Requirement: prompts-push-through-clm
Every prompt `clm draft` writes, with or without `--fold`, SHALL tell the session to push its branch with `clm branch push` wherever it says to push, and to delete a branch on `origin` with `clm branch delete`, and SHALL NOT contain `git push` or `--force-with-lease` outside the items' own text and the human's answers. The fold prompt SHALL say to push the fold's new branch with `clm branch push` once it is cut.

#### Scenario: draft and fold prompts
- GIVEN untaken `bug` item `a`, and `origin/spec/20261027-a` and `origin/spec/20261027-b`, each holding its `changes/<id>/change.md` with `status: applied`
- WHEN `clm draft a` and `clm draft --fold 20261027-a 20261027-b` run
- THEN both prompt files contain `clm branch push`, the draft's contains `clm branch delete`, and neither contains `git push` or `--force-with-lease`

#### Needs
- host/branch-push
- host/branch-delete

### Requirement: intake-verify-failures
After the host's lines, with or without a host, `clm intake` SHALL take the fold's `unresolved_reds` over `origin/main`'s first-parent chain (`ledger/verified-record`) and print one line for each, in first-parent order: `verify <sha> imported <slug>...` when items on `origin/main`'s backlog name the commit in their `verify_failures`, sorted by slug, and otherwise `verify <sha> new <culprit> <component>...`, `<culprit>` being the record's culprit or `unknown`, both commits abbreviated as `git rev-parse --short` abbreviates them, and the components sorted. After each `verify <sha> new` line it SHALL print `verify <sha> log <path>`, `<sha>` as on the line before it, when `verify/kept-log` returns a path for the red commit, `<path>` being that path, and nothing more when it returns none; an `imported` line SHALL get no `log` line. The capability test of `slow-tests-after-land`, whose clone keeps the log of the failing local run it makes, then sees that `log` line after its `new` line, and its build SHALL check that line and that the file it names exists. A red result followed by a green one on `main` SHALL print nothing.

#### Scenario: a new failure
- GIVEN a passing `verified` record for `origin/main~2` and a red one for `origin/main` with culprit `origin/main~1` and components `b` and `a`, and no log kept for it
- WHEN `clm intake` runs with `host = "none"`
- THEN it prints `verify <sha> new <culprit> a b`, no `verify <sha> log` line, and exits 0

#### Scenario: a kept log
- GIVEN the same records, and `.clm/verify-logs/<sha>.log` in the clone's main worktree, `<sha>` the red commit abbreviated
- WHEN `clm intake` runs with `host = "none"` in a linked worktree of the clone
- THEN it prints `verify <sha> new <culprit> a b` and, on the next line, `verify <sha> log <path>` with that file's absolute path

#### Scenario: taken in
- GIVEN the same records and kept log, and `origin/main` holding `backlog/b-broke.md` with `kind: bug` and `verify_failures` naming the red commit
- WHEN `clm intake` runs
- THEN it prints `verify <sha> imported b-broke` and no `verify <sha> log` line

#### Scenario: fixed since
- GIVEN the red record and a passing record for a later commit on `main`
- WHEN `clm intake` runs
- THEN it prints no `verify` line

#### Needs
- ledger/verified-record
- model/parse-backlog
- verify/kept-log

#### Serves
- capability/slow-tests-after-land

### Requirement: bounded-test-runs
The `clm draft` prompt SHALL tell the session that every command it runs itself, such as the checks of the code-fix path, other than `clm land` in any form, which it runs as written, ends within a time limit (`bounded-test-runs`): it runs the command in the foreground, where the tool call's own timeout ends it, or, when it runs it in the background, as `timeout <limit> <command>`, with a limit well past the time the run should take, whatever the command is, a test or check run or any other, and for each command of a chain it starts in the background. It SHALL tell the session that no command it runs reads standard input that nothing feeds it, such as `cat` with no file to read or a `cat >> <file>` with no input given, since nothing ever types into a command it runs, and that it writes a file with its editing tools instead; a command given its input by a heredoc or a pipe, such as `clm defect` or `clm workaround` with the report on stdin, is fine, since its input ends. It SHALL give the reason: a command that never ends by itself, such as a test that hangs on a polling wait with no deadline of its own or a command waiting on standard input, never ends a run started in the background, so its completion notice never comes, and nothing wakes a session that waits for it, nor the triage session, which counts a waiting draft as still running, so the draft sits idle with no one told. It SHALL tell the session that a test or check run its limit ends counts as a failing run, whose hanging test it finds as for any failure: it fixes the test when it is in the files of the component it is fixing, and otherwise records it as a backlog item and names that item in its pull request body. These instructions SHALL be in every prompt, in the paragraph that tells the session to end its final message with `landed <sha>`, `proposed <id> review` or `stopped: <reason>`.

#### Scenario: prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft` runs
- THEN the paragraph of the prompt it writes that contains `stopped: <reason>` also contains `timeout <limit> <command>`, `foreground`, `hangs`, `standard input`, `editing tools`, `bounded-test-runs` and `backlog item`

#### Serves
- capability/triage-unattended

### Requirement: draft-prompt-setup
When `clm.toml` sets `[build] setup`, the prompt `clm draft` writes SHALL tell the session, in the paragraph that describes fixing code-only bugs by component, that once it is on the branch it fixes code on (the `impl/<component>` branch `clm claim <component> --fix` switched it to, or a branch of its own for test plumbing) and before it runs any test there, it runs `clm setup` as one plain command, which runs the project's `[build] setup` so that its tests run against its own worktree's code and dependencies, never the main checkout's (`tests-run-in-their-own-tree`), and runs it again after a `clm rebase` or an edit of its own that changes the project's dependency declarations; and that a `clm setup` that fails, or that Claude Code refuses or denies, leaves the item it was fixing with `left <slug>: setup failed`, reported with `clm defect` when it was refused or denied, since no test result is then to be trusted. When `clm.toml` sets no `[build] setup`, the prompt SHALL NOT name `clm setup`. `clm draft` SHALL take the setting from the settings `crates/clm` loads and passes it, as it passes `max_builds`, so `clm-triage` reads no settings itself. The text SHALL keep to the wording `draft` allows.

#### Scenario: setup declared
- GIVEN a repository whose `clm.toml` sets `[build] setup = "scripts/setup.sh"`, and a backlog item `b`
- WHEN `clm draft b` writes its prompt
- THEN the paragraph of the prompt file that contains `code-fix-needs-a-written-rule` also contains `clm setup`, `tests-run-in-their-own-tree` and `setup failed`

#### Scenario: nothing declared
- GIVEN a repository whose `clm.toml` has no `[build]` table, and a backlog item `b`
- WHEN `clm draft b` writes its prompt
- THEN the prompt file doesn't contain `clm setup`

#### Needs
- config/build-setup-setting

#### Serves
- architecture/tests-run-in-their-own-tree

### Requirement: draft-fetch-retries-race
Every fetch of `origin` that `clm draft`, `clm draft --finish` and `clm draft --fold` make SHALL go through `clm-git`'s `fetch_retrying` (`vcs/fetch-retries`) with the arguments they fetch with today (`fetch --prune --quiet origin`), so a draft's start, finish or fold whose fetch lost a race with a build in another worktree of the same clone over a remote-tracking ref goes on. When `fetch_retrying` fails, the command SHALL exit 2 as a failed fetch makes it exit today, with the error's message: git's message from the last fetch for `FetchError::Git`, and `origin can't be reached` for `FetchError::Offline`. `clm intake` keeps fetching through `clm-git`'s `fetch`, which tries again itself (`vcs/fetch-tries-again`), and `clm triage` through `orchestrator`'s `taken_items`. No file under `crates/clm-triage/src` SHALL run `git fetch` other than through `fetch_retrying` or `fetch`.

#### Scenario: a draft whose fetch loses a ref race
- GIVEN a `bug` item `s` on `origin/main`'s backlog, unclaimed, a free place, and an executable `git` first on `PATH` that, for the first `fetch` naming `origin`, prints `error: cannot lock ref 'refs/remotes/origin/impl/a'` on stderr and exits 1, and runs the real `git` for every other call
- WHEN `clm draft s` runs
- THEN it exits 0, prints its launcher, and `origin` holds `propose/s`

#### Scenario: a draft whose fetch keeps failing
- GIVEN the same, with a `git` that fails every `fetch` naming `origin` that way
- WHEN `clm draft s` runs
- THEN it exits 2 with a message containing `cannot lock ref`, `origin` holds no `propose/s`, and the clone holds no place

#### Needs
- vcs/fetch-retries
- vcs/fetch-tries-again

#### Serves
- capability/work-concurrently

### Requirement: draft-records-promised-code-work
The prompt `clm draft` writes SHALL tell the session, in the paragraph that has it design the rest as one change, that a change whose deltas add, modify and remove no requirement makes no component pending, so no build does code work its outcome or its Design edit relies on (`component-reconciliation`), and that before it applies the change it records each piece of such work: work that a requirement would have to state is added to the change as that requirement, so the change has something to build (`dependency-work-is-a-requirement`); work that changes nothing a requirement states, such as removing code nothing calls, goes into a `kind: deferred` backlog item whose `components` name the components whose `paths` hold the code and whose body names that code, which a code branch later resolves with the trailer `Resolves: backlog/<slug>` (`deferred-work-resolves-in-code`). It SHALL tell the session, in the paragraph that names the builder check, to ask the subagent too whether the change's outcome promises code work that no requirement it adds or modifies makes pending, and to record each such piece of work the same way. The text says what `plugin.propose/propose-archives-design-only` has `/clm:propose` say, so the two paths that write spec changes record promised code work alike, and keeps to the wording `draft` allows.

#### Scenario: prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that contains `gate/undeclared-burden` also contains `component-reconciliation`, `kind: deferred` and `deferred-work-resolves-in-code`, and the paragraph that contains `builder-check-asks-order` also contains `no requirement it adds or modifies makes pending`

### Requirement: draft-resume
`clm draft --resume <change-id>` SHALL fetch `origin`, take the place `<change-id>` (reusing it when it still counts, `orchestrator/places`), and, when `origin/spec/<change-id>` or `origin/autosync/spec/<change-id>` exists, write to `.clm/prompts/draft/<change-id>.md` the prompt for a session that switches its checkout to that branch (the newer of the two when both exist) as `spec/<change-id>`, reads its `changes/<change-id>/change.md` and the items it names, and carries the draft on from where its commits leave off under the same rules as a draft `clm draft` writes, then print the launcher as `clm draft` does and `draft <change-id>` as the last line on stderr, and exit 0. When neither branch exists it SHALL free the place it took, claim nothing, print that the draft left nothing to resume, and exit 1; the items the draft claimed stay held until their claims lapse. It SHALL exit 3, taking nothing, when no place is free.

#### Scenario: a draft that pushed
- GIVEN `origin/spec/20261112-x` holding `changes/20261112-x/change.md`, whose draft's session ended without returning
- WHEN `clm draft --resume 20261112-x` runs
- THEN it exits 0, `.clm/prompts/draft/20261112-x.md` names `spec/20261112-x`, the place `20261112-x` counts, and stderr ends `draft 20261112-x`

#### Scenario: nothing pushed
- GIVEN no `spec/20261112-y` or `autosync/spec/20261112-y` on `origin`
- WHEN `clm draft --resume 20261112-y` runs
- THEN it exits 1 saying the draft left nothing to resume, and no place `20261112-y` is left

#### Serves
- capability/build-from-spec
- capability/triage-unattended

### Requirement: triage-web-answers
`clm triage` SHALL print, right after each `human <slug> ...` line, one line `answered <slug> <writer> <at> <answer>` per open answer of the ledger fold over `origin`'s ledger refs and the clone's own (`ledger/answer-records`) that names that item, oldest first, and none for an item no open answer names. It SHALL print no `answered` line for an item that has no `human` line. When the ledger refs can't be read it SHALL print no `answered` line and say so on stderr. Its other lines, their order and its exit code SHALL be unchanged by it.

#### Scenario: answered in the web app
- GIVEN an `origin` whose backlog holds question `q` and `human-task` `h`, and `origin/clm/ledger/alice/web` holding an `answered` record naming `q` with answer `yes` at `2026-11-12T09:00:00Z`
- WHEN `clm triage` runs
- THEN the line after `human q ...` is `answered q alice/web 2026-11-12T09:00:00Z yes`, and no `answered` line follows `h`'s line

#### Serves
- capability/follow-from-browser
- capability/triage-unattended

#### Needs
- ledger/answer-records

### Requirement: draft-question-choices
The prompts `clm draft` writes SHALL tell the session that when it adds a `question` item that a short answer settles, it lists the answers that settle it under `choices` (two to five, each a short line, `web-answers-as-ledger-records`), so the human can answer it with a tap in the web app, and lists none for a question that needs explaining.

#### Scenario: prompt
- GIVEN the prompt `clm draft` writes for an item
- WHEN the tests read it
- THEN it names `choices` beside the `question` kind and says to list none for a question that needs explaining

#### Serves
- capability/follow-from-browser
- capability/triage-unattended

#### Needs
- model/backlog-choices

### Requirement: finish-delete-loses-race
When `origin` refuses a delete `clm draft --finish` makes of `spec/<change>` or `autosync/spec/<change>` (`draft-finish`) because the branch is no longer at the tip the fetch read, `clm draft --finish` SHALL read that branch on `origin` again and go on, as it does for a claim: a branch `origin` no longer has SHALL be skipped as one that doesn't exist, since another session or command deleted it after the fetch, and a branch whose tip moved SHALL be kept. It SHALL print no `removed origin/<branch>` line for either, SHALL still do the rest of the finish, the local branch's delete and its `removed spec/<change>` line among it, and the refusal SHALL NOT make it exit 2. When a delete fails and `origin` still has the branch at the tip the fetch read, the failure is git's and it SHALL exit 2.

#### Serves
- capability/work-concurrently

#### Scenario: autosync branch deleted since the fetch
- GIVEN the branches and worktree of "folded draft's branches deleted", and an executable `git` first on `PATH` that, before it runs the first `push` naming `origin`, deletes `autosync/spec/20261027-f` on `origin`, and runs the real `git` for every call
- WHEN `clm draft --finish 20261027-f` runs
- THEN it exits 0, prints `removed origin/spec/20261027-f` and no `removed` line naming `autosync/spec/20261027-f`, the worktree and the local `spec/20261027-f` are gone, and `origin` has neither branch

#### Scenario: spec branch pushed since the fetch
- GIVEN the same, with a `git` that pushes a new commit to `spec/20261027-f` on `origin` instead
- WHEN `clm draft --finish 20261027-f` runs
- THEN it exits 0, prints no `removed` line naming `origin/spec/20261027-f`, `origin/spec/20261027-f` is at the new commit, and `origin` has no `autosync/spec/20261027-f`

#### Scenario: a delete git fails
- GIVEN the same branches, with a `git` that fails every `push` naming `origin` with `fatal: unable to access` and exit 128, changing nothing on `origin`
- WHEN `clm draft --finish 20261027-f` runs
- THEN it exits 2
