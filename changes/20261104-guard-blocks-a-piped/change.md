---
status: proposed
resolves:
  - guard-blocks-a-piped-clm-hashes-land-as-a-piped-land
---

# Guard reads a cargo run's land after its `--`

## Intent

`cargo run -q -p clm -- hashes land | grep x` runs `clm hashes` with the component `land` and lands nothing, yet `clm hook guard` refuses it under `guard-refuses-piped-land`. A `cargo run` that has a `--` hands `clm` the tokens after it, so the guard should count such a segment as running `clm land` only when `land` is the first of them, the same test it applies to a plain `clm` call.
