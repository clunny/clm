---
status: done
resolves: [a-linked-worktree-of-a-clone-with-a-separate-git-directory-reads]
---

## Intent

Accept that a root in a linked worktree of a clone whose common git directory isn't named `.git` (one made with `git init --separate-git-dir`) reads no `clm.local.toml`, and delete `backlog/a-linked-worktree-of-a-clone-with-a-separate-git-directory-reads`. The change edits only the backlog.

The behaviour is already the one the spec states, so the code is not wrong: `config/local-settings` says that when the common git directory the worktree's files lead to isn't named `.git`, a root in that worktree has no local layer, and its scenario with a common git directory named `repo.git` proves it. The config Design says why: such a clone has no main worktree that the files git keeps can find. The item's open question was whether `clm` should find the main worktree some other way. It should not:

- Git keeps no link from a separate git directory back to the main worktree. `git init --separate-git-dir` writes a `gitdir:` file into the work tree and doesn't set `core.worktree`, so nothing under the common git directory names the work tree. Git's own `git worktree list` from a linked worktree of such a clone names the git directory itself as the main worktree. The *main worktree* the Contract defines (the first one `git worktree list --porcelain` lists) is then not a work tree, and holds no `clm.local.toml`.
- Any other way of finding it would be a guess, such as searching the disk for a `.git` file that names the git directory, or reading the file beside the linked worktree's root. Reading beside the root would give each linked worktree its own untracked settings, which breaks the rule that one `clm.local.toml` serves the whole clone. That rule is what the requirement keeps (`yagni`, `redesign-before-workaround`).
- Such clones are rare, and the keys stay reachable from them: the user's `config.toml` and the `CLM_*` variables still apply in every worktree, and the main worktree reads its `clm.local.toml` as usual.

No change or code fix since 2026-11-12 touches this. The item was filed by `20261119-wave-1-by-the-rule`, which wrote the requirement it describes.
