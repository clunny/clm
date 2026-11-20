---
status: applied
resolves: [the-fingerprint-covers-only-the-clm-root-s-subdirectory-and-the]
---

## Intent

One definition of the approval fingerprint that `clm fingerprint` and the web review both hold to: the CLM root's tree, with paths relative to it (`fingerprint-covers-the-clm-root`). `gate.fingerprint/fingerprint` states that a path outside a CLM root that is a subdirectory never counts, and `web.worker.review/fingerprint-from-trees` states that the Worker's value equals `clm fingerprint`'s in a repository whose CLM root is its top, the only kind the Worker lists.

## Problem

`clm fingerprint` runs `git ls-tree -r -z` in the CLM root, so for a CLM repository in a subdirectory of its git repository its listings hold that subdirectory alone, with paths relative to it. The Worker hashes the Trees API's listings of the whole repository. The requirements didn't say which is right: `gate.fingerprint/fingerprint` says only that the listings are run in `repo`, and `fingerprint-from-trees`'s scenario says the Worker's value equals the command's in any repository.

## Approaches

- **The CLM root's tree (chosen).** What the command already computes. Every rule inside the fingerprint (the `specs/` skip, `added_changes`) reads root-relative paths, as `land` and `rename` do with the same listings. The Worker needs no change: it lists only repositories with `specs/SPEC.md` at the top, where the whole repository is the CLM root. In a subdirectory root an approval written from the web names a fingerprint `clm land` never computes, so it counts for nothing and the land fails closed.
- **The whole repository.** Covers files outside the root, but puts a prefix into `fingerprint_of`, `added_changes` and each caller's listings, and the Worker, which doesn't know the root, still disagrees for a subdirectory root.
- **Teach the Worker the root.** Serves no repository the Worker lists.

## Outcome

- `gate.fingerprint/fingerprint` gains the sentence on paths under `repo` and the scenario "CLM root in a subdirectory"; its build adds the test for it. The command's behaviour doesn't change.
- `web.worker.review/fingerprint-from-trees` gains the sentence tying its value to `clm fingerprint`'s for a repository whose CLM root is its top, and its scenario "same as the CLI" names that layout; its Design says why the subdirectory case fails closed. The Worker's behaviour doesn't change.

## Edge cases and workarounds

- A commit changing only files outside a subdirectory CLM root leaves an approval standing (accepted, `fingerprint-covers-the-clm-root`).
- An approval written from the web for a subdirectory-root repository counts for nothing (fails closed; accepted).

## Open questions

- The gate reads changed paths with `git diff --name-status`, whose paths are relative to the git repository's top, not to the CLM root, so in a subdirectory root `tool/specs/...` doesn't read as a spec path. Filed with `clm defect` on `gate`; whether the gate refuses a spec branch that also changes files outside the root is that fix's question.
