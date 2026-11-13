---
component: gate
---

## MODIFIED Requirements

### Requirement: up-to-date <!-- base: 6cf6950ef1e9fb74 -->
The gate SHALL block, with rule `gate/up-to-date` and a message telling the author to run `clm rebase`, when `HEAD` doesn't merge cleanly into `<ref>`, or when a file under `specs/` changed both between the merge base and `<ref>` and between the merge base and `HEAD`. A branch that is merely behind `<ref>` SHALL NOT be blocked for that.

#### Scenario: behind, merges cleanly
- GIVEN `main` has a commit changing `crates/a.rs` that the branch lacks, and the branch changes only `crates/b.rs`
- WHEN `clm gate --base main` runs
- THEN it reports no `gate/up-to-date` violation and exits 0

#### Scenario: conflict
- GIVEN `main` and the branch each change the same line of `crates/a.rs` differently since their merge base
- WHEN `clm gate --base main` runs
- THEN it exits 1 with a `gate/up-to-date` violation naming `clm rebase`

#### Scenario: spec changed on both sides
- GIVEN `main` and the branch each change a different requirement in `specs/n/SPEC.md` since their merge base
- WHEN `clm gate --base main` runs
- THEN it exits 1 with a `gate/up-to-date` violation, although the text would merge cleanly

#### Needs
- clm/renamed-commands

#### Serves
- capability/land-safely
- capability/work-concurrently
- architecture/hash-based-staleness

### Requirement: evaluate-at <!-- base: 0bfe14a19c47f753 -->
`clm-gate` SHALL offer `evaluate_at(repo, base, head, approved)`, which SHALL return the kind, whether the branch needs review, and the violations that `clm gate --base <base>` gives with `head` checked out, reading `head` only as a revision, so that what is checked out, the working tree and the index change nothing. `clm gate` SHALL be `evaluate_at` with `HEAD`.

#### Scenario: another commit
- GIVEN a repository with `main`, a commit `s` on top of it that only adds a requirement to `specs/a/SPEC.md`, and `HEAD` checked out at `main` with an uncommitted edit to `ARCHITECTURE.md`
- WHEN `evaluate_at(repo, "main", s, false)` runs
- THEN it returns kind `spec` needing no review, as `clm gate --base main` prints with `s` checked out, and `HEAD`, the index and the working tree are as they were

#### Scenario: a merge commit against its first parent
- GIVEN a merge commit `m` of a branch that only changes `ARCHITECTURE.md` into `main`, with `main` as its first parent `p`
- WHEN `evaluate_at(repo, p, m, false)` runs
- THEN it returns kind `spec` needing review, with `gate/approval` among its violations

#### Serves
- capability/land-safely
- architecture/git-is-truth
