---
title: "The web review sees a signature, not its validity"
date: 2026-11-24
status: accepted
---

In a signed repository (`signed-ledger-records`), `clm land` counts an `approved` record only when its `sig` verifies with `ssh-keygen -Y verify` under `clm-approve` against the base's `docs/allowed_signers`. That rule covers approvals and the approvals that clear a reviewer's request for changes (`ledger`'s `cleared_at`, `land.approval/request-refusal`). The Worker can't run `git` or `ssh-keygen` in wasm and holds no SSH verifier, so it can see that a signature is present but not that it is valid. Verifying signatures in wasm stays declined (`web-approve-off-when-signed`).

So the web review uses presence. In a repository whose default branch's tip holds `docs/allowed_signers`, an approval counts toward clearing a request only when its line carries a non-empty `sig` (`ledger`'s `cleared_by_sig`), and an approval's `signed` mark uses the same test (`carries_sig`). An unsigned approval counts for nothing, on the web as at the land.

Over the same ledgers and the same tip, the web can show a request as not held where `clm land` holds it. It never shows the opposite. Every case has a `sig` present that doesn't verify:
1. a forged or garbage `sig`;
2. a `sig` by a key `docs/allowed_signers` doesn't list for the writer's person;
3. a writer whose person part is empty;
4. a `sig` made under another namespace, such as `clm-land`;
5. a `sig` of whitespace alone.

A `docs/allowed_signers` that is a tree at the tip is signed to both, but the land can't read it and stops with an error rather than holding.

Where the two read different inputs, they can differ either way:
- The web shows a request as held where the land clears it:
  - the web's fold is up to a minute old, so a fresh `clm approve` may not be folded yet;
  - a `clm approve` whose push failed is only in the lander's clone;
  - a branch with several merge bases can give the review a different fingerprint from the land's.
- The web shows it as not held where the land holds it: `main` gains `docs/allowed_signers` between the review's read and the land's fetch.

This is safe because the web review decides nothing. `clm land` still runs its own check with the signature verified, so a request the web shows cleared by a bad signature still holds the land, and the person learns it there. The review's held flag is advice about what the land will do, exact over the same ledgers and tip in every case but a signature that doesn't verify. The gap is of the kind `approval-gap-accepted` already accepts in writing: what holds the line is the land's check and `clm ledger verify`'s audit after the fact, not the web's view.
