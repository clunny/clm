A marker now counts only inside a test: see what stops counting before the next build
A `req:` marker now counts as proof only when it sits inside a test, so a marker in a comment, a doc string or other code outside a test no longer makes its requirement current. A requirement whose only markers sit outside a test shows as unbuilt again, and a build brings it up to date.

Run `clm coverage` before your next build: its `uncounted` lines list the markers that stopped counting, each with the file it sits in. Move each into the test it was meant to prove, or delete it.

Two further checks are off until the project sets them. `wiring`, a key in the frontmatter of the root spec, `specs/SPEC.md`, switches on the ownership rule: `clm land` refuses a build's branch, `impl/<component>`, that changes a file the rule doesn't count as its own, and no review lets it through. A project whose tests share files first gives each test file to its component with a `tests` glob in that component's frontmatter, then names `wiring`. Naming `wiring` also has `clm check` report, under `tests-cite-their-owner`, each `req:` marker in a file a component owns that cites a requirement of a component other than the file's owner. `[verify] mutate`, a setting under `[verify]` in `clm.toml`, switches on the mutation pass, which breaks code on purpose after the land and reports a requirement whose tests don't notice. Leave both out to keep today's behaviour.

Taking this note edits no file and leaves nothing to decline: `clm upgrade --apply` stamps `upgraded`, which marks it taken.
