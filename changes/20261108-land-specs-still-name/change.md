---
status: applied
resolves: [land-specs-still-name-a-local-main-base]
---

## Intent

Make `land/derived-specs` and the decision `land-policy-from-base` name the base every land is made on, `origin/main`, alone, as `land/offline-land` and `land.approval/approval-from-base` already do (`none-keeps-its-own-remote`). This resolves `land-specs-still-name-a-local-main-base`.

## Problem

Since `none-keeps-its-own-remote`, `clm land` lands through `origin` on every host, `host = "none"` included, and never on the local `main` alone (`land/offline-land`); `clm-approval` no longer keeps a local-main base either (`14b1f5b`). Two texts still describe the old base:

- `land/derived-specs` regenerates `specs/` "onto the base (`origin/main`'s, or the local `main`'s when landing offline)". No land is made offline or on the local `main` any more, so the parenthesis names a case that can't occur.
- `docs/decisions/land-policy-from-base.md` reads `approval` from "the `origin/main` it fetched (or the local `main` without a remote)", with no `Refined by` line naming `none-keeps-its-own-remote`, as `land-locally` and `host-adapters` have.

## Approaches

1. Modify `land/derived-specs` to name `origin/main` alone, and add a `Refined by` line to the decision, leaving its original text as recorded (chosen). A decision records what was decided at the time; the repository refines a decision by appending a dated `Refined by` line, as `land-locally` and `host-adapters` do.
2. Rewrite the decision's sentence in place. Rejected: it erases what was decided then, and the other two decisions refined by `none-keeps-its-own-remote` were not rewritten.

## Outcome

- `land/derived-specs` regenerates `specs/` onto `origin/main`'s tree. Its behaviour is unchanged: `clm-land` already reads the base as `origin/main` on every host (`crates/clm-land/src/lib.rs`), so its build re-stamps the requirement's tests (`crates/clm-land/tests/derived.rs`) at the new version and changes no code.
- `docs/decisions/land-policy-from-base.md` gains a line `Refined by none-keeps-its-own-remote (2026-11-07)`, saying the base is `origin/main` on every host.

## Constraints

- No edge cases or workarounds are known.
- `land/reconnect-names-rebase`'s scenario still names a branch landed on the local `main` while offline: `clm reconnect` exists for lands a clone made before `none-keeps-its-own-remote`, so that text stays.
