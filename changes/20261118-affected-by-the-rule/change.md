---
status: proposed
resolves: [affected-base-leaves-out-exits-its-design-states]
---

## Intent

`check.affected` is brought to the one rule for a requirement in a single pass: each of its requirements states what someone outside the component can observe, and every exit, message and refusal its Contract or Design states is stated by a requirement. It is the pilot of phase 5 of `backlog/a-passing-check-proves-less-than-the-spec-claims`, written by hand to learn what one pass costs before the tooling and the other 50 components. It removes no requirement, no scenario and no test, so nothing in it needs the human's reading.

## Problem

Measured on 2026-11-18, `check.affected` held six sentences in its Contract or Design that state an exit or a message no requirement states, six of its ten requirements named a call, a test's place or a file's text, and two tests in its files cited nothing:

- The Contract says `clm affected` exits 2 "when git fails otherwise", and `overlap-selection` says so "as `affected-base` says for `--base`", but `affected-base` doesn't say it. `a_git_failure_exits_2` proves it and cites nothing.
- The Design says `--readers` reads no graph, and `affected-base` requires exit 1 when the graph can't be loaded with no exception. `readers_alone_ignore_the_graph_and_the_settings` proves the Design and cites nothing.
- The messages `unknown base <ref>` and `no merge base between <ref> and HEAD` are in the Design alone. A test asserts the first while citing `commands-in-crate`, which doesn't state it.
- No requirement gives the word a component line starts with. The command prints `node <id>`, tests assert it, and `affected-components` says only "a component line (the Contract's)".
- The Contract's synopsis leaves out `--overlap`, and names `why(root, base, path)`, which takes the `[affected] all` globs too.
- `commands-in-crate` says where the component's tests "sat" and must sit, `why-chain` and `overlap-selection` state library calls beside the command, and `doc-only-selects-nothing` defines its paths by another crate's function.

The first two are `backlog/affected-base-leaves-out-exits-its-design-states`, a question for the human, answered on 2026-11-18 with the phase's design: a sentence that is relied on becomes a scenario and its test cites it.

## Approaches

- **Taken: one delta for the whole component, by the rule** (`requirements-state-what-the-contract-shows`). Every reword, every newly stated scenario and the Contract's and Design's mending are in one delta, so the component is built once. Scenario names are kept, so a program that lists removed names lists none.
- **Taken: the Contract keeps its summary of the exits.** A dependant's build receives the Contract and not the requirements (`ancestor-scoped-context`), so what `land` and `ci` rely on stays there, and each of its statements is now a requirement's too. The Design loses the sentences that state behaviour.
- **Taken: `git-through-layer` is left as it is.** It is a scan of source text that no entry of `ARCHITECTURE.md` names, held in 17 components. `vcs/one-git-layer` scans every crate that lists `clm-git` but doesn't require a crate to list it, and looks for two calls where this one looks for any process, so removing this copy now would lose both. The wave that holds `vcs` makes the one check cover them and removes the copies together, on one list for the human.
- **Not taken: adding only the two clauses the question asks for.** It is what the four mends before it did, one fault and one build each, and leaves the other four sentences and the wording for a second build.
- **Not taken: narrowing `overlap` and `affected_between` to the crate.** Nothing outside the crate calls them today, but `land`'s Design keeps the second for its overlap re-check and `clm.toml`'s allow list names it, which is not this component's file. The Contract now offers them, as the requirement did.

## Outcome

- `affected-base` states exit 2 for any other git failure, that `--readers` loads no graph, and the two messages. The two uncited tests cite it.
- `affected-components` and `affected-files` give the lines as printed: `node <id>` and `file <path>`.
- `why-chain` and `overlap-selection` state the command; the calls are the Contract's. `commands-in-crate` states what the call does and nothing of where tests sit.
- `doc-only-selects-nothing` names the paths by `ledger/doc-only`.
- The Contract's synopsis holds `--overlap`, and its calls are the ones other crates use.
- `backlog/affected-base-leaves-out-exits-its-design-states` is answered and deleted.

## Constraints

- No behaviour changes: every statement added is what the code does today and a test in `crates/clm-affected/tests/` already asserts. The build re-cites the tests of the seven reworded requirements, moves the two uncited tests' markers on, and changes no source file unless a test it must now cite fails.
- No scenario is renamed or removed.
