---
component: web.worker.review
---

## MODIFIED Requirements

### Requirement: pr-review <!-- base: f2b30179bf3f14af -->
For a `spec/*` branch, the review SHALL answer 200 with:
- `branch`, `base` (the default branch), `base_sha` (its tip) and `head_sha` (the branch's);
- per touched component, every added, modified and removed requirement, every changed Contract, Design and frontmatter, and every added, modified and removed asset, each with its text before and after, a binary asset's text being empty on each side that holds it;
- `change`: the `id`, `status`, `from` and `resolves` of the live change the branch touches, its `intent`, the body of its `change.md` after the frontmatter, and its `intent_section`, the body of its `## Intent` section, null when that is missing or empty;
- `decisions` and `backlog`, the decisions and backlog items it adds, modifies or removes, each with its text before and after;
- `documents`: every path whose entries differ between the Trees API listings of the merge base and of the head that `gate/classify`'s `is_spec_path` accepts, and `clm.toml` when the approval setting changed (`approval-setting-branch`), each with its text before and after, null on a side whose listing doesn't hold it and empty on a side whose text GitHub withholds;
- `fingerprint` (`fingerprint-from-trees`); `approvals`, the `approved` records the ledger holds for that fingerprint, each with its `writer` and `at`; and `checked`, whether a `checked` record names the head's code key, false when the head's listing is truncated;
- `page`, the `number` and `url` of the open pull request on the default branch whose head is the branch in this repository, not a fork's, null when there is none or GitHub doesn't answer;
- `review` and `cuts` (`gate-answer`), `signed` (`signed-repository`), and `comments` and `change_requests` (`review-feedback`).

It SHALL answer 404 for a name `spec-branches` refuses and for a branch GitHub says doesn't exist (`web.worker.platform/branch-tip`), and 502 when GitHub fails the read of the branch's ref, of the default branch's tip, of the comparison or of the ledgers.

#### Scenario: modified requirement
- GIVEN a spec branch that modifies requirement `n/r`, adds asset `assets/m.html` to `n` and adds decision `d`
- WHEN its review is requested
- THEN the response lists `n/r` as modified with both texts, `assets/m.html` as added, and `d` as added

#### Scenario: approval state
- GIVEN a spec branch and a ledger with an `approved` record for its fingerprint by writer `clunny/web`
- WHEN its review is requested
- THEN the response says it is approved by `clunny/web`

#### Scenario: not a reviewable branch
- GIVEN `impl/n`, `spec/../main`, `spec/missing`, which doesn't exist, `spec/x`, whose ref lookup GitHub fails with a 502, and `spec/y`, whose compare GitHub can't answer
- WHEN each review is requested
- THEN the first three answer 404 and the last two 502

#### Scenario: what the branch changes, whole
- GIVEN a spec branch that modifies `specs/n/SPEC.md`, edits `CAPABILITIES.md` and `CLAUDE.md`, adds `docs/allowed_signers` and a binary `docs/img.png`, and changes `.gitattributes`
- WHEN its review is requested
- THEN its `documents` hold `specs/n/SPEC.md`, `CAPABILITIES.md` and `CLAUDE.md` with their texts before and after, `docs/allowed_signers` with `before` null and its text after, and `docs/img.png` with `before` null and `after` empty, and not `.gitattributes`

#### Scenario: a binary asset
- GIVEN a branch that adds `assets/logo.png`, whose bytes aren't UTF-8, to `n` and modifies a binary asset `assets/a.bin` of `n`
- WHEN its review is requested
- THEN `assets/logo.png` is added with `after` empty, and `assets/a.bin` modified with both texts empty

#### Scenario: the page
- GIVEN `spec/c2` with open pull request #5 on the default branch and a fork's pull request #6 from a branch of the same name, and then a GitHub that fails the pull-request query
- WHEN the review is requested each time
- THEN `page` is `{number: 5, url}` and then null, the review answering 200 both times

#### Scenario: a pull request onto another base
- GIVEN `spec/c3` whose only open pull request, #8, is onto `release`, not the default branch `main`
- WHEN its review is requested
- THEN `page` is null

#### Serves
- capability/approve-intent
- architecture/humans-review-intent

#### Needs
- web.worker.platform/branch-tip
- web.worker.platform/spec-files
- web.worker.platform/fake-github
- gate/classify
