---
component: model
---

## ADDED Requirements
### Requirement: backlog-choices
A backlog item's frontmatter SHALL accept `choices`, a list of strings, on a `kind: question` item only, and the model SHALL offer them on the parsed item, each trimmed, empty when the key is absent or the list is empty. Loading SHALL report `model/backlog-choices`, naming the item, for `choices` on an item of any other kind, and for a list with fewer than 2 or more than 5 entries, an entry that is empty or over 40 characters once trimmed or holds a line break, or two equal entries. `parse_backlog_item` SHALL apply the same rules.

#### Scenario: a question with choices
- GIVEN `backlog/q.md` with `kind: question` and `choices: [yes, no]`
- WHEN the graph loads
- THEN item `q` carries the choices `yes` and `no`, and there is no violation

#### Scenario: refused
- GIVEN `backlog/a.md` with `kind: idea` and `choices: [yes, no]`, and `backlog/b.md` with `kind: question` and `choices: [yes]`
- WHEN the graph loads
- THEN it reports `model/backlog-choices` for `a` and for `b`

#### Serves
- capability/follow-from-browser
- capability/keep-a-backlog
