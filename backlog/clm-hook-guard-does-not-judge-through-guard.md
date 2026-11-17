---
title: "`hook-budgets` says `clm hook guard` judges through `guard`, which it doesn't"
kind: deferred
components: [plugin.hooks.guard]
---

`plugin.hooks.guard/hook-budgets` says "`clm hook guard` SHALL judge through `guard`". Since `guard-per-agent-harness` and `commands-in-crate`, the command's call `guard_command` judges through `guard_hook_in`, which renders for the agent harness its arguments name and loads settings with the call's own environment lookup, and passes `Budgets::HOOK` to `judge_event` itself; `guard` is Claude Code's alone and reads the process's environment. The budgets the hook uses are the same, and `the_hook_judges_with_the_hooks_budgets` checks both places name `Budgets::HOOK`, but the sentence as written is false. The spec edit: say `clm hook guard` judges with `Budgets::HOOK`, through `guard_hook`, rather than through `guard`. Due with the spec change that settles `backlog/the-guards-spec-still-describes-it-before-its-call`.
