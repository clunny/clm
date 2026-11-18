---
title: "The mutation pass's StrykerJS sandbox can't see repository files a web test reads"
kind: bug
components: [ci]
---

During the build loop's `run verify` (`clm verify --due`), the mutation pass (`mutate.sh`) ran StrykerJS for a `web` requirement whose citing test reads a file from the repository root, outside the `web` package. StrykerJS copies the package into its own sandbox (`.stryker-tmp-*`) and runs the tests there, so the file wasn't found. The initial test run failed with ENOENT, StrykerJS refused to mutate, and `mutate.sh` exited 1 for that requirement. `clm verify` still exited 0 and printed `4 left`.

Expected: a test that passes in the land check also passes in the mutation pass's sandbox, so its requirement can be judged. Instead, a requirement whose test reads a repository file is never judged and stays due on every verify (`verify/mutation-pass`, `ci/mutate-script`). The fix is likely to make such files reachable from the sandbox: StrykerJS's working directory or files setting, or a path the test resolves from the repository root rather than from its own working directory.
