---
component: plugin.hooks
base: { frontmatter: "03f48184e56b6cb7" }
---

## MODIFIED Frontmatter

```yaml
serves: [capability/guard-sessions, capability/work-from-any-agent-harness, architecture/approval-policy, architecture/land-locally, architecture/agent-harness-adapters]
```

## MODIFIED Requirements

### Requirement: autosync-hook <!-- base: 3a75312dc930969d -->
`clm hook autosync` SHALL find the CLM repository for the event's `cwd` as `session-start` does and run `clm autosync` there, unless `config`'s `autosync` setting for that repository is `false` or can't be loaded. It SHALL print nothing and exit 0 in every case, including when there is no repository, the push fails, or its input can't be parsed.

#### Scenario: spec branch
- GIVEN a `Stop` event whose `cwd` is a CLM repository on branch `spec/x` with an uncommitted delta
- WHEN `clm hook autosync` runs
- THEN `origin/autosync/spec/x` holds the delta, it prints nothing, and it exits 0

#### Scenario: turned off
- GIVEN the same repository and `CLM_AUTOSYNC=0`
- WHEN `clm hook autosync` runs
- THEN it pushes nothing and exits 0

#### Scenario: turned off in the repository
- GIVEN the same repository with a `clm.toml` that sets `autosync = false`
- WHEN `clm hook autosync` runs
- THEN it pushes nothing and exits 0

#### Scenario: push fails
- GIVEN a CLM repository on `spec/x` whose `origin` can't be reached
- WHEN `clm hook autosync` runs
- THEN it prints nothing and exits 0

#### Serves
- capability/wrap-up-session
- capability/work-concurrently
