---
component: ledger
base: { frontmatter: "2e23fa95f17672f0" }
---

## MODIFIED Frontmatter

```yaml
serves: [capability/approve-intent, capability/land-safely, architecture/git-is-truth]
```

## MODIFIED Requirements

### Requirement: verify <!-- base: ff3140018fc2eadb -->
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
- architecture/git-is-truth

### Requirement: answer-records <!-- base: 96b5bcca88252e44 -->
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
- architecture/git-is-truth
