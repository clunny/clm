---
status: done
resolves: []
---

## Intent

Corrects `backlog/unjudged-model-88470edf.md`'s `components` from `model` to `ci`, since what kept the mutation pass from a verdict on `model/move-records` and `model/parse-changes` is `.github/scripts/mutate.sh` ending the pass when StrykerJS judged no test for five `web` blocks, not anything in `model`.
