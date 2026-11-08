---
status: done
priority: P1
---

## Intent

Make the upgrade corpus test decline the allow-rules note migration, `20261107-projects-set-up-before`, only for an entry that has it pending. Today `clm/upgrade-corpus` declines it for every entry, but an entry written by a `clm init` that already seeds the allow rules (`init/init-seeds-permissions`) doesn't have it pending, so `clm upgrade --decline` exits 1 with "is not a pending note migration". No committed entry is that new yet, so the tests pass on `main`. But a release records its own entry before it runs the tests (`ci/release-records-upgrade-corpus`), so every release from v0.21.0 on fails in `prepare`. The v0.21.0 release failed this way. This is P1 because no release can be cut until it lands.

## Problem

`check_entry` in `crates/clm/tests/upgrade.rs` runs `clm upgrade --decline 20261107-projects-set-up-before` unconditionally after `--apply`, as `clm/upgrade-corpus` says. The rule was written when every corpus entry predated `init-seeds-permissions`, so the migration was pending for all of them.

## Approaches

- **Decline only when pending** (chosen by the human): after `--apply`, the test declines the migration only when the `clm upgrade` listing names it as pending for that entry. This changes nothing a user sees, keeps `--decline`'s refusal of a migration that isn't pending (which catches a mistyped ID), and states exactly what the test means: a project that doesn't need the hand edit isn't asked to decline it.
- **Make `clm upgrade --decline` of a migration that isn't pending a no-op**: fixes the test without touching it, but changes the CLI for every user and loses the error that catches a wrong ID, to suit a test.
- **Leave the decline out once the corpus passes some version**: hard-codes a release number into the test, and the next migration of the same kind needs the same edit.

## Outcome

`clm/upgrade-corpus` says the test declines `20261107-projects-set-up-before` only when `clm upgrade`, run after `--apply`, lists it as pending, with a scenario `an entry set up with the allow rules`. The build changes `check_entry` and adds a test for an entry newer than the migration, which a fresh `clm init` can provide, so the case is proven before a release adds such an entry to the corpus.

## Constraints

Every other step of the corpus check stays as it is: `up to date` at the end, `clm check` passes, and the files compare as before.
