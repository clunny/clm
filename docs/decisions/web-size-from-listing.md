---
title: "The web Worker counts node sizes from its tree listing, with check's counting"
date: 2026-10-27
status: accepted
---

`oversized-is-a-state` makes an over-budget node visible in `clm size`, `clm status` and the `spec` view. The `web` graph draws the same `spec` model, but the Worker has no working tree, so it passed no size report and showed no state rather than a wrong one. To show the state, the Worker has to reach the answer `clm size` gives on a checkout of the same commit.

What `clm size` reads, and where the Worker gets it:
- **File lengths**: the Trees API listing (`?recursive=1`) the snapshot already makes gives each blob's `size`.
- **Markers**: the marker index already holds every non-Markdown file's markers, as `clm_check::scan` finds them.
- **Binary files** (a NUL byte in the first 8000 bytes): the index already fetches every non-Markdown blob's text to scan it, so it records the binary ones as it goes.
- **`linguist-generated` and `linguist-vendored`**: the listing names every `.gitattributes`; the snapshot fetches their text in the query it already makes, and `clm_check::linguist_excluded` matches them without git.
- **Budget defaults**: the commit's `clm.toml`, read with `clm_config::node_size_from`, since `[node_size]` comes only from `clm.toml` or a clone's `clm.local.toml` (`config/repository-only`), and a hosted commit has no local file.

The counting rule lives once, in `clm_check::size_of`, which `size` calls after probing the working tree; the Worker calls it on the listing. So a change to what counts reaches both, as far as a listing can carry it (see below).

Rejected:
- **The Worker re-implements the count.** It would drift from `clm size` the first time a counting rule changes, and the Worker's Contract is that it decides nothing `clm` doesn't.
- **Fetch every counted file's text to probe for NULs.** The index already holds that fact for every non-Markdown file; fetching again costs subrequests the Free plan doesn't have (`web-on-free-plan`).
- **Commit a size report** (a file `clm sync` writes for the Worker to read). It would be a second copy of a fact git already holds, out of date between syncs, and a commit no change asked for.
- **Leave linguist attributes out on the Worker.** A lockfile under a node's `paths` would push it over its budget on the web and not in `clm size`: a wrong state, which is worse than none.

What can still differ, each accepted because closing it would cost fetches the Free plan doesn't have or settings the host never sees:
- `git check-attr` also reads `.git/info/attributes` and `core.attributesFile`, which a clone may set and a hosted listing never has. Those are a clone's own settings, as `clm.local.toml` is, so the Worker answers for a clone without them.
- The listing gives a blob's stored size; a checkout's file differs under eol conversion (`text eol=crlf`, `core.autocrlf`) and for Git LFS pointers.
- Inline test parts: `clm size` counts a `.rs` file's `#[cfg(test)]` part with the node its markers cite (`inline-tests-count-where-cited`), from the file's text; the listing has sizes but no contents, so the Worker counts every file as one part (`Length::from`), and a node whose Rust files hold inline tests citing another node's rules can differ. Fetching every `.rs` blob for it would cost subrequests.
- The binary rule: the Worker records a blob binary when GitHub's `isBinary` says so or its text has a NUL in its first 8000 bytes. GitHub's detection also uses encodings, so a UTF-16 file can come back as text, and an unusual encoding can be called binary, where git decides by NULs alone.

Symlinks and submodules aren't among these: the listing gives each entry's mode, and only regular files count, as in `size`. Nor is `clm.toml`: `clm_config::node_size_from` fails wherever loading the file would, so the Worker shows no sizes where `clm size` would exit 1.
