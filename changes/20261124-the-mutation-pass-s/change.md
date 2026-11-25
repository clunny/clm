---
status: applied
resolves: [the-mutation-pass-s-strykerjs-sandbox-copies-the-site-tests]
---

## Intent

The mutation pass's StrykerJS sandbox copies nothing git ignores. `ci/mutate-script` has `mutate.sh` give StrykerJS, in the configuration file it already writes, `ignorePatterns` taken from the repository's `.gitignore` files that reach the npm directory being judged, so another run's transient output there, such as the site tests' `web/site/.test-dist/`, is never copied into the sandbox, whether it exists when StrykerJS starts or appears while it copies. The StrykerJS fixture proves the real tool honours them: it fails when a directory its own `.gitignore` names reaches the sandbox. This is plan work under `backlog/a-passing-check-proves-less-than-the-spec-claims`, as the earlier mutation-pass changes were: the mutation pass is phase 3's evidence, and a pass that dies on another suite's files judges nothing for every `web` requirement still due.

## Problem

On 2026-11-24 `clm verify --due`, run by the build loop, judged a `web` requirement with StrykerJS. StrykerJS copies the whole npm directory (`web/`) into its sandbox, leaving out only its own default patterns (`node_modules`, `.git`, its reports and temporary directories) and the script's `.stryker-tmp-clm-*`. That copy took in `web/site/.test-dist/`, git-ignored output the site's own tests create and delete (fixture guides and blogs under `.test-dist/<name>-src`), and a directory there was removed while StrykerJS scanned it: ENOENT on `site/.test-dist/drift-src/docs`. `mutate.sh` failed the block as `mutate-script` asks for any tool failure, and the pass ended with 20 requirements not judged and 58 left. The script behaved as written; the design let files outside the requirement's code and tests decide whether the tool runs.

No requirement says what the sandbox holds beyond the links `mutate-script` puts in its `tempDirName`, so the fix is a spec change (`code-fix-needs-a-written-rule`).

## Approaches

- **Taken: ignore what git ignores, by its rules, not by what exists.** StrykerJS's `ignorePatterns` use `.gitignore` syntax, so the script carries the rules of the top `.gitignore` that reach the npm directory (re-rooted there: `web/node_modules/` becomes `/node_modules/`, an unanchored `.wrangler/` stays unanchored) and those of each `.gitignore` under it (re-rooted at their own directory: `web/site/.gitignore`'s `.test-dist/` becomes `/site/**/.test-dist/`), beside the `.stryker-tmp-clm-*` it already names. Rules, not paths, so a directory that appears after the script starts is left out too: that is the race the item saw. A file git ignores is one no fresh clone has, so no test that passes in a fresh clone's land check needs it in the sandbox.
- **Not taken: list the ignored paths that exist when the script starts** (`git ls-files --others --ignored --exclude-standard --directory`). Simpler to write, but a directory another run creates after the listing is copied and can still vanish mid-copy.
- **Not taken: copy the tracked files into a clean staging directory and run StrykerJS there.** It would hold only what git tracks, but the vitest configuration the script writes imports the directory's own `vite.config.ts`, and StrykerJS's `node_modules` link and the repository links would all move: a larger redesign for the same outcome.
- **Not taken: stop the other run from writing there while the pass runs.** The writer may be any concurrent suite (a block's tests, the site's tests, a land check in the same tree); `mutate.sh` can't know or order them, and the sandbox is what it controls.

## Outcome

- `ci/mutate-script` (modified): StrykerJS copies into its sandbox nothing the repository's `.gitignore` files mark ignored under the npm directory, existing or appearing during the copy; the StrykerJS fixture also fails when a directory its own `.gitignore` names, which the script makes beside it, reaches the sandbox. Two scenarios: *StrykerJS copies nothing git ignores* and *the StrykerJS fixture fails on what git ignores*.

## Constraints and edge cases

- The fakes can't show that StrykerJS reads `ignorePatterns` as git reads `.gitignore`; the fixture does, at each pass, with the real tool (`mutation-after-land`), as the link fixture does for the repository's files.
- The scenario's test reads the configuration's `ignorePatterns` as `.gitignore` rules with git itself (for instance `git check-ignore --no-index` over a `.gitignore` holding them), so the re-rooting is proven by git's own matcher, not by a copy of it.
- A top `.gitignore` rule anchored under another directory (`crates/clm-web/build/`) doesn't reach the npm directory and is left out; a negated rule (`!x`) is carried with its prefix.
- `node_modules/` stays StrykerJS's: it links it, as before.
- Nothing changes for `coverage.sh` or the Rust path, whose tree is a fresh `git clone --shared` holding nothing ignored.

## Open questions

None.
