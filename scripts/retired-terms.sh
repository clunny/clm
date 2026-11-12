#!/bin/sh
# Scan the text CLM ships or works from for retired terms (clm/retired-terms).
#
#   sh scripts/retired-terms.sh [--list <file>] [<path>...]
#
# Run from the repository root. Reads the retired terms from <file> (default
# scripts/retired-terms.txt), one per line, and searches each <path>, a file or a directory
# read recursively, or with no path the default text below. Prints `<path>:<line>: <term>` for
# each term a line holds as a whole word, and exits 1 when it printed any, 0 when none, 2 when
# the list can't be read or a search fails. Needs only sh and grep.

list=scripts/retired-terms.txt
if [ "${1-}" = "--list" ]; then
    if [ $# -lt 2 ]; then
        echo "retired-terms: --list needs a file" >&2
        exit 2
    fi
    list=$2
    shift 2
fi

if [ ! -f "$list" ] || [ ! -r "$list" ]; then
    echo "retired-terms: cannot read the list: $list" >&2
    exit 2
fi

tab=$(printf '\t')
nl='
'

# Each term as `<pattern><TAB><term>` lines in PAIRS, and the patterns alone in PATS.
PAIRS=
PATS=
while IFS= read -r line || [ -n "$line" ]; do
    term=$(printf '%s' "$line" | tr -d '\r' | sed 's/^[[:blank:]]*//; s/[[:blank:]]*$//; s/[[:blank:]][[:blank:]]*/ /g')
    case $term in
    '' | '#'*) continue ;;
    esac
    esc=$(printf '%s' "$term" | sed 's/[].[*^$+?(){}|\\]/\\&/g; s/ /[[:blank:]]+/g')
    pat="(^|[^[:alnum:]_])${esc}([^[:alnum:]_]|\$)"
    PAIRS="${PAIRS}${pat}${tab}${term}${nl}"
    PATS="${PATS}${pat}${nl}"
done <"$list"

if [ -z "$PATS" ]; then
    exit 0
fi
# A trailing newline would add an empty pattern, which matches every line.
PATS=${PATS%"$nl"}

found=0

# Print the lines of file $1 that hold a term, as `<file>:<line>: <term>`.
scan_file() {
    matches=$(grep -n -I -i -E -e "$PATS" -- "$1")
    rc=$?
    if [ "$rc" -gt 1 ]; then
        echo "retired-terms: search failed: $1" >&2
        exit 2
    fi
    [ "$rc" -eq 0 ] || return 0
    printf '%s\n' "$matches" | grep -v 'clm:remove-by' | while IFS= read -r hit; do
        n=${hit%%:*}
        text=${hit#*:}
        printf '%s' "$PAIRS" | while IFS="$tab" read -r pat term; do
            if printf '%s\n' "$text" | grep -q -i -E -e "$pat"; then
                printf '%s:%s: %s\n' "$1" "$n" "$term"
            fi
        done
    done
}

report() {
    out=$(scan_file "$1") || exit 2
    if [ -n "$out" ]; then
        printf '%s\n' "$out"
        found=1
    fi
}

walk() {
    dir=${1%/}
    for entry in "$dir"/* "$dir"/.[!.]*; do
        if [ -d "$entry" ]; then
            walk "$entry"
        elif [ -f "$entry" ]; then
            report "$entry"
        fi
    done
}

visit() {
    if [ -d "$1" ]; then
        walk "$1"
    else
        report "$1"
    fi
}

if [ $# -gt 0 ]; then
    for path in "$@"; do
        if [ ! -e "$path" ]; then
            echo "retired-terms: no such path: $path" >&2
            exit 2
        fi
        visit "$path"
    done
else
    for path in docs/guide plugin specs web/src docs/spec-format.md ARCHITECTURE.md CAPABILITIES.md CLAUDE.md AGENTS.md README.md; do
        [ -e "$path" ] || continue
        visit "$path"
    done
fi

[ "$found" -eq 0 ] || exit 1
