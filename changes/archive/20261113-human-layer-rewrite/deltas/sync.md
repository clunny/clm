---
component: sync
base: { frontmatter: "fc97e4004adb39e6" }
---

## MODIFIED Frontmatter

```yaml
serves: [capability/work-concurrently, architecture/hash-based-staleness]
```

## MODIFIED Requirements

### Requirement: autosync <!-- base: 9720eb377a5a376a -->
On a branch named `spec/*`, `clm autosync` SHALL push to `refs/heads/autosync/<branch>` on `origin` a commit whose parent is `HEAD` and whose tree is `HEAD`'s with the spec paths taken from the working tree, untracked files included, leaving the working tree, the index and the branch unchanged. It SHALL include no file outside the spec paths. It SHALL push nothing on any other branch, or when the snapshot's tree equals that of `origin/<branch>` or `origin/autosync/<branch>`.

#### Scenario: uncommitted delta
- GIVEN branch `spec/x` with an uncommitted new file `changes/x/deltas/a.md` and an untracked `notes.txt` at the root
- WHEN `clm autosync` runs
- THEN `origin/autosync/spec/x` holds `changes/x/deltas/a.md` and not `notes.txt`, and `git status` is as before

#### Scenario: nothing new
- GIVEN branch `spec/x` just after a `clm autosync` with no edits since
- WHEN `clm autosync` runs
- THEN it pushes nothing

#### Scenario: other branch
- GIVEN branch `impl/x/a` with uncommitted spec edits
- WHEN `clm autosync` runs
- THEN it pushes nothing

#### Serves
- capability/work-concurrently

### Requirement: file-reconsider-item <!-- base: 1f67b16bcdd73338 -->
On a branch `impl/<component>`, after archiving and filing proposals, `clm rebase` SHALL, for each requirement of `<component>` whose reconsider score (`check.health/reconsider-score`) reaches the `reconsider` setting and for which no `backlog/reconsider-<component>-<slug>.md` exists in the working tree, tracked or not, write the item `check.health`'s `reconsider` returns (`title: "Reconsider <component>/<slug>"`, `kind: question`, `components: [<component>]`, a body with the score, its evidence and the three ways out), commit it alone as `File reconsider-<component>-<slug>`, and print `filed backlog/reconsider-<component>-<slug>.md`. It SHALL NOT file an item on any other branch, for a requirement of another component, for a requirement below the threshold, or when the item exists, and SHALL NOT edit or delete an existing item. When the settings couldn't be loaded it SHALL file nothing, `judged-by-check` reporting why. The score SHALL NOT change its outcome line or exit code.

#### Scenario: requirement at the threshold
- GIVEN branch `impl/a`, the default `reconsider` of 3, and requirement `a/x` with three committed `workaround` items naming it at its current version
- WHEN `clm rebase` runs
- THEN it prints `filed backlog/reconsider-a-x.md`, `HEAD` is a commit `File reconsider-a-x` adding `backlog/reconsider-a-x.md` with `kind: question` and `components: [a]` whose body holds `score 3` and the three items' paths, and it exits 0

#### Scenario: item already filed
- GIVEN the same branch, with `backlog/reconsider-a-x.md` at `HEAD`
- WHEN `clm rebase` runs
- THEN it prints no `filed backlog/reconsider-a-x.md` line and the item is unchanged

#### Scenario: below the threshold
- GIVEN branch `impl/a` and requirement `a/x` with two `workaround` items
- WHEN `clm rebase` runs
- THEN it prints no `filed` line for `reconsider-a-x`

#### Scenario: another node's requirement
- GIVEN branch `impl/a`, where `b/y` has three `workaround` items
- WHEN `clm rebase` runs
- THEN it prints no `filed` line and `HEAD` has no `backlog/reconsider-b-y.md`

#### Scenario: spec branch
- GIVEN branch `spec/y` and requirement `a/x` with three `workaround` items
- WHEN `clm rebase` runs
- THEN it prints no `filed` line

#### Serves
- capability/know-health

#### Needs
- check.health/reconsider-score
- config/health-setting
- model/component-keys
