---
component: replica
base: { design: "a7e21229229621d6" }
---

## MODIFIED Design

The state is plain data: maps from path to text and blob SHA, the marker index, ledger sources by branch, refs, pull requests and at most 100 events. It is serialised with `serde_json` and split into chunks by `save`. The spec files are those `web.worker.platform`'s `files::is_spec_path` and `is_size_path` select. The change archive is kept as `changes/archive/<id>/change.md` only, since nothing but `resolved` reads it.

**Recompute.** It runs the code the Worker ran on request before this component:
- the snapshot: `clm_model::load_files`, then `clm_check::report` with the index's markers, then `clm_view`'s `spec` model with sizes from `clm_check::size_of` over the listing;
- the implementation's shared part: `flight_view`, `blockers_view`, `taken_from_refs`, the activity feed, the pages with their `checked` and the open proposals;
- the capabilities report: `clm_effort::report` with the flags' text, no history;
- the architecture report: `clm_effort::architecture` with the current requirements from the index once it is complete.

The code moves here from `clm-web` (`no-dead-code`), keeping its tests, which this component's build re-points at the new requirement IDs. Computation is whole-state: nothing is recomputed per component until a repository shows it needs that (`yagni`).

**Marker index.** A `Tip` change rescans only the files it carries, with `clm_check::markers_in`. A file is binary when the source says so, or when its text has a NUL byte in its first 8000 bytes, as `clm size` decides. While `pending_fetch` holds paths, the snapshot's coverage is `Indexing { done }` and its `sizes` is `"indexing"`.

**Writes.** `validate` answers what the Worker's routes checked before: a decision's entry, its version against the entry's version now, and whether the tip is signed; a proposal's slug against the backlog, `resolved` and the open proposals. Since the replica holds the ledgers and the tip, the check needs no request. The runtime first brings the replica up to the tip, so "now" means the tip, as `decide` requires.
