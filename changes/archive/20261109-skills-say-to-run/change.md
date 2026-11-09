---
status: done
resolves: [skills-say-to-run-clm-pr-open-without-the-arguments-it-requires]
---

## Intent

A bare `clm pr open` opens the checked-out branch's pull request, with a title and body drawn from the branch, so the skills that write it that way work as written.

## Problem

Five skills say to open the pull request with a bare `clm pr open`: `upgrade` (its landing step), `configure`, `init`, `propose` (the small-change route) and `triage` (its intake step). The command requires a branch, `--title` and `--body-file`, so a session that follows the text exactly gets a usage error. The land still goes through, since `clm land` doesn't depend on the pull request, but the step as written can't succeed. Patching each skill's text would leave the next skill written the same way to repeat it.

## Approaches

- **Defaults in the command (chosen).** The branch defaults to the checked-out one; the title and body to the branch's change's `## Intent` on a `spec/<id>` branch, or to its commits' subjects otherwise. Given arguments still override, for the drafts and folds that write their own body. One fix covers every skill present and future, and fits a pull request being only for reading the diff (`host-adapters`): its text needs to orient a reader, not to be crafted.
- **Full form in every skill, with a test forbidding the bare form.** Repeats the arguments, and a body file to write, in each skill, and the test is one more rule to keep; the defect would come back in any text the test doesn't scan (prompts, the guide).

## Outcome

`host` gains `pr-open-defaults`; its Contract and Design say how the defaults are drawn. No skill changes: the bare form they already name becomes correct. Prompts and skills that pass all three arguments behave as before.

## Constraints

- The derivation lives in `clm-host`, and runs git only through `clm-git` (`git-through-layer`); `clm-host` gains no dependency on `model`, so it reads `## Intent` as text.
- The defaults are filled by one `clm-host` library call, `pr_open`, so `crates/clm/src/main.rs` stays dispatch only (`path-ownership`, from the architecture audit). The `Host` trait's `page_open` is unchanged, so the Azure and GitHub adapters and the fakes in other crates' tests are untouched.
- The derived body goes to a file in `std::env::temp_dir()`, so `clm-host` gains no runtime dependency.
- Edge cases known: on `git` and `none` the setting is checked first and nothing is derived or refused; a detached `HEAD`, a missing `origin/main`, and a branch with no change and no commit beyond `origin/main` each exit 1 before any host call; a whitespace-only `## Intent`, or a `spec/<id>` branch without `changes/<id>/change.md`, falls back to the commits.

## Open questions

None.
