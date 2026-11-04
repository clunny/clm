# CLM

A Claude Code plugin for spec-driven development by teams.

Humans own the spec and design. AI writes all the code. Deterministic checks keep the two in sync, and git is the only channel between people and sessions.

CLM implements the *Spec Growth Engine* ([arXiv:2606.27045](https://arxiv.org/html/2606.27045v2)), which gives it context scoped to a component and its ancestors, per-component contracts and blocking drift checks. Its team workflow draws on the [AI-native SDLC playbook](https://claude.com/blog/the-ai-native-sdlc-playbook).

- [docs/guide/](docs/guide/README.md): the user guide, from why CLM to a quickstart
- [ARCHITECTURE.md](ARCHITECTURE.md): the architectural constraints (and [CAPABILITIES.md](CAPABILITIES.md): the capabilities)
- [docs/spec-format.md](docs/spec-format.md): how specs and changes are written
- [docs/decisions/](docs/decisions/): one file per decision
- [backlog/](backlog/): ideas, deferred work, open questions (`clm backlog`)
- [specs/](specs/): CLM's own spec. CLM is built with CLM.
- [changes/](changes/): proposed changes to the spec, in flight
