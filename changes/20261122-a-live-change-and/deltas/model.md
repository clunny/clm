---
component: model
---

## ADDED Requirements

### Requirement: unique-change-ids
A change's ID is its directory's name, so a live change `changes/<id>/` and an archived one `changes/archive/<id>/` that both load share an ID. The model SHALL report each such pair at the live change's `change.md`, naming the archived one's `change.md`, and SHALL keep the live change in the graph and leave the archived one out of it, so the change still to be built or archived is the one every command reads. A change that isn't loaded (`parse-changes`) makes no pair.

#### Scenario: live and archived change share an ID
- GIVEN `changes/x/change.md` with `status: applied` and `changes/archive/x/change.md` with `status: done`
- WHEN the model loads `changes/`
- THEN loading reports `model/unique-change-ids` at `changes/x/change.md`, naming `changes/archive/x/change.md`, and the graph holds one change `x`, not archived, with status `applied`
