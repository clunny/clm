---
title: "StrykerJS reports Survived, not NoCoverage, when no test loads a mutated file"
kind: workaround
components: [ci]
requirement: ci/mutate-script@5f0ee0b6
---

The requirement says that running each vitest test whether or not it imports a mutated file (`vitest.related: false`) lets a block whose tests import nothing reach StrykerJS's `NoCoverage` verdict, and the new fixture run checks exactly that. A real run of StrykerJS 10.1.1 with its vitest runner says otherwise: with `related: false` alone, the fixture's test file that imports nothing gets every mutant `Survived`. Only instrumented code creates the runner's coverage object (`__stryker__.mutantCoverage`), so a run in which no test loads a mutated file reports no coverage at all, `TestCoverage.hasCoverage` is false, and StrykerJS runs every test against every mutant.

To satisfy the requirement, `run_stryker` now writes a vitest setup file into the configuration it gives StrykerJS. In the initial run only, that file creates the empty coverage object under the namespace the runner injects, so the coverage is reported, empty, and a mutant no test runs is `NoCoverage`. This relies on StrykerJS internals no requirement mentions: the namespace, the `mode` and `globalNamespace` values the runner provides, and the shape of the coverage object. A StrykerJS upgrade that changes them would fail the new fixture run, which is what the fixture is for.
