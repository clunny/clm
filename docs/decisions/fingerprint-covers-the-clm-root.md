---
title: "The fingerprint covers the CLM root's tree, with paths relative to it"
date: 2026-11-19
status: accepted
---

A CLM repository may be a subdirectory of its git repository (`root-from-cwd`). `clm fingerprint` lists the two trees with `git ls-tree -r -z` run in the CLM root, so its listings hold that subdirectory alone, with paths relative to it, while the web Worker hashes the Trees API's listings of the whole repository. The two agree only when the CLM root is the repository's top, so one definition had to be chosen.

The fingerprint is the CLM root's: the entries under it, each path relative to it, as `clm fingerprint` already computes. Every rule the fingerprint applies reads a path relative to the CLM root: it skips `specs/` on a branch that adds a change, and `added_changes` finds `changes/<id>/change.md`, which `land/derived-specs` and `rename/replay` read from listings of the root too. An approval is of the intent the CLM repository states, and every file that states it is under the root.

The Worker is unchanged. It lists only repositories whose default branch holds `specs/SPEC.md` at its top, whose CLM root is the top, where the whole repository's listing is the CLM root's, so both sides compute one value. In a repository whose CLM root is a subdirectory, the Worker can't learn the root from the Trees API alone, and an approval it writes there, for a branch that touches a spec path at the repository's top, names a fingerprint `clm land` never computes: it counts for nothing, so the land fails closed and `clm approve` still works.

What it gives up: a commit that changes only files outside the CLM root leaves the fingerprint, and so an approval, as it was. So for a subdirectory root, "the diff" of `approve-pins-the-review` and "a file the branch adds, modifies or deletes" of `tree-diff-fingerprint` mean the diff and the files under the CLM root; at a repository's top, the only layout the Worker lists, nothing changes. `approval-policy` stays whole only while no approved branch can carry a file outside the root, which is the gate's to refuse: a spec branch that also changes one should be `mixed`. The gate reads the paths a branch changes relative to the CLM root and counts a file outside it as code (`gate/paths-under-the-root`), so a spec branch that also changes one is `mixed` and blocked; until that requirement is built, it reads them relative to the repository's top and an approval in a subdirectory root doesn't cover files outside the root.

Alternatives considered:
- **Hash the whole repository** (`git ls-tree --full-tree`), with the `specs/` and `changes/` rules read through the root's prefix. Every approval would then cover files outside the root, but `fingerprint_of`, `added_changes` and each caller that builds listings (`land`, `rename`, the Worker) would have to carry a prefix, and the Worker, which doesn't know the root, would still disagree for a subdirectory root.
- **Teach the Worker the CLM root**, by finding `specs/SPEC.md` in the listing, and filter and strip its listings. It serves no repository the Worker lists today.
