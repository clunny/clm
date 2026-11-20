---
component: upgrade
base: { frontmatter: "09581b3d94858a16" }
---

## MODIFIED Frontmatter
`upgrade`'s unit tests make a git repository and a linked worktree, and `vcs/one-git-layer` now has every crate start `git` only through `clm-git`, so the component may import it.

```yaml
depends_on: [config, vcs]
```
