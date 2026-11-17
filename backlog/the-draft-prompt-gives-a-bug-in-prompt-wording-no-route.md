---
title: "The draft prompt gives a bug in prompt wording no route"
kind: bug
components: [orchestrator.triage]
origin: 20261116-a-build-whose-only
---

The draft prompt `clm draft` writes (`crates/clm-triage/src/draft.rs`) calls a bug code-only only when a requirement or scenario as written states the behaviour its fix restores, and sends every other bug to a spec change that adds a requirement (`code-fix-needs-a-written-rule`). A bug in the fixed text of a prompt or skill can't take either route: `prompt-text-is-source` says that text is source and that no requirement may state it, so no requirement exists to cite and none may be added. A draft meeting such a bug has to work out for itself that the fix is code with no requirement behind it. The way out is a `deferred` item resolved with a `Resolves:` trailer, or a fix by component under `clm claim --fix`, and the prompt names neither for this case. Met while drafting `20261116-a-build-whose-only`, whose item said its prompt fix "likely needs a spec change".
