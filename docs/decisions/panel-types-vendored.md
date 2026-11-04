---
title: "The panel type-checks against a vendored copy of the plugin API declarations, refreshed from what Claude Code writes beside the plugin-authoring skill"
date: 2026-10-28
status: superseded
superseded_by: panel-types-hand-written
---

`claude plugin validate` reports what the engine would refuse to load, and the panel's vitest suite strips types, so a type error in `plugin-panel/` (a type used without its import, a prop a surface's element doesn't have) lands unnoticed (`backlog/panel-type-check`). A `tsc` step needs the `'claude-code'` declarations, and `panel-validate-skips-without-claude` left it out for want of a stable source. The search for one found:

- No package ships them. `@anthropic-ai/claude-code` on npm holds seven files, the only declarations among them `sdk-tools.d.ts`, the SDK's tools, not the plugin API.
- No command writes them. `claude plugin validate` and `claude plugin test` lay nothing beside the plugin they read.
- The engine writes them in two places, both at paths that vary by version and process: beside a mod it hot-reloads (`.claude-plugin/types/`, three files: `claude-code/`, `claude-code-tools/` and `claude-code-mcp/`, the last listing the MCP tools connected on that machine), and beside the `plugin-authoring` skill each time a session loads it, as one file, `types/claude-code.d.ts`, holding the API and the built-in tools' inputs and results and nothing of the machine. The file's first line names the Claude Code version that wrote it, and its header marks the surface early access: it may change between releases.

So the source this repository controls is a copy: `plugin-panel/vendor/claude-code/index.d.ts`, the skill's file as the engine wrote it, read through `tsconfig.json`'s one type root. Any Claude Code session refreshes it by loading the `plugin-authoring` skill and copying `types/claude-code.d.ts` from beside the skill's `reference.md` over it; a build session can do that as a human's can, so no step waits on the human. It is refreshed when the panel starts using something the copy lacks, or when `tsc` and `claude plugin validate` disagree about the module, and the panel follows what a newer copy changes in the same change, as it would a dependency it had bumped. A copy from 2.1.311 and the files 2.1.312 laid beside a mod type today's panel identically, in a second.

The copy is about 860 KB (22,700 lines: the API, then the built-in tools' inputs and results). `plugin-panel/.gitattributes` marks `vendor/**` `linguist-vendored`, the convention `node-size-budgets` reads, so it counts toward no size budget, and git's own tools show it as vendored. `typescript` is a devDependency the lockfile pins, so `npm ci` brings `tsc` and the check needs nothing else on the machine; it runs as the first command of `npm test`, since unattended sessions may run `npm test` and `npm run` but no `npx` beyond Playwright (`unattended-commands`), and since a human typing `npm test` in `plugin-panel/` then gets the same check as a land.

Alternatives considered:
- Finding the engine's own copy at check time, under `~/.claude/mods/*/.claude-plugin/types/` or the skill's temporary folder. The paths vary by version and process, a machine with no hot-reloaded mod and no skill loaded has none, and skipping when none is found would make the check rare where a type error is likeliest, a build in a fresh worktree.
- `claude plugin test`. It runs a mod's `*.test.ts` under the engine's own kit, imported from `'claude-code/testing'`; the panel's tests are vitest's, it lays no types, and it doesn't type-check either.
- Reading the declarations out of the Claude Code binary, which holds them to write. Fragile, and a build that depends on the binary's layout.
- Waiting for a published package. None is announced, and the item asked for a source to settle before the step is added; the copy can be replaced by a package the day one ships, with only `tsconfig.json`'s type root changing.
