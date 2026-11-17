---
status: applied
resolves: [a-setting-s-value-is-offered-to-build-before-the-install-item]
---

## Intent

Accept that nothing checks whether the build of a settings parser filed its `install-clm-for-<key>` item, and delete `backlog/a-setting-s-value-is-offered-to-build-before-the-install-item`. The change edits only the backlog.

## Problem

`20261116-a-mutation-pass-that-stops` added the `[verify]` fields `mutate_jobs` and `mutate_minutes` and had this repository's `clm.toml` set them (`config/verify-here`). `config`'s Design (`settings-before-use`) says the build that makes the parsing requirement current files `backlog/install-clm-for-<key>.md`, blocking the requirement that sets the value. That build (`ba6623d4`) filed none, so `clm next` offered `config/verify-here` as ready and the next build was told to write the values. `/code-review` caught it, and that build reverted the values and filed `backlog/install-clm-for-mutate_jobs.md` itself, which now holds `config/verify-here` back as the rule intends.

## Why it is accepted

- **What the skipped step protects is narrow, and fails loudly.** On this clone, `scripts/dev-install.sh`'s `reference-transaction` hook rebuilds the installed `clm` whenever `origin/main` moves (`rebuild-on-origin-main`), so by the time a build writes the value the installed `clm` already has the parser that landed before it. The item only protects other clones the human uses that haven't been rebuilt, and those refuse the file naming the unknown key, which `settings-before-use` already accepts as the enforcement ("Nothing checks the rule beforehand. The installed `clm` refusing the file is what enforces it").
- **It is rare.** It applies only to a key this repository's own `clm.toml` sets, a few times a month, and the build of the second requirement, the code review and the spec auditor all read `config`'s Design, which states the step. The one miss was caught before anything landed.
- **Neither suggested remedy fits the plan in force.** A sentence in the build prompt naming the item is prompt text a requirement could only prove present (`prompt-text-is-source`), the kind of rule `backlog/a-passing-check-proves-less-than-the-spec-claims` is removing. A check that refuses a `clm.toml` value whose key is new while no install item for it has landed and been deleted would read backlog history across commits, new mechanics in `config` or `check` for a one-repository rule, while the plan's freeze answers defects with "accept" and its phase 7 is where brakes become gates.

## Outcome

- `backlog/a-setting-s-value-is-offered-to-build-before-the-install-item.md` is deleted. No requirement, Design or decision changes.
- `backlog/install-clm-for-mutate_jobs.md` stays: it is the step the rule asks for, filed late.
