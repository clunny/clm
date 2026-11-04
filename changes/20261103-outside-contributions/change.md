---
status: applied
priority: P3
human_burdens:
  - "Changes public settings on clunny/clm once this lands: profile, features, merge options, a ruleset on main, security features and Actions approvals (docs/decisions/github-repo-settings.md)"
  - "Every contributor's pull request waits for your manual review, and each one you accept spends your budget on its builds"
  - "You set up forwarding for conduct@clm-app.dev, which publishes that address; the code of conduct isn't built until you have"
---

# Outside contributions

## Intent

Make `clunny/clm` ready for contributors, and give every project built with CLM a safe way to take contributions from people without write access. Outsiders contribute intent only: issues, backlog suggestions at any level of elaboration, and spec changes in pull requests from forks, each read by the maintainer by hand. Direct code changes are rejected, and `clm land` refuses a `spec/*` branch holding files outside the spec paths, so code from outside can't reach `main` or run on the maintainer's machine by the route accepted contributions take. This repository also gets a proper README, contributing instructions, community files and GitHub settings. Collaborators with write access stay out for now.

## Problem

`clunny/clm` is public, but GitHub scores its community profile at 28%: a short README and a licence, with no description, topics, security policy, code of conduct or templates. `CONTRIBUTING.md` covers only `scripts/dev-install.sh`.

The usual open-source route doesn't fit CLM. Claims, ledgers and `clm land` all push to `origin`, which a fork can't. `clm land` runs the merged tree's check script on the lander's machine and nobody reviews code (`ai-writes-code`), so landing a stranger's code means running it. And nothing stops that today: `clm land` accepts `code` branches, because normal builds are code branches. Any public project under CLM has these same limits.

## Approaches

- **A written route, the maintainer's review and one land rule (chosen).** A guide page and two skills say what contributors and maintainers do; `clm land` refuses a `spec/*` branch with files outside the spec paths. It adds a small rule and no new command or hosted job, and the maintainer already reads every pull request. Skill text and the guide reach adopters (`method-in-the-plugin`); `CLAUDE.md` alone wouldn't.
- **A `clm pr accept <n>` command.** The program fetches, judges and pushes. Stricter, but a product feature built before a single pull request has arrived. The land rule already stops what matters at the last step; add the command if volume justifies it (`yagni`).
- **An Actions workflow that closes code pull requests.** Instant feedback, but it needs `pull_request_target`, a known risk, a second hosted decision-maker beside `land-locally`, and a copy of the gate's path list.
- **Build it for CLM only, and generalise after the first real pull request.** Smaller, but it writes a working rule only in this repository from day one, against `method-in-the-plugin`.
- **Judging by the gate's first line.** Simple to state, but the gate prints `code` for a branch that only adds a backlog item, which would turn suggestions away. The rule checks the files instead (`spec-branches-hold-spec-paths`).

## Outcome

**For every CLM project**
- `land/spec-branch-holds-spec-paths`, using `gate/files-outside-spec-paths`: `clm land` refuses such a branch before any check runs.
- Capability `contribute-from-outside`, with its test.
- The guide page `contributions`: the levels, credit, security reasoning, limits and a `CONTRIBUTING.md` starter.
- `host/contribution-operations` and `clm/contribution-command`: `clm contribution` reads, fetches, comments on, closes and labels contributions, and opens a pull request on a fork's parent. The skills may not name `gh pr` or `gh api` (`plugin/skills`), and `host-adapters` holds everything host-specific in the adapter, so the skills use these rather than `gh`.
- `plugin/propose-in-a-fork`: `/clm:propose` asks whether a fork contributes back and, if so, stops at a pull request on the parent.
- `plugin/contribution-skill`: `/clm:contribution <number>` takes in a pull request or issue on the maintainer's side.

**For this repository**
- `clm/readme-front-page` and `clm/contributing-intent-only`.
- `ci/security-policy`, `ci/code-of-conduct`, `ci/issue-forms` and `ci/pull-request-template`.
- The settings in `github-repo-settings`, applied by the proposing session once this lands, with the human's yes to this change as the authority.

## Constraints

- A contribution counts when every file it changes is on a spec path: `specs/`, `changes/`, `backlog/`, `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md` or `CLAUDE.md`. So a typo fix in `README.md` arrives as an issue.
- Spec text and `CLAUDE.md` instruct future sessions, so a malicious edit is prompt injection. The maintainer's manual review is the defence, and no program stands in for it.
- The rule can't catch a fork's branch pushed under another name, and GitHub's merge button stays available on a personal repository, since a ruleset restricting updates would block `clm land` too. `clm ledger verify` reports a merge-button merge afterwards.
- Accepted contributions stay credited to their authors: `clm land` and rebasing keep commit authorship. Whether the badge shows "Merged" after a rebase pushed back to a fork is untested, and the fallback is closing with a link.
- Collaborators, signing (`docs/allowed_signers`) and `approval = "other"` are left for when a second person gets write access.
