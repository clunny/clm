#!/usr/bin/env bash
# The mutation pass's command (`clm.toml`'s `[verify] mutate`; specs/ci/SPEC.md,
# ci/mutate-script): run from the repository root with `CLM_MUTATE` naming the plan `clm verify`
# wrote and `CLM_MUTATE_RESULT` the file to write a verdict line per requirement to:
# `<id> killed`, `<id> survived <n>` or `<id> not-judged`. No land, check or workflow runs it.
set -euo pipefail

# The newest releases at build time.
MUTANTS_VERSION=27.1.0
STRYKER_VERSION=10.1.1

plan=${CLM_MUTATE:?CLM_MUTATE names the plan}
result=${CLM_MUTATE_RESULT:?CLM_MUTATE_RESULT names the result file}
root=$PWD
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
: >"$result"
nl='
'
tab=$(printf '\t')

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

lines_in() { if [ -f "$1" ]; then wc -l <"$1" | tr -d ' '; else echo 0; fi; }

# --- Rust: cargo-mutants, tests chosen as ci-check.sh maps a file to a package and a target ---

rust_ready=false
rust_tools() {
  if $rust_ready; then return; fi
  case $(cargo mutants --version 2>/dev/null || true) in
    *" $MUTANTS_VERSION") ;;
    *) cargo install --locked --version "$MUTANTS_VERSION" cargo-mutants ;;
  esac
  rust_ready=true
}

# A nextest filterset naming a test's package, target and name, and its package, from the plan's
# `test` line `<file>	<name>`. A file under `crates/<dir>/`: `tests/<x>.rs` is the target `<x>`,
# any other file under `tests/` all of them, `src/` the library and the binaries.
rust_test_filter() {
  local file=$1 name=$2 dir rest package target filter
  rest=${file#crates/}
  dir=${rest%%/*}
  rest=${rest#*/}
  [ -f "crates/$dir/Cargo.toml" ] || return 0
  package=$(package_name "crates/$dir/Cargo.toml")
  [ -n "$package" ] || return 0
  filter="package(=$package)"
  case $rest in
    tests/*/*) filter="$filter & kind(test)" ;;
    tests/*.rs)
      target=${rest#tests/}
      filter="$filter & binary(=$package::${target%.rs})"
      ;;
    src/*) filter="$filter & (kind(lib) | kind(bin))" ;;
    *) filter="$filter & kind(test)" ;;
  esac
  if [ -n "$name" ]; then filter="$filter & test(~$name)"; fi
  printf '%s\t%s\n' "$package" "($filter)"
}

# Counts of one cargo-mutants output directory, added to `caught` and `missed`.
caught=0
missed=0
count_outcomes() {
  local out=$1/mutants.out
  caught=$((caught + $(lines_in "$out/caught.txt") + $(lines_in "$out/timeout.txt")))
  missed=$((missed + $(lines_in "$out/missed.txt")))
}

# Runs cargo-mutants once, with the given selection arguments, in output directory $1.
run_mutants() {
  local out=$1 status=0
  shift
  mkdir -p "$out"
  cargo mutants --test-tool nextest --output "$out" "${mutants_args[@]}" "$@" \
    -- -E "$filterset" || status=$?
  # 0 all caught, 2 some missed, 3 some timed out; anything else judged nothing.
  case $status in 0 | 2 | 3) count_outcomes "$out" ;; esac
}

# Judges the Rust files of a block. Sets `verdict` to killed, survived <n> or not-judged.
judge_rust() {
  local f t file name package filter packages= filters= p out list line
  local files=() n=0 re_args=()
  mutants_args=()
  for f in ${code[@]+"${code[@]}"}; do
    case $f in crates/*.rs) files+=("$f") ;; esac
  done
  for t in ${tests[@]+"${tests[@]}"}; do
    file=${t%%"$tab"*}
    name=
    case $t in *"$tab"*) name=${t#*"$tab"} ;; esac
    case $file in crates/*.rs) ;; *) continue ;; esac
    out=$(rust_test_filter "$file" "$name")
    [ -n "$out" ] || continue
    package=${out%%"$tab"*}
    filter=${out#*"$tab"}
    case " $packages " in *" $package "*) ;; *) packages="$packages $package" ;; esac
    if [ -n "$filters" ]; then filters="$filters | $filter"; else filters=$filter; fi
  done
  if [ "${#files[@]}" -eq 0 ] || [ -z "$filters" ]; then
    verdict=not-judged
    return
  fi
  rust_tools
  filterset=$filters
  for p in $packages; do mutants_args+=(--test-package "$p"); done
  for f in ${files[@]+"${files[@]}"}; do mutants_args+=(--file "$f"); done
  caught=0
  missed=0
  n=$((n + 1))
  if [ -n "$diff" ]; then
    run_mutants "$work/m$n" --in-diff "$diff"
  fi
  if [ "$caught" -eq 0 ]; then
    # The further mutants: the first `limit` of the files' own, chosen by name.
    n=$((n + 1))
    mkdir -p "$work/m$n"
    list=$(cargo mutants --list "${mutants_args[@]}" 2>/dev/null | head -n "$limit" || true)
    re_args=()
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      re_args+=(--re "^$(printf '%s' "$line" | sed -e 's/[][\.*^$(){}?+|/]/\\&/g')\$")
    done <<EOF
$list
EOF
    if [ "${#re_args[@]}" -gt 0 ]; then
      run_mutants "$work/m$n" "${re_args[@]}"
    fi
  fi
  if [ "$caught" -gt 0 ]; then
    verdict=killed
  elif [ "$missed" -gt 0 ]; then
    verdict="survived $missed"
  else
    verdict=not-judged
  fi
}

# --- TypeScript: StrykerJS with its vitest runner, in the directory of the files ---

ts_ready=
ts_tools() {
  case " $ts_ready " in *" $1 "*) return ;; esac
  (
    cd "$root/$1"
    npm ci --ignore-scripts
    # Added to `node_modules/` alone, so no `package.json` or lockfile changes.
    npm install --no-save --ignore-scripts \
      "@stryker-mutator/core@$STRYKER_VERSION" "@stryker-mutator/vitest-runner@$STRYKER_VERSION"
  )
  ts_ready="$ts_ready $1"
}

# The `<path>:<first>-<last>` ranges of the diff's added lines in the files under `$1/`, paths
# relative to it, which StrykerJS mutates.
diff_ranges() {
  awk -v dir="$1/" '
    /^\+\+\+ / { path = ""; p = $2; sub(/^b\//, "", p)
      if (index(p, dir) == 1) path = substr(p, length(dir) + 1); next }
    /^@@/ && path != "" {
      split($3, a, ","); first = substr(a[1], 2) + 0; count = (a[2] == "" ? 1 : a[2] + 0)
      if (count > 0) print path ":" first "-" (first + count - 1)
    }' "$2"
}

# Runs StrykerJS in $1 on the `--mutate` values after it, with the test files `tests`; adds its
# killed and survived mutants to `caught` and `missed`.
run_stryker() {
  local dir=$1 report=$2 base cfg status=0 args=() m
  shift 2
  mkdir -p "$work/$report"
  base=
  for f in vite.config.ts vite.config.mts vite.config.js vitest.config.ts vitest.config.mts vitest.config.js; do
    if [ -f "$root/$dir/$f" ]; then
      base="import base from '$root/$dir/$f';"
      break
    fi
  done
  includes=
  for f in ${ts_tests[@]+"${ts_tests[@]}"}; do includes="$includes'${f#"$dir"/}',"; done
  mkdir -p "$root/$dir/node_modules/.clm-mutate"
  cfg=$root/$dir/node_modules/.clm-mutate/vitest.config.mts
  {
    echo "$base"
    [ -n "$base" ] || echo "const base: any = {};"
    echo "export default { ...base, root: '$root/$dir', test: { ...base.test, include: [$includes] } };"
  } >"$cfg"
  for m in "$@"; do args+=(--mutate "$m"); done
  (
    cd "$root/$dir"
    npx stryker run --testRunner vitest --vitest.configFile "$cfg" --reporters json \
      --jsonReporter.fileName "$work/$report/mutation.json" --concurrency 2 "${args[@]}"
  ) || status=$?
  rm -f "$cfg"
  if [ -f "$work/$report/mutation.json" ]; then
    local json=$work/$report/mutation.json k s
    k=$({ grep -Eo '"status": ?"(Killed|Timeout)"' "$json" || true; } | wc -l | tr -d ' ')
    s=$({ grep -Eo '"status": ?"(Survived|NoCoverage)"' "$json" || true; } | wc -l | tr -d ' ')
    caught=$((caught + k))
    missed=$((missed + s))
  fi
}

judge_ts() {
  local f dir= files=() ranges=() capped=() t file
  ts_tests=()
  for f in ${code[@]+"${code[@]}"}; do
    case $f in
      web/src/*.ts | web/src/*.tsx | web/src/*/*) [ -n "$dir" ] || dir=web ;;
      plugin-panel/*) [ -n "$dir" ] || dir=plugin-panel ;;
      *) continue ;;
    esac
    case $f in "$dir"/*) files+=("${f#"$dir"/}") ;; esac
  done
  for t in ${tests[@]+"${tests[@]}"}; do
    file=${t%%"$tab"*}
    # A vitest file of the directory; Playwright's specs are judged by no tool here.
    case $file in "$dir"/e2e/* | capabilities/*) continue ;; "$dir"/*) ts_tests+=("$file") ;; esac
  done
  if [ -z "$dir" ] || [ "${#files[@]}" -eq 0 ] || [ "${#ts_tests[@]}" -eq 0 ]; then
    verdict=not-judged
    return
  fi
  ts_tools "$dir"
  caught=0
  missed=0
  if [ -n "$diff" ]; then
    while IFS= read -r r; do
      [ -z "$r" ] || ranges+=("$r")
    done < <(diff_ranges "$dir" "$diff")
    if [ "${#ranges[@]}" -gt 0 ]; then run_stryker "$dir" s1 "${ranges[@]}"; fi
  fi
  if [ "$caught" -eq 0 ]; then
    # The further mutants: the first `limit` lines of each file, whose mutants StrykerJS can't
    # count beforehand.
    for f in ${files[@]+"${files[@]}"}; do capped+=("$f:1-$limit"); done
    run_stryker "$dir" s2 "${capped[@]}"
  fi
  if [ "$caught" -gt 0 ]; then
    verdict=killed
  elif [ "$missed" -gt 0 ]; then
    verdict="survived $missed"
  else
    verdict=not-judged
  fi
}

# --- the plan ---

id=
code=()
tests=()
diff=
limit=20

# Writes the verdict of the block read so far.
finish_block() {
  local rust_v ts_v v total x
  [ -n "$id" ] || return 0
  verdict=not-judged
  rust_v=not-judged
  ts_v=not-judged
  judge_rust
  rust_v=$verdict
  judge_ts
  ts_v=$verdict
  if [ "$rust_v" = killed ] || [ "$ts_v" = killed ]; then
    v=killed
  else
    total=0
    for x in "$rust_v" "$ts_v"; do
      case $x in "survived "*) total=$((total + ${x#survived })) ;; esac
    done
    if [ "$total" -gt 0 ]; then v="survived $total"; else v=not-judged; fi
  fi
  echo "$id $v" >>"$result"
}

while IFS= read -r line || [ -n "$line" ]; do
  case $line in
    "requirement "*)
      finish_block
      id=${line#requirement }
      code=()
      tests=()
      diff=
      limit=20
      ;;
    "code "*) code+=("${line#code }") ;;
    "diff "*) diff=${line#diff } ;;
    "test "*) tests+=("${line#test }") ;;
    "limit "*) limit=${line#limit } ;;
  esac
done <"$plan"
finish_block

.github/scripts/coverage.sh
if [ -n "${CLM_LAND_DIR:-}" ]; then
  cp target/lcov.info "$CLM_LAND_DIR/lcov.info"
fi
