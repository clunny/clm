# Architecture

The architectural constraints: what must always hold of this project, whatever is built. The architect owns this file.
An edit to it needs a human's review before it lands.
It is prepended to every context bundle, so keep each entry short.
Add the project's own as entries under `## Constraints` (format: `clm guide spec-format`). Each entry names the rules that enforce it under `#### Enforced by`, or says why no program can check it under `#### Untested`, or both; it is current when every rule it names is current. A test never cites an entry: it cites the rules that enforce it.
CLM's own carried constraints are not copied here: they reach this project from the `clm` it runs, and `clm guide constraints` prints them. A project replaces a `replaceable` one, or drops a `default` one, with a `#### Replaces` block in an entry of its own, which a human reviews; a `method` one can't be replaced.

## Constraints
