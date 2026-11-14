---
title: "A spec change needs review or it doesn't; nothing is reviewed after it lands"
date: 2026-10-29
status: accepted
---

Refines `humans-review-intent` (`docs/the-clm-project.md`, `changes/archive/20261029-refounding/design.md` §5). Three review tiers once had humans read every change to the implementation rules: a modified requirement waited for approval (`HARD`), a new one was reviewed after it landed (`SOFT`), and a declared floor, a significance rubric and a list of significant nodes grew around them. Most of that review was of rules the human never needed to see, and what went unreviewed was what each feature cost.

So review is yes or no. A spec branch **needs review** exactly when:
- its diff touches `ARCHITECTURE.md` or `CAPABILITIES.md`, the human layers, or `docs/allowed_signers`, the keys a signed repository trusts (`signed-ledger-records`);
- it changes the `approval` setting in `clm.toml` (`approval-setting-is-spec`);
- it adds or widens a component's exemption from a code rule; removing or narrowing one doesn't (`temporary-exemptions`);
- a `change.md` it adds or modifies declares `human_burdens`, or can't be read: what the change spends of the human's money or time, or can't undo, one short line each.

`clm gate` prints `spec review` for such a branch, which lands only after a human's `clm approve` (`approval-policy`), and `spec` for any other spec branch, which lands on checks. Nothing is reviewed after it lands: there is no `clm review`, no `reviewed` record and no review queue. A spec branch that adds a `kind: human-task` backlog item and declares no burden is blocked (`gate/undeclared-burden`), since a step only the human can take spends their time; builds still file `question` and `human-task` items freely, and those block only their own requirements.

A `landed` record carries `review`, a bool; older records, which carried a `tier`, read `HARD` as true and every other tier as false. The audit recomputes the bool for each signed land, on demand with `clm ledger verify --review` and in the checkpoint (`checkpoint-recomputes-new-lands`).

`approval-policy`, `per-writer-ledgers` and `backlog-triage` hold under this rule: where they speak of tiers, `## Significant`, `clm review` or `reviewed` records, this decision replaces that part, and the rest of them stands.

## The fold: one approval for a triage run's drafts

A `/clm:triage` run drafts each group in its own subagent. A draft whose gate prints `spec` lands itself with `clm land`. One whose gate prints `spec review` opens no pull request: it stops after its last push and closes with `proposed <id> review`. Were each such draft approved on its own branch, the human would approve them one at a time, and each land would leave the others stale wherever they edit the same Designs and requirements, each needing a merge and a fresh approval, since an approval is pinned to the diff (`approve-pins-the-review`). One run of 34 bug groups left 23 such branches, and 9 went stale against each other after the first landed.

So once every draft has returned, the `triage` skill runs `clm draft --fold <id>...` with those IDs, in `clm triage`'s order, and gives the prompt it writes to one more background subagent, the fold (`orchestrator.triage/fold`). With one change the fold only opens its pull request. With several, it cuts a fold branch `spec/<batch>` from `origin/main` and brings each draft's change over as one commit of its diff outside `specs/` (`git diff origin/main...origin/spec/<id>`, applied with `git apply --3way`, committed as `Fold <id>`). It never merges: `clm rebase` rebases without keeping merge commits, so a merge's conflict resolutions would be lost at the next rebase. Each change keeps its ID, its `change.md` and its deltas: one branch carries several changes, each of whose `change.md` the gate reads for `human_burdens`, which `clm rebase` re-applies in ID order and `clm land` records together (`landed`'s `changes`). Where two changes modify the same element, `clm rebase` reports the later one's delta stale against the earlier (`stale-delta-names-movers`), and the fold merges the earlier edit into it, once and before the approval. The fold then runs the builder check on the elements where two changes met, has the design audited (`spec-architecture-audit`), writes one pull request body holding each change's sections, opens one pull request and stops; a change it can't fold without a decision only the human can make is left out, with `left <id>: <reason>`, and its branch stays as its draft left it. The skill asks the human once, through the guard (`approval-policy`), lands the branch, and runs `clm draft --finish` on each folded draft, which deletes a draft's branches and removes its worktree once its change is on `main`. A fold takes none of the clone's build places. A rename change is folded only alone, since the gate classifies a rename branch by replaying it (`rename-by-replay`).

- **Each change is kept, not rewritten into one.** Folding the drafts' text into a single new change would need every delta for a component rewritten as one, and the drafts' changes would never reach `main`, so nothing would tell their branches, worktrees and items apart from abandoned work. Kept whole, each change reaches `main` under its own ID: `clm draft --finish` finds it there, and the drafts' branches keep holding their items (*proposed*) right up to the land, with no change to what holds an item (`propose-claims`). Nothing is pushed to a draft's branch, so a fold that stops loses nothing: each draft still shows as `open` for `/clm:propose`.
- **The fold is a subagent.** The merge of overlapping sections is design work, which the triage session never does in its own context, and the fold's prompt is CLI text, as the draft's is.

Rejected for the fold:
- **One branch per draft that needs review, approved in order.** N approvals, and each land staling the rest.
- **Drafts in sequence on one shared branch.** Loses the parallelism `backlog-triage` is for, and a draft's failure would block the rest.
- **Folding every draft.** Those that need no review land on checks alone, so folding them only delays them behind the approval.
- **Relying on `clm rebase`'s three-way merge instead of a fold** (`stale-delta-three-way-merge`). It makes each fresh merge cheaper, but every land would still stale the rest and cost a fresh approval.

## Rejected

Keeping `SOFT` as a lighter review (it is the after-the-fact queue that piled up 31 unread items); keeping the tiers and only removing the rubric (the human would still review rules one by one); having a model judge whether a change needs review (`deterministic-enforcement`).

## The rule in full

`ARCHITECTURE.md`'s entry `humans-review-intent` states this in short (`human-layer-entries-stay-short`); the rule in full:

Humans SHALL review intent, not code. A spec branch needs a human's approval, given with `clm approve` before it lands, exactly when it edits `ARCHITECTURE.md`, `CAPABILITIES.md` or the trusted keys in `docs/allowed_signers`, changes `clm.toml`'s `approval` setting, adds or widens a component's exemption from a code rule (`exemptions-end`), or adds or edits a change that declares `human_burdens` or whose `change.md` can't be read, which fails closed. A branch whose whole diff `clm rename` reproduces from the base and the renames its change records is a spec branch, its rewritten test markers included, and its replayed edits to `ARCHITECTURE.md` and `CAPABILITIES.md` don't need review, since renaming an ID changes no intent; one that swaps a term does, since only a human can judge that a swapped word keeps a rule's meaning (`rename-by-replay`). Every other change lands on checks, and nothing is reviewed after it lands (`yes-no-review`).

**Scenario: capability edit**
- GIVEN a spec branch that edits only `CAPABILITIES.md`
- WHEN `clm gate` judges it
- THEN its first line is `spec review`, and `clm land` refuses it until a human has approved it

**Scenario: node rule edit**
- GIVEN a spec branch that only rewords a component's requirement and declares no `human_burdens`
- WHEN `clm gate` judges it
- THEN its first line is `spec`, and it lands on checks

**Scenario: exemption added**
- GIVEN a spec branch that only adds an exemption to a component's frontmatter, with the backlog item that ends it
- WHEN `clm gate` judges it
- THEN its first line is `spec review`

**Scenario: rename of an architecture entry**
- GIVEN a branch holding only what `clm rename` produced from a change renaming an `ARCHITECTURE.md` entry's slug
- WHEN `clm gate` judges it
- THEN its first line is `spec`, and it lands on checks
