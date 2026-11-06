#!/bin/sh
# Writes corpus/v<version>.tar.gz: a project as the `clm` given set it up (`clm init --lang rust
# --no-detectors --host none` in an empty directory under version control), plus corpus/sample/
# with its @REQ@ placeholder replaced by the marker prefix. Fails, writing no entry, unless that
# `clm check` passes on the result. The tarball is deterministic: sorted names, mtime 0, owner 0.
#
# Usage: make.sh <version> <clm>
set -eu

[ $# -eq 2 ] || { echo "usage: make.sh <version> <clm>" >&2; exit 2; }
version=$1
clm=$2
here=$(cd "$(dirname "$0")" && pwd)
case $clm in /*) ;; *) clm=$(pwd)/$clm ;; esac

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
project=$work/project
mkdir "$project"
cd "$project"
git init -q
"$clm" init --lang rust --no-detectors --host none >/dev/null

# The sample, with the marker prefix put in the placeholder's place. The prefix is split here so
# this script holds no whole marker.
(cd "$here/sample" && find . -type f) | while read -r file; do
  mkdir -p "$(dirname "$file")"
  sed 's/@REQ@/req/' "$here/sample/$file" >"$file"
done

git add -A
git -c user.name=corpus -c user.email=corpus@example.invalid commit -q -m "corpus v$version"
"$clm" check >"$work/check.log" 2>&1 || { cat "$work/check.log" >&2; echo "make.sh: clm check fails on the v$version project" >&2; exit 1; }

out=$here/v$version.tar.gz
tmp=$work/entry.tar.gz
# Everything but .git, in byte order of the names, with no time, owner or group.
LC_ALL=C find . -path ./.git -prune -o -type f -print | sed 's|^\./||' | LC_ALL=C sort >"$work/names"
tar --format=ustar --mtime=@0 --owner=0 --group=0 --numeric-owner --no-recursion -cf - -T "$work/names" | gzip -n >"$tmp"
mv "$tmp" "$out"
