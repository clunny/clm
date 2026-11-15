---
title: "clm land's clean-up runs git from a directory it already removed"
kind: bug
components: [land]
---

Step: an unattended archive run (`clm build`'s archive prompt) running `clm land` from its own worktree.

What happened: `clm land` landed the branch and printed `landed <sha> code`, but its two clean-up steps then failed with warnings. Deleting the landed branch on origin failed with `git remote: No such file or directory (os error 2)`, and clearing the build's `target/` failed with `git rev-parse --show-toplevel: No such file or directory`. The landed `impl/archive/<id>` branch stayed on origin and `target/` was kept. Both errors suggest that the steps ran git with a working directory that no longer existed, perhaps because the worktree was removed earlier in the same land. An earlier build run this session also reported that its worktree was gone after its land.

Expected: `clm land` orders its clean-up so that it deletes the remote branch and clears `target/` before removing the worktree it runs in, or runs those steps from a directory that still exists, such as the common git directory. A successful land should leave no landed branch on origin.
