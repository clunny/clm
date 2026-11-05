---
status: done
resolves: [marker-text-in-tests-has-no-escape]
---

# Marker text in tests: documented, not escaped

## Intent

Accept that the marker scanner reads every `req:<component>/<slug>` it finds outside Markdown, and tell adopters how to write such text without citing: `docs/spec-format.md`'s test references and the guide's `how-it-works.md` now say that a file which must hold a requirement ID after `req:` without citing it builds the string so the literal never appears whole (`concat!("req", ":a/x")`, `"req" + ":a/x"`), and why CLM offers no escape. The change deletes `backlog/marker-text-in-tests-has-no-escape` and changes no spec.

## Problem

A test that searches other files for citations, or a checker's fixture, holds the marker text as a plain string. The scanner reads it as a citation: one without a version keeps its requirement `behind` (`stale-markers-keep-behind`), and one naming no requirement fails `check/dangling-markers`. The way round it, splitting the string, was stated only in `check`'s Design and this repository's `CLAUDE.md`, which adopters don't read.

## Approaches

- **An escape or ignore comment the scanner honours** (a pragma on the line, or a prefix such as `\req:`). Rejected. It gives any file a way to hide a real citation from the check, so `proven-by-tests` and `stale-markers-keep-behind` would rest on nobody writing the pragma beside a marker; it adds a second syntax every language's tests and the Worker's `markers_in` must agree on; and no capability needs it, since every language the scanner reads can already split a string literal (`yagni`, `reuse-first`). A test written this way is a little less direct, and that cost is confined to the rare test about markers themselves.
- **Skip unversioned forms in files that also hold versioned markers.** Ruled out by the item: `stale-markers-keep-behind` keeps a bare marker `behind` even beside a current one.
- **Document the existing rule** (chosen). The behaviour is right and already stated in `check`'s Design (`check/SPEC.md`, the paragraph on markers); what was missing is the adopter-facing text. Per `redesign-before-workaround`, the design isn't wrong, so the behaviour is accepted in writing.

## Outcome

- `docs/spec-format.md`, Test references: a rule that every match counts wherever it sits, how to split the string in Rust, Python and TypeScript, and that CLM has no escape.
- `docs/guide/how-it-works.md`, Proof: one paragraph saying the same and pointing to the spec format.
- No requirement, scenario or design changes; nothing to build.

## Constraints

- `proven-by-tests`, `stale-markers-keep-behind`: no text may hide a citation from the scan.
- `method-in-the-plugin`: the rule reaches adopters through the guide that `clm guide` carries.

## Open questions

None.
