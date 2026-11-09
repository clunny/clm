---
status: applied
resolves: [init-readers-scenarios-need-check-affected]
---

## Intent

One repo-wide rule marks the tests that read this repository's doc-only files: `clm/doc-readers-marked` widens from `crates/clm/tests/` to every crate's tests, and its scenarios, proven through `clm-affected`'s real `readers`, name the readers in other crates too. `init/doc-readers-marked`, whose scenarios no `init` test could prove that way, is removed with its copied test.

## Problem

`init/doc-readers-marked`'s scenarios say what `readers` selects for a changed doc, but a `clm-init` test can't call `clm-affected`: `check.affected` depends on `check`, which depends on `init`, so the dev-dependency would be an import `depends_on` can't declare without a cycle (`check/undeclared-imports`). The citing test, `crates/clm-init/tests/doc_readers.rs`, matches globs with its own copy of the logic instead, so the scenarios are not proven through the code they describe (`backlog/init-readers-scenarios-need-check-affected`).

## The human's answers

From the triage session of 2026-11-09:

> Question: init can't depend on check.affected (check already depends on init, so that would be a cycle). How should init's `doc-readers-marked` scenarios be proven?
>
> Answer: "One repo-wide rule on clm (Recommended)": widen clm/doc-readers-marked to every crate's tests, proven through the real readers; remove init/doc-readers-marked and its copied test.

## Approaches

- **One repo-wide rule on the root (chosen, the human's answer).** `clm` depends on every component, so `crates/clm/tests/doc_readers.rs` already runs the real `readers` on this repository's markers; widening the rule to every crate's tests lets it prove the scenarios `init` couldn't, and other crates' readers (`clm-model`, `clm-context`, `clm-config`) gain scenarios without a rule each. Recorded as `doc-readers-one-rule`.
- **Give `init` a dependency on `check.affected`.** A cycle (`check.affected` → `check` → `init`). Rejected.
- **Keep `init`'s rule and move only its proof to `crates/clm/tests/`.** A test under the root citing `init`'s requirement counts toward `init` (`tests-count-where-cited`) but lives in the root's paths; two rules saying the same thing for two crates, and a third crate's readers would need a third. Rejected for one rule (`yagni`).

## Outcome

- `clm/doc-readers-marked` covers every file of a crate under `crates/` holding a test (integration tests under `tests/`, unit-test modules under `src/`), takes `init`'s sentence that a reader of the guide as a directory marks `docs/guide/**`, and adds scenarios for `clm-init`'s guide and `ARCHITECTURE.md` readers, `clm-model`'s and `clm-context`'s `ARCHITECTURE.md` readers and `clm-config`'s settings-page reader. Its build extends `crates/clm/tests/doc_readers.rs` and re-cites it at the new version. Every file the new scenarios name already holds its marker, so the build changes nothing outside `crates/clm/` (`path-ownership`); a reader it finds in another crate without a marker is filed as a `kind: bug` backlog item on the component whose `paths` hold it, citing `clm/doc-readers-marked`, for that component to fix (from the architecture audit). The builder check found one: `crates/clm-config/tests/config.rs` reads the repository's `clm.toml` with no marker, filed as `backlog/config-test-reads-clm-toml-without-a-marker`. `clm-model`'s `include_str!` of `ARCHITECTURE.md` is compiled-in code, not a test; its tests hold the marker.
- `init/doc-readers-marked` is removed; `init`'s build removes `crates/clm-init/tests/doc_readers.rs`, whose marker becomes `obsolete` (`no-dead-code`). The markers in `clm-init`'s files stay.
- `web.site/doc-readers-marked` stays: the site's tests are not a crate's.
- `docs/decisions/guide-pages-are-doc-only.md` no longer names `init`'s rule, and `docs/decisions/doc-readers-marked.md` gains an amendment naming `doc-readers-one-rule` (hand edits).

## Constraints

- No workaround is known to be needed. Edge case: a scenario names files in other crates, so moving or renaming one of them (such as `crates/clm-config/tests/settings_guide.rs`) fails the root's test; that is the rule doing its job, and the move updates the scenario.
- From the builder check: neither requirement relies on one not yet current (`check.affected/affected-readers` is current); a scenario's "selects" means its files are among those selected, since other readers match too (`clm-model`'s other tests, `crates/clm/tests/plugin*.rs`).
- No capability test sees anything different at the product's boundary: the rule concerns this repository's own tests.
