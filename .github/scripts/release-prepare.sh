#!/usr/bin/env bash
# Checks a release can be cut and, when the version is new, pushes its bump commit to
# release/v<version> (specs/ci/SPEC.md, ci/release). Run from the repo root.
# Args: the version to release. Env: GITHUB_REF, GITHUB_SHA, GITHUB_OUTPUT.
# Writes `sha` (the commit to release) and `bumped` to $GITHUB_OUTPUT.
set -euo pipefail

version=${1:-}
cargo_toml=Cargo.toml
plugin_json=plugin/.claude-plugin/plugin.json
panel_json=plugin-panel/.claude-plugin/plugin.json
shim=plugin/bin/clm
workflow=.github/workflows/release.yml

fail() {
  echo "release: $*" >&2
  exit 1
}

# `[workspace.package] version`; dependency lines also have `version =`.
workspace_version() {
  sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' "$cargo_toml"
}

[ "$GITHUB_REF" = refs/heads/main ] || fail "releases run only from main, not $GITHUB_REF"
# No leading zeros, as in semver: `0.01.0` would equal `0.1.0` yet name another tag.
number='(0|[1-9][0-9]*)'
[[ $version =~ ^$number\.$number\.$number$ ]] || fail "version '$version' isn't MAJOR.MINOR.PATCH"

current=$(workspace_version)
[[ $current =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "can't read the workspace version from $cargo_toml"
IFS=. read -r a1 a2 a3 <<<"$version"
IFS=. read -r b1 b2 b3 <<<"$current"
if ((10#$a1 != 10#$b1)); then
  cmp=$((10#$a1 > 10#$b1 ? 1 : -1))
elif ((10#$a2 != 10#$b2)); then
  cmp=$((10#$a2 > 10#$b2 ? 1 : -1))
elif ((10#$a3 != 10#$b3)); then
  cmp=$((10#$a3 > 10#$b3 ? 1 : -1))
else
  cmp=0
fi
((cmp >= 0)) || fail "version $version is lower than the workspace version $current"

# Every asset the shim downloads is one the release builds (shim-assets-before-release): the
# shim's `target=<target>` assignments, each a `target:` of the build matrix. Read as text.
[ -r "$shim" ] || fail "can't read $shim"
[ -r "$workflow" ] || fail "can't read $workflow"
shim_targets=$(grep -oE '(^|[^[:alnum:]_$])target=[[:alnum:]_.-]+' "$shim" | sed 's/.*target=//' | sort -u || true)
[ -n "$shim_targets" ] || fail "$shim maps no platform to a target"
built=$(sed -n 's/^[[:space:]]*\(- \)\{0,1\}target:[[:space:]]*\([[:alnum:]_.-]*\).*/\2/p' "$workflow")
unbuilt=$(grep -vxF -f <(printf '%s\n' "$built") <<<"$shim_targets" | sed 's/.*/clm-&.tar.gz/' | paste -sd ' ' - || true)
[ -z "$unbuilt" ] ||
  fail "$shim downloads assets that $workflow doesn't build: $unbuilt"

tag=v$version
releases=$(gh release list --limit 1000 --json tagName --jq '.[].tagName') ||
  fail "can't list releases"
if grep -qxF "$tag" <<<"$releases"; then fail "release $tag already exists"; fi
if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then fail "tag $tag already exists"; fi
status=0
git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null || status=$?
case $status in
  0) fail "tag $tag already exists on origin" ;;
  2) ;;
  *) fail "can't list origin's tags" ;;
esac

if ((cmp == 0)); then
  # A release without a bump records no corpus entry, so the version's must already be there
  # (release-records-upgrade-corpus).
  corpus=crates/clm-upgrade/corpus/v$version.tar.gz
  [ -f "$corpus" ] || fail "no upgrade corpus entry $corpus for the unbumped version $version"
  echo "sha=$GITHUB_SHA" >>"$GITHUB_OUTPUT"
  echo "bumped=false" >>"$GITHUB_OUTPUT"
  exit 0
fi

sed -i "/^\[workspace.package\]/,/^\[/s/^version = \".*\"/version = \"$version\"/" "$cargo_toml"
[ "$(workspace_version)" = "$version" ] || fail "couldn't set the version in $cargo_toml"
# Both plugins, so `claude plugin update` delivers each (plugin.panel/panel-package).
for json in "$plugin_json" "$panel_json"; do
  sed -i "0,/\"version\": *\"[^\"]*\"/s//\"version\": \"$version\"/" "$json"
  grep -q "\"version\": \"$version\"" "$json" || fail "couldn't set the version in $json"
done
cargo update --workspace
cargo test --all

git add "$cargo_toml" Cargo.lock "$plugin_json" "$panel_json"
git -c user.name='github-actions[bot]' \
  -c user.email='41898282+github-actions[bot]@users.noreply.github.com' \
  commit -q -m "Release $tag"
git push --force origin "HEAD:refs/heads/release/$tag"

echo "sha=$(git rev-parse HEAD)" >>"$GITHUB_OUTPUT"
echo "bumped=true" >>"$GITHUB_OUTPUT"
