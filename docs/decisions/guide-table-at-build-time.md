---
title: "clm guide's page table is generated when clm-init builds, from docs/guide/, so a new page is one spec change"
date: 2026-10-28
status: accepted
---

`guide-in-the-binary` compiles every page of `docs/guide/` into `clm` with `include_str!`, and `guide-from-docs` makes the guide a spec path. Written by hand, one `include_str!` per page, the table was code that had to name each page: a spec branch adding a page failed `clm land`'s tests (the test that every page is compiled in fails on the new file), a code branch adding the table entry first didn't compile, since the file wasn't there, and a branch doing both was `mixed`, which the gate blocks. `20261027-guide-links-outside-the` had to leave the spec format at `docs/spec-format.md` because of it, and `README.md`'s numbered list was pinned the same way by tests.

The table is now generated: `crates/clm-init/build.rs` lists `docs/guide/*.md`, orders them as `README.md` links them (unlinked pages after, by name; `spec-format` last), and writes a `const PAGES` of `include_str!` lines that `guide.rs` includes. The binary still holds every page's bytes and reads nothing at run time; a page added on a spec branch, with its `README.md` line, is in the next build with no edit under `crates/`, and its land runs `clm-init`'s guide tests through their `clm:reads docs/guide/**` marker (`guide-pages-are-doc-only`). The build's own order is the only one: no test lists the pages by hand.

Rejected:
- **Let the gate rate `docs/guide/` as code.** Then adding a page with its table entry is a code branch, and the guide, what every adopter reads, stops being a spec path, which `guide-from-docs` makes it. Which tests a guide edit runs is a separate question, answered by the tests' `clm:reads` markers; it says nothing about what kind of branch carries the edit.
- **An exemption from `mixed` for a branch whose code part is the page table alone.** A special case in the gate for one file, which every later table-like file would want too.
- **A crate that embeds a directory (`include_dir` or the like).** A dependency for a dozen lines of build script, and no say over the order.
