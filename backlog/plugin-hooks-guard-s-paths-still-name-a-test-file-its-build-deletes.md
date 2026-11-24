---
title: "plugin.hooks.guard's paths still name crates/clm-guard/tests/git_layer.rs, which its build deletes"
kind: deferred
components: [plugin.hooks.guard]
origin: 20261123-wave-5-hooks-by-the-rule
---

Wave 5 (`20261123-wave-5-hooks-by-the-rule`) removes `plugin.hooks.guard/git-through-layer`, and `crates/clm-guard/tests/git_layer.rs::runs_git_and_gh_only_through_the_layer` with it (`deleted-tests.txt`), so the guard's build deletes the file. The component's frontmatter `paths` keeps `crates/clm-guard/tests/git_layer.rs` until then: dropping it on the spec branch leaves the file in no component's `paths` while it still exists, which `clm check` refuses (`check/unowned-files`).

Once the guard's build has landed and the file is gone, a spec change drops the entry from `plugin.hooks.guard`'s `paths`. `clm check` doesn't flag a path that matches no file, so nothing else will remind anyone. No requirement changes, so it needs no yes from the human.
