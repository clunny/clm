---
title: "A comment or a change request made in the web app is a ledger record, and a change request holds the land"
date: 2026-10-28
status: accepted
---

A reviewer reading a spec branch's diff in the web app could approve it and nothing else (`backlog/web-proposals`). Now they can comment on a requirement, or on the change as a whole, and ask for changes. Each is a record in their web ledger, appended with their token as an approval is (`per-writer-ledgers`, `web-marks-soft-lands-reviewed`): `commented` with the `branch`, the `fingerprint` of the review they were shown, a `target` (a requirement ID, or null for the change) and the `text`; `changes_requested` with the `branch`, the `fingerprint` and the `text`. Neither is a GitHub write: no host page, no App permission beyond the `contents: write` the ledger append already has, and a branch without a page can be commented on.

A change request holds the branch: `clm land` exits 1, printing the request and the comments at that fingerprint, while a `changes_requested` record names the branch's current fingerprint and its writer's person hasn't approved that fingerprint since. It is pinned as an approval is (`pinned-approval`): a push that changes the diff orphans it, since the author answered it by changing something, and the requester clears their own by approving. That is the one way a person who isn't at a session can stop an `AUTO` or `SOFT` branch from landing on checks alone, and the land's message tells the session what was asked, so it is the whole conversation a reviewer needs. A comment holds nothing: the Worker records it against whatever fingerprint the review carried, without checking the branch moved, because a comment on an earlier version still reads and the record says which version it was on. A change request is checked (409 when the branch moved), since it holds only a diff the person saw.

A request counts whether or not the repository is signed, and whether or not the record is signed (`signed-ledger-records`): a signature proves who granted something, and a request withholds a land rather than granting one. The Worker takes a request only from a person GitHub says has `write` or `admin` on the repo, who could already keep a branch from landing by other means, and `clm land` names the writer. A person's web records name their GitHub login, so a requester whose CLI `writer` isn't that login sets it (`config`) before clearing a request with `clm approve`. So the web app offers Request changes in a signed repository, where its Approve is off (`web-approve-off-when-signed`), and the requester there clears their request with `clm approve <branch>`.

- Only `spec/*` branches that touch spec paths take comments and requests, the branches the app reviews.
- `clm ledger compact` drops both kinds once a `landed` record names their fingerprint, as it drops approvals: the diff is on `main`.
- Neither kind reaches the activity feed or Web Push in this change; a reviewer's text is read on the branch's review and at `clm land`.
- An older `clm land` doesn't know the kind, skips the line with a warning and lands: a team on mixed versions gets no hold, as it got no signed approvals before every clone was rebuilt.

Rejected:
- **GitHub review comments and "request changes" on the branch's page**, what the item first imagined. Needs `pull_requests: write`, which `forge-adapters` removed; a branch may have no page (`host-adapters`); and `clm land` reads no host, so a request there would hold nothing.
- **A request that holds until the requester lifts it, whatever the author pushes.** A reviewer who isn't watching would hold a branch for days over a line the author already changed; pinning to the diff is what approvals do, and it errs toward the author.
- **Refusing requests in a signed repository, as approvals are refused.** It would take the one hold a phone can place away exactly where a team cares most, for a record that grants nothing.
- **Comments pinned with a 409 like approvals.** A comment on a line that moved is still worth reading; the version mark tells the reader.
