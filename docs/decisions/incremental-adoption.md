---
title: "An existing codebase can be adopted a part at a time; the rest is declared outside every node"
date: 2026-10-27
status: accepted
---

`brownfield-survey` specs a whole existing codebase in its first change. On a large codebase that is a large first review, and the change finishes only when every node's tests cite its requirements. A team can instead adopt one service or module first and widen the graph in later changes.

**What isn't adopted is declared, in the spec.** The root node's frontmatter holds `outside`, a list of globs written as `paths` are: the code no node owns (`model/outside-globs`). It is the code CLM makes no claims about, adopted later or never, such as vendored code. Adopted code that doesn't meet a rule yet is not `outside`: its component holds an exemption naming the rule and the backlog item that ends it (`temporary-exemptions`). It lives in `specs/SPEC.md`, so narrowing it, adding to it or removing it is a spec change a human reviews, never a code or settings edit. Only the root holds it, so there is one list to read.

**Nothing is unowned by accident.** `clm check` gains its path-ownership rule (`check/unowned-files`): every tracked code file, meaning every file the index holds outside the spec paths `gate` names (`specs/`, `changes/`, `backlog/`, `docs/`, `ARCHITECTURE.md`, `CLAUDE.md`), must be in some node's `paths` or matched by an `outside` glob. Before this, `check`'s Contract listed "path ownership" but no rule checked it, so a file outside every node was simply unprotected, adopted or not. The rule runs only in a repository whose root sets `outside`, even to `[]`: setting the key is how a repository opts in. `clm init` always sets it; this repository opts in once the model knows the key (`backlog/opt-in-to-path-ownership`). A repository that predates the key, and every test fixture across the workspace that commits code under a bare root, keeps passing, so turning the rule on breaks nobody's `main` and no build has to edit another node's fixtures (`expand-contract`). A glob that matches no file outside every node is reported too, so the set shrinks as the graph widens instead of lingering as a blanket exemption. A file a node's `paths` and an `outside` glob both match is the node's: `outside: ["src/**"]` with a node on `src/api/**` reads as "all of `src/` but the API", which is how adoption proceeds. Only tracked files count, so a scratch file in a checkout never fails the check; in a tree that isn't a git work tree, such as the one `clm checkpoint` extracts from a commit, every file under it is read, which is the same set.

**`clm init` declares the existing code outside.** The root it writes owns the files `clm init` writes (`.gitignore`, `clm.toml`, `scripts/clm-check.sh`) and lists every other top-level entry holding code in `outside` (`[]` when there is none), so the bootstrap commit passes `clm check` before any node exists (`init/init-root-ownership`). The first spec graph then replaces that list: with `[]` when it adopts everything, as before, or with the globs of the part left for later.

**`clm survey --nodes` accepts the same declaration**, as the map's `outside`, and checks coverage over the same code files `clm check` does (`init/survey-nodes`), so a map it accepts passes `check/unowned-files` once written. An import edge into or out of code no node covers isn't reported: there is no node to declare it on.

**The trade-off**, which `/clm:init` tells the human before they choose (`plugin/init-skill`): code outside every node has no requirements, so no test proves it to CLM (`proven-by-tests`) and nothing reports its drift (`no-silent-drift`) until a later change gives it a node. It still lands only through `clm land`, whose check script runs its existing tests. Builds of the adopted nodes see only their own nodes' context, so a node that leans on outside code gets no Contract for it. Widening the graph is an ordinary `propose`: add a node for the next part and take its globs out of `outside`.

Rejected:
- The "not yet" set in `clm.toml` or `clm.local.toml`. They are code paths, edited on code branches with no human review, so a build could exempt the code it should own.
- An `outside` key on any node. Several lists would have to be merged to answer one question, and a nested node's list would read as that node's business.
- Treating code outside every node as the root's. The root would then "own" a codebase it has no requirements for, and `clm size` would count all of it against the root's budget.
- Reporting a file that both a node and `outside` match. It forces `outside` to list every sibling of an adopted directory, which grows with the codebase rather than shrinking with adoption.
- The rule on for every repository, with no opt-in. Every fixture in the workspace that commits `src/` files under a root without `paths` (`sync`'s, the root's command tests, `orchestrator`'s) would fail `clm_check::run`, and `check`'s build would have to edit fixtures other nodes own.
- Checking untracked files as well. A human's scratch notes in their checkout would fail `clm check`.

Whether people may keep editing code outside every node by hand during adoption, which `ai-writes-code` doesn't allow, is left open: changing a Layer 1 invariant is the human's call.
