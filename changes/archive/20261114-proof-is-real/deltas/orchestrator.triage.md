---
component: orchestrator.triage
---

## MODIFIED Requirements

### Requirement: triage <!-- base: 91b10ab7b8c41e3d -->
`clm triage` SHALL fetch `origin` and print, sorted as the Design says (`group` lines first, `deferred` lines next to last and `workaround` lines last), one `group` line per group of untaken `bug` items with the same sorted `components`, cut into groups of at most the draft budget; for each proposed or on-main change that names backlog items one `designed` line when it is proposed and its `change.md` has `status: applied`, one `open` line when it is proposed otherwise, and one `main` line when it is on main; one `human` line per untaken `question` or `human-task` item; one `idea` line per untaken `idea` item; one `deferred` line per untaken `deferred` item; and one `workaround` line per untaken `workaround` item (`workaround-flags`); each as `<form> <name> [<components>] <P> <n> <slug>...`, with `<P>` the highest priority of its items and `<n>` the number of distinct requirements in their `blocks`. An item held only by a claim, an untaken item of a kind no form takes, and a `bug` item that holds `unproven` entries, which its component's build resolves by mending the test (`verify/files-unproven`), SHALL appear on no line. It SHALL exit 0, and 2 when git fails.

#### Scenario: groups by kind and nodes
- GIVEN untaken items `a` and `b` of kind `bug` with `components: [check]`, `c` of kind `idea` with `components: [check]`, `d` of kind `deferred` with `components: [check]`, and `e` of kind `bug` with no `components`
- WHEN `clm triage` runs
- THEN it prints, in this order, `group bug: [] P5 0 e`, `group bug:check [check] P5 0 a b`, `idea c [check] P5 0` and `deferred d [check] P5 0`, and no other line

#### Scenario: budget cuts a group
- GIVEN eight untaken `bug` items `a` to `h` with `components: [check]`, where `g` has `priority: P2`
- WHEN `clm triage` runs
- THEN it prints two `group bug:check` lines, the first `P2` with `g a b c d e` and the second `P5` with `f h`

#### Scenario: releases and order
- GIVEN untaken `idea` item `q` with `components: [gate]` and `blocks: [gate/x, gate/y]`, untaken `idea` item `r` with `components: [check]` and no `blocks`, untaken `question` item `s` with `components: [model]` and `blocks: [model/z]`, and untaken `bug` item `t` with `components: [view]`
- WHEN `clm triage` runs
- THEN it prints, in this order, `group bug:view [view] P5 0 t`, `human s [model] P5 1`, `idea q [gate] P5 2` and `idea r [check] P5 0`

#### Scenario: open and taken work
- GIVEN `origin/spec/20260101-c` whose `change.md` has `status: proposed` and `resolves: [q]` with a delta for `check`, where `q` is an `idea` with `components: [model]` and `blocks: [model/z]`; `origin/propose/t` committed 1 hour ago, where `t` is a `bug`; and untaken `bug` item `u`
- WHEN `clm triage` runs
- THEN it prints a `group` line for `u` first, then `open 20260101-c [check,model] P5 1 q`, and no line names `t`

#### Scenario: designed and on main
- GIVEN `origin/spec/20260101-d` whose `change.md` has `status: applied` and `resolves: [x]` with a delta for `check`, `origin/main` holding `changes/20260101-m/change.md` with `status: applied` and `resolves: [y]` and a delta for `gate`, where `x` and `y` are untaken `bug` items with no `components`, and untaken `bug` item `u`
- WHEN `clm triage` runs
- THEN it prints, in this order, a `group` line for `u`, `designed 20260101-d [check] P5 0 x` and `main 20260101-m [gate] P5 0 y`

#### Scenario: deferred and workaround come last
- GIVEN `origin/main` whose component `check` has a requirement `w`, an untaken item `f` of kind `workaround` with `components: [check]` and `requirement: check/w@<version>`, where `<version>` is `w`'s current version, an untaken `deferred` item `k` with `components: [gate]`, an untaken `idea` item `i` with no `components`, and an untaken `bug` item `u`
- WHEN `clm triage` runs
- THEN it prints, in this order, a `group` line for `u`, `idea i [] P5 0`, `deferred k [gate] P5 0` and `workaround f [check] P5 0`

#### Scenario: an unproven item is no draft
- GIVEN an untaken `bug` item on `a` with `unproven: [a/x@0123abcd]`
- WHEN `clm triage` runs
- THEN no line names it

#### Needs
- model/parse-backlog
