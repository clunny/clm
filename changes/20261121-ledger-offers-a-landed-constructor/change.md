---
status: proposed
---

## Intent

`ledger` offers `Body::landed(commit, branch, kind, review, fingerprint)`, a constructor of the `landed` record body, beside the variant, with one method for each of its optional fields. It is a prerequisite of wave 3's branch A (`20261121-wave-3-land-by-the-rule`), whose `ledger/landed-record-fields` gives the `landed` record a field `light`. The field marks a land the gate reads as `light` (spec and code together, `gate/light`) while `land_kind` stays `code`, so a `clm` on an older release reads such a land as a code land.

## Problem

Rust can't add a field to an enum struct variant without breaking every struct literal of it. Eleven literals of `Body::Landed` sit outside `clm-ledger`:
- one in `land.approval`'s tests (`crates/clm-approval/tests/keys.rs:81`);
- two in `orchestrator.checkpoint`'s tests (`crates/clm-checkpoint/tests/wrap_up.rs:231` and `:1739`);
- one in `check.health`'s tests (`crates/clm-health/tests/land_time.rs:21`);
- one in `verify`'s tests (`crates/clm-verify/tests/recheck.rs:26`);
- three in the root's tests (`crates/clm/tests/health.rs:52`, `ledger.rs:42` and `none_remote.rs:93`);
- three in `land`'s code (`crates/clm-land/src/batch.rs:664`, `finish.rs:194` and `land.rs:653`).

Every other use outside the crate is a pattern with `..`, which a new field doesn't break. A build changes only its own component's files (`coupled-changes-sequence`). So `ledger` can add the field only once those callers build the body through a constructor, and the constructor must be on `main` before they can move to it. `land`'s three must move too: `land` builds after `ledger` in branch A, and `main` has to compile in between.

## The constructor's form

A `landed` record has eleven fields. Five are set by every caller: `commit`, `branch`, `kind`, `review` and `fingerprint`. Six are optional: `code_key`, `changes`, `check_seconds`, `steps`, `recheck` and `recheck_files`. Two of the test literals leave all six empty, the other six tests set one to three, and only `land`'s three set most of them. So `Body::landed` takes the five, and one method per optional field sets it, as `Body::mutated` and `with_left` do for the `mutated` record. Today's literals then read:
- a test's literal becomes one call, with a method only where it sets something: `Body::landed(c, "impl/x".into(), LandKind::Code, false, "f".into()).with_check_seconds(Some(50))`;
- `land`'s `land.rs:653` becomes the five and a chain of six methods, each taking the field's own type (`Option<_>` or `Vec<_>`), so an expression such as `timing.as_ref().map(|t| t.seconds)` passes as it stands.

Each method returns any other body unchanged, so a caller can't fail on the wrong kind. A field branch A adds later (`light`) starts at the value a record without it reads as (false), with a method of its own, so no call changes meaning. A single constructor of all eleven fields was rejected: every test would spell six empty values, and the next field would break every call again.

## Outcome

- `ledger/landed-constructor` is added and the Contract names the constructor and its methods. Nothing else changes, and nothing is cut.
- After this change is built, the session moves each component's literals onto the constructor on small code-only branches (`small-change-no-claim`), one per component: `land.approval`, `orchestrator.checkpoint`, `check.health`, `verify`, the root and `land`. Branch A is then rebased onto `main`; its `ledger/landed-record-fields` adds the field and `Needs` this requirement.
