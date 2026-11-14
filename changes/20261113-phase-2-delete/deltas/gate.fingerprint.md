---
component: gate.fingerprint
base: { contract: "46669253edee3aa3" }
---

## MODIFIED Contract
`clm fingerprint --base <ref>` prints the fingerprint of the changes `HEAD` makes relative to `<ref>`, so an approval can be pinned to the diff it was given for (`approve-pins-the-review`). It depends on the two commits alone, never on git config or the host that computes it (`tree-diff-fingerprint`). Crate `clm-fingerprint` holds it: `fingerprint(repo, base)` is the command's answer; `fingerprint_of(base, head)` computes it from the two trees' entries (`Entry`), each a mode, type, full object ID and path as `git ls-tree -r` lists it and `parse_ls_tree` reads it, with no process, so the `web` Worker computes the same value from the Trees API's listings. `adds_change(base, head)` says whether `head` adds a change, when the fingerprint skips `specs/`, and `added_changes(base, head)` names the `change.md` paths it counts. `clm-gate` re-exports all six (`gate/fingerprint-reexported`).
