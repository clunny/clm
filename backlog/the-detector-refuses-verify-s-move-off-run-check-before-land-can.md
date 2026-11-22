---
title: "The detector refuses verify's move off run_check before land can remove it"
kind: bug
components: [verify, land, config]
blocks: [verify/verify-run]
---

`20261122-the-check-runner-s` sequences the runner's rename by `expand-contract`: `land` offers `run_check_as` beside `run_check` (marked `clm:remove-by 20261122-the-check-runner-s`), `verify` moves off `run_check` (`verify/verify-run`), and `land/runner-names-its-caller` then removes it. But `verify` is `run_check`'s last user, so the build that moves it leaves `clm_land::run_check` with no user, and `clm check`'s `unreferenced-public-items` detector (`clm unreferenced`) fails the land (`crates/clm/tests/cli.rs` `this_repository_is_clean` and `unreferenced_is_clean_with_the_allow_list`): "clm_land::run_check is public but nothing outside its library uses it". A build can't add an `--allow` entry (`gate/own-files` admits only dropping one), and can't remove `run_check` from `clm-land`. So neither step can land alone.

Found by verify's wave-3 build, which left `verify/verify-run` pending with `run_check` still called. Possible fixes for a spec change: the detector skips an item a `clm:remove-by` marker holds (the old path of an expand-contract is unused by design once its users move); or the change adds the `--allow` entry with the item that ends it; or one change moves verify and removes `run_check` together.
