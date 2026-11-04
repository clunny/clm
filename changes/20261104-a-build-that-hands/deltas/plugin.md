---
component: plugin
---

## ADDED Requirements

### Requirement: build-unclosed-builds
`skills/build/SKILL.md` SHALL tell Claude that a build whose subagent ends without a closing line of its own, neither `landed <sha>` nor `stopped: <reason>` (for example out of context, or after an earlier result that ended with `waiting: <command>`), and isn't one the waiting rule (`build-waiting-builds`) or the limit rule (`build-resumes-limited-builds`) holds back, is judged by what `clm run --finish <component>` printed for it, which it runs for that build as for any other return (`unclosed-build-reads-finish`): when that output holds the line `landed impl/<component>`, the build landed, so Claude tells the human that it landed without its closing line, doesn't count it as a stop and doesn't build it again for that, and runs `clm next` again as after any landed build; otherwise it counts as a stop, retried as a stopped build is. This rule SHALL be in the step that reads a build's return, where the skill says what a subagent that ends without either line counts as. It SHALL NOT tell Claude to judge such a build by reading `origin` with a `git` command of its own.

#### Scenario: build reads the finish of an unclosed build
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `landed impl/<component>` and `unclosed-build-reads-finish`, and the sentence that contains `landed impl/<component>` also contains `clm run --finish <component>`

#### Scenario: build that counts every unclosed build as a stop
- GIVEN a `build` `SKILL.md` that doesn't mention `landed impl/<component>`
- WHEN the test's check runs on it
- THEN it reports `landed impl/<component>`

#### Needs
- orchestrator.run/run-finish

#### Serves
- capability/build-from-spec
- capability/work-concurrently
