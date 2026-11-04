#!/bin/sh
# Makes this clone of CLM's repository run main's `clm` and the checkout's plugin, where it
# would otherwise run the release (dogfood-from-main, dev-install-script). A human runs it
# once per clone; builds never run it (no-shared-install).
#
#   scripts/dev-install.sh [--force]   set up: install clm, the reference-transaction hook, the settings
#   scripts/dev-install.sh --rebuild   what the hook runs: rebuild clm from origin/main in the background
#   scripts/dev-install.sh --remove    undo the hooks and the settings
set -eu

MARK='# clm dev-install'
INSTALL='cargo install --locked --path crates/clm --target-dir target/install'

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
bin=${CARGO_INSTALL_ROOT:-${CARGO_HOME:-$HOME/.cargo}}/bin/clm
# cargo runs in the main worktree, so a relative install root is relative to it.
case $bin in /*) ;; *) bin=$main/$bin ;; esac

# Whether the hook file $1 is one this script wrote.
ours() { [ -f "$1" ] && grep -qxF "$MARK" "$1"; }

# Whether `$settings` is missing, empty, or one JSON object whose `env` and
# `extraKnownMarketplaces`, when present, are objects: what `edit_settings` can edit.
settings_ok() {
	[ ! -f "$settings" ] || jq -se 'length <= 1 and ((.[0] // {}) | type == "object"
		and ((.env // {}) | type == "object")
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
			(cd "$main" && cargo install --locked --path "$src/crates/clm" --target-dir "$main/target/install") || status=$?
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
	for h in "$hook" "$old_hook"; do
		if ours "$h"; then
			rm -f "$h"
			say "removed the hook $h"
		elif [ -f "$h" ]; then
			say "left $h, which dev-install didn't write"
		fi
	done
	if [ -f "$settings" ]; then
		command -v jq >/dev/null 2>&1 || die "jq is not on PATH"
		settings_ok || die "$settings isn't a JSON object CLM can edit"
		edit_settings 'del(.env.CLM_BIN) | del(.extraKnownMarketplaces.clm)
			| if .env == {} then del(.env) else . end
			| if .extraKnownMarketplaces == {} then del(.extraKnownMarketplaces) else . end'
		say "removed CLM_BIN and the clm marketplace from $settings"
	fi
	say "$bin is still installed"
	say "Claude Code's user-level marketplace record still points at the checkout until you run: claude plugin marketplace add clunny/clm"
	exit 0
fi

# 1. Refuse before changing anything.
command -v jq >/dev/null 2>&1 || die "jq is not on PATH"
command -v cargo >/dev/null 2>&1 || die "cargo is not on PATH"
jq -e '.name == "clm"' "$main/.claude-plugin/marketplace.json" >/dev/null 2>&1 ||
	die "$main has no .claude-plugin/marketplace.json named clm: is this a clone of CLM's repository?"
branch=$(cd "$main" && git symbolic-ref --short -q HEAD || true)
[ "$branch" = main ] || die "the main worktree $main is on ${branch:-a detached HEAD}, not main"
settings_ok || die "$settings isn't a JSON object CLM can edit"
if [ -e "$hook" ] && ! ours "$hook" && [ -z "$force" ]; then
	die "a reference-transaction hook dev-install didn't write is at $hook; run with --force to replace it"
fi

# 2. Install main's clm.
say "installing clm from $main: $INSTALL"
status=0
(cd "$main" && $INSTALL) || status=$?
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

# 4. The settings point Claude Code at this clm and this checkout's plugin.
mkdir -p "$main/.claude"
edit_settings '.env.CLM_BIN = $bin | .extraKnownMarketplaces.clm = {source: {source: "directory", path: $path}}' \
	--arg bin "$bin" --arg path "$main"
say "set env.CLM_BIN to $bin and the clm marketplace to $main in $settings"

# 5. What runs now.
say "installed: $("$bin" --version 2>/dev/null || echo "$bin (couldn't run --version)")"
say "the next Claude Code session loads the clm plugin from $main"
say "clm is rebuilt from origin/main whenever it moves"
