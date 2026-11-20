---
title: "The fingerprint covers only the CLM root's subdirectory, and the web review's covers the whole repository"
kind: bug
components: [gate.fingerprint, web.worker.review]
origin: 20261119-wave-1-by-the-rule
---

`clm fingerprint` lists the two trees with `git ls-tree -r -z` run in the CLM root, without `--full-tree`, so in a CLM repository that is a subdirectory of its git repository the listings hold only that subdirectory, with paths relative to it. The web review computes the fingerprint from the host's listing of the whole repository. The two agree only when the CLM root is the repository's top, so an approval given in the web app for a repository in a subdirectory would not match what `clm land` computes, and a change outside the subdirectory moves one fingerprint and not the other.

Expected: one definition both sides hold to, stated by `gate.fingerprint/fingerprint` and `web.worker.review/fingerprint-from-trees`. Wave 1 of phase 5 states what the command does today; which side changes needs a design.
