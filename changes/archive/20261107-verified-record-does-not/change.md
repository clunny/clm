---
status: done
resolves: [verified-record-does-not-say-commit-ids-are-lowercase]
---

## Intent

Say in `ledger/verified-record` that a `verified` record's commit IDs are lowercase, as git writes them, so the rule the fold already follows is written down. The requirement asks for "full 40-hex commit IDs" in `commit`, `since`, `lands` and `culprit`. The build also skips a line whose ID is uppercase (`full_id` in `crates/clm-ledger/src/record.rs`, checked by the `bad_result` test), because `Fold::verifications` and `unresolved_reds` look records up by exact string, so an uppercase ID would be held but never matched. The spec auditor flagged that as behaviour no requirement states. This change words the clause as "full 40-hex lowercase commit IDs, as git writes them" and adds a scenario for an uppercase ID. That answers the `question` item `verified-record-does-not-say-commit-ids-are-lowercase`.

## The human's answers

> Human's answer: "Lowercase required (Recommended)" — Word the clause 'full 40-hex lowercase commit IDs, as git writes them'. This matches the code as built.

## Problem

A requirement that says only "full 40-hex" leaves open whether an uppercase ID is valid. A builder reading it as written could accept uppercase and hold records that `unresolved_reds` and every lookup by commit then miss, so a red result would be lost without a word. The code chose the safe reading; the spec should say so.

## Approaches

- **Lowercase required** (the human's choice): the fold skips an uppercase ID with a warning, as it does any malformed ID. This matches the code as built. `verify` takes IDs from git, which writes them in lowercase. An import takes its run's commit from the host's data, so an uppercase ID there is skipped with a warning, never held without a word.
- **Accept uppercase and lowercase it on read**: more lenient, but it adds a normalisation step nothing needs (`yagni`), since no writer produces uppercase IDs.

## Outcome

`ledger/verified-record` is modified: its clause reads "full 40-hex lowercase commit IDs, as git writes them", and a new scenario, `uppercase commit`, says the fold skips a line whose `commit` is uppercase hex, with a warning. The code already does this, so the build only cites the tests at the new version.

## Constraints

None beyond the requirement's own.

## Edge cases and workarounds

None.

## Open questions

None.
