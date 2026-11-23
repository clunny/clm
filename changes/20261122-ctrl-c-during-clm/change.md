---
status: applied
resolves: [ctrl-c-during-clm-verify-s-filing-still-kills-its-git-children]
---

## Intent

`verify/files-unproven` says what a stop does to the filing of unproven requirements. A stop request set before the filing reaches a step means it starts no more steps, deletes its branch and prints `unproven: not filed: stopped`. A Ctrl-C at a terminal still ends the `git` command or land that is running at that moment, as it ends any `clm land`. The step it ends fails like any other, the branch is deleted, and the next call files the items. Resolves `backlog/ctrl-c-during-clm-verify-s-filing-still-kills-its-git-children`.

## Problem

`clm/verify-command` catches `SIGINT` and `SIGTERM` from the `verify passed ` line until `clm-verify`'s call returns, setting the call's stop request, so that a stop ends the mutation pass in an orderly way. The filing that follows the pass (`verify/files-unproven`) doesn't watch the stop request, and its children (the `git` commands and `clm_land::land`'s check and pushes) run in `clm verify`'s process group, so a Ctrl-C at the terminal ends them directly. Today that means:

- a Ctrl-C during the pass stops the pass, and then the filing starts anyway, running a whole land with its check, after the person asked to stop;
- a Ctrl-C during the filing ends whatever child is running. The filing then fails that step and goes on to the next, and no requirement says what should happen there.

## Approaches

- **Say what a stop does, and honour the stop request between steps (chosen).** The filing checks the stop request before writing, pushing and landing, and starts none of those once it is set. A Ctrl-C ends the running child, which fails its step. The cleanup that already follows any failure (the branch deleted here and on `origin`) runs after the signal has been delivered, so it runs. Nothing is lost: the items are worked out again from the ledger and `main`'s history on every call. The change is one check of a flag in `clm-verify` and nothing in any other component.
- **Run the filing's children in a process group of their own** (the item's first suggestion). That would be a new option on `clm-git`'s runner and on `land`'s check runner, used only here. A `git push` in a background process group can't ask for credentials at the terminal (it gets `SIGTTIN`), and a Ctrl-C would then wait out the land's whole `[land] check`, which can take many minutes, instead of stopping. That widens two other components to make the stop slower (`yagni`, `reuse-first`).
- **Ignore `SIGINT` in the children** (`SIG_IGN` set before `exec`). This has the same reach into `clm-git` and `land` as a process group, and the same wait.
- **Leave it unstated.** The item was filed because the requirement promises a stop window that covers the filing, and says nothing of the filing's children.

## Outcome

- `verify`'s Contract: the stop request in `Call` also keeps the filing from starting what it hasn't started yet.
- `verify`'s Design, **Filing**: why the filing honours the stop between steps, why its children stay in `clm verify`'s process group, and what a Ctrl-C leaves.
- `verify/files-unproven`: the stop request and the process group, with two scenarios, "stopped before it files" and "a filing command ended by a signal".
- `clm/verify-command` is unchanged. Its window already covers the filing, and what the filing does in that window is `verify`'s to say.

## Edge cases and workarounds

- A stop request set while the filing's land runs doesn't end the land: `clm land` has no stop request, so the land runs to its end, and the filing reports as it would have. Under `SIGTERM` sent to `clm` alone, that can take as long as the land's check.
- A Ctrl-C that ends the land's push to `main` part-way leaves what a Ctrl-C leaves of a `clm land` run by hand. `land` already handles that case, and nothing more is added here.
- With nothing to file, a stop prints nothing. The `stopped` note is printed only once an item has been found.

## Constraints

- `clm verify`'s outcome line, stdout and exit status stay as they are (`files-unproven`, `clm/verify-command`).
- No other component's code changes (`path-ownership`).

## Settled by the builder check and the audit

- The audit pointed out that a SHALL running the filing's children *in* `clm verify`'s process group depends on how `vcs`'s and `land`'s runners start them, which neither Contract promises. The requirement now says only what `verify` controls: it SHALL NOT put them in a group of their own.
- The stop is judged once an entry still at its current version is found, before each step that writes and commits, pushes and lands, so a stop with nothing current to file prints nothing. The scenario proves the first of those judgements. The later ones are the same flag read at a step boundary.
- The signal scenario's check is the root `clm.toml`'s `[land] check`, which `clm_land::land` reads from the clone. Its second run follows one more land that sets a passing check, so `R` and the branch's name are worked out again from the new `origin/main`.

## Open questions

None.
