---
status: applied
resolves: [spec-slimmer-proposes-removing-requirements-that-are-the-only]
---

## Intent

Have the spec-slimmer look for what names a requirement before it proposes removing or merging it: an `#### Enforced by` block in `ARCHITECTURE.md`, a decision under `docs/decisions/`, or another component's `#### Needs`. When one does, it says so beside the proposal, quoting the reference and saying what the cut would leave unproven or waiting, so the human never sees such a cut as a plain recommendation.

## Problem

On 2026-11-11 `/clm:health` sent the spec-slimmer over the flagged components, and it proposed deleting `host/shared-plumbing`, `view.repo/shared-plumbing` and `sync.regenerate/git-through-layer` as "implementation detail, not behaviour". The two `shared-plumbing` rules are the per-crate scans that `shared-plumbing-in-git` and `remote-plumbing-scan` say prove those decisions, crate by crate; `sync.regenerate/git-through-layer` is the scan `sync-regenerate-split` adds for the crate it splits out; and the same kind of rule, `host/git-through-layer`, is named in `host-adapters`' `#### Enforced by` block. Removing them would leave those decisions and that constraint without a test (`proven-by-tests`, `deterministic-enforcement`). `20261111-slim-flagged-specs` caught it and rejected the cut, but only after the proposal reached the human as a recommended cut. `plugin.agents/spec-slimmer` tells the slimmer to look for requirements that are implementation detail, and nothing tells it to check what relies on one.

## Approaches

1. **Tell the slimmer to search for references before it proposes removing or merging a requirement, and to say what it finds (chosen).** The three places a requirement ID is named as proof or as a dependency are plain text under `ARCHITECTURE.md`, `docs/decisions/` and `specs/`; the slimmer has `Grep` and `Bash`, so a search for `<component>/<slug>` finds them. It still proposes the cut, since a decision's proof can move to another requirement, but the human reads the cost beside it. The slimmer advises (`health-advises`), so a check in its prompt is the right weight.
2. **A `clm` command that lists what names a requirement.** Rejected (`yagni`, `reuse-first`): a text search finds the same references, and nothing else asks for the list.
3. **Have `/clm:health` or `/clm:propose` check the slimmer's cuts.** Rejected: `/clm:propose` already checks them against the spec, which is how this was caught; the gap is that the proposal reached the human first. Fixing it where the proposal is made keeps one place that judges a cut.

## Outcome

`plugin.agents/spec-slimmer` gains the search and the report, and a scenario whose test fails when the prompt leaves out any of `#### Enforced by`, `docs/decisions/` and `#### Needs`. `/clm:health` already lists each cut with its quotes, and the reference is quoted beside the cut, so it reaches the human with no skill change. Its build edits `plugin/agents/spec-slimmer.md` and the test in `crates/clm/tests/plugin_agents.rs`, re-citing it.

## Constraints

- `health-advises`: the slimmer only advises; nothing gates on its report.
- `proven-by-tests`, `deterministic-enforcement`: a requirement named as the proof of a constraint or decision keeps that proof unless something else takes it over.

- `ancestor-scoped-context`: the search reads past the component's bundle, as the slimmer's reading of its code and tests already does. That is accepted: what names a requirement is a reverse lookup no bundle holds, the slimmer only advises, and it reads only the lines that name `<component>/<slug>`, not other components' specs whole. The constraint governs the bundle `clm context` builds (`context/bundle-contents`), which doesn't change.

No edge cases or workarounds are known.
