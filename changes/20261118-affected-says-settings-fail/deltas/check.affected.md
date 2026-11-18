---
component: check.affected
---

## MODIFIED Requirements

### Requirement: affected-base <!-- base: 3d9c4e19a3eb29cc -->
`clm affected` SHALL compare the working tree, untracked files included, with the merge base of `--base <ref>` and `HEAD`, defaulting `<ref>` to `origin/main`. It SHALL exit 1 naming the ref when `<ref>` doesn't resolve to a commit or has no merge base with `HEAD`, 1 with the error when the graph can't be loaded, and, except with `--readers`, which reads no setting, 1 when its caller can't load the `[affected]` settings (`config/affected-setting`), writing the line the caller gives for the failure to standard error and nothing to standard output; the line names the file and the key, which is the root's to print (`clm/routes-only`).

#### Scenario: main moved on
- GIVEN a branch that changes `crates/a/src/lib.rs`, and `main` that has since changed `crates/d/src/lib.rs`
- WHEN `clm affected --base main` runs on the branch
- THEN it prints the component line of `a` and none of `d`

#### Scenario: unknown base
- GIVEN a repository with no ref `nope`
- WHEN `clm affected --base nope` runs
- THEN it exits 1 naming `nope`

#### Scenario: invalid graph
- GIVEN a repository whose `specs/` has a dependency cycle
- WHEN `clm affected --base main` runs
- THEN it exits 1 with the error and prints no component, `file` or `all` line

#### Scenario: settings that can't be loaded
- GIVEN a caller whose loading of the settings fails with a line
- WHEN `clm-affected`'s call for `clm affected --base main` runs
- THEN it returns 1, writes that line to standard error and nothing to standard output
