# Working in this repository

This repository is built with CLM. Read `ARCHITECTURE.md` first; its architectural constraints apply to all work here, and `CAPABILITIES.md` says what the project is for. Run `clm status` for the current state and `clm backlog` for what's next. The CLM guide explains the method: `clm guide` lists its pages and `clm guide <page>` prints one, from the `clm` you run.

- Spec and design edits go in `changes/<YYYYMMDD-slug>/deltas/` and reach `specs/` only through `clm apply`. Use `/clm:propose` to turn an intent into one spec branch, and `/clm:build` to build approved changes. `/clm:help` answers questions about working with CLM. These are the `propose`, `build` and `help` skills whatever the agent harness calls them.
- Requirement IDs are `<component-id>/<slug>`. Tests cite the ones they prove by containing the ID after `req` and a colon.
- Record significant choices as `docs/decisions/<name>.md`, one file each, and refer to architectural constraints, capabilities and decisions by name.
- Before claiming work is done, run the project's build and tests (`scripts/clm-check.sh`) and `clm check`.
- Never stack branches: every branch lands on `main` through `clm land`, and nothing else pushes `main`.
- Never run `clm approve` yourself: have the human run it, through the agent harness's confirmation or in their own terminal, only after they have just said yes to that change's intent in this conversation. Only a human approves spec changes.
- When CLM itself gets in the way (a check that misfires, a rule you must work around, a wrong or missing instruction), report it with `clm defect`.
- Fold small discoveries into the current change; record significant out-of-scope ones as `backlog/<slug>.md` items.

The rules that must hold are checked by `clm` and the CLM plugin's hooks, not by this file.
