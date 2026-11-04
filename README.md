# CLM

A Claude Code plugin for spec-driven development, solo or as a team. Site: <https://clm-app.dev>.

Humans own the spec and design. AI writes all the code. Deterministic checks keep the two in sync, and git is the only channel between people and sessions.

## Who it is for

People, solo or in a team, who build with AI sessions and want to review intent, not code: product owners, architects and developers who would rather settle what a system must do than read every diff.

## Why it is useful

- Each session gets context scoped to one component and its ancestors, not the whole repository.
- It writes per-component contracts that say what each part promises its neighbours.
- Every requirement is proven by a test that cites it, and a check blocks drift between spec and code.
- Specs, claims, approvals and lands are committed files and branches: no server owns them.

CLM implements the *Spec Growth Engine* ([arXiv:2606.27045](https://arxiv.org/html/2606.27045v2)). Its loop is a [controller](https://kubernetes.io/docs/concepts/architecture/controller/)'s, reconciling code to the declared spec, its team workflow draws on the [AI-native SDLC playbook](https://claude.com/blog/the-ai-native-sdlc-playbook), and its builds follow [superpowers](https://github.com/obra/superpowers).

## Install

In Claude Code:

```
/plugin marketplace add clunny/clm
/plugin install clm@clm
```

Then follow the [quickstart](docs/guide/quickstart.md).

## How a change flows

1. Someone states an intent, and a session drafts it as a spec branch: a change with deltas to the component specs.
2. A human reviews the intent when the change needs it, and approves it.
3. The branch lands on `main` through `clm land`, which runs the checks.
4. Sessions build each component until every requirement has a passing test citing it.
5. That code lands the same way: checked by the land checks and reviewed by AI, not read by a human.

## Status

CLM is young and changes quickly. Expect rough edges and changes to the commands and formats.

## Contributing

CLM takes contributions of intent. See [CONTRIBUTING.md](CONTRIBUTING.md), and [.github/SECURITY.md](.github/SECURITY.md) for reporting a vulnerability.

## Learn more

- [docs/guide/](docs/guide/README.md): the user guide, from why CLM to a quickstart
- [ARCHITECTURE.md](ARCHITECTURE.md): the architectural constraints (and [CAPABILITIES.md](CAPABILITIES.md): the capabilities)
- [docs/spec-format.md](docs/spec-format.md): how specs and changes are written
- [docs/decisions/](docs/decisions/): one file per decision
- [backlog/](backlog/): ideas, deferred work, open questions (`clm backlog`)
- [specs/](specs/): CLM's own spec. CLM is built with CLM.
- [changes/](changes/): proposed changes to the spec, in flight

## Licence

MIT, see [LICENSE](LICENSE).
