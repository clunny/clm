---
component: web.worker.review
base: {}
---

## MODIFIED Requirements

### Requirement: pr-review <!-- base: 574ed19f1e28b258 -->
For a `spec/*` branch, the Worker SHALL return, per touched component, every added, modified and removed requirement, every changed Contract, Design and frontmatter, and every added, modified and removed asset, each with its content before and after, together with the change's intent, its `## Intent` section as a separate field, decisions and backlog edits, the branch's fingerprint, the `approved` records the ledger holds for it with their writers, whether a `checked` record names the head's code key, the pull request's URL when the host has one, and a top-level `review`: whether the branch needs a human's review, by `clm_gate::needs_review_of` (`gate/needs-review-of`) over the paths whose entries differ between the Trees API listings of the merge base and of the head, whether the approval setting changed (`approval-setting-branch`), and the head's `change.md` frontmatter of each change one of whose files is among those paths, `None` for one that is missing or doesn't parse; `review` SHALL also be true when `clm_gate::widened_exemptions` or `clm_gate::cuts_or_unloaded` returns anything for the two graphs the review compares, each with the violations its loading gave, and the texts of each change's `deleted-tests.txt` among the files it fetches (`gate/needs-review-of`, `gate/cuts`), and the review SHALL carry a top-level `cuts`, the lines `clm gate` prints for those cuts, in its order, empty when there is none; so a side whose specs don't load gives the gate's one line for it and `review` true. `review` SHALL be true when a changed path is a `deleted-tests.txt` whose text GitHub withholds, and when either listing is truncated `cuts` SHALL be empty, since they would be read from part of the files. `review` SHALL be true when either listing is truncated. The change SHALL carry no `review` of its own. A comparison cached before this requirement SHALL NOT be served: the cache key is named anew.

#### Scenario: modified requirement
- GIVEN a spec branch that modifies requirement `n/r`, adds asset `assets/m.html` to `n` and adds decision `d`
- WHEN its review is requested
- THEN the response lists `n/r` as modified with both texts, `assets/m.html` as added, and `d` as added

#### Scenario: approval state
- GIVEN a spec branch and a ledger with an `approved` record for its fingerprint by writer `clunny/web`
- WHEN its review is requested
- THEN the response says it is approved by `clunny/web`

#### Scenario: a removed requirement
- GIVEN a spec branch that removes requirement `n/r` and changes nothing else
- WHEN its review is requested
- THEN the response's `review` is true and its `cuts` hold `specs/n/SPEC.md: gate/cut: requirement n/r is gone`

#### Scenario: specs that don't load
- GIVEN a spec branch whose head's `specs/n/SPEC.md` holds a requirement with no scenario
- WHEN its review is requested
- THEN the response's `review` is true and its `cuts` hold only `specs: gate/cut: the specs at <head> don't load, so what the branch cuts isn't listed`, `<head>` as the review names the head

#### Scenario: a truncated listing
- GIVEN a spec branch that removes requirement `n/r`, and a head listing GitHub truncates
- WHEN its review is requested
- THEN the response's `review` is true and its `cuts` are empty

#### Scenario: a widened exemption
- GIVEN a spec branch whose only change adds an exemption to `specs/n/SPEC.md`'s frontmatter
- WHEN its review is requested
- THEN the response's `review` is true and its `cuts` are empty

#### Scenario: a human layer without a change
- GIVEN a spec branch that only edits `ARCHITECTURE.md`
- WHEN its review is requested
- THEN the response's `review` is true and it has no `change`

#### Scenario: a change with burdens
- GIVEN a spec branch that adds `changes/x/change.md` with `human_burdens: [b]`, and one that adds `changes/y/change.md` with no `human_burdens` and edits `docs/decisions/d.md`
- WHEN each review is requested
- THEN the first's `review` is true, the second's is false, and neither `change` has a `review` field

#### Scenario: truncated listing
- GIVEN a spec branch whose head listing GitHub truncates
- WHEN its review is requested
- THEN its `review` is true and it carries no fingerprint

#### Serves
- capability/approve-intent
- architecture/humans-review-intent

#### Needs
- gate/cuts
- web.worker.platform/shared-layer
- gate/needs-review-of
