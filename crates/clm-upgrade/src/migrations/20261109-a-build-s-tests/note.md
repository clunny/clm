Builds run your tests in a fresh worktree: declare [build] setup
Each build runs in a fresh worktree inside the main checkout, with nothing installed. A toolchain whose lookup walks up the parent directories, or an environment the checkouts share, can then resolve the main checkout's packages or source instead of the branch's, and a build's red or green test means nothing (`tests-run-in-their-own-tree`, which `clm guide constraints` states).

Declare in `clm.toml`, under `[build]`, a `setup` command: a shell command that makes a fresh worktree ready to run its tests, such as one installing its dependencies into it. `clm setup` runs it, which builds and drafts do before their first test run. `[land] check` stays a command that needs no setup before it, since CI and `[verify]` run it in trees no build prepared.

To check your project, create a worktree under the main checkout, run its tests there without installing anything, and see whether they fail for want of their dependencies or resolve them from outside the worktree. A project whose tests use npm packages can set `setup = "test ! -f package.json || npm ci"`.

Taking this note edits no file and leaves nothing to decline: `clm upgrade --apply` stamps `upgraded`, which marks it taken.
