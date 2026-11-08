---
title: "The upgrade skill puts [verify] settings on the notes branch, so a notes branch with a spec edit is refused as mixed"
kind: bug
components: [plugin]
issues: [372]
---

When `clm upgrade --apply` takes `20261107-slow-tests-after-land` and the human answers yes to the slow-test questions, step 4 of `skills/upgrade/SKILL.md` writes the real `[verify]` table into `clm.toml` on the notes branch. The gate leaves only stamp-only `clm.toml` edits out of classification (`gate/upgrade-stamp-left-out`). A settings change is code, so a notes branch that also carries a note's spec edit, such as a change with a delta and its applied `SPEC.md`, is classified `mixed` and can't land. The adopter had to move the edit to a separate code branch by hand.

Expected: the skill puts the `[verify]` settings with the other changes outside spec paths, on the first `upgrade/<YYYYMMDD>` branch, as `plugin/upgrade-skill` already says. Only the `upgraded` and `declined_migrations` lines go on the notes branch. Settle with the fix:
- The questions must be asked before the first branch is committed.
- When the first branch has nothing else to land, the settings form a code branch of their own.
- The skill should say so plainly, so a session doesn't commit the stamp and the settings together. Both live in the same file, so committing them on separate branches means splitting `clm.toml`'s edit.

`plugin/upgrade-skill-offers-verify` names no branch, so the skill text alone is wrong. No requirement needs to change.
