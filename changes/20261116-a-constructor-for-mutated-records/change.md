---
status: applied
resolves: [adding-left-to-mutated-records-breaks-verifys-struct-literals]
priority: P2
---

## Intent

Lets the `left` field of `20261116-a-mutation-pass-that-stops` land without breaking `verify`'s build, in the steps `coupled-changes-sequence` asks for. `ledger` first offers `Body::mutated(...)`, a constructor of the `mutated` body, beside the variant, its shape unchanged (`ledger/mutated-constructor`); `verify` then builds every `mutated` body, in its code and its tests, through that constructor (`verify/mutated-through-constructor`); and only then does `ledger/mutated-record` add `left`, with the constructor setting it empty and `Body::with_left` setting it, so no caller changes when the field lands. `verify/mutation-pass`, which needs `ledger/mutated-record`, then sets `left` through `with_left`.

## Problem

The `ledger` build of `ledger/mutated-record` stopped: adding `left` to `clm_ledger::Body::Mutated` breaks the struct literals `verify` builds it with, in `crates/clm-verify/src/mutate.rs`, `tests/file.rs` and `tests/mutate.rs`, and a build may change only its own component's files (`path-ownership`). The change that added `left` wrote no step that moves `verify` off the literal first. Every pattern of the variant, in `ledger` and `verify` alike, already ends in `..`, so only the literals break.

## Approaches

- **A constructor, then the move, then the field (taken).** As `coupled-changes-sequence` and the item lay out; `left` is set by `Body::with_left` on what the constructor returns, so the constructor's signature never changes and no caller's call changes when the field lands.
- **A second constructor that takes `left`.** Rejected: two constructors to keep in step for one variant, where a method that sets one field does.
- **Setting `left` by matching the constructor's result mutably in `verify`.** Rejected: it puts the variant's field names back in `verify`, the coupling this change removes.
- **A `Mutated` struct inside the variant, with `Default`.** Rejected: it changes the variant's shape, which breaks every pattern of it in `ledger` and `verify`.

## Outcome

`ledger` (`deltas/ledger.md`):
- `ledger/mutated-constructor` (added): `Body::mutated(commit, since, killed, unproven, not_judged)` beside the variant.
- `ledger/mutated-record` (modified): the constructor sets `left` empty and `Body::with_left` sets it; `#### Needs` `verify/mutated-through-constructor`.

`verify` (`deltas/verify.md`):
- `verify/mutated-through-constructor` (added): every `mutated` body in `verify`'s code and tests is built through `Body::mutated`, and `Body::Mutated` appears only in patterns ending in `..`; `#### Needs` `ledger/mutated-constructor`.

The build order is `ledger/mutated-constructor`, then `verify/mutated-through-constructor`, then `ledger/mutated-record`, then `verify/mutation-pass`, as each `#### Needs` holds. The workaround flag `backlog/workaround-ledger-mutated-record-1.md` is deleted: the requirement it names changes, and the obstacle it records is gone.

## Constraints

- `ledger`'s own test `crates/clm-ledger/tests/mutated.rs` builds the variant by literal; it is `ledger`'s file, so the build of `ledger/mutated-record` updates it with the field.
- `verify/mutated-through-constructor` is proven by reading `verify`'s files, which a build that adds a literal later would fail.

- The constructor's parameters take the variant's field types (`String`, `Option<String>`, `Vec<String>`), so `verify`'s calls pass what its literals passed; `with_left` returns any body but a `mutated` one unchanged.
- The builder check and the architecture audit counted `verify`'s literals differently (four or five, one line in `tests/mutate.rs` read as a literal or a pattern); the requirement names none by line, so its build moves every literal it finds.

## Open questions

None.
