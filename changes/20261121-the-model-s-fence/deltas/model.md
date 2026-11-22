---
component: model
base: { contract: "1374ba783e72e2c2" }
---

## MODIFIED Contract
Loads a repository's `specs/`, `changes/`, `backlog/` and `docs/decisions/`, and its root `ARCHITECTURE.md` and `CAPABILITIES.md` (docs/spec-format.md), from a directory (`load(root)`) or from a map of repo-relative paths to contents (`load_files(files)`), with the same result: a graph and every violation found. The graph holds the components (each with its frontmatter, Contract, Design, requirements, assets, parent and content hashes), the changes with their deltas, the backlog items, the decisions, the two human layers' entries as requirements with IDs `architecture/<slug>` and `capability/<slug>` (`human-layers`), and CLM's carried constraints (`carried-constraints`). A violation is a path, the ID of the requirement whose rule it breaks and a message, and prints as `<path>: <rule>: <message>` (`violation-line`). A file with no `---` frontmatter block, or whose frontmatter its kind's parser refuses (a key the kind doesn't know, a key every record of the kind must hold left out, a value of the wrong type or outside the key's listed values), is reported and left out of the graph. A record that parses and then breaks a rule, one that turns on another key's value or on another element of the graph among them, is reported and stays, unless its requirement says it is not loaded. The model offers no query of whether a component is optional: `optional` is read by its own rules alone (`optional-nodes`).

A component's assets are the files under `specs/<component path>/assets/` (`node-assets`). A component may name its test files with `tests` (`test-ownership-keys`), the rules its files are exempt from for a time with `exemptions`, each naming the backlog item that ends it (`exemptions`), and, on the root alone, the code no component owns yet with `outside` (`outside-globs`) and the files every build may edit with `wiring` (`wiring-globs`). `architecture` is a component ID no component may take, and component `capability` is the capability layer's own (`reserved-ids`).

The graph offers `requirement(id)`, a component's requirement or a layer's entry; `entry(id)`, an entry alone; `requirements()`, every component's requirements, then the architecture entries, then the capabilities; `effective_serves(id)`, the entries a rule serves (`serves`); `heading(layer)`, the heading a layer's file writes its entries under; `capability_node()` and `capability_tests_node()` (`reserved-ids`, `capability-tests-component`); `backlog_node_known(id)` (`backlog-components`); the root's `outside()` and `wiring()`, absent when the root doesn't set the key; and `declared_frontmatter_keys()`, the component frontmatter keys live changes declare (`declared-frontmatter-keys`). A requirement gives its `id`, `text`, `hash` and `version()` (`content-hash`), its `needs` (`requirement-needs`) and `serves`, and, for an entry, its `enforced_by` rules and its `#### Untested` text.

The model offers the checks loading runs, so another component judges by the same rules: `parse_node(id, path, text, declared)` and `check_graph(graph)` (`graph-check`), `check_node_frontmatter(id, path, frontmatter_text, declared)`, which gives the parsed frontmatter with the root rules' violations, or the violations of the rules the text alone decides (`optional-nodes`), and `check_exemptions(value)` (`exemptions`). It offers the parse of one record's text, `parse_change_frontmatter(text)` (`parse-changes`) and `parse_backlog_item(slug, text)` (`parse-backlog-item`); `content_hash(text)` and `asset_hash(bytes)`; `split_frontmatter(text)`, a file's `---` block and the rest; the `outline` it reads a spec's sections, requirement blocks and code fences by (`code-fences`); `carried()`; `Layer`, with each layer's ID prefix and file; `Priority`, `P1` the highest; whether a change is pending, `ChangeStatus::is_pending`; the term lists `STATUS_TERMS`, `CHANGE_STATUS_TERMS`, `BACKLOG_KIND_TERMS` and `CHANGE_KEYS` (`term-lists`); and `ROOT_ID` and `ARCHIVE_DIR`, the root's ID and the archive's directory name. It builds for `wasm32-unknown-unknown`, so the `web` Worker can use it.

## ADDED Requirements

### Requirement: code-fences
The model SHALL read the code fences of every Markdown text it outlines (a component's `SPEC.md`, a delta, a human layer's file, a carried constraint) as CommonMark reads them, and `outline::fence_mask(lines)` SHALL mark each line that opens or closes a fence or lies inside one. A fence opens on a line that, after leading white space, starts with a run of three or more backticks or three or more tildes, unless the run is of backticks and another backtick follows it on the line. It closes on the first later line that, with leading and trailing white space taken off, is only a run of the opening run's character at least as long as that run; a fence never closed runs to the end of the text. A line inside a fence opens, closes and starts nothing. This is the reading `check/asset-references` gives fences, so a requirement's statement, which ends at its first `####` line outside a fence (`gate/cuts`, `check.health/rule-signals`), ends where the code spans read in it end.

#### Scenario: a backtick run with a backtick after it opens no fence
- GIVEN the lines ```` ```x``` ````, `#### Scenario: s` and `text`
- WHEN `fence_mask` reads them
- THEN no line is marked

#### Scenario: a fence closes only on a bare run at least as long
- GIVEN the lines ```` ```` ````, ```` ``` ````, ```` ```rust ````, `#### Scenario: fake`, ```` ```` ```` and `after`
- WHEN `fence_mask` reads them
- THEN the first five lines are marked and `after` is not

#### Scenario: a tilde fence closes only on tildes
- GIVEN the lines `~~~`, ```` ``` ````, `~~~ not a close`, `~~~~` and `after`
- WHEN `fence_mask` reads them
- THEN the first four lines are marked and `after` is not

#### Scenario: a fence never closed
- GIVEN the lines ```` ``` ```` and `## Design`
- WHEN `fence_mask` reads them
- THEN both lines are marked

#### Scenario: a heading in a longer fence
- GIVEN a `SPEC.md` whose requirement `a` holds a fence of four backticks containing a line of three backticks and then `### Requirement: fake`
- WHEN the model loads it
- THEN the component's one requirement is `a`, and its text holds `### Requirement: fake`
