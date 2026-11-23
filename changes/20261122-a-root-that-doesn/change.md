---
status: applied
resolves: [a-root-that-doesn-t-exist-is-reported-as-origin-that-can-t-be-reached]
---

## Intent

A fetch that can't start `git` in its directory says so, naming the directory, instead of reporting `origin can't be reached`. `clm triage --root /nonexistent/x` and each form of `clm draft` given such a root then print a message that names the path the person gave, and exit 2 as before.

## Problem

`vcs/fetch-retries` says that when the last run of the fetch fails, `fetch_retrying` runs `git ls-remote` and returns `Offline` when that fails too. In a directory that doesn't exist neither `git` can start, both runs return `Error::Spawn`, and the rule as written returns `Offline`: the code does what the requirement says, and the message sends the person to their network when the fault is the path they gave. No requirement states the right behaviour, so the fix is a spec change (`code-fix-needs-a-written-rule`).

## Approaches

- **Return `Git` with a spawn error that names `root`** (chosen). `fetch_retrying` stops at the first run that returns `Error::Spawn`, runs no `ls-remote`, and returns `FetchError::Git(Error::Spawn { command: "<command> in <root>", source })`, displayed `<command> in <root>: <I/O error>`. No type changes shape: every caller (`clm-triage`, `clm-run`, `clm-orchestrator`, `clm-checkpoint`, `clm-land`, `clm-approval`) already displays `Git`'s error as it is, and `clm-land` and `clm-approval` match `FetchError`'s two variants exhaustively, which this leaves building.
- **A third `FetchError` variant**, such as `NoDirectory(path)`. Rejected: it widens a type `clm-land` and `clm-approval` match exhaustively, which would need those components' requirements sequenced around it (`coupled-changes-sequence`) for a message the existing variant already carries (`yagni`).
- **Check `root.is_dir()` before the first run.** Rejected: it names one cause of a spawn failure and misses the others (a `root` that is a file, `git` missing from `PATH`); reading the outcome of the run itself covers them all.
- **Name the directory in every `Error::Spawn`.** Rejected as wider than the bug: it changes the display of every `clm-git` call for a fault only the fetch misreports.

## Outcome

- `vcs/fetch-retries` gains the sentence and one scenario, *a root that doesn't exist*; every existing scenario keeps its name and text.
- `vcs`'s Contract says `Git` also covers a `git` that couldn't be started in `root`, its error then naming `root`.
- A spawn failure is not retried: it is no race over a ref, and retrying a mistyped path only costs the person 3 seconds.
- Callers change nothing: `orchestrator.triage/draft-fetch-retries-race` says `clm draft` exits 2 with git's message from the last fetch, or `origin can't be reached` when `origin` doesn't answer, which stays true.

## Edge cases and workarounds

- `Error::Spawn` also covers a `git` that started but couldn't be waited for or read. Such a run is also not retried and its message names `root`, which is still true of it; no workaround is needed.
