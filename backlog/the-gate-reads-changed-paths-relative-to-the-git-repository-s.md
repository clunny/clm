---
title: "The gate reads changed paths relative to the git repository's top, not the CLM root"
kind: bug
components: [gate]
origin: 20261119-the-fingerprint-covers-only
---

`clm-gate`'s `changed_files` runs `git diff --name-status --no-renames -z <range>` in the CLM root, and `git diff` prints paths relative to the git repository's top whatever directory it runs in. In a CLM repository that is a subdirectory `tool/` of its git repository, a branch editing `tool/specs/a/SPEC.md` lists `tool/specs/a/SPEC.md`, which `is_spec_path` doesn't count, so the gate classes a spec branch as `code` and never asks for review. (Checked by hand: in a repo with `tool/a` and `b` changed, `git diff --name-status HEAD~1..HEAD` run in `tool/` prints `b` and `tool/a`.) The `diff --name-only ... -- :(glob)<paths>` call that looks for code files has the same shape.

Expected: the gate reads the paths under the CLM root, relative to it, as `gate.fingerprint/fingerprint` does (`fingerprint-covers-the-clm-root`), and counts a path outside the root as code, so a spec branch that also changes one is `mixed` and blocked. That is what keeps `approval-policy` whole for a subdirectory root, since the fingerprint an approval is pinned to covers only the root's files (`fingerprint-covers-the-clm-root`). Which requirement states this needs a design.
