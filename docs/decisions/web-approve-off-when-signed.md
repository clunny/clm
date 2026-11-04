---
title: "In a signed repository the web app turns Approve off and points to `clm approve`, and marks approvals signed or unsigned without verifying them"
date: 2026-10-27
status: accepted
---

Once `main` holds `docs/allowed_signers` (`signed-ledger-records`), `clm land` counts an approval only when its `sig` is a valid SSH signature by a key the file lists for the record's writer's person. The Worker holds no one's SSH key, so the `approved` record its Approve appends to `clm/ledger/<login>/web` never counts there, yet the review then listed it as an approval like any other, and the person who tapped Approve had no sign that they still had to run `clm approve`.

So the Worker reads whether the default branch's tip holds `docs/allowed_signers`, the base `clm land` will judge the approval against, and says so in the review as `signed`. In a signed repository its approve route refuses with 422 and writes nothing, and the app disables Approve and shows `clm approve <branch>` in its place. Each approval the review lists says whether its record carries a `sig`, and the app marks it "signed" or "unsigned, doesn't count". The Worker checks no signature: a signed approval is one `clm land` will check, never one the app calls counted, so the app shows what it knows and decides nothing (`web.worker`'s Contract).

Rejected:
- **Verify signatures in wasm** (an `ssh-key` style crate parsing `docs/allowed_signers` and checking `sshsig` blocks). It would let the app say which approvals count, but it is a second implementation of `ssh-keygen -Y verify`'s matching (principal patterns, `namespaces=`, `valid-after`/`valid-before`, `cert-authority`, `sk-` keys) beside the one `clm land` runs, and where the two disagree the app would call an approval counted that the land refuses. Declined, not deferred (`20261030-clm-serve`): `clm land` stays the one place a signature is checked, and whether an approval counts is learned when `clm land` checks it.
- **Keep Approve and write the unsigned record.** It is the problem: a record that looks like an approval and counts for nothing.
- **Sign in the browser** (a passkey or a key the Worker holds per person). The signature must be by a key `docs/allowed_signers` lists for the person, which the person manages with git's `user.signingkey`; a key held by the operator would make the host vouch for the person, which `signed-ledger-records` rules out (*A host's identity*).
- **Judge signedness at the branch's merge base or head.** `clm land` judges by the base it lands on, `main`'s tip, so a branch that adds the file is approved under the unsigned rule, as `signed-ledger-records`'s bootstrap says.
