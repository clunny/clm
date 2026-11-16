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
fixture_dirs=
cleanup() {
  rm -rf "$work"
  for d in $fixture_dirs; do rm -rf "${root:?}/$d/clm-mutate-fixture"; done
}
trap cleanup EXIT
: >"$result"
nl='
'
tab=$(printf '\t')

# A tool that doesn't work leaves no verdict: the script stops, naming the tool and what it was
# judging, and writes nothing for the block, so its requirement stays due (`verify/mutation-pass`).
die() {
  echo "mutate.sh: $*" >&2
  exit 1
}

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
  # Each mutant's tests run under nextest, which only test-tools.sh installs (ci/test-tools).
  if ! cargo nextest --version >/dev/null 2>&1; then
    echo "mutate.sh: cargo-nextest is missing; install cargo-nextest first" >&2
    exit 1
  fi
  case $(cargo mutants --version 2>/dev/null || true) in
    *" $MUTANTS_VERSION") ;;
    *) cargo install --locked --version "$MUTANTS_VERSION" cargo-mutants ||
      die "cargo mutants could not be installed (requirement $id)" ;;
  esac
  rust_fixture
  rust_ready=true
}

# Judges a fixture with cargo-mutants before it judges anything: a function with a test that
# fails when it is changed and one that passes whatever it returns. The tool goes on only when it
# reports the first's mutants caught and the second's missed, so a tool that accepts nothing, or
# everything, can't be taken for a verdict. The crate lives in the temporary directory, as a git
# repository of its own whose tests pass only inside one, so a tool that leaves `.git` out of the
# copy it tests fails the fixture, as it would fail this repository's tests that ask git about it.
rust_fixture() {
  local fix=$work/fixture-rust kind out status
  mkdir -p "$fix/src"
  printf '[package]\nname = "clm-mutate-fixture"\nversion = "0.0.0"\nedition = "2021"\n' >"$fix/Cargo.toml"
  cat >"$fix/src/lib.rs" <<'EOF'
pub fn double(x: i32) -> i32 {
    x * 2
}

#[cfg(test)]
mod tests {
    use super::*;

    // The crate's own directory must be the top of a git repository: one found further up, as
    // when the copy sits under another checkout, doesn't count.
    fn in_git_repository() {
        let top = std::process::Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .unwrap();
        assert!(top.status.success(), "not in a git repository");
        let top = String::from_utf8(top.stdout).unwrap();
        let here = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap();
        assert_eq!(std::fs::canonicalize(top.trim()).unwrap(), here);
    }

    #[test]
    fn fixture_notices() {
        in_git_repository();
        assert_eq!(double(3), 6);
    }

    #[test]
    fn fixture_blind() {
        in_git_repository();
        let _ = double(3);
    }
}
EOF
  git -C "$fix" init -q || die "cargo mutants' fixture could not be made a git repository"
  for kind in notices blind; do
    out=$work/fixture-$kind
    rm -rf "$out"
    mkdir -p "$out"
    status=0
    (cd "$fix" && cargo mutants --test-tool nextest --copy-vcs true --output "$out" -- -E "test(~fixture_$kind)") ||
      status=$?
    case $status in 0 | 2 | 3) ;; *) die "cargo mutants failed on its fixture (status $status)" ;; esac
    caught=0
    missed=0
    count_outcomes "$out"
    case $kind in
      notices) [ "$caught" -gt 0 ] && [ "$missed" -eq 0 ] ;;
      blind) [ "$missed" -gt 0 ] && [ "$caught" -eq 0 ] ;;
    esac || die "cargo mutants judged its fixture wrongly: $caught caught and $missed missed for the $kind test"
  done
}

# A nextest filterset naming a test's package, its test target by binary ID and its name, and its
# package, from the plan's `test` line `<file>	<name>`. A file under `crates/<dir>/`:
# `tests/<x>.rs` is the target `<x>`, any other file under `tests/` all of them, `src/` the
# library and the binaries.
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
      filter="$filter & binary_id(=$package::${target%.rs})"
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
  rm -rf "$out"
  mkdir -p "$out"
  # `--no-tests=pass`: cargo-mutants runs its unmutated baseline in the mutated files' package,
  # where the filterset matches nothing when the citing tests are another package's.
  # `--copy-vcs true`: the copy it tests holds the checkout's `.git`, so a test that asks git
  # about this repository passes there as it does here.
  cargo mutants --test-tool nextest --copy-vcs true --output "$out" "${mutants_args[@]}" "$@" \
    -- --no-tests=pass -E "$filterset" || status=$?
  # 0 all caught, 2 some missed, 3 some timed out; anything else judged nothing: the tests fail
  # in the unmutated tree, or a filterset doesn't parse.
  case $status in
    0 | 2 | 3) count_outcomes "$out" ;;
    *) die "cargo mutants failed (status $status) for requirement $id" ;;
  esac
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
    cargo mutants --list "${mutants_args[@]}" >"$work/list" 2>/dev/null ||
      die "cargo mutants failed to list mutants for requirement $id"
    list=$(head -n "$limit" "$work/list")
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
  ) || die "StrykerJS could not be installed in $1 (requirement $id)"
  ts_fixture "$1"
  ts_ready="$ts_ready $1"
}

# Judges a fixture with StrykerJS in the npm directory $1, with that directory's own vitest and
# TypeScript: a source file whose first function a test notices a change to and whose second
# function no test does. It goes on only when the first function's mutants are caught and the
# second's missed. The files sit inside the directory, which a vitest configuration reaches, and
# are removed afterwards.
ts_fixture() {
  local dir=$1 fix=clm-mutate-fixture saved=("${ts_tests[@]+"${ts_tests[@]}"}")
  fixture_dirs="$fixture_dirs $dir"
  rm -rf "${root:?}/$dir/$fix"
  mkdir -p "$root/$dir/$fix"
  cat >"$root/$dir/$fix/fixture.ts" <<'EOF'
export const double = (x: number): number => x * 2;
export const triple = (x: number): number => x * 3;
EOF
  cat >"$root/$dir/$fix/fixture.test.ts" <<'EOF'
import { expect, test } from 'vitest';
import { double, triple } from './fixture';

test('fixture_notices', () => {
  expect(double(3)).toBe(6);
});

test('fixture_blind', () => {
  triple(3);
});
EOF
  ts_tests=("$dir/$fix/fixture.test.ts")
  caught=0
  missed=0
  run_stryker "$dir" f1 "$fix/fixture.ts:1-1" ||
    die "StrykerJS failed on its fixture in $dir"
  { [ "$caught" -gt 0 ] && [ "$missed" -eq 0 ]; } ||
    die "StrykerJS judged its fixture wrongly in $dir: $caught caught and $missed missed for the test that notices"
  caught=0
  missed=0
  run_stryker "$dir" f2 "$fix/fixture.ts:2-2" ||
    die "StrykerJS failed on its fixture in $dir"
  { [ "$missed" -gt 0 ] && [ "$caught" -eq 0 ]; } ||
    die "StrykerJS judged its fixture wrongly in $dir: $caught caught and $missed missed for the test that doesn't"
  rm -rf "${root:?}/$dir/$fix"
  ts_tests=("${saved[@]+"${saved[@]}"}")
}

# The `<path>:<first>-<last>` ranges of the diff's added lines in the files under `$1/`, paths
# relative to it, which StrykerJS mutates.
diff_ranges() {
  local dir=$1 diff=$2 wanted
  shift 2
  wanted=$(printf '%s\n' "$@")
  awk -v dir="$dir/" -v wanted="$wanted" '
    BEGIN { n = split(wanted, w, "\n"); for (i = 1; i <= n; i++) ok[w[i]] = 1 }
    /^\+\+\+ / { path = ""; p = $2; sub(/^b\//, "", p)
      if (index(p, dir) == 1 && (substr(p, length(dir) + 1) in ok)) path = substr(p, length(dir) + 1); next }
    /^@@/ && path != "" {
      split($3, a, ","); first = substr(a[1], 2) + 0; count = (a[2] == "" ? 1 : a[2] + 0)
      if (count > 0) print path ":" first "-" (first + count - 1)
    }' "$diff"
}

# Runs StrykerJS in the npm directory $1 on the `mutate` ranges after $2, the name of its report,
# with the vitest tests `ts_tests` alone; adds its killed and survived mutants to `caught` and
# `missed`. StrykerJS takes a runner's options from no command line, so it is given them in a
# configuration file under the directory's `node_modules/`: the test runner, the vitest
# configuration that names those tests, the reporter and the ranges. It sets no
# `thresholds.break`, so StrykerJS's exit status never depends on a score. Returns 1 when StrykerJS
# exits non-zero or writes no report: it judged nothing. `tsconfigFile` names a file that doesn't
# exist: StrykerJS 10.1.1 reads a `tsconfig.json` it finds with `ts.parseConfigFileTextToJson`,
# which the TypeScript 7 of `web/` and `plugin-panel/` lacks, to rewrite the paths in it for its
# sandbox. Nothing here needs that rewrite: the run mutates and runs vitest, with no type checker.
run_stryker() {
  local dir=$1 report=$2 base vcfg scfg status=0 includes= ranges= f m json k s
  shift 2
  rm -rf "${work:?}/$report"
  mkdir -p "$work/$report"
  base=
  for f in vite.config.ts vite.config.mts vite.config.js vitest.config.ts vitest.config.mts vitest.config.js; do
    if [ -f "$root/$dir/$f" ]; then
      base="import base from '$root/$dir/$f';"
      break
    fi
  done
  for f in ${ts_tests[@]+"${ts_tests[@]}"}; do includes="$includes'${f#"$dir"/}',"; done
  mkdir -p "$root/$dir/node_modules/.clm-mutate"
  vcfg=$root/$dir/node_modules/.clm-mutate/vitest.config.mts
  scfg=$root/$dir/node_modules/.clm-mutate/stryker.config.json
  {
    echo "$base"
    [ -n "$base" ] || echo "const base: any = {};"
    echo "export default { ...base, test: { ...base.test, include: [$includes] } };"
  } >"$vcfg"
  for m in "$@"; do ranges="$ranges\"$m\","; done
  cat >"$scfg" <<EOF
{
  "testRunner": "vitest",
  "vitest": { "configFile": "$vcfg" },
  "reporters": ["json"],
  "jsonReporter": { "fileName": "$work/$report/mutation.json" },
  "tsconfigFile": "$root/$dir/node_modules/.clm-mutate/no-tsconfig.json",
  "concurrency": 2,
  "mutate": [${ranges%,}]
}
EOF
  (
    cd "$root/$dir"
    npx stryker run "$scfg"
  ) || status=$?
  rm -f "$vcfg" "$scfg"
  json=$work/$report/mutation.json
  if [ "$status" -ne 0 ] || ! [ -f "$json" ]; then return 1; fi
  k=$({ grep -Eo '"status": ?"(Killed|Timeout)"' "$json" || true; } | wc -l | tr -d ' ')
  s=$({ grep -Eo '"status": ?"(Survived|NoCoverage)"' "$json" || true; } | wc -l | tr -d ' ')
  caught=$((caught + k))
  missed=$((missed + s))
}

judge_ts() {
  local f dir= files=() ranges=() capped=() t file
  ts_tests=()
  for f in ${code[@]+"${code[@]}"}; do
    case $f in
      web/src/*.ts | web/src/*.tsx) [ -n "$dir" ] || dir=web ;;
      plugin-panel/*.ts | plugin-panel/*.tsx) [ -n "$dir" ] || dir=plugin-panel ;;
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
    done < <(diff_ranges "$dir" "$diff" "${files[@]}")
    if [ "${#ranges[@]}" -gt 0 ]; then
      run_stryker "$dir" s1 "${ranges[@]}" || die "StrykerJS failed for requirement $id"
    fi
  fi
  if [ "$caught" -eq 0 ]; then
    # The further mutants: the first `limit` lines of each file, whose mutants StrykerJS can't
    # count beforehand.
    for f in ${files[@]+"${files[@]}"}; do capped+=("$f:1-$limit"); done
    run_stryker "$dir" s2 "${capped[@]}" || die "StrykerJS failed for requirement $id"
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

while IFS= read -r -u 3 line || [ -n "$line" ]; do
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
done 3<"$plan"
finish_block

.github/scripts/coverage.sh
if [ -n "${CLM_LAND_DIR:-}" ]; then
  cp target/lcov.info "$CLM_LAND_DIR/lcov.info"
fi
