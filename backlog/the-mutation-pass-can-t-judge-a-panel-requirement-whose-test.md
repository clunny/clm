---
title: "The mutation pass can't judge a panel requirement whose test file uses vi.mock"
kind: bug
components: [plugin.panel]
---

Step: `clm verify --due`, run by the build loop after lands; its mutation pass (`.github/scripts/mutate.sh`) runs StrykerJS with the vitest runner on `plugin-panel`.

What happened: for every `plugin.panel` requirement whose citing tests include a vitest file that calls `vi.mock` at its top level, StrykerJS's initial test run fails ("1 call ... was defined outside of the module's top level scope: vi.mock()"), because the file as Stryker's sandbox serves it no longer has the call at the top level. mutate.sh prints `StrykerJS failed for requirement plugin.panel/<slug>` and those requirements are left unjudged. Plain `vitest run` passes the same file, so the land's checks never see it. It happened in three verifies in a row (up to five requirements each), and `clm verify` still printed `verify passed`, the mutation pass's failure showing only as a note after that line.

Expected: either the panel's tests mock modules in a form StrykerJS's sandbox keeps valid (for instance `vi.doMock` before each dynamic import, which these tests already use with `vi.resetModules()`), or the mutation pass judges such files; and a requirement left unjudged because the tool failed is visible in the loop's record rather than only in the output after the outcome line.

Rules involved: `mutation-after-land`.
