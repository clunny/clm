#!/usr/bin/env bash
# Writes `target/lcov.info`, the whole repository's coverage, which `clm health`'s `unexercised`
# signal reads (`clm.toml`'s `[health] lcov`; specs/ci/SPEC.md, ci/coverage-script). Run by hand,
# from the repository root, when that signal should be fresh: no land, check or workflow runs
# it, since it rebuilds every crate instrumented and runs the full suite.
set -euo pipefail

# The suite's `this_repository_is_clean` runs `clm check`, detectors and all.
.github/scripts/detector-tools.sh
rustup component add llvm-tools-preview
cargo install --locked --version 0.9.1 cargo-llvm-cov

mkdir -p target/coverage
root=$PWD
real=$(pwd -P)

# Copies lcov file $1 to $2 with this tree's root stripped from each absolute `SF:` path, as
# `$PWD` names it and as `pwd -P` does (a checkout may be reached through a symbolic link), so the
# tracefile reads the same from any checkout of the repository. `index` compares plainly, so a
# root with regex characters needs no escaping.
relative_to_root() {
  awk -v a="SF:$root/" -v b="SF:$real/" '
    index($0, a) == 1 { print "SF:" substr($0, length(a) + 1); next }
    index($0, b) == 1 { print "SF:" substr($0, length(b) + 1); next }
    { print }' "$1" >"$2"
}

# Builds in `target/llvm-cov-target/`, so the ordinary build stays warm. Its `SF:` paths are
# absolute under the root, which `relative_to_root` strips. `--no-rustc-wrapper` puts the instrumentation in `RUSTFLAGS`, which
# the suite's nested `wasm32-unknown-unknown` builds remove, so those aren't instrumented (with
# the wrapper they were, and failed on `profiler_builtins`). A failing test fails the run.
cargo llvm-cov --workspace --no-rustc-wrapper --lcov --output-path target/coverage/cargo-abs.info
relative_to_root target/coverage/cargo-abs.info target/coverage/cargo.info

# One npm directory's vitest suite, with `@vitest/coverage-v8` at the installed vitest's version
# added to `node_modules/` alone (`--no-save`), so neither `package.json` nor the lockfile
# changes. `--coverage.include` names the sources, so a file no test loads still counts, at 0.
# The tracefile's relative `SF:` paths get the directory as a prefix, its absolute ones are made
# relative to the root.
vitest_coverage() {
  local dir=$1 version
  shift
  (
    cd "$dir"
    npm ci --ignore-scripts
    version=$(npx vitest --version)
    version=${version%% *}
    version=${version#vitest/}
    if ! [[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+ ]]; then
      echo "coverage.sh: no vitest version in $dir" >&2
      exit 1
    fi
    npm install --no-save --ignore-scripts "@vitest/coverage-v8@$version"
    npx vitest run --coverage.enabled --coverage.provider=v8 --coverage.reporter=lcovonly \
      "--coverage.reportsDirectory=../target/coverage/$dir" "$@"
  )
  sed -e "/^SF:\//!s|^SF:|SF:$dir/|" "target/coverage/$dir/lcov.info" >"target/coverage/$dir-abs.info"
  relative_to_root "target/coverage/$dir-abs.info" "target/coverage/$dir.info"
}
vitest_coverage web --coverage.include='src/**'
vitest_coverage plugin-panel --coverage.include='src/**' --coverage.include='hooks/**/*.tsx'

# lcov is a sequence of records, so concatenating merges. Renamed into place, so a failure above
# leaves the previous tracefile as it was.
cat target/coverage/cargo.info target/coverage/web.info target/coverage/plugin-panel.info \
  >target/coverage/lcov.info
mv target/coverage/lcov.info target/lcov.info
