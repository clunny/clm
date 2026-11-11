---
status: applied
from: backlog/init-skill-writes-build-setup-on-the-first-spec-branch-which-the
resolves:
  - init-skill-writes-build-setup-on-the-first-spec-branch-which-the
---

## Intent

`/clm:init` writes the project's `[build] setup` into `clm.toml` before the bootstrap commit, beside the `[verify]` settings, instead of on the first spec branch, where the edit made the branch `mixed` and unlandable (`backlog/init-skill-writes-build-setup-on-the-first-spec-branch-which-the`). `plugin.init/init-skill-build-setup` is modified to say so, with a scenario that fails a skill whose setup sentence names the spec branch or doesn't name the bootstrap commit.

## Problem

`init-skill-build-setup` tells the skill to write the confirmed setup command into `clm.toml` "on the first spec branch". `clm init` puts `clm.toml` in the root component's `paths` (`init/init-root-ownership`), so the gate counts that edit as code and classifies the spec branch `mixed`; following the skill as written leaves the first spec graph unlandable until the edit moves to a code branch. The rule itself is wrong, not just the skill's text: the code fix needs a spec change (`code-fix-needs-a-written-rule`). The `[verify]` settings and the check script's moved CI steps already go in the bootstrap commit (step 3 and step 4 run before step 5's commit of "everything so far"); the setup was the one `clm.toml` edit sent elsewhere.

## Approaches

1. **Write the setup before the bootstrap commit** (chosen). The question is asked in step 3, right after `clm init` writes `clm.toml` and before step 5 commits, exactly where the `[verify]` settings are written; it needs only the toolchain reading the step already does, not the component map. One commit, no extra branch, and the same shape the skill already uses for every other non-spec file (`reuse-first`).
2. **Land the setup as a small code change before the spec branch** (`small-change-no-claim`). Rejected: an extra branch, gate and land for one line that the bootstrap commit, made moments earlier, can hold; and before the bootstrap is pushed there is no `origin/main` to branch from.
3. **Make the gate treat `clm.toml` as a spec path.** Rejected: `clm.toml` holds `[land] check` and the detectors, which change what lands; it is code by design.

## Outcome

- `plugin.init/init-skill-build-setup` (modified): the setup is written into `clm.toml` before the bootstrap commit, never on the spec branch, and says why. A new scenario checks the sentence names `bootstrap commit` and not `spec branch`.
- The build rewrites the last clause of step 3's setup paragraph in `plugin/skills/init/SKILL.md` and extends `init_skill_build_setup` in `crates/clm/tests/plugin_init.rs`.

## Constraints

- `path-ownership`: `clm.toml` stays the root component's file; only the moment it is edited moves.

## Edge cases and workarounds

None known.
