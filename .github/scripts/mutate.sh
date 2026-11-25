#!/usr/bin/env bash
# The mutation pass's command (`clm.toml`'s `[verify] mutate`; specs/ci/SPEC.md,
# ci/mutate-script): run from the repository root with `CLM_MUTATE` naming the plan `clm verify`
# wrote and `CLM_MUTATE_RESULT` the file to append a verdict line per requirement to, as soon as
# it is judged: `<id> killed`, `<id> survived <n>` or `<id> not-judged`. It judges up to
# `CLM_MUTATE_JOBS` blocks at once. No land, check or workflow runs it.
set -euo pipefail

# The newest releases at build time.
MUTANTS_VERSION=27.1.0
STRYKER_VERSION=10.1.1
# The version `coverage.sh` pins, which finds the lines a block's tests run.
LLVM_COV_VERSION=0.9.1

plan=${CLM_MUTATE:?CLM_MUTATE names the plan}
result=${CLM_MUTATE_RESULT:?CLM_MUTATE_RESULT names the result file}
root=$PWD
# Every git command here, and in the trees the tool tests, asks about the directory it runs in:
# a git directory or index inherited from a caller would point them at another repository.
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR GIT_OBJECT_DIRECTORY
work=$(mktemp -d)
# The pass's own temporary directory, which each block's `work` lies under.
pass_work=$work
fixture_dirs=
cleanup() {
  rm -rf "$work"
  # The fixtures, and what StrykerJS's runs leave in each npm directory when a pass is ended.
  for d in $fixture_dirs; do
    rm -rf "${root:?}/$d/clm-mutate-fixture" "${root:?}/$d/node_modules/.clm-mutate" \
      "${root:?}/$d"/.stryker-tmp-clm-*
  done
}
trap cleanup EXIT

# Every process this script started, at any depth: the blocks, and the tools they run; not the
# subshell that runs this function, nor the `ps` and `awk` it starts.
descendants() {
  ps -A -o pid= -o ppid= | awk -v root=$$ -v self="$BASHPID" '
    { parent[$1] = $2 }
    END {
      for (p in parent) {
        if (p == self || parent[p] == self) continue
        q = p
        while (q != root && (q in parent) && q > 1) q = parent[q]
        if (q == root && p != root) print p
      }
    }'
}
# Ended, as `clm verify` ends a pass at its limit or on a stop request, or by a person's Ctrl-C:
# the blocks running and whatever they run are ended, and gone, for at most ten seconds, before
# the temporary directory is removed and the script exits. A process started while they are
# signalled is found and signalled the next time round. The lines already written stay
# (`verify/mutation-pass`).
on_term() {
  local pids i=0
  trap - TERM INT
  while pids=$(descendants || true) && [ -n "$pids" ] && [ "$i" -lt 100 ]; do
    # shellcheck disable=SC2086 # one pid per word
    kill -TERM $pids 2>/dev/null || true
    sleep 0.1
    i=$((i + 1))
  done
  wait 2>/dev/null || true
  exit 143
}
trap on_term TERM INT

# The blocks it may judge at once, and the slot and build directory of each job number.
jobs_max=${CLM_MUTATE_JOBS:-1}
case $jobs_max in '' | *[!0-9]*) jobs_max=1 ;; esac
jobs_max=$((10#$jobs_max))
[ "$jobs_max" -gt 0 ] || jobs_max=1
slot_base=${CLM_SLOT:-0}
case $slot_base in *[!0-9]*)
  echo "mutate.sh: CLM_SLOT is not a whole number: $slot_base" >&2
  exit 1
  ;;
esac
slot_base=$((10#$slot_base))
target_base=${CARGO_TARGET_DIR:-target}
case $target_base in /*) ;; *) target_base=$root/$target_base ;; esac
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
# The tree `cargo mutants` runs in for every block, made once before the first (`make_tree`).
tree=$work/tree
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
  # Installed as coverage.sh installs it; `llvm-tools-preview` is a no-op once there.
  case $(cargo llvm-cov --version 2>/dev/null || true) in
    *" $LLVM_COV_VERSION") ;;
    *) { rustup component add llvm-tools-preview &&
      cargo install --locked --version "$LLVM_COV_VERSION" cargo-llvm-cov; } ||
      die "cargo llvm-cov could not be installed (requirement $id)" ;;
  esac
  make_tree "$root" "$tree" || die "cargo mutants' tree could not be made from the checkout (requirement $id)"
  rust_ready=true
}

# A `<file>	<line>	<count>` line for each line the lcov reports $@ give a count above 0, paths
# relative to the tree, the count the sum of those its `DA:` lines give it in them all. Their
# `SF:` paths are absolute under the tree, as `$tree` names it or as its real path.
lcov_counts() {
  local real
  real=$(cd "$tree" && pwd -P)
  a="SF:$tree/" b="SF:$real/" awk -F'[:,]' '
    BEGIN { a = ENVIRON["a"]; b = ENVIRON["b"] }
    index($0, a) == 1 { file = substr($0, length(a) + 1); next }
    index($0, b) == 1 { file = substr($0, length(b) + 1); next }
    /^SF:/ { file = substr($0, 4); next }
    /^DA:/ { n[file "\t" $2] += $3 }
    END { for (k in n) if (n[k] > 0) print k "\t" n[k] }' "$@" | sort
}

# Writes `$work/covered`, a `<file>	<line>	<count>` line for each line the block's tests run:
# one run of the block's tests alone, under its filterset, over its test packages, in the tree.
# A mutant on no such line is one no test citing the requirement could notice, so it is never
# tried. The files under a crate's `tests/`, which `cargo llvm-cov` leaves out by default, are
# reported too, and with them the dependencies' files, whose paths stay absolute.
covered_lines() {
  local report=$work/covered.info p args=()
  rm -f "$report"
  for p in $packages; do args+=(--package "$p"); done
  (cd "$tree" && cargo llvm-cov nextest --no-rustc-wrapper --no-default-ignore-filename-regex --no-tests=pass \
    --lcov --output-path "$report" "${args[@]}" -E "$filterset") ||
    die "cargo llvm-cov failed to find the lines the tests of requirement $id run"
  [ -s "$report" ] || die "cargo llvm-cov wrote no report for requirement $id"
  lcov_counts "$report" >"$work/covered"
}

# The report of a run of every test of package $1 in the tree, kept under the package's name for
# the rest of the pass: a block that starts once it is kept runs none of its own. It is written to
# a path of the block's job and only then kept, so two blocks at once never write one report.
package_report() {
  local kept=$pass_work/package-$1.info out=$work/package-$1.info
  if ! [ -s "$kept" ]; then
    rm -f "$out"
    (cd "$tree" && cargo llvm-cov nextest --no-rustc-wrapper --no-tests=pass --ignore-run-fail --lcov \
      --output-path "$out" --package "$1") ||
      die "cargo llvm-cov failed to run every test of package $1 for requirement $id"
    [ -s "$out" ] || die "cargo llvm-cov wrote no report of package $1's tests for requirement $id"
    mv -f "$out" "$kept"
  fi
}

# The mutants of list $1 by their share, highest first, and in its order among equal shares. A
# line's share is its count in the block's tests' run over the sum of its counts in the runs of
# every test of the block's test packages, or 1 when that sum is 0 or the share larger; a
# mutant's is the highest among the lines of its span the block's tests run. So the mutants of
# code only the requirement's tests run come first, and those of code every test runs last.
by_share() {
  local p reports=()
  for p in $packages; do
    package_report "$p"
    reports+=("$pass_work/package-$p.info")
  done
  lcov_counts "${reports[@]}" >"$work/package-counts"
  awk -F'\t' '
    FILENAME == ARGV[1] { run[$1 "\t" $2] = $3; next }
    FILENAME == ARGV[2] { all[$1 "\t" $2] = $3; next }
    {
      best = 0
      for (l = $2; l <= $3; l++) {
        k = $1 "\t" l
        if (!(k in run)) continue
        s = (all[k] + 0 > 0) ? run[k] / all[k] : 1
        if (s > 1) s = 1
        if (s > best) best = s
      }
      printf "%.17g\t%d\t%s\n", best, FNR, $0
    }' "$work/covered" "$work/package-counts" "$1" | LC_ALL=C sort -t "$tab" -k1,1gr -k2,2n | cut -f3-
}

# Writes to $1 a `<file>	<first line>	<last line>	<name>` line for each mutant
# `cargo mutants --list --json` lists for the block's files, in its order, with the further
# arguments given. With `--in-diff`, the tool prints nothing at all, not `[]`, when the diff
# holds no mutant of the files: that lists none.
list_mutants() {
  local out=$1
  shift
  (cd "$tree" && cargo mutants --list --json "${mutants_args[@]}" "$@") >"$out.json" ||
    die "cargo mutants failed to list mutants for requirement $id"
  if ! grep -q '[^[:space:]]' "$out.json"; then
    : >"$out"
    return
  fi
  node -e '
    for (const m of JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")))
      console.log([m.file, m.span.start.line, m.span.end.line, m.name].join("\t"));
  ' "$out.json" >"$out" || die "cargo mutants listed mutants that could not be read for requirement $id"
}

# The mutants of list $1 with a line from the start to the end of their span the tests run.
tryable() {
  awk -F'\t' 'FILENAME == ARGV[1] { run[$1 "\t" $2] = 1; next }
    { for (l = $2; l <= $3; l++) if (($1 "\t" l) in run) { print; next } }' "$work/covered" "$1"
}

# Sets `re_args` to a `--re` naming each mutant of list $1 exactly.
names_of() {
  local f a b name
  re_args=()
  while IFS="$tab" read -r f a b name; do
    [ -n "$name" ] || continue
    re_args+=(--re "^$(printf '%s' "$name" | sed -e 's/[][\.*^$(){}?+|/]/\\&/g')\$")
  done <"$1"
}

# Appends to $1 a `<file>	<first line>	<last line>	<name>	<diff file>` line for each mutant
# `cargo mutants --Zmutate-file` lists for file $3 of tree $2, in its order, writing each
# mutant's diff to a file of its own beside $1; $4 names what it judges, for an error. The tool
# names the file by its base name alone, so each mutant is keyed by $3, the path it was given.
list_file_mutants() {
  local out=$1 dir=$2 file=$3 what=$4 n
  n=$(lines_in "$out")
  mkdir -p "$out.d"
  (cd "$dir" && cargo mutants --Zmutate-file "$dir/$file" --json) >"$out.json" ||
    die "cargo mutants --Zmutate-file failed on $file for $what"
  node -e '
    const fs = require("fs");
    const [json, file, dir, start] = process.argv.slice(1);
    const list = JSON.parse(fs.readFileSync(json, "utf8"));
    if (!Array.isArray(list)) throw new Error("not a list");
    list.forEach((m, i) => {
      const s = m.span.start.line, e = m.span.end.line;
      if (!Number.isInteger(s) || !Number.isInteger(e) || typeof m.name !== "string" ||
        typeof m.diff !== "string") throw new Error("a mutant without its span, name or diff");
      const d = dir + "/" + (Number(start) + i) + ".diff";
      fs.writeFileSync(d, m.diff);
      console.log([file, s, e, m.name.replace(/[\t\n]/g, " "), d].join("\t"));
    });
  ' "$out.json" "$file" "$out.d" "$n" >>"$out" ||
    die "cargo mutants --Zmutate-file listed mutants of $file that could not be read for $what"
}

# Tries the mutants of list $1 (`list_file_mutants`' lines) in tree $2, one at a time, up to $3
# of them, and none after the first caught when $4 is `first`: writes the mutant into its file
# as its diff changes it, runs `cargo nextest run` over the packages `$packages` under
# `$filterset`, and puts the file back. Adds to `caught` when nextest exits 100, as when a test
# fails, and to `missed` when it exits 0; 101, a build that failed, counts neither way, as
# cargo-mutants counts an unviable mutant. Any other status, or a diff that doesn't apply, fails
# the script, naming $5, after the file is put back.
try_file_mutants() {
  local list=$1 dir=$2 max=$3 stop=$4 what=$5 f a b name d status n=0 p args=()
  for p in $packages; do args+=(--package "$p"); done
  while IFS="$tab" read -r -u 4 f a b name d; do
    [ "$n" -lt "$max" ] || break
    if [ "$stop" = first ] && [ "$caught" -gt 0 ]; then break; fi
    n=$((n + 1))
    cp -p "$dir/$f" "$work/file-mutant.orig"
    if ! patch -s -f --no-backup-if-mismatch -r - "$dir/$f" <"$d" >/dev/null; then
      cp "$work/file-mutant.orig" "$dir/$f"
      die "cargo mutants --Zmutate-file listed a mutant of $f that could not be written for $what"
    fi
    status=0
    (cd "$dir" && cargo nextest run --no-tests=pass "${args[@]}" -E "$filterset") || status=$?
    # Not `-p`: put back with a time newer than the mutant's build, so cargo rebuilds it.
    cp "$work/file-mutant.orig" "$dir/$f"
    case $status in
      0) missed=$((missed + 1)) ;;
      100) caught=$((caught + 1)) ;;
      101) ;;
      *) die "cargo nextest failed (status $status) on a mutant of $f for $what" ;;
    esac
  done 4<"$list"
}

# The block's test files under a crate's `tests/`, a test target's files, which cargo-mutants
# never mutates, in the order of its `test` lines, each once.
test_target_files() {
  local t file rest seen=
  for t in ${tests[@]+"${tests[@]}"}; do
    file=${t%%"$tab"*}
    case $file in crates/*.rs) ;; *) continue ;; esac
    # Under `crates/<dir>/tests/`, not a `tests/` deeper in the crate.
    rest=${file#crates/}
    rest=${rest#*/}
    case $rest in tests/*) ;; *) continue ;; esac
    case "$nl$seen" in *"$nl$file$nl"*) continue ;; esac
    seen="$seen$file$nl"
    printf '%s\n' "$file"
  done
}

# Makes `$2`, the tree `cargo mutants` runs in and copies with `--copy-vcs true`, from the
# checkout `$1`: a git repository whose `.git` is a directory of its own, borrowing `$1`'s object
# store by an absolute path and holding none of it (`git clone --shared`), checked out at `$1`'s
# `HEAD`, with `$1`'s branches and remote-tracking branches under the same names, and every
# tracked file marked unchanged in its index. So a test that asks git about the repository
# passes in each copy as it does in the checkout, git there reports no tracked file changed
# from `HEAD` whatever mutant the tool writes, and no copy reads or writes `$1`'s index, refs
# or git directory. Uncommitted edits in `$1` are not in the tree.
make_tree() {
  local src=$1 dest=$2 head
  head=$(git -C "$src" rev-parse --verify HEAD) || return 1
  # Its remote is named here, whatever `clone.defaultRemoteName` says, and removed with its
  # branches named for `$1`'s, which would shadow `$1`'s own refs.
  git clone -q --shared --no-checkout -o clm-mutate-source "$src" "$dest" || return 1
  git -C "$dest" remote remove clm-mutate-source || return 1
  git -C "$dest" -c advice.detachedHead=false checkout -q --detach "$head" || return 1
  git -C "$dest" fetch -q --no-tags "$src" '+refs/heads/*:refs/heads/*' '+refs/remotes/*:refs/remotes/*' ||
    return 1
  git -C "$dest" ls-files -z | git -C "$dest" update-index -z --assume-unchanged --stdin
}

# Judges a fixture with cargo-mutants before it judges anything: a function with a test that
# fails when it is changed and one that passes whatever it returns. The tool goes on only when it
# reports the first's mutants caught and the second's missed, so a tool that accepts nothing, or
# everything, can't be taken for a verdict. The crate is committed to a git repository in the
# temporary directory, and the tool runs in a tree made from it as `make_tree` makes the
# checkout's. Its tests pass only inside a git repository whose top is the crate, and the blind
# one only while git reports no tracked file changed from `HEAD`: a tool that leaves `.git` out of
# the copy it tests, or a copy in which git sees the mutant, fails the fixture, as it would fail
# or falsely kill this repository's tests that ask git about it.
rust_fixture() {
  local src=$work/fixture-src fix=$work/fixture-rust kind out status
  mkdir -p "$src/src"
  printf '[package]\nname = "clm-mutate-fixture"\nversion = "0.0.0"\nedition = "2021"\n' >"$src/Cargo.toml"
  cat >"$src/src/lib.rs" <<'EOF'
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

    // Git sees no tracked file changed from `HEAD`, though the tool changed `src/lib.rs`.
    fn tracked_files_unchanged() {
        let status = std::process::Command::new("git")
            .args(["status", "--porcelain", "--untracked-files=no"])
            .output()
            .unwrap();
        assert!(status.status.success(), "git status failed");
        assert_eq!(String::from_utf8(status.stdout).unwrap(), "");
    }

    #[test]
    fn fixture_notices() {
        in_git_repository();
        assert_eq!(double(3), 6);
    }

    #[test]
    fn fixture_blind() {
        in_git_repository();
        tracked_files_unchanged();
        let _ = double(3);
    }
}
EOF
  # A test target's file, whose function no test is the one its tests judge. Its git checks are
  # macros, which no tool mutates, so the function holds its only mutants.
  mkdir -p "$src/tests"
  cat >"$src/tests/clm_mutate_fixture.rs" <<'EOF'
// The crate's own directory must be the top of a git repository.
macro_rules! in_git_repository {
    () => {{
        let top = std::process::Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .unwrap();
        assert!(top.status.success(), "not in a git repository");
        let top = String::from_utf8(top.stdout).unwrap();
        let here = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap();
        assert_eq!(std::fs::canonicalize(top.trim()).unwrap(), here);
    }};
}

// Git sees no tracked file changed from `HEAD`, though the mutant changed this file.
macro_rules! tracked_files_unchanged {
    () => {{
        let status = std::process::Command::new("git")
            .args(["status", "--porcelain", "--untracked-files=no"])
            .output()
            .unwrap();
        assert!(status.status.success(), "git status failed");
        assert_eq!(String::from_utf8(status.stdout).unwrap(), "");
    }};
}

fn triple(x: i32) -> i32 {
    x * 3
}

#[test]
fn fixture_file_notices() {
    in_git_repository!();
    assert_eq!(triple(3), 9);
}

#[test]
fn fixture_file_blind() {
    in_git_repository!();
    tracked_files_unchanged!();
    let _ = triple(3);
}
EOF
  { git -C "$src" init -q &&
    git -C "$src" add Cargo.toml src/lib.rs tests/clm_mutate_fixture.rs &&
    git -C "$src" -c user.name=clm -c user.email=clm@localhost -c commit.gpgsign=false \
      commit -q --no-verify -m fixture &&
    make_tree "$src" "$fix"; } || die "cargo mutants' fixture could not be made a git repository"
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
  # Its test file's mutants, every one the tool lists, tried as a block's test file's are, with
  # no run to find the lines, in a copy made as a block's is: so a copy in which git sees the
  # mutant, a listing misread or a nextest whose exit status means something else fails the
  # fixture too.
  local copy=$work/fixture-copy packages=clm-mutate-fixture filterset
  make_tree "$src" "$copy" || die "cargo mutants' fixture could not be copied"
  : >"$work/fixture-file"
  list_file_mutants "$work/fixture-file" "$copy" tests/clm_mutate_fixture.rs "cargo mutants' fixture"
  for kind in notices blind; do
    filterset="binary_id(=clm-mutate-fixture::clm_mutate_fixture) & test(~fixture_file_$kind)"
    caught=0
    missed=0
    CARGO_TARGET_DIR=$work/fixture-target try_file_mutants "$work/fixture-file" "$copy" \
      "$(lines_in "$work/fixture-file")" all "cargo mutants' fixture"
    case $kind in
      notices) [ "$caught" -gt 0 ] && [ "$missed" -eq 0 ] ;;
      blind) [ "$missed" -gt 0 ] && [ "$caught" -eq 0 ] ;;
    esac || die "cargo mutants and cargo nextest judged their fixture's test file wrongly: $caught caught and $missed missed for the $kind test"
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
untested=0
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
  # Run in the tree `make_tree` made, with `--copy-vcs true`: each copy it tests holds that
  # tree's own `.git`, so a test that asks git about this repository passes there as it does
  # here, and nothing a copy runs touches the checkout's git directory.
  (cd "$tree" && cargo mutants --test-tool nextest --copy-vcs true --output "$out" "${mutants_args[@]}" "$@" \
    -- --no-tests=pass -E "$filterset") || status=$?
  # 0 all caught, 2 some missed, 3 some timed out; anything else judged nothing: the tests fail
  # in the unmutated tree, or a filterset doesn't parse.
  case $status in
    0 | 2 | 3) count_outcomes "$out" ;;
    *) die "cargo mutants failed (status $status) for requirement $id" ;;
  esac
}

# The block's Rust files, `rust_files`, and the packages and filterset of its Rust tests,
# `packages` and `filterset`; fails when it has none of either, and cargo-mutants judges nothing.
rust_selection() {
  local f t file name package filter out
  rust_files=()
  packages=
  filterset=
  for f in ${code[@]+"${code[@]}"}; do
    case $f in crates/*.rs) rust_files+=("$f") ;; esac
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
    if [ -n "$filterset" ]; then filterset="$filterset | $filter"; else filterset=$filter; fi
  done
  [ "${#rust_files[@]}" -gt 0 ] && [ -n "$filterset" ]
}

# Whether the block's tests run a line of a Rust file of the repository other than its code
# files and its test files. The report's files outside the tree, the dependencies' and the
# standard library's, keep absolute paths and are none of the repository's.
runs_other_files() {
  local t own=
  for t in ${rust_files[@]+"${rust_files[@]}"} ${tests[@]+"${tests[@]}"}; do own="$own${t%%"$tab"*}$nl"; done
  own="$own" awk -F'\t' 'BEGIN { n = split(ENVIRON["own"], o, "\n"); for (i = 1; i <= n; i++) mine[o[i]] = 1 }
    $1 !~ /^\// && $1 ~ /\.rs$/ && !($1 in mine) { found = 1; exit }
    END { exit !found }' "$work/covered"
}

# Judges the block by the mutants of its test files its tests run, `$work/file-try`: up to its
# `limit`, none after the first caught, each written into a copy of the tree of the block's job
# number, made the first time the job needs one, never the tree cargo-mutants copies or the
# checkout. The block's `diff` plays no part. Sets `verdict`.
judge_test_files() {
  local copy=$pass_work/copy-$job
  if ! [ -d "$copy" ]; then
    # From the tree, so the mutants land on the revision they were listed and their lines found
    # for, wherever the checkout's `HEAD` has moved since.
    make_tree "$tree" "$copy" || die "a copy of the tree could not be made for requirement $id"
  fi
  caught=0
  missed=0
  try_file_mutants "$work/file-try" "$copy" "$limit" first "requirement $id"
  if [ "$caught" -gt 0 ]; then
    verdict=killed
  elif [ "$missed" -gt 0 ]; then
    verdict="survived $missed"
  else
    verdict=not-judged
  fi
}

# Writes `$work/file-all`, the mutants of the block's test files, listed from the tree's copies.
list_test_file_mutants() {
  local f
  : >"$work/file-all"
  for f in $(test_target_files); do
    [ ! -f "$tree/$f" ] || list_file_mutants "$work/file-all" "$tree" "$f" "requirement $id"
  done
}

# Judges the Rust files of a block, its tools ready. Sets `verdict` to killed, survived <n> or
# not-judged.
judge_rust() {
  local f p n=0 re_args=() files=()
  mutants_args=()
  if ! rust_selection; then
    verdict=not-judged
    return
  fi
  files=("${rust_files[@]}")
  for p in $packages; do mutants_args+=(--test-package "$p"); done
  for f in ${files[@]+"${files[@]}"}; do mutants_args+=(--file "$f"); done
  # The diff's mutants, and the further ones: the files' others, in the tool's order until
  # ordered by share.
  list_mutants "$work/all"
  # And those of its test files, which judge the block only when its code offers none its
  # tests run: listed only then, since the tool's option for them is one it marks unstable.
  : >"$work/file-all"
  if ! [ -s "$work/all" ]; then
    list_test_file_mutants
    if ! [ -s "$work/file-all" ]; then
      verdict=not-judged
      return
    fi
  fi
  : >"$work/diff"
  if [ -n "$diff" ] && [ -s "$work/all" ]; then list_mutants "$work/diff" --in-diff "$diff"; fi
  covered_lines
  awk -F'\t' 'FILENAME == ARGV[1] { d[$4] = 1; next } !($4 in d)' "$work/diff" "$work/all" >"$work/further"
  # No mutant of its code on a line its tests run, and no line run of any other Rust file of the
  # repository but its test files: its rule may be code in a test's own file, as a scan of the
  # repository's sources is, and the mutants of its test files judge it, alone. Never beside the
  # code's: a mutant of a helper that builds a test's fixture is caught however little the test
  # asserts, and would hide the code's survivors. A test that runs another component's code
  # reads as before.
  tryable "$work/all" >"$work/all-try"
  if ! [ -s "$work/all-try" ] && ! runs_other_files; then
    [ ! -s "$work/all" ] || list_test_file_mutants
    tryable "$work/file-all" >"$work/file-try"
    if [ -s "$work/file-try" ]; then
      judge_test_files
      return
    fi
  fi
  # Tests that run no line of any file with a mutant can notice no change to it: every mutant
  # they would be given survives, untried. Unless they read the repository's own files, whose
  # data may be the rule they prove: then nothing here judges them.
  if ! awk -F'\t' 'FILENAME == ARGV[1] { has[$1] = 1; next } ($1 in has) { found = 1; exit }
      END { exit !found }' "$work/all" "$work/covered"; then
    if [ -n "$reads" ]; then
      verdict=not-judged
      return
    fi
    n=$(lines_in "$work/further")
    [ "$n" -le "$limit" ] || n=$limit
    n=$((n + $(lines_in "$work/diff")))
    if [ "$n" -gt 0 ]; then verdict="survived $n"; else verdict=not-judged; fi
    return
  fi
  tryable "$work/diff" >"$work/diff-try"
  # The limit counts only the mutants that may be tried. Cut from a file, not a pipe: a reader
  # that stops at the limit would kill a longer writer with SIGPIPE, and `pipefail` the script.
  tryable "$work/further" >"$work/further-all"
  caught=0
  missed=0
  if [ -s "$work/diff-try" ]; then
    names_of "$work/diff-try"
    run_mutants "$work/m1" --in-diff "$diff" "${re_args[@]}"
  fi
  # More than the limit, and the further mutants are to be tried: those of code only the
  # requirement's tests run first.
  if [ "$caught" -eq 0 ] && [ "$(lines_in "$work/further-all")" -gt "$limit" ]; then
    by_share "$work/further-all" >"$work/further-ordered"
    mv -f "$work/further-ordered" "$work/further-all"
  fi
  head -n "$limit" "$work/further-all" >"$work/further-try"
  if [ "$caught" -eq 0 ] && [ -s "$work/further-try" ]; then
    names_of "$work/further-try"
    run_mutants "$work/m2" "${re_args[@]}"
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

ts_tools() {
  (
    cd "$root/$1"
    npm ci --ignore-scripts
    # Added to `node_modules/` alone, so no `package.json` or lockfile changes.
    npm install --no-save --ignore-scripts \
      "@stryker-mutator/core@$STRYKER_VERSION" "@stryker-mutator/vitest-runner@$STRYKER_VERSION"
  ) || die "StrykerJS could not be installed in $1 (requirement $id)"
  ts_fixture "$1"
}

# Judges a fixture with StrykerJS in the npm directory $1, with that directory's own vitest and
# TypeScript: a source file whose first function a test notices a change to and whose second
# function no test does, and which fails when a directory the fixture's `.gitignore` names reaches
# StrykerJS's sandbox. It goes on only when the first function's mutants are caught and the
# second's missed. The files sit inside the directory, which a vitest configuration reaches, and
# are removed afterwards.
ts_fixture() {
  local dir=$1 fix=clm-mutate-fixture saved=("${ts_tests[@]+"${ts_tests[@]}"}") up
  fixture_dirs="$fixture_dirs $dir"
  rm -rf "${root:?}/$dir/$fix"
  mkdir -p "$root/$dir/$fix"
  # The repository's top, relative to the fixture's test file.
  up=$(printf '%s' "$dir/$fix" | sed 's#[^/][^/]*#..#g')
  cat >"$root/$dir/$fix/fixture.ts" <<'EOF'
export const double = (x: number): number => x * 2;
export const triple = (x: number): number => x * 3;
EOF
  # A directory one level below the files, which a `.gitignore` beside them names by a rule with
  # no slash but a trailing one, the shape StrykerJS matches only at the top unless it is carried
  # anchored. It holds a file, since a copy may make no empty directory.
  mkdir -p "$root/$dir/$fix/nested/clm-ignored"
  : >"$root/$dir/$fix/nested/clm-ignored/x"
  printf 'clm-ignored/\n' >"$root/$dir/$fix/.gitignore"
  # The test that notices also reads this script, a file of the repository outside the npm
  # directory, by a path relative to its own file, so a sandbox from which the repository's
  # files can't be reached fails the fixture. It reads it with `fs`, in node's environment. It
  # also fails when it finds the ignored directory, so a StrykerJS that copies what git ignores
  # fails the fixture.
  cat >"$root/$dir/$fix/fixture.test.ts" <<EOF
// @vitest-environment node
import { existsSync, readFileSync } from 'node:fs';
import { expect, test } from 'vitest';
import { double, triple } from './fixture';

test('fixture_notices', () => {
  expect(readFileSync(new URL('$up/.github/scripts/mutate.sh', import.meta.url), 'utf8')).toContain('mutate');
  expect(existsSync(new URL('./nested/clm-ignored', import.meta.url))).toBe(false);
  expect(double(3)).toBe(6);
});

test('fixture_blind', () => {
  triple(3);
});
EOF
  ts_tests=("$dir/$fix/fixture.test.ts")
  caught=0
  missed=0
  untested=0
  run_stryker "$dir" f1 "$fix/fixture.ts:1-1" ||
    die "StrykerJS failed on its fixture in $dir"
  missed=$((missed + untested))
  { [ "$caught" -gt 0 ] && [ "$missed" -eq 0 ]; } ||
    die "StrykerJS judged its fixture wrongly in $dir: $caught caught and $missed missed for the test that notices"
  caught=0
  missed=0
  untested=0
  run_stryker "$dir" f2 "$fix/fixture.ts:2-2" ||
    die "StrykerJS failed on its fixture in $dir"
  # Survived, not `NoCoverage`: the blind test runs the function, and a StrykerJS that saw no
  # test run it would count a block's mutants wrongly.
  { [ "$missed" -gt 0 ] && [ "$caught" -eq 0 ] && [ "$untested" -eq 0 ]; } ||
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
# `missed`, and those it reports `NoCoverage`, which no test runs, to `untested`, and writes
# its mutants to `mutants.tsv` beside its report. StrykerJS takes a runner's options from no
# command line, so it is given them in a
# configuration file under the directory's `node_modules/`: the test runner, the vitest
# configuration that names those tests, the reporter and the ranges. It sets `vitest.related`
# false, so vitest runs those tests whether or not they import a mutated file: left true, vitest
# keeps only the tests that import one, and a block whose tests read a file instead, as a test of
# `web/.npmrc` does, runs none, which StrykerJS reports as an error, and its mutants are never
# counted `NoCoverage`. It sets no
# `thresholds.break`, so StrykerJS's exit status never depends on a score. Returns 1 when StrykerJS
# exits non-zero or writes no report: it judged nothing. `tsconfigFile` names a file that doesn't
# exist: StrykerJS 10.1.1 reads a `tsconfig.json` it finds with `ts.parseConfigFileTextToJson`,
# which the TypeScript 7 of `web/` and `plugin-panel/` lacks, to rewrite the paths in it for its
# sandbox. Nothing here needs that rewrite: the run mutates and runs vitest, with no type checker.
# Its configuration files and its sandbox are named for `tag`, the block or the fixture, so runs
# of two blocks at once in one directory share neither.
run_stryker() {
  local dir=$1 report=$2 base vcfg scfg status=0 includes= ranges= ignores f m json k s u all
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
  mkdir -p "$root/$dir/node_modules/.clm-mutate/$tag"
  vcfg=$root/$dir/node_modules/.clm-mutate/$tag/vitest.config.mts
  scfg=$root/$dir/node_modules/.clm-mutate/$tag/stryker.config.json
  {
    echo "$base"
    [ -n "$base" ] || echo "const base: any = {};"
    echo "export default { ...base, test: { ...base.test, include: [$includes] } };"
  } >"$vcfg"
  for m in "$@"; do ranges="$ranges\"$m\","; done
  ignores=$(stryker_ignores "$dir") || return 1
  cat >"$scfg" <<EOF
{
  "testRunner": "vitest",
  "vitest": { "configFile": "$vcfg", "related": false },
  "reporters": ["json"],
  "jsonReporter": { "fileName": "$work/$report/mutation.json" },
  "tsconfigFile": "$root/$dir/node_modules/.clm-mutate/no-tsconfig.json",
  "tempDirName": ".stryker-tmp-clm-$tag",
  "ignorePatterns": [".stryker-tmp-clm-*"${ignores:+,${ignores%,}}],
  "concurrency": 2,
  "mutate": [${ranges%,}]
}
EOF
  # A link it can't make fails the run, as StrykerJS failing would.
  if stryker_links "$dir" ".stryker-tmp-clm-$tag"; then
    (
      cd "$root/$dir"
      npx stryker run "$scfg"
    ) || status=$?
  else
    status=1
  fi
  # Gone as soon as StrykerJS returns, so no later run, such as `coverage.sh`'s vitest, finds
  # the links.
  rm -rf "${root:?}/$dir/.stryker-tmp-clm-$tag"
  rm -f "$vcfg" "$scfg"
  json=$work/$report/mutation.json
  if [ "$status" -ne 0 ] || ! [ -f "$json" ]; then return 1; fi
  # A `<status>	<file>:<span>:<mutator>:<replacement>` line per mutant, which names it the same
  # in every run, so a mutant two runs report counts once.
  node -e '
    const r = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"));
    for (const [file, f] of Object.entries(r.files || {}))
      for (const m of f.mutants || []) {
        const l = m.location || {}, s = l.start || {}, e = l.end || {};
        console.log(m.status + "\t" + [file, s.line, s.column, e.line, e.column, m.mutatorName,
          m.replacement].join(":").replace(/[\t\n]/g, " "));
      }
  ' "$json" >"$work/$report/mutants.tsv" || return 1
  count_stryker "$work/$report/mutants.tsv"
  caught=$((caught + k))
  missed=$((missed + s))
  untested=$((untested + u))
}

# The rules of the `.gitignore` files git reads that reach the npm directory $1, for StrykerJS's
# `ignorePatterns`, each a JSON string and a comma: the top's and those of the directories down
# to $1 first, then those under $1, tracked or untracked and not ignored, but none in a StrykerJS
# sandbox of this script's, shallower before deeper, so the last rule that matches wins as in git.
# StrykerJS tries a pattern against an entry's name and its path from $1, so each is re-rooted at
# $1 and anchored there: an unanchored rule gains `/**/` (`.test-dist/` in `site/.gitignore` is
# `/site/**/.test-dist/`), since StrykerJS would match it only at the top, and a rule with a
# slash before its end is joined to its own file's directory, its leading parts matched, as
# globs, against $1's, and left out when they don't match. Rules, not the paths that exist, so a
# directory another run makes while StrykerJS copies is left out too. Comments, blank lines and
# trailing spaces go; a `!` stays in front. A file that is gone by the time it is read, as one in
# another run's sandbox may be, is skipped.
stryker_ignores() {
  local dir=$1 up= part f files=()
  files+=(.gitignore)
  while IFS= read -r -d / part; do
    up=$up$part/
    [ "$up" = "$dir/" ] || files+=("${up}.gitignore")
  done <<<"$dir/"
  while IFS= read -r -d '' f; do
    files+=("$f")
  done < <(git -C "$root" ls-files -z --cached --others --exclude-standard -- "$dir" |
    awk -v RS='\0' -v ORS='\0' -F/ '
      $NF == ".gitignore" && $0 !~ /(^|\/)\.stryker-tmp-clm-[^\/]*\// { print NF "\t" $0 }' |
    sort -z -n -s -k1,1 | cut -z -f2)
  for f in "${files[@]}"; do
    [ -f "$root/$f" ] || continue
    awk -v d="$(dirname "$f")" -v n="$dir" '
      # A regular expression matching the glob $1, one part of a path.
      function globre(g,   re, i, c, j) {
        re = "^"
        for (i = 1; i <= length(g); i++) {
          c = substr(g, i, 1)
          if (c == "*") re = re "[^/]*"
          else if (c == "?") re = re "[^/]"
          else if (c == "[" && (j = index(substr(g, i + 2), "]")) > 0) {
            c = substr(g, i + 1, j)
            if (substr(c, 1, 1) == "!") c = "^" substr(c, 2)
            re = re "[" c "]"
            i += j + 1
          } else {
            if (c == "\\" && i < length(g)) c = substr(g, ++i, 1)
            if (c ~ /[A-Za-z0-9]/) re = re c
            else if (c == "\\" || c == "^") re = re "\\" c
            else re = re "[" c "]"
          }
        }
        return re "$"
      }
      BEGIN { if (d == ".") d = ""; k = split(n, np, "/") }
      {
        r = $0
        sub(/\r$/, "", r)
        if (r == "" || substr(r, 1, 1) == "#") next
        while (r ~ / $/ && r !~ /\\ $/) r = substr(r, 1, length(r) - 1)
        neg = ""
        if (substr(r, 1, 1) == "!") { neg = "!"; r = substr(r, 2) }
        body = r
        sub(/\/$/, "", body)
        if (body == "" || body == "/") next
        if (substr(r, 1, 3) == "**/") { anchored = 0; r = substr(r, 4) }
        else anchored = index(body, "/") > 0
        if (anchored) {
          sub(/^\//, "", r)
          m = split(d == "" ? r : d "/" r, fp, "/")
          out = ""
          for (i = 1; i <= k; i++) {
            if (fp[i] == "**") { out = "/**"; i++; break }
            # The rule names $1 itself or a directory above it, or misses it.
            if (i >= m || fp[i] == "" || np[i] !~ globre(fp[i])) next
          }
          for (; i <= m; i++) out = out "/" fp[i]
          # Nothing left below $1: the rule names $1 itself.
          if (out ~ /^[\/*]*$/) next
        } else if (index(d "/", n "/") == 1 && d != n) {
          out = "/" substr(d, length(n) + 2) "/**/" r
        } else {
          out = "/**/" r
        }
        out = neg out
        gsub(/\\/, "\\\\", out)
        gsub(/"/, "\\\"", out)
        printf "\"%s\",", out
      }' "$root/$f" || [ ! -e "$root/$f" ] || return 1
  done
}

# Makes $2, the directory under the npm directory $1 that StrykerJS makes its sandbox in, afresh,
# holding a link for each entry of the repository's top but `.git` and $1, to that entry, dot
# entries and untracked ones too: the sandbox then stands where $1 does among the repository's
# other files, so a path a test resolves from its own file to a file outside $1, as
# `../../specs/web/SPEC.md` from `web/src/`, reaches it there as in the checkout.
stryker_links() {
  local dir=$1 tmp=$root/$1/$2 e n
  rm -rf "${tmp:?}"
  mkdir -p "$tmp" || return 1
  for e in "$root"/* "$root"/.*; do
    n=${e##*/}
    case $n in . | .. | .git | "${dir%%/*}") continue ;; esac
    [ -e "$e" ] || [ -L "$e" ] || continue
    ln -s "$e" "$tmp/$n" || return 1
  done
}

# Sets `k`, `s`, `u` and `all` to the mutants of the reports' lines $@ caught (killed or timed
# out), survived, reported `NoCoverage`, and reported at all, each mutant once.
count_stryker() {
  local lines
  lines=$(cat "$@" | sort -t "$tab" -k2 -u)
  k=$(printf '%s\n' "$lines" | grep -Ec '^(Killed|Timeout)'"$tab" || true)
  s=$(printf '%s\n' "$lines" | grep -c '^Survived'"$tab" || true)
  u=$(printf '%s\n' "$lines" | grep -c '^NoCoverage'"$tab" || true)
  all=$(printf '%s\n' "$lines" | grep -c "$tab" || true)
}

# The block's npm directory, `ts_dir`, its TypeScript files there, `ts_files`, relative to it,
# and its vitest tests, `ts_tests`; fails when it has none of either, and StrykerJS judges
# nothing.
ts_selection() {
  local f t file
  ts_dir=
  ts_files=()
  ts_tests=()
  for f in ${code[@]+"${code[@]}"}; do
    case $f in
      # A vitest test file is neither kind: StrykerJS's instrumentation of one moves a hoisted
      # `vi.mock` call out of the module's top level and fails its first run.
      *.test.ts | *.test.tsx) continue ;;
      web/src/*.ts | web/src/*.tsx) [ -n "$ts_dir" ] || ts_dir=web ;;
      plugin-panel/*.ts | plugin-panel/*.tsx) [ -n "$ts_dir" ] || ts_dir=plugin-panel ;;
      *) continue ;;
    esac
    case $f in "$ts_dir"/*) ts_files+=("${f#"$ts_dir"/}") ;; esac
  done
  for t in ${tests[@]+"${tests[@]}"}; do
    file=${t%%"$tab"*}
    # A vitest file of the directory; Playwright's specs are judged by no tool here.
    case $file in "$ts_dir"/e2e/* | capabilities/*) continue ;; "$ts_dir"/*) ts_tests+=("$file") ;; esac
  done
  [ -n "$ts_dir" ] && [ "${#ts_files[@]}" -gt 0 ] && [ "${#ts_tests[@]}" -gt 0 ]
}

# Judges the TypeScript files of a block, its directory's tools ready. Sets `verdict`.
judge_ts() {
  local f dir files=() ranges=() capped=() k s u all
  if ! ts_selection; then
    verdict=not-judged
    return
  fi
  dir=$ts_dir
  files=("${ts_files[@]}")
  caught=0
  missed=0
  untested=0
  rm -rf "${work:?}/s1" "${work:?}/s2"
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
  # Each mutant once, though both runs report those on the diff's lines among the first `limit`.
  # A mutant no test runs is neither caught nor survived, unless every mutant reported is one no
  # test runs: then the tests can notice no change to the code, and every one survives, as for
  # an empty test, unless the block reads the repository's own files, when none is judged.
  count_stryker "$work"/s[12]/mutants.tsv
  caught=$k
  missed=$s
  if [ "$caught" -eq 0 ] && [ "$missed" -eq 0 ] && [ "$u" -gt 0 ] && [ "$u" -eq "$all" ] && [ -z "$reads" ]; then
    missed=$u
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

# Each block's lines, in the plan's order.
blocks=()
block=
while IFS= read -r -u 3 line || [ -n "$line" ]; do
  case $line in
    "requirement "*)
      [ -z "$block" ] || blocks+=("$block")
      block=$line
      ;;
    *) [ -z "$block" ] || block="$block$nl$line" ;;
  esac
done 3<"$plan"
[ -z "$block" ] || blocks+=("$block")

# Sets `id`, `code`, `tests`, `diff`, `limit` and `reads`, set when the block holds a `reads`
# line, from block $1.
load_block() {
  local l
  id=
  code=()
  tests=()
  diff=
  limit=20
  reads=
  while IFS= read -r l; do
    case $l in
      "requirement "*) id=${l#requirement } ;;
      "code "*) code+=("${l#code }") ;;
      "diff "*) diff=${l#diff } ;;
      "test "*) tests+=("${l#test }") ;;
      "limit "*) limit=${l#limit } ;;
      "reads "*) reads=1 ;;
    esac
  done <<<"${blocks[$1]}"
}

# Appends the verdict of the block loaded, in one write, ending in a newline.
finish_block() {
  local rust_v ts_v v total x
  verdict=not-judged
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
  printf '%s\n' "$id $v" >>"$result"
}

# Every tool the plan's blocks need is installed, and every fixture judged, before any block
# starts: cargo-mutants' once, and StrykerJS's once for each npm directory, named for the first
# block that needs it.
tag=fixture
rust_needed=
ts_dirs=
ts_ids=()
for i in ${blocks[@]+"${!blocks[@]}"}; do
  load_block "$i"
  if [ -z "$rust_needed" ] && rust_selection; then rust_needed=$id; fi
  if ts_selection; then
    case " $ts_dirs " in *" $ts_dir "*) ;; *)
      ts_dirs="$ts_dirs $ts_dir"
      ts_ids+=("$id")
      ;;
    esac
  fi
done
if [ -n "$rust_needed" ]; then
  id=$rust_needed
  rust_tools
fi
n=0
for d in $ts_dirs; do
  id=${ts_ids[$n]}
  n=$((n + 1))
  ts_tools "$d"
done

# Judges block $1 as job $2, in a work directory of its own, with the job's `CLM_SLOT` and
# `CARGO_TARGET_DIR`. Run in a subshell of its own: a tool that fails ends it alone.
run_block() {
  trap - EXIT TERM INT
  work=$work/block-$1
  mkdir -p "$work"
  tag=$1
  job=$2
  export CLM_SLOT=$((slot_base + $2 - 1)) CARGO_TARGET_DIR=$target_base/mutate-job-$2
  load_block "$1"
  finish_block
}

# Starts the blocks in the plan's order, each in a job number no running block holds. Once a
# block fails, it starts no other, and lets those running finish and write their lines.
count=${#blocks[@]}
[ "$jobs_max" -le "$count" ] || jobs_max=$((count > 0 ? count : 1))
pid_of=()
next=0
running=0
failed=false
while :; do
  for ((j = 1; j <= jobs_max; j++)); do
    p=${pid_of[j]:-}
    if [ -n "$p" ] && ! kill -0 "$p" 2>/dev/null; then
      status=0
      wait "$p" || status=$?
      [ "$status" -eq 0 ] || failed=true
      pid_of[j]=
      running=$((running - 1))
    fi
  done
  if ! $failed; then
    for ((j = 1; j <= jobs_max && next < count; j++)); do
      [ -z "${pid_of[j]:-}" ] || continue
      run_block "$next" "$j" &
      pid_of[j]=$!
      running=$((running + 1))
      next=$((next + 1))
    done
  fi
  [ "$running" -gt 0 ] || break
  sleep 0.1
done
if $failed; then exit 1; fi


.github/scripts/coverage.sh
if [ -n "${CLM_LAND_DIR:-}" ]; then
  cp target/lcov.info "$CLM_LAND_DIR/lcov.info"
fi
