---
title: "A change other components must follow is built in steps, each in its own component's files; no build makes forced fixes elsewhere"
date: 2026-11-12
status: accepted
---

`path-ownership` let a build edit other components for the fixes its change forced: compile-only fixes for a removal (2026-11-03), then for an added field (`forced-fixes-cover-additions`), then the arm for an added variant, then the split move (`nested-splits`, 2026-11-12). Each covered only the case that came up, so the next kind of tie stalled a build again, and the constraint every builder reads grew a clause each time (`backlog/compile-coupled-changes-across-components`). Every one of them is a change that another component's code must follow, and `expand-contract` already says how such a change lands without breaking `main`. It just wasn't used for them, because `path-ownership-by-prompt` judged that turning each removal into several builds cost too much.

That cost has changed. `#### Needs` orders builds across components (`needs-alone-order`) and the build loop runs them unattended (`build-loop-in-clm`), so the extra builds are machine time, not the human's. So every change another component's code or tests must follow is written as steps, each a requirement of the component whose files it changes:

- **Removal or rename:** the provider expands (the new beside the old), each user moves in a requirement of its own that needs the expansion, and the provider contracts, needing every move (`rename-adds-then-removes`, `contract-removal-needs`, whose forced-fix exception is withdrawn).
- **Added variant a user matches exhaustively:** the user first adds a default arm, marked `clm:remove-by` the change, in a requirement of its own; the widening needs it; and the user's requirement that handles the new value needs the widening and removes the arm (`no-dead-code`).
- **Added field a user builds by struct literal:** the provider first offers a constructor or defaults, the user moves to them, and the field comes last.
- **Split:** the child's requirements build its code in its own files, and the parent gets a requirement, needing them, that deletes its copy and reaches the code through the child.
- **Optional user of a required provider:** a required component can't `Need` an optional one (`optional-nodes`), so the contraction waits as a contract step (`contract-step-deferred`), as it already did for removals.

The builder check, which already searches other components' code for uses of what a change removes or widens, writes the steps. A tie it misses makes the build stop with a `question` item, and a small spec change adds the missing step. That's slower than a forced fix, but it's visible and leaves no build in another component's files.

Two ties don't sequence, because `main` fails between the halves. **A capability test and the behaviour it observes** stay a permanent exception: a capability test observes the whole product, so it is atomic with any behaviour at the boundary (`capability-tests-follow-rules`). **A template `clm init` writes and its `upgrade` migration** stay an exception until the line between the components moves so that both live in one component (`backlog/a-template-and-its-migration-live-in-different-components`).

Rejected:
- **A `#### Touches` block per requirement, naming the other components its build edits and claiming them with it.** It makes the reaching-out declared and safe from parallel builds, but builds still edit files they don't own.
- **Keeping the forced-fix exceptions and stating them more generally.** Generality makes the rule shorter but no stricter, and the next kind of tie still arrives as a judgement in a build.

Supersedes `forced-fixes-cover-additions`. The shared wiring a root's Design names stays a permanent, named exception of `path-ownership`: here the root's dispatch files (`crates/clm/src/main.rs`, `Cargo.toml`, `Cargo.lock`) and `clm.toml`'s detector allowances, which a build edits only to add or change its own component's entry.
