Builds' checks now overlap: your tests must pass side by side
A build's `clm land --check-only` no longer waits for the other checks of the clone, so it runs at the same time as other builds' checks, the tests builds run by hand and a land's check. Lands' own checks still run one at a time.

So your project's tests and check command must pass while other runs of them overlap on one host (`tests-run-concurrently`, which `clm guide constraints` states): no fixed port, no fixed path outside the checkout and the system's temporary directory, and no server or database two runs share. A port or path a run must know in advance, such as the port of a server the check starts, is keyed on `CLM_SLOT`, the number CLM gives each build and its checks, and 0 when unset.

To check your project:

- Look through the check script (`[land] check` in `clm.toml` names it) and the tests for literal ports and `localhost` or `127.0.0.1` addresses, absolute paths outside the checkout and the temporary directory, and servers or databases the check starts or reaches at a fixed address.
- Run the check in two checkouts at once with different `CLM_SLOT` values. Both must pass.
- Record each collision you find as a `bug` backlog item naming `tests-run-concurrently`.

Meanwhile:

- A project that keeps its runs apart another way, such as a container per run, replaces the constraint with an entry of its own `ARCHITECTURE.md` holding `#### Replaces` naming `tests-run-concurrently` and keeping its guarantee. A human reviews that change.
- Until your tests are fixed, `max_builds = 1` in the clone's `clm.local.toml` makes overlapping runs rarer, though not impossible.

Taking this note edits no file and leaves nothing to decline: `clm upgrade --apply` stamps `upgraded`, which marks it taken.
