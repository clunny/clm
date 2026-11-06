# Architecture

The architectural constraints: what must always hold of this project, whatever is built. The architect owns this file.
An edit to it needs a human's review before it lands.
It is prepended to every context bundle, so keep each entry short.
Add the project's own as entries under `## Constraints` (format: `clm guide spec-format`). Each entry names the rules that enforce it under `#### Enforced by`, or says why no program can check it under `#### Untested`, or both; it is current when every rule it names is current. A test never cites an entry: it cites the rules that enforce it.

## Enforced by clm

`clm` enforces these in every repository, and the rules its own entries name prove them, so they are not entries here. Refer to each by its name.

- **`ai-writes-code`**: AI SHALL write all code. Humans do not author or review code, and code changes only through the build flow.
- **`humans-review-intent`**: Humans SHALL review intent, not code. A spec branch needs a human's approval, given with `clm approve` before it lands, exactly when it edits `ARCHITECTURE.md`, `CAPABILITIES.md` or the trusted keys in `docs/allowed_signers`, changes `clm.toml`'s `approval` setting, or adds or edits a change that declares `human_burdens` or whose `change.md` can't be read, which fails closed. Every other change lands on checks, and nothing is reviewed after it lands.
- **`deterministic-enforcement`**: Every rule that must hold SHALL be checked by a program (the CLI, a hook or the land checks), never by prose or model judgment alone. Model judgment may advise but never gates.
- **`proven-by-tests`**: A requirement SHALL count as implemented only when a passing test cites its ID at its current version.
- **`no-silent-drift`**: A divergence between spec and code that CLM can detect SHALL block: `clm check` fails a marker that names no requirement and a delta whose base has moved, and `clm land` fails a branch whose applied `specs/` don't match their sources. A requirement not yet built is pending, not drift (`node-reconciliation`).
- **`git-is-truth`**: Git SHALL be the source of truth and the only channel. All shared state (specs, changes, claims, approvals, checks and lands) is committed files or branches, and no server or database owns any. A service may keep a cache derived only from git (and from a host's own data, such as a pull request's URL) that can be deleted and rebuilt with nothing lost; no check, gate or approval reads anything but git.
- **`hash-based-staleness`**: Anything derived from the spec SHALL record the content hashes of the elements it was based on, and on a mismatch it is stale: a delta must be rebased before use, and a requirement cited at an old version is behind and is built again.
- **`regenerate-dont-merge`**: Derived files SHALL be regenerated, never merged. Indexes, registries and the applied `specs/` are rebuilt from their sources.
- **`ancestor-scoped-context`**: An agent working on a component SHALL receive this file, the ancestors (root to component), the one-hop contracts of what it needs and the capabilities its rules serve, and nothing else.
- **`spec-is-current`**: `specs/`, `ARCHITECTURE.md` and `CAPABILITIES.md` SHALL describe only the system as it is now. Removed requirements are deleted, not marked deprecated. History lives in `changes/archive/` and git, and future intent in `backlog/`.
- **`fold-small-defer-large`**: Small work discovered during a change SHALL be folded into it: anything that needs no spec change and stays within the change's components. Significant out-of-scope work SHALL be recorded as a `backlog/` item instead: anything that needs a new or modified requirement, touches other components, or would noticeably grow the change.
- **`no-dead-code`**: Code SHALL exist only to serve a current requirement. A change that removes or replaces behaviour removes the code, tests and dependencies that served it, in the same change or a later one it names with a `clm:remove-by <change-id>` marker. Each project names its dead-code detectors in `clm.toml`, and `clm check` runs them.

## Constraints
