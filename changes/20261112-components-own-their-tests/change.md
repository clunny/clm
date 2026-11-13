---
status: applied
resolves: [a-build-has-no-way-to-give-its-component-a-new-test-file, compile-coupled-changes-across-components]
---

## Intent

Make `path-ownership` hold again instead of growing an exception for each case. Each component owns the tests that prove it, through `tests` globs that may sit inside another component's `paths`, and reaches the product's entry point or a shared harness through a test-only dependency, `test_depends_on`. A change that another component's code or tests must follow is built in steps, each by the build of the component whose files it changes, so no build makes forced fixes elsewhere. The forced-fix and split-move exceptions go. Capability tests stay as the one permanent exception, and two transitional ones each get a backlog item that ends them.

## Problem

`path-ownership` grew from one exception to five between 2026-11-03 and 2026-11-12: forced compile fixes for a removal, then for an addition, the arm for an added variant, capability tests, `upgrade` migrations, and the split move. Each covered only the case that came up (`backlog/compile-coupled-changes-across-components`).

Two projects adopting CLM met a related wall (`backlog/a-build-has-no-way-to-give-its-component-a-new-test-file`). Their existing test suites shared a harness and imported the app's entry point, so recovery gave each suite to one component, listing its files one by one. Every build that needed a test then wrote into the suite's files, against `path-ownership`, or created a file no component owned, which only a spec edit could claim and `gate/classify` refused as `mixed`. Builds stopped, and some got around the checks by moving an import inside a function, asking for a history rewrite, or proposing a glob that hid the breach. This repository has the same shape in `crates/clm/tests/`.

## Approaches

- **Another exception for tests added to a shared suite file.** It legalises what builds do, but two builds then edit one file, and the list keeps growing. Rejected.
- **A "holding component" builds may write into and that may only shrink.** Proposed by one of the reporting projects. It names the situation but keeps the breach, and "only shrinks" contradicts "builds add to it". Rejected.
- **A `#### Touches` block that names the other components a requirement's build edits, claiming them with it.** It makes the reaching-out declared and safe from parallel builds, but builds still edit files they don't own. Rejected.
- **Clean Architecture as a principle of every CLM project.** It removes most tests' need for the entry point, but end-to-end tests still need it, it doesn't stop a Contract change rippling to its users, and its layering doesn't suit every project. Rejected as a method rule; a project may adopt it in its own `ARCHITECTURE.md`.
- **Chosen: tests owned where they prove, and coupled changes sequenced.** `tests` and `test_depends_on` (`components-own-their-tests`) put every test in its component's own files. Expand, each user's move, then contract (`coupled-changes-sequence`) keeps every build in its own files, at the cost of more, smaller builds, which the build loop runs unattended.

## Outcome

- `model/test-ownership-keys`: the `tests` and `test_depends_on` frontmatter keys.
- `check`: a file's owner is the component whose `tests` match it, else whose `paths` do (`paths-disjoint`); `unowned-files` and `undeclared-imports` read that owner, and a test file is judged against `depends_on` and `test_depends_on` together; `tests-cite-their-owner` reports a marker in a `tests` file that cites another component.
- `init/survey-components`: the component map takes the same keys and rules.
- `plugin.init/init-skill-gives-tests-their-component`: recovery writes `tests` globs, never file lists, and `test_depends_on` instead of a whole-suite component.
- `orchestrator.run`: `stays-in-its-component` replaces `removal-breaks-dependant` and `split-move-fix`. A build changes only its own files, puts new tests under its own globs, and, when a requirement has no legal home, leaves it unbuilt with a `question` item, never moving an import, hiding a file, editing a spec path or asking for a history rewrite to get past a check.
- `plugin/builder-check`, `orchestrator.triage/draft-contract-step-deferred` and `plugin.propose/propose-nests-splits` write coupled changes and splits as sequenced steps.
- `gate/classify`: a `mixed` refusal on an `impl/*` branch tells the build what to do.
- `ARCHITECTURE.md`: `path-ownership` rewritten around one rule, one permanent exception and two transitional ones; `expand-contract` covers every contract change another component uses, additions included.
- Decisions `components-own-their-tests` and `coupled-changes-sequence`; `forced-fixes-cover-additions` superseded; `path-ownership-by-prompt`, `nested-splits` and `tests-count-where-cited` amended.
- Backlog: `a-template-and-its-migration-live-in-different-components` and `clm-s-cli-tests-belong-to-the-root`, the two items that end the transitional exceptions.

## Constraints

- Existing projects keep loading and passing: both keys are optional, and `tests-cite-their-owner` judges only files a `tests` glob owns, so a project moves to them one glob at a time.
- The re-citing exception for a test in a file no `tests` glob owns stays until a project gives its tests to their components.
- The root's dispatch files (`crates/clm/src/main.rs`, `Cargo.toml`, `Cargo.lock`), which the root's Design lets every build that adds a command edit, are unchanged.

## Open questions

- A capability test changing with the behaviour it observes stays a permanent exception. Giving `capability` requirements of its own that each behaviour change names would remove it, but that is a larger redesign of the capability layer.
