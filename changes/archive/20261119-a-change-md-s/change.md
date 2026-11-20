---
status: done
resolves: [a-change-md-s-gate-key-is-rejected-by-loading-and-accepted-by]
---

## Intent

Accept that loading and the one-record parser of `change.md` treat the key `gate` differently, and delete `backlog/a-change-md-s-gate-key-is-rejected-by-loading-and-accepted-by`. No spec or code changes: the branch only deletes the item.

## Why the difference is accepted

`model/parse-changes` already states both sides exactly, as wave 1 left it: loading rejects `gate` on a live change, reports it and keeps the change loaded, and accepts and ignores it on an archived one; `parse_change_frontmatter` accepts `gate` wherever the change lies. The item asked for one rule or the reason they differ. The reason is the input each reader has:

- **The one-record parser gets text alone, so it can't tell a live change from an archived one.** Its callers read archived `change.md` files too: the gate reads the `change.md` of each change a branch archives (`added_change` in `crates/clm-gate/src/lib.rs`), and an archived change keeps the keys it was written with (`archive-immutable`, `renamed-words-still-load`), `gate` among them in projects that recorded it before `yes-no-review`. A parser that refused `gate` would give nothing for those files, and the gate would lose their `status` and `resolves`. Taking the location as an argument would make every caller pass what most of them don't know.
- **The key carries no value any reader acts on.** Both readers sink it (`_gate: Ignored` in `crates/clm-model/src/change.rs`), so accepting it in the one-record parser changes no answer its callers give.
- **The refusal already gates every land.** A live change that sets `gate` is a loading violation, which `clm check` reports and `clm land` runs on the merged tree. A branch carrying it never reaches `main`, whatever a one-record caller saw first.

So the two readers don't disagree on any tree that can land, and one rule would cost either the archive's old keys or a location argument for no change in behaviour. The plan in force (`backlog/a-passing-check-proves-less-than-the-spec-claims`) keeps the freeze, under which a defect's default answer is to accept it; this one has nothing to fix besides.

## Approaches

- **Taken: accept.** The requirement already states where the readers differ; the reasons above are why.
- **Not taken: reject `gate` in `parse_change_frontmatter` on every change.** It would give nothing for an archived `change.md` that kept the key, which the gate reads when a branch archives a change.
- **Not taken: give the parser the change's location.** Every caller would pass it, for a key no one reads and a refusal `clm check` already makes.
- **Not taken: write the reason into `model`'s Design.** The Design is `TBD.`, and phase 5's waves rewrite `model`; the freeze holds such an edit back, and this change records the reason.
