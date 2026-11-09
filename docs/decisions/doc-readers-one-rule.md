---
title: "One repo-wide rule marks the readers of doc-only paths, proven on the root"
date: 2026-11-09
status: accepted
---

`doc-readers-marked` was three requirements: `clm/doc-readers-marked` for `crates/clm/tests/`, `init/doc-readers-marked` for `crates/clm-init/`, and `web.site/doc-readers-marked` for the site's tests. Each states its scenarios as what `clm-affected`'s `readers` selects from this repository's markers, and `#### Needs check.affected/affected-readers`. A test in `clm-init` can't call `readers`: `check.affected` depends on `check`, which depends on `init`, so a dev-dependency of `clm-init` on `clm-affected` is an import `depends_on` can't declare without a cycle (`check/undeclared-imports`). `init`'s test checked its markers with its own glob matching instead, a copy of what `readers` does, and its scenarios went unproven through the real code (`backlog/init-readers-scenarios-need-check-affected`).

Now `clm/doc-readers-marked` covers every crate's tests under `crates/`, test files under `tests/` and unit-test modules under `src/` alike, and its scenarios name readers in other crates (`clm-init`'s guide tests, `clm-model`'s and `clm-context`'s readers of `ARCHITECTURE.md`, `clm-config`'s settings-page test). They are proven by `crates/clm/tests/doc_readers.rs`, which runs the real `readers` on this repository's markers: the root depends on every component. `init/doc-readers-marked` and its copied test are removed. `web.site/doc-readers-marked` stays: the site's tests are not a crate's, and its scenarios are proven where they are.

**Why.** The human chose it in triage on 2026-11-09: "One repo-wide rule on clm (Recommended): widen clm/doc-readers-marked to every crate's tests, proven through the real readers; remove init/doc-readers-marked and its copied test." Giving `init` a dependency on `check.affected` would make a cycle, and a rule per crate would need each crate's own copy of the matching, or none of them proven through `readers` (`reuse-first`). The marker is a convention of this repository's tests, not of `init`'s behaviour, so the root, which owns the repository's test layout and depends on everything, is where it belongs.
