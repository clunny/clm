#!/usr/bin/env bash
# Every check of this repository (specs/ci/SPEC.md): `clm.toml`'s `[land] check`, so `clm land`
# runs it on the merged tree, and then runs `clm check` itself. Run from the repo root.
set -euo pipefail

# Which tests run (ci/selected-tests). With `CLM_SELECTION` naming a file, as `clm land` sets
# it, only those its `file <path>` lines call for; unset, empty, missing or starting with an
# `all` line, every test. The file is read with builtins only, so selecting costs no build.
full=true
web=false
panel=false
plugin=false
# The capability packages selected by their own files (ci/web-checks); each also runs after the
# suite it follows.
cap_web=false
cap_panel=false
# Each selected `<package> <target flag>`, tab-separated, in selection order. A plain array,
# not an associative one, so the script runs on macOS's bash 3.2 too.
selected=()

# The package `crates/<dir>/Cargo.toml` names: the `name` of its `[package]` section.
package_name() {
  local line section=
  while IFS= read -r line || [ -n "$line" ]; do
    if [[ $line =~ ^\[([^]]*)\] ]]; then
      section=${BASH_REMATCH[1]}
    elif [ "$section" = package ] && [[ $line =~ ^name[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]]; then
      echo "${BASH_REMATCH[1]}"
      return
    fi
  done <"$1"
}

# Selects the `cargo test` target flag $2 of the package in `crates/$1`. A deleted crate
# selects nothing.
add() {
  local manifest=crates/$1/Cargo.toml package
  [ -f "$manifest" ] || return 0
  package=$(package_name "$manifest")
  [ -n "$package" ] || return 0
  selected+=("$package	$2")
}

select_file() {
  local path=$1 dir rest name
  case $path in
    web/*) web=true ;;
    plugin-panel/*) panel=true ;;
    capabilities/web/*) cap_web=true ;;
    capabilities/panel/*) cap_panel=true ;;
    plugin/*) plugin=true ;;
    crates/*/*)
      rest=${path#crates/}
      dir=${rest%%/*}
      rest=${rest#*/}
      case $rest in
        tests/*/*) add "$dir" --tests ;; # a helper module integration tests include
        tests/*.rs)
          name=${rest#tests/}
          add "$dir" "--test ${name%.rs}"
          ;;
        src/*)
          if [ -f "crates/$dir/src/lib.rs" ]; then add "$dir" --lib; fi
          if [ -f "crates/$dir/src/main.rs" ] || [ -d "crates/$dir/src/bin" ]; then
            add "$dir" --bins
          fi
          ;;
        *) add "$dir" --tests ;;
      esac
      ;;
  esac
}

selection=${CLM_SELECTION:-}
if [ -n "$selection" ] && [ -f "$selection" ]; then
  first=
  IFS= read -r first <"$selection" || true
  if [[ $first != all* ]]; then
    full=false
    while IFS= read -r line || [ -n "$line" ]; do
      if [[ $line == "file "* ]]; then select_file "${line#file }"; fi
    done <"$selection"
  fi
fi
if $full; then
  web=true
  panel=true
  plugin=true
fi

# `cargo test` runs `clm check`, detectors and all (`this_repository_is_clean`), so their tools
# go first (ci/detector-tools).
.github/scripts/detector-tools.sh

cargo fmt --all --check
cargo clippy --all-targets -- -D warnings

# The crates the `web` Worker links build for wasm (ci/web-checks). Before the tests, because
# they fail faster. Each `--no-default-features` crate builds on its own: built with a crate
# that depends on it with its default features, cargo would turn them back on.
cargo build --target wasm32-unknown-unknown -p clm-model -p clm-context
cargo build --target wasm32-unknown-unknown --no-default-features -p clm-check
cargo build --target wasm32-unknown-unknown --no-default-features -p clm-orchestrator
cargo build --target wasm32-unknown-unknown --no-default-features -p clm-view
# Without its `git` feature (ledger/ledger-builds-for-wasm).
cargo build --target wasm32-unknown-unknown --no-default-features -p clm-ledger
cargo build --target wasm32-unknown-unknown -p clm-web

if $full; then
  cargo test --all
else
  # One run per package, in package order, so a `--test` flag never meets a package without
  # that target. A selected run leaves out doctests: `--doc` can't join other targets.
  packages=$(for entry in ${selected[@]+"${selected[@]}"}; do echo "${entry%%	*}"; done | LC_ALL=C sort -u)
  for package in $packages; do
    flags=
    for entry in "${selected[@]}"; do
      [ "${entry%%	*}" = "$package" ] || continue
      case " $flags " in
        *" ${entry#*	} "*) ;;
        *) flags="$flags ${entry#*	}" ;;
      esac
    done
    # shellcheck disable=SC2086 # the flags split into words
    cargo test -p "$package" $flags
  done
fi

# A plugin directory as Claude Code loads it, with whatever `claude` this machine has. No
# `--strict` or `--profile`: only what the engine would refuse fails, not warnings or the plugin
# directory's listing checks. Stdin is closed, so an unattended land never waits on a prompt.
validate() {
  if command -v claude >/dev/null; then
    claude plugin validate "$1" </dev/null
  else
    echo "skipped claude plugin validate: claude is not on PATH" >&2
  fi
}

# The browser, in the current directory, the Chromium revision its `@playwright/test` pins
# (ci/browser-install). The install does nothing when that revision is already cached. Only CI
# adds `--with-deps`, which installs Chromium's system libraries with `sudo`: elsewhere a human
# installs them once per machine (`npx playwright install-deps chromium`), so `clm land` and a
# session never reach `sudo`.
browser_install() {
  if [ "${CI:-}" = true ]; then
    npx playwright install --with-deps chromium
  else
    npx playwright install chromium
  fi
}

# A capability package exists once `capability` has built it (capability-browser-packages).
has_package() {
  [ -f "capabilities/$1/package.json" ]
}

# `plugin-panel/`'s and `web/`'s tests, each followed by its capability package
# (ci/web-checks): the panel's first, since they're quick.
if $panel; then
  (cd plugin-panel && npm ci --ignore-scripts && npm test)
  # Its tests strip types and never load the module as the engine does (ci/panel-validate).
  validate plugin-panel
fi
if { $panel || $cap_panel; } && has_package panel; then
  (cd capabilities/panel && npm ci --ignore-scripts && npm test)
fi
# The `clm` plugin's manifest, hooks, skills, agents and command (ci/plugin-validate), which its
# tests read only as text.
if $plugin; then
  validate plugin
fi
if $web; then
  (
    cd web
    npm ci --ignore-scripts
    browser_install
    npm test
  )
fi
# `web/site/`'s package (`web.site`) follows `web/`'s suite, once it has a `package.json`
# (ci/web-checks).
if $web && [ -f web/site/package.json ]; then
  (cd web/site && npm ci --ignore-scripts && npm test)
fi
# `capabilities/web/`'s tests build the app with `web/`'s own dependencies, so `web/`'s `npm ci`
# goes first, once.
if { $web || $cap_web; } && has_package web; then
  $web || (cd web && npm ci --ignore-scripts)
  (
    cd capabilities/web
    npm ci --ignore-scripts
    browser_install
    # Under the load of other builds a test can miss its timeout once and pass on a rerun; the
    # package fails only when a test fails twice (ci/capability-web-retries).
    npm test -- --retries=1
  )
fi
