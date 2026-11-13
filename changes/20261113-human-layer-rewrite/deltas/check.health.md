---
component: check.health
base: { frontmatter: "a93e9f83c1937033" }
---

## MODIFIED Frontmatter

```yaml
serves: [capability/know-health]
```

## MODIFIED Requirements

### Requirement: exemptions-signal <!-- base: f1b32bd2a6ed4e60 -->
`clm health` SHALL report, under `exemption`, each exemption a component holds (`model/exemptions`), in component ID then written order, as `specs/<component path>/SPEC.md: health/exemption: <component>: <rule> until backlog/<slug>: <n> files`, `<n>` the number of tracked files the component owns (`check.coverage/owner`) that its `files` match, followed by `; item gone` when `backlog/<slug>.md` doesn't exist and `; matches nothing` when `<n>` is 0, so an exemption whose work is done, or whose files are gone, shows as one to remove (`temporary-exemptions`). The summary line SHALL count the exemptions, as `health/exemption: <n> exemptions`.

#### Scenario: exemption being paid down
- GIVEN component `a` owning `tests/a/x.py` and `tests/a/y.py`, with an exemption from `undeclared-imports` for `tests/a/**` until `backlog/r`, and `backlog/r.md`
- WHEN `clm health` runs
- THEN it prints `specs/a/SPEC.md: health/exemption: a: undeclared-imports until backlog/r: 2 files`

#### Scenario: work done
- GIVEN the same exemption, with no `backlog/r.md` and no file under `tests/a/`
- WHEN `clm health` runs
- THEN it prints `specs/a/SPEC.md: health/exemption: a: undeclared-imports until backlog/r: 0 files; item gone; matches nothing`

#### Serves
- architecture/exemptions-end
- capability/know-health

#### Needs
- model/exemptions
