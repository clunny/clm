---
status: applied
resolves:
  - azure-pull-requests-can-fail-to-open-and-the-build-s-review
---

## Intent

On `host = "azure"`, `clm pr open` and `clm pr body` cut a body over Azure DevOps's 4000-character description limit to fit, with a note saying so, instead of `az` refusing it; when `az repos pr create` answers without the pull request's ID or web URL, the adapter lists the branch's active pull request again before failing, and a failure quotes what `az` said. Whenever a body doesn't reach a pull request whole (the host refused it, or Azure's limit cut it), `clm pr open` keeps the whole body under the clone's common git directory and names the path, so it outlives the build's worktree; the build prompt tells the session that a host may cut a long body, to land anyway when the pull request doesn't open, and to name the kept body in its final message.

## Problem

Three builds on an Azure DevOps project landed with no new pull request. One body was over 4000 characters, which Azure refuses. Two got `az repos pr: no pullRequestId or repository.webUrl in the answer`, a message that doesn't say what `az` did answer; the pull request may even have opened, since `create` succeeded. `clm land` went on each time, as designed (`land-locally`), but the review record (mutants, reviewer findings, gaps left) lived only in the build worktree's `.clm/pr-body.md`, which `clm run --finish` removes with the worktree, so the orchestrating session copied the files out by hand.

## Approaches

- **Fit the body in the adapter, keep the whole outside the worktree, and say so in the prompt (chosen).** The limit is a fact of one host, so the Azure adapter owns it (`host-adapters`): the pull request still opens with a diff to read, and the note on it says where the whole is. Keeping the body is `pr_open`'s, the one call every `clm pr open` passes through, so builds, drafts and a person running the command all keep theirs, with no change to `--finish`, `wrap-up` or the draft's end (`reuse-first`).
- **Refuse a body over the limit up front.** Leaves the session to shorten and retry; an unattended build that gets it wrong lands with no pull request at all, the very outcome reported. Rejected.
- **Have `clm run --finish` copy `.clm/pr-body.md` out before removing the worktree.** Copies every build's body, most of which reached their pull request, and misses drafts and fold branches, whose worktrees go by other routes. Rejected.
- **Have the session paste the body into its final report.** Prompt only, but a long body bloats the orchestrating session's context and its relay summarises it away. Rejected; the report names the kept file instead.
- **Read the pull request's URL from other fields of `create`'s answer (`url`, `repository.url`).** Those are REST API URLs; turning them into a web page guesses at Azure's URL scheme. Rejected: the retry lists the branch's active pull request instead, the call and parse the adapter's first step already makes and reads.

## Outcome

- `host/azure-body-limit` (added): `pr open` and `pr body` on Azure cut a body over 4000 UTF-16 code units at a line break, add a note naming the kept copy, keep the whole, and say so on standard error.
- `host/azure-unreadable-answer` (added): a `create` answer without `pullRequestId` or `repository.webUrl` is followed by another `pr list`; failing that, the error quotes the answer.
- `host/keep-unopened-body` (added): a failed `pr open` keeps its body at `<common git dir>/clm/pr-bodies/<branch>.md` and its error names the path.
- `host`'s Contract says `pr open` keeps a body that didn't reach its pull request whole and `pr body` cuts to the host's limit; its Design gains the three paragraphs.
- The guide's Azure paragraph (`docs/guide/ci-and-deployment.md`) names the limit, the cut and where a body is kept.
- `orchestrator.run/prompt-unopened-pr` (added): the build prompt says a host may cut a long body, naming no host or limit (`host-adapters`), lands when the pull request doesn't open, and has the final message name a kept body. Needs `host/keep-unopened-body`.

## Constraints

- Edge case known: kept bodies accumulate under `<common git dir>/clm/pr-bodies/`, one per branch, overwritten by the branch's next failure; nothing deletes them, since one is written only when a body failed to reach a pull request whole, and the person reading it deletes it (`yagni`).
- Edge case known: Azure's own count may differ from UTF-16 code units for some text; counting UTF-16 never counts fewer than a count of characters, so a cut body is never refused for length.
- The draft prompt (`orchestrator.triage`) isn't changed: a draft's `clm pr open` keeps its body the same way, and a draft whose pull request didn't open ends with its land line as before.
