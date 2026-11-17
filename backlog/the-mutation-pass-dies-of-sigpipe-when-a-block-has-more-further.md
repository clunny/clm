---
title: "The mutation pass dies of SIGPIPE when a block has more further mutants than its limit"
kind: bug
components: [ci]
---

What happened: a build loop's `clm verify --due` passed its check and ran the mutation pass, which ended with `mutation: .github/scripts/mutate.sh failed with status 141; nothing recorded`. Status 141 is SIGPIPE. The script runs under `set -euo pipefail` and cuts a block's list of further mutants to the plan's `limit` by piping it into `head -n <limit>`; when the list is longer than the limit, `head` exits early, the writer gets SIGPIPE, `pipefail` makes the pipeline fail, and `set -e` ends the whole script. So every pass in which some block has more than `limit` further mutants its tests run fails and records nothing, and its requirements stay due.

Expected: cutting a list to the limit never fails the script (`ci/mutate-script` says it tries "up to the block's `limit` further mutants"), so the pass records its verdicts.

Found by: the first pass after the change that made the script list mutants and filter them by the lines a block's tests run.
