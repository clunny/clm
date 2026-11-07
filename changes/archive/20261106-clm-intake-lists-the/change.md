---
status: done
resolves: [clm-intake-lists-the-repository-s-own-branch-pull-requests-as]
---

# clm intake lists only pull requests from forks

## Intent

`clm intake` takes in outside contributions, which come from forks (`outside-contributions-are-intent`), but it lists every open pull request, and the repository's own branches open one each for reading a diff (`clm pr open`). `/clm:triage` then sent in-flight `impl/*` build pull requests through the issue-verifier and the `contribution` skill, which recommends closing a pull request that touches code. This change makes `host`'s `list` (`host/list-contributions`) return only the open pull requests whose head is in another repository, as `gh` reports in `isCrossRepository`, so `clm intake`'s `pull <n> <author>` lines name only contributions. `orchestrator.triage`'s Contract says the same; its `intake` requirement prints one line for each pull request `host` lists and stays true as written.

## Problem

`backlog/clm-intake-lists-the-repository-s-own-branch-pull-requests-as`: `clm intake` printed `pull <n>` lines for the repository's own `impl/*` pull requests, the verifier called them "from a fork", and following the skill would have closed them. `host/list-contributions` as written returns every open pull request, so the fix needs a spec change (`code-fix-needs-a-written-rule`).

## Approaches

- **`host`'s `list` leaves out same-repository pull requests (chosen).** `list` exists only for `clm intake`, and its name and Contract bullet are the contribution operations; GitHub already answers the question in `isCrossRepository`, so the filter is one field in the `gh pr list --json` call and one condition. Nothing else changes.
- `list` returns a fork flag and `clm intake` filters or marks the lines. More surface for no caller that wants the same-repository pull requests (`yagni`).
- The `triage` skill skips `pull` lines whose branch starts with `impl/` or `spec/`. The lines carry no branch, a branch name is the author's choice in a fork, and it would keep a rule in a prompt that the code can hold.

## Outcome

- `gh pr list` asks for `isCrossRepository` beside the number and author, and `list` drops a pull request where it is false.
- `clm intake` prints no `pull` line for the repository's own branches' pull requests.
- The guide's `contributions` page says `clm intake` lists the open pull requests from forks (`method-in-the-plugin`).
- The build updates `crates/clm-host/tests/contribution.rs`'s `gh pr list` fixture, which has no `isCrossRepository` today. No capability test feeds `gh pr list` output, so none sees a difference.

## Edge cases

- A collaborator with write access who opens a pull request from a branch of the repository itself is not listed. Accepted: such a person lands through `clm land`, and a pull request there is only for reading a diff (`land-locally`); contributions of intent come from forks.

- A pull request `gh` reports without `isCrossRepository` (an older `gh`) is left out rather than listed, so a doubt never sends a repository's own branch to the contribution flow.

## Constraints

- `untrusted-text-read-only-verifier`: `isCrossRepository` is a boolean `gh` reports, not author text, and nothing new is printed.
