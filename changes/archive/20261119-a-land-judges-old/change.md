---
status: done
resolves: [a-land-judges-old-test-names-from-the-touched-files-alone]
---

## Intent

Deletes `backlog/a-land-judges-old-test-names-from-the-touched-files-alone`, which `20261119-the-land-and-clm` already fixed: `land/deleted-tests-approved` now has the land run `deleted_tests` over every file of both trees under the CLM root, as `clm coverage --deleted-tests` does, so a name an untouched file's test carries at the base counts as old. Its build landed in `2e2f8f91` ("Read every file of both trees for land/deleted-tests-approved") and that change is `done`. Nothing is left to design or build.
