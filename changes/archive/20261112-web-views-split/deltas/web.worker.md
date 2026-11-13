---
component: web.worker
---

## MODIFIED Requirements
### Requirement: pr-routes-removed <!-- base: 62c5fe09a51565d1 -->
The Worker SHALL answer 404 to `GET /api/repos/{o}/{r}/pulls`, `GET /api/repos/{o}/{r}/pulls/{n}/review` and `POST /api/repos/{o}/{r}/pulls/{n}/approve`, making no write to the repository for the last.

#### Serves
- architecture/no-dead-code
- architecture/spec-is-current

#### Needs
- web.views/spec-branch-review
- web/approve-by-branch

#### Scenario: old routes
- GIVEN a signed-in user who can read `o/a`, and an open pull request #7 whose head is the spec branch `spec/x`
- WHEN they request `o/a`'s `pulls`, PR 7's review, and then post PR 7's approve
- THEN each answer is 404, and no ledger ref or other ref of `o/a` changes
