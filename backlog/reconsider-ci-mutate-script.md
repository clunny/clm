---
title: "Reconsider ci/mutate-script"
kind: question
components: [ci]
---

`ci/mutate-script` scores 16 against the reconsider threshold of 3 (workarounds 1, rewritten 13, scenarios 1, exceptions 1).

Workaround flags:
- backlog/workaround-ci-mutate-script-1.md: StrykerJS reports Survived, not NoCoverage, when no test loads a mutated file

Archived changes that modified it:
- 20261114-proof-switched-on
- 20261116-a-mutation-pass-that-stops
- 20261116-the-mutants-copy-is-a-git-repository
- 20261116-the-mutants-copy-shares
- 20261116-the-mutation-pass-judges
- 20261117-the-mutation-pass-files
- 20261118-the-mutation-pass-s
- 20261119-the-mutation-pass-can
- 20261119-the-mutation-pass-judges
- 20261121-wave-3-verify-by-the-rule
- 20261123-the-mutation-pass-can
- 20261124-mutate-script-s-strykerjs
- 20261124-the-mutation-pass-s

Scenarios: 48 (setting 6). Exception clauses: 10 (setting 4).

Three ways out, each a spec change: drop the requirement, change it, or redesign around it. Deleting this item without one brings it back on the component's next build.
