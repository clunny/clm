---
title: "An extracted tree holds the files a checkout of the revision would write"
date: 2026-10-27
status: accepted
---

CLM reads a revision's files from a temporary directory in several places: `clm next`, `clm run`, `clm checkpoint` and `clm draft` through `clm-git`'s `extract_tree`, and `clm gate` and `clm sync` through pipelines of their own (`git archive <rev> specs | tar -x`, and `git checkout <rev> -- <paths>` against a temporary index). Those pipelines don't agree on what a file's bytes are. A checkout applies line-ending conversion, `ident` and filter drivers and ignores the `export-*` attributes; `git archive` is built for distribution tarballs, so it also drops `export-ignore` paths and expands `$Format:...$` in `export-subst` files. For `specs/` in this repository the results are the same, but a repository that marks `specs/` `export-ignore` would give the gate an empty base, and one with `export-subst` on a spec would hash it differently at the base than in the working tree.

Every hash CLM compares a tree against comes from a working tree: `clm hashes` in a session, a delta's `base`, the markers a build writes. So `clm-git` promises one thing for every extraction, whole tree (`extract_tree`) or some paths (`extract_paths`): each file holds the bytes `git checkout` of that revision writes in this repository, with the attributes read from the revision's own tree and the repository's configuration, and the `export-*` attributes ignored (`git/extract-as-checkout`). A hash read from an extracted tree then equals the hash read from a checkout of the same revision, whatever the repository's attributes.

Rejected:
- **`git archive` semantics.** It is what the gate extracts with today, but it ties CLM's hashes to attributes meant for release tarballs, which nobody setting them expects to change what CLM sees.
- **Raw blobs, no conversion at all** (`git cat-file` per file). It would make the extracted tree independent of the machine, but a working tree with `core.autocrlf` or an `eol` attribute would then hash differently from the same revision extracted, which is the drift the extraction is meant to avoid.
- **Leaving it unstated.** Each migration to the layer would pick its own behaviour by accident of the git command it used, which is how `gate` and `sync` came to disagree.
