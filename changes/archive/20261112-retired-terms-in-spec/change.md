---
status: done
resolves: [retired-terms-in-spec-text-are-caught-only-at-land]
---

# A retired term in spec text fails `clm check`

## Intent

A spec branch whose text names a retired term (a renamed command, flag or coined term) passes `clm check` and `clm gate` and fails only at `clm land`, in the root's retired-terms test, after a draft has queued for its land and, for a change that needs review, after the human approved it, so the fix changes the approved fingerprint. This change moves the retired-terms scan out of the Rust test into a POSIX `sh` script with its list in a text file, and runs the script as a detector in this repository's `clm.toml`, so `clm check`, which every draft runs, fails on it. The Rust test runs the same script on the tree and on every command's help, so one matcher serves both.

## Problem

`crates/clm/tests/retired_terms.rs` holds the list and the matcher, and reads `specs/` among the text CLM ships. It is a cargo test, so only a check that compiles the workspace runs it: `clm land`, through its `clm:reads specs/**` marker, or `clm land --check-only`, which the draft prompt asks for only for hand edits outside `specs/`, `changes/` and `backlog/` (`draft-checks-hand-edits`), since it compiles and runs every suite. `clm check` and the gate read no test. A delta that names a retired term reaches `specs/` at `clm apply`, so nothing a draft runs catches it.

## Approaches

- **The scan becomes a `sh` script run as a detector (chosen).** `clm check` already runs this repository's own detectors (`check/detectors`); a grep over a few megabytes of text costs well under a second, needs no build, and runs on every `clm check`, in drafts, builds and lands alike. Keeping one matcher, the script, and having the Rust test call it (for the tree and for the help texts, which only the built binary prints) avoids two matchers that drift (`reuse-first`).
- **Ask every draft for `clm land --check-only`.** Rejected, as `draft-checks-hand-edits` already rejected it: it compiles the workspace and runs every suite per draft, for a failure a text search finds.
- **A detector that runs `cargo test --test retired_terms`.** Rejected: it builds the `clm` binary in each worktree's own target on every `clm check`, minutes in a fresh draft worktree.
- **A built-in `clm check` rule with a `clm.toml` list of retired terms, for every project.** Rejected for now (`yagni`): `plain-terms` governs CLM's own vocabulary, no adopter has asked for it, and it would add a settings key, a check rule and a guide section where a repository-local detector does the job.
- **Accept it.** Rejected: the failure reaches the human as a re-approval when the change needed review, the harm `draft-checks-hand-edits` exists to avoid.

## Outcome

- `clm/retired-terms`: the list moves to `scripts/retired-terms.txt` and the scan to `scripts/retired-terms.sh` (`sh` and `grep` only), with the same files, matching rules and `clm:remove-by` exemption; the root's `paths` gain both files. The test in `crates/clm/tests/retired_terms.rs` runs the script on fixtures, on the tree and on the help texts written to files, and its list checks (plain words, the old command, flag, state and key names) read the list file.
- `config/retired-terms-detector`: this repository's `clm.toml` names a third detector, `retired-terms`, `["sh", "scripts/retired-terms.sh"]`; `config`'s Design lists it.
- `ci/detector-tools`: a detector whose command is `sh` and a tracked script needs no install line, as one whose command is `clm` needs none.
- `docs/decisions/plain-terms-renames.md` and `docs/decisions/draft-checks-hand-edits.md` carry an amendment saying so.

## Edge cases and workarounds

- The script's whole-word test is `grep -w`'s (a letter, digit or `_` is a word character), which is the rule as stated; a multi-word term becomes an extended regex with ` +` between its words, and the script escapes the regex metacharacters a term holds (a backtick needs none; a leading `-` needs `-e`).
- `grep -I` skips binary files, where the Rust walk skipped files that weren't UTF-8; the effect on this repository's text is the same.
- A help text is written to a temporary file named for its command (`clm-land`); the test strips the directory when it reports.
- Until `config`'s build lands, `clm check` doesn't run the scan; the land still runs the test through its marker, as now.

## Constraints

`plain-terms`, `reuse-first`, `yagni`, `path-ownership` (each build edits only its own component's files: the script and list are the root's, `clm.toml` is `config`'s, the ci scripts' test is `ci`'s), `draft-checks-hand-edits`.

## Open questions

- None. The audit's two findings were taken: `plain-terms-renames` and `draft-checks-hand-edits` each carry an amendment saying the scan now runs as this repository's `clm check` detector.
