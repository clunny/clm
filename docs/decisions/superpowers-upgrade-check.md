---
title: "A scheduled local command moves the superpowers pin when the eval's skills arm passes"
date: 2026-10-27
status: superseded
superseded_by: cut-evals
---

Refines `unattended-superpowers`, whose rule stays: a new superpowers version is adopted only after the `superpowers-unattended` cases pass with it. Until now someone had to notice a release, run the cases and move the pin by hand, so improvements waited for whoever happened to look.

`clm superpowers upgrade` (`orchestrator.run`) does it:
1. It compares the newest `vX.Y.Z` tag of the superpowers repository with the pin, and stops when there is nothing newer, which costs nothing.
2. It runs the eval with the new release and judges arm D, the arm that matches `clm run`'s prompt, by fixed criteria read from its `stream-json` log and its repository: every named skill used in order, no execution skill, a failing `cargo test` before a passing one, no question to a human and a `success` result, and the plan file and a commit present. A program judges, so the upgrade gate is deterministic (`deterministic-enforcement`); the model's run is the thing being judged, never the judge.
3. When arm D passes, it claims `impl/orchestrator.run` and pushes one commit that changes `crates/clm-run/superpowers.pin`. `clm land` lands it after the full check command, like any code.

**Where it runs: on this machine, not on GitHub Actions.** The item suggested a scheduled workflow. `land-locally` took every check and land off hosted CI, `ci`'s `release` requires `release.yml` to be the only workflow, and `host-ci-for-releases` keeps Actions for the macOS builds alone. A workflow would also need a Claude API key as a repository secret, and it could only push a branch, since only `clm land` pushes `main`. On this machine the eval's `claude -p` sessions use the human's own login, and the same job can run `clm land`. A `/schedule`d cloud routine was also rejected: it runs elsewhere, needs its own toolchain, Claude login and clone, and still couldn't run the land here. The human schedules the command with the machine's own scheduler (cron or a systemd timer), a step only they can take, recorded as a `human` backlog item.

**The pin is a data file.** `crates/clm-run/superpowers.pin` holds `<tag> <commit>`, read at compile time, so the program writes one line instead of editing Rust. Moving it is a mechanical edit made and checked by a program, as `release-by-dispatch`'s version bump is: no code is authored outside the implement flow (`ai-writes-code`), and the deterministic checks gate it like any code (`humans-review-intent`).

**The claim.** `orchestrator.run` is usually converged, and `clm claim` refuses a converged node, while `claim --fix` needs a `bug` item, which a new release isn't. So `orchestrator` gains `claim_upkeep`, a library claim with `claim-fix`'s rules and no item, offered by no command. A live build of the node refuses it, and the next scheduled run tries again.

**Cost.** One eval run per new release, about $1 for all five arms; a failed release is recorded in `.clm/evals/superpowers-<tag>/verdict` and not run again unless `--retry`, so a weekly schedule doesn't pay for the same failure every week. Only arm D decides; the others are kept for the record, since `run.sh` runs them together.

Rejected:
- Judging with `analyze.py` or with a model reading the logs: the first prints a summary for a human and decides nothing, and the second would make a model the gate.
- Filing a backlog item for a human to move the pin: it brings back the watching the item set out to remove.
- Running only arm D: `run.sh` runs all arms in parallel and has no arm filter, and changing the eval belongs to `plugin`; the extra arms cost cents.

`continuous-evals` remains the general form: running the cases whenever skills, hooks or the prompt change.
