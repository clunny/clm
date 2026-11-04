---
title: "clm guide's page table is generated when clm-init builds, from docs/guide/, so a new page is one spec change"
date: 2026-10-28
status: accepted
---

`guide-in-the-binary` compiles every page of `docs/guide/` into `clm` with `include_str!`, and `guide-from-docs` makes the guide a spec path, reviewed as the specs it explains (`HARD` by `gate/path-tiers`). Written by hand, one `include_str!` per page, the table was code that had to name each page: a spec branch adding a page failed `clm land`'s tests (`docs/guide/` counts as code for the ledger, and the test that every page is compiled in fails on the new file), a code branch adding the table entry first didn't compile, since the file wasn't there, and a branch doing both was `mixed`, which the gate blocks. `20261027-guide-links-outside-the` had to leave the spec format at `docs/spec-format.md` because of it, and `README.md`'s numbered list was pinned the same way by `clm-init`'s and `web`'s tests.

The table is now generated: `crates/clm-init/build.rs` lists `docs/guide/*.md`, orders them as `README.md` links them (unlinked pages after, by name; `spec-format` last, the order `web`'s build already gives the same files), and writes a `const PAGES` of `include_str!` lines that `guide.rs` includes. The binary still holds every page's bytes and reads nothing at run time; a page added on a spec branch, with its `README.md` line, is in the next build with no edit under `crates/`, and `clm land` runs the code checks on that branch because the ledger counts `docs/guide/` as code. `web`'s tests derive the page order they expect from `README.md` rather than listing it (`web/guide-list-from-files`), the one place a written list remained.

Rejected:
- **Let the gate rate `docs/guide/` as code, as the ledger does.** Then adding a page with its table entry is a code branch, and the guide, what every adopter reads, lands with no review at all: `guide-from-docs` wants it reviewed like the specs, and `humans-review-intent` makes that review the human's. The ledger counts the directory as code only so that a guide edit runs the tests that read it; that says nothing about who reviews it.
- **An exemption from `mixed` for a branch whose code part is the page table alone.** A special case in the gate for one file, which every later table-like file would want too.
- **A crate that embeds a directory (`include_dir` or the like).** A dependency for a dozen lines of build script, and no say over the order.

**Amended 2026-10-29** (`20261029-refounding`). `web` no longer bundles the guide, so `web/guide-list-from-files` went with its Guide view; the build's own order is the only one left.
