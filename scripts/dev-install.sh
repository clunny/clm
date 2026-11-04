#!/bin/sh
# Makes this clone of CLM's repository, and its worktrees, run main's `clm` and the checkout's
# plugin, and no other project on the machine (dogfood-from-main, dev-install-scoped). A human
# runs it once per clone; builds never run it (no-shared-install).
#
#   scripts/dev-install.sh [--force]   set up: install clm, the hook, the plugin marketplace, the settings
#   scripts/dev-install.sh --rebuild   what the hook runs: rebuild clm from origin/main in the background
#   scripts/dev-install.sh --remove    undo the setup
set -eu

MARK='# clm dev-install'
DEV_MARKET=clm-dev

say() { printf 'dev-install: %s\n' "$*"; }
die() {
	printf 'dev-install: %s\n' "$*" >&2
	exit 1
}

mode=install
force=
for arg in "$@"; do
	case $arg in
	--rebuild) mode=rebuild ;;
	--remove) mode=remove ;;
	--force) force=1 ;;
	*) die "unknown argument: $arg" ;;
	esac
done

# The clone's main worktree: the first one `git worktree list` lists, whichever worktree
# this runs from, since Claude Code reads `.claude/settings.local.json` only there. Found,
# with the common git directory, before anything else: the hook runs this with git's own
# GIT_DIR set.
main=$(git worktree list --porcelain | sed -n '1s/^worktree //p')
[ -n "$main" ] || die "not in a git repository"
common=$(git rev-parse --path-format=absolute --git-common-dir)
self=$(cd "$(dirname "$0")" && pwd)/$(basename "$0")
hook=$(cd "$main" && git rev-parse --path-format=absolute --git-path hooks/reference-transaction)
old_hook=$(cd "$main" && git rev-parse --path-format=absolute --git-path hooks/post-merge)
settings=$main/.claude/settings.local.json
dev=$common/clm-dev
tracked_market=$main/.claude-plugin/marketplace.json

# Whether the hook file $1 is one this script wrote.
ours() { [ -f "$1" ] && grep -qxF "$MARK" "$1"; }

# Whether `$settings` is missing, empty, or one JSON object whose `env`, `enabledPlugins` and
# `extraKnownMarketplaces`, when present, are objects: what `edit_settings` can edit.
settings_ok() {
	[ ! -f "$settings" ] || jq -se 'length <= 1 and ((.[0] // {}) | type == "object"
		and ((.env // {}) | type == "object")
		and ((.enabledPlugins // {}) | type == "object")
		and ((.extraKnownMarketplaces // {}) | type == "object"))' "$settings" >/dev/null 2>&1
}

# Rewrites `$settings` (`{}` when it's missing or empty) with what jq's filter $1 makes of it,
# the remaining arguments going to jq before the filter (`--arg`),
# keeping the file's mode. jq's output is the same for the same input, so a second run leaves
# the file's content as it was.
edit_settings() {
	tmp=$(mktemp "$settings.XXXXXX")
	if [ -f "$settings" ]; then src=$settings; else src=/dev/null; fi
	filter=$1
	shift
	if ! jq -s "$@" "(.[0] // {}) | $filter" "$src" >"$tmp"; then
		rm -f "$tmp"
		die "couldn't update $settings"
	fi
	if [ -f "$settings" ]; then
		cat "$tmp" >"$settings"
		rm -f "$tmp"
	else
		mv "$tmp" "$settings"
	fi
}

# The names of the plugins the tracked marketplace lists, one per line.
plugin_names() { jq -r '.plugins[]?.name' "$tracked_market" 2>/dev/null || true; }

# The directory $1 with symlinks resolved, or nothing when there is none.
physical() { (cd "$1" 2>/dev/null && pwd -P) || true; }

# Where Cargo installs: its install root's file `.crates.toml` records what it installed.
cargo_root=${CARGO_INSTALL_ROOT:-${CARGO_HOME:-$HOME/.cargo}}

# Runs `claude` with the arguments in the main worktree, saying so.
claude_in_main() {
	say "claude $*"
	(cd "$main" && claude "$@") || return 1
}

# Step 5: an earlier setup's directory marketplace `clm` at this checkout goes back to GitHub.
# $1 is the output of `claude plugin marketplace list --json`; $2 is `die` or `warn`.
retire_directory_marketplace() {
	mainp=$(physical "$main")
	found=$(printf '%s' "$1" | jq -r '.[] | select(.name == "clm" and .source == "directory") | .path // empty' |
		while IFS= read -r p; do
			if [ "$(physical "$p")" = "$mainp" ]; then
				echo yes
				break
			fi
		done)
	[ -n "$found" ] || return 0
	if claude_in_main plugin marketplace remove clm && claude_in_main plugin marketplace add clunny/clm; then
		say "each project that enables the plugin installs it again at its next session"
	elif [ "$2" = die ]; then
		die "claude plugin marketplace remove clm or add clunny/clm failed"
	else
		say "claude couldn't move the clm marketplace back to GitHub; going on"
	fi
}

# Step 8: a `cargo install` of this checkout, from an earlier setup, goes; a `clm` still on PATH
# that isn't a plugin's shim is named.
retire_cargo_install() {
	if grep -q '^"clm [^ ]* (path+file:' "$cargo_root/.crates.toml" 2>/dev/null; then
		say "cargo uninstall clm"
		cargo uninstall clm || say "cargo uninstall clm failed; going on"
	fi
	left=$(command -v clm 2>/dev/null || true)
	if [ -n "$left" ] && [ ! -e "$(dirname "$left")/../.claude-plugin/plugin.json" ]; then
		say "$left is still on PATH and runs ahead of the plugin's shim in every project"
	fi
}

# The installed binary the rebuild replaces: the settings' CLM_BIN when it ends /bin/clm (a clone
# set up before dev-install-scoped), else the one under clm-dev.
installed_bin() {
	b=
	if [ -f "$settings" ] && command -v jq >/dev/null 2>&1; then
		b=$(jq -r '.env.CLM_BIN // empty' "$settings" 2>/dev/null || true)
	fi
	case $b in */bin/clm) ;; *) b=$dev/bin/clm ;; esac
	printf '%s' "$b"
}

# origin/main's commit, or nothing and a failure when the clone has none.
origin_main() { git -C "$main" rev-parse -q --verify refs/remotes/origin/main 2>/dev/null; }

# Whether the installed binary is built from commit $1, or from one that changed no Rust code
# since: a missing binary, one naming no commit, or a commit the clone lacks is not current.
current() {
	ver=$("$bin" --version 2>/dev/null || true)
	short=$(printf '%s' "$1" | cut -c1-12)
	case $ver in *"($short)") return 0 ;; esac
	built=$(printf '%s' "$ver" | sed -n 's/.*(\([0-9a-f]*\))$/\1/p')
	[ -n "$built" ] || return 1
	changed=$(git -C "$main" diff --name-only "$built" "$1" -- crates Cargo.toml Cargo.lock 2>/dev/null) || return 1
	[ -z "$changed" ]
}

# The background build: builds origin/main's commit from clm-install-src, again while
# origin/main has moved on, then frees the lock (rebuild-on-origin-main).
rebuild_loop() {
	src=$common/clm-install-src
	failed=
	c=
	while :; do
		c=$(origin_main) || break
		if ! current "$c"; then
			echo "building $c"
			if [ ! -d "$src" ]; then
				git clone -q --shared --no-checkout "$main" "$src" || { failed=1; break; }
			fi
			git -C "$src" -c advice.detachedHead=false checkout -q --detach "$c" || { failed=1; break; }
			status=0
			(cd "$main" && cargo install --locked --path "$src/crates/clm" --root "$root" --target-dir "$main/target/install") || status=$?
			if [ "$status" -ne 0 ]; then
				echo "cargo install failed with exit status $status"
				failed=1
				break
			fi
		fi
		[ "$(origin_main || true)" != "$c" ] || break
	done
	rm -rf "$common/clm-install.lock"
	# A move that came while the lock was being released.
	if [ -z "$failed" ] && [ -n "$c" ] && [ "$(origin_main || true)" != "$c" ]; then
		exec "$self" --rebuild
	fi
}

if [ "$mode" = rebuild ]; then
	origin_main >/dev/null || exit 0
	lock=$common/clm-install.lock
	if [ -d "$lock" ]; then
		pid=$(cat "$lock/pid" 2>/dev/null || true)
		[ -n "$pid" ] || exit 0
		! kill -0 "$pid" 2>/dev/null || exit 0
		rm -rf "$lock"
	fi
	mkdir "$lock" 2>/dev/null || exit 0
	log=$common/clm-install.log
	bin=$(installed_bin)
	root=${bin%/bin/clm}
	# The build's own `git` calls (build.rs) must find the repository from the crate's
	# directory, not through the GIT_DIR and the like that git set for the hook.
	unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_PREFIX
	# `exec` redirects for good: a redirection on the call would leave the shell holding the
	# original stdout, and whoever waits on it would wait for the build.
	(
		exec >>"$log" 2>&1 </dev/null
		# This subshell's own process ID, which `$$` doesn't give in a subshell.
		sh -c 'echo $PPID' >"$lock/pid"
		rebuild_loop
	) &
	say "rebuilding clm in the background; see $log"
	exit 0
fi

if [ "$mode" = remove ]; then
	command -v jq >/dev/null 2>&1 || die "jq is not on PATH"
	have_claude=
	command -v claude >/dev/null 2>&1 && have_claude=1
	if [ -d "$dev" ] && [ -z "$have_claude" ]; then
		die "claude is not on PATH, and $dev holds the plugin marketplace it must remove"
	fi
	if [ -f "$settings" ]; then
		settings_ok || die "$settings isn't a JSON object CLM can edit"
	fi
	for h in "$hook" "$old_hook"; do
		if ours "$h"; then
			rm -f "$h"
			say "removed the hook $h"
		elif [ -f "$h" ]; then
			say "left $h, which dev-install didn't write"
		fi
	done
	names=$(plugin_names)
	if [ -d "$dev" ]; then
		for n in $names; do
			claude_in_main plugin uninstall "$n@$DEV_MARKET" --scope local || say "going on"
		done
		claude_in_main plugin marketplace remove "$DEV_MARKET" || say "going on"
		rm -rf "$dev"
		say "removed $dev"
	fi
	if [ -f "$settings" ]; then
		names_json=$(printf '%s\n' "$names" | jq -R . | jq -sc 'map(select(. != ""))')
		edit_settings 'del(.env.CLM_BIN) | del(.extraKnownMarketplaces.clm)
			| (if .enabledPlugins then .enabledPlugins |= with_entries(select(.key | endswith("@clm-dev") | not)) else . end)
			| reduce $names[] as $n (.; if .enabledPlugins[$n + "@clm"] == false then del(.enabledPlugins[$n + "@clm"]) else . end)
			| if .env == {} then del(.env) else . end
			| if .enabledPlugins == {} then del(.enabledPlugins) else . end
			| if .extraKnownMarketplaces == {} then del(.extraKnownMarketplaces) else . end' \
			--argjson names "$names_json"
		say "removed CLM_BIN and the plugin entries from $settings"
	fi
	if [ -n "$have_claude" ]; then
		list=$(cd "$main" && claude plugin marketplace list --json 2>/dev/null) || list='[]'
		printf '%s' "$list" | jq -e 'type == "array"' >/dev/null 2>&1 || list='[]'
		retire_directory_marketplace "$list" warn
	fi
	retire_cargo_install
	say "the clone's next session runs the release"
	exit 0
fi

# 1. Refuse before changing anything.
command -v jq >/dev/null 2>&1 || die "jq is not on PATH"
command -v cargo >/dev/null 2>&1 || die "cargo is not on PATH"
command -v claude >/dev/null 2>&1 || die "claude is not on PATH"
jq -e '.name == "clm"' "$tracked_market" >/dev/null 2>&1 ||
	die "$main has no .claude-plugin/marketplace.json named clm: is this a clone of CLM's repository?"
jq -e '[.plugins[]? | (.name | type == "string") and (.source | type == "string" and test("^\\./."))] | all' "$tracked_market" >/dev/null 2>&1 ||
	die "$tracked_market lists a plugin without a name or whose source doesn't start with ./"
branch=$(cd "$main" && git symbolic-ref --short -q HEAD || true)
[ "$branch" = main ] || die "the main worktree $main is on ${branch:-a detached HEAD}, not main"
settings_ok || die "$settings isn't a JSON object CLM can edit"
list=$(cd "$main" && claude plugin marketplace list --json 2>/dev/null) ||
	die "claude plugin marketplace list --json failed"
printf '%s' "$list" | jq -e 'type == "array"' >/dev/null 2>&1 ||
	die "claude plugin marketplace list --json didn't print a JSON array"
if [ -e "$hook" ] && ! ours "$hook" && [ -z "$force" ]; then
	die "a reference-transaction hook dev-install didn't write is at $hook; run with --force to replace it"
fi

# 2. Install main's clm under the clone's git directory.
say "installing clm from $main into $dev"
status=0
(cd "$main" && cargo install --locked --path crates/clm --root "$dev" --target-dir target/install) || status=$?
[ "$status" -eq 0 ] || die "cargo install failed with exit status $status"

# 3. The hook rebuilds when origin/main moves; the earlier post-merge hook of ours goes.
mkdir -p "$(dirname "$hook")"
quoted=$(printf '%s' "$main/scripts/dev-install.sh" | sed "s/'/'\\\\''/g")
tmp=$(mktemp "$hook.XXXXXX")
{
	printf '#!/bin/sh\n%s\n' "$MARK"
	printf '%s\n' '[ "${1:-}" = committed ] || exit 0'
	printf '%s\n' "grep ' refs/remotes/origin/main\$' >/dev/null || exit 0"
	printf "exec '%s' --rebuild\n" "$quoted"
} >"$tmp"
chmod 755 "$tmp"
mv "$tmp" "$hook"
say "wrote the reference-transaction hook $hook"
if ours "$old_hook"; then
	rm -f "$old_hook"
	say "removed the earlier post-merge hook $old_hook"
fi

# 4. The marketplace: the tracked one under another name, each plugin's folder a symlink to the
# main worktree's, so the plugins load in place.
names=$(plugin_names)
mkdir -p "$dev/marketplace/.claude-plugin"
jq --arg name "$DEV_MARKET" '.name = $name' "$tracked_market" >"$dev/marketplace/.claude-plugin/marketplace.json"
jq -r '.plugins[]?.source' "$tracked_market" | while IFS= read -r source; do
	rel=${source#./}
	mkdir -p "$(dirname "$dev/marketplace/$rel")"
	ln -sfn "$main/$rel" "$dev/marketplace/$rel"
done
say "wrote the marketplace $DEV_MARKET in $dev/marketplace"

# 5. An earlier setup's marketplace record goes back to GitHub.
retire_directory_marketplace "$list" die

# 6. Claude Code learns the marketplace and installs its plugins for this clone only.
claude_in_main plugin marketplace add "$dev/marketplace" || die "claude plugin marketplace add $dev/marketplace failed; run the script again to finish"
for n in $names; do
	claude_in_main plugin install "$n@$DEV_MARKET" --scope local ||
		die "claude plugin install $n@$DEV_MARKET --scope local failed; run the script again to finish"
done

# 7. The settings point sessions of this clone at this clm and these plugins.
mkdir -p "$main/.claude"
names_json=$(printf '%s\n' "$names" | jq -R . | jq -sc 'map(select(. != ""))')
edit_settings '.env.CLM_BIN = $bin
	| reduce $names[] as $n (.; .enabledPlugins[$n + "@clm-dev"] = true | .enabledPlugins[$n + "@clm"] = false)
	| del(.extraKnownMarketplaces.clm)
	| if .extraKnownMarketplaces == {} then del(.extraKnownMarketplaces) else . end' \
	--arg bin "$dev/bin/clm" --argjson names "$names_json"
say "set env.CLM_BIN to $dev/bin/clm and enabled the $DEV_MARKET plugins in $settings"

# 8. What an earlier setup left behind.
retire_cargo_install

# 9. What runs now.
say "installed: $("$dev/bin/clm" --version 2>/dev/null || echo "$dev/bin/clm (couldn't run --version)")"
say "the next Claude Code session of this clone and its worktrees loads the plugins from $main"
say "clm is rebuilt from origin/main whenever it moves"
say "sessions that are already running should be restarted"
say "a plain \`clm\` outside a session doesn't find this build"
