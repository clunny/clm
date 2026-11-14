---
title: "Approvals and lands are signed with the person's SSH key once a repository lists its signers, and the audit checks them"
date: 2026-10-27
status: accepted
---

A ledger record's `writer` is a claim: anyone with push access can append an `approved` record naming any person, or set a commit's author to any name, so `approval = "other"` was honoured on trust (`per-writer-ledgers`, `approval-policy`). Now a repository that commits an OpenSSH allowed-signers file at `docs/allowed_signers`, its principals being writer persons, is *signed*: from the first land whose base holds that file, an `approved` record counts only when its `sig` field is a valid SSH signature, namespace `clm-approve`, by a key the file lists for the record's writer's person, and every `landed` record carries one too, namespace `clm-land`. Signing and checking reuse `ssh-keygen -Y sign` and `-Y verify` and git's own `user.signingkey` (`reuse-first`); CLM keeps no key and adds no setting.

**Where the check bites.** `clm land` runs on the lander's machine (`land-locally`), and the lander is the person `other` guards against, so a check there alone could be skipped with a local `approval = "any"` or a patched binary. The guarantee is the audit: `clm ledger verify`, which `clm check` runs on `main` in every clone, reports a land in the signed era whose `landed` record isn't signed by its writer (`ledger/unsigned-land`), or a land that needed review whose fingerprint, recomputed from the merge commit's parents, no counted approval names (`ledger/unapproved-land`), under the `approval` that the committed `clm.toml` at the land's base sets. Under `other` a counted approval's person must differ from the branch's commit authors and from the lander, the signed `landed` record's writer: author names are free text, so without the lander's own signature an author could name someone else as the author and approve their own branch.

**The file needs review to change.** The gate names `docs/allowed_signers` among the files whose edit needs review (`gate/needs-review`, `yes-no-review`), so adding a key, or removing the file to leave the signed era, needs an approval that the then-current rule accepts. The first file lands under the unsigned rule, which is the bootstrap every trust root needs.

**Key custody.** A signature proves which person's key signed, not that a human rather than that person's agent pressed: the key on a machine is as reachable to Claude as the person's `gh` token. The guard's `ask` prompt stays what makes an approval the human's (`approval-policy`). A key that needs a touch (`sk-ssh-ed25519`) adds a human's hand, and a project can require one for approvals by listing only such keys with `namespaces="clm-approve"`; CLM doesn't require it. One key, `user.signingkey`, signs both kinds, so a touch key also makes every land, unattended builds' included, wait for a touch; a separate key per act is left open.

**A host's identity.** An approval appended by `web.worker` is named by the person's GitHub login and committed with their token, which only the host can vouch for, and no check reads the host (`git-is-truth`). It carries no signature, so in a signed repository it doesn't count; `clm approve` does.

**Records from before.** The era is judged per land, by its base: lands whose base has no `docs/allowed_signers` keep the old rule, and the audit doesn't re-judge them. An approval written before the switch, unsigned, doesn't count for a land after it: the human approves again with `clm approve`.

What stays out: `clm check`'s run of the audit trusts the `landed` record's kind and whether it needed review, attributably since the record is signed, and `clm ledger verify --review` recomputes them (`yes-no-review`, `checkpoint-recomputes-new-lands`); `checked` records stay unsigned, since nothing gates on who wrote them. A change to `clm.toml`'s `approval` needs review like the signers file (`approval-setting-is-spec`).

Because the audit reads the approval behind each signed land that needed review, compaction keeps those `approved` records: one line per such land, like the `landed` lines.

Rejected:
- **Signed commits on the ledger branch.** A record would be proven by the commit that added it, so the fold would walk each branch's history rather than read its tip, compaction's rewrite would re-sign every kept record as of the compaction, and `web.worker` couldn't check a record without fetching commits. A signature inside the record travels with the line wherever it is copied.
- **A `signing` setting in `clm.toml`.** The signers file is the setting: without it there is nothing to verify against, and a key added in `clm.toml` would land as code.
- **Signing every record.** Each signature with a touch key is a touch; `checked` records gate nothing on their writer.
