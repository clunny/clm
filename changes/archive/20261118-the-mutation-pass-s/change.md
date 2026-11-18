---
status: done
resolves: [the-mutation-pass-s-strykerjs-sandbox-can-t-see-repository-files]
---

## Intent

Plan work under the freeze (`backlog/a-passing-check-proves-less-than-the-spec-claims.md`): the mutation pass is phase 3's evidence, and it can't judge a `web` requirement whose test reads a file of the repository outside `web/`. A web test that passes in the land check now passes in the mutation pass's StrykerJS sandbox too, so its requirement can be judged. `ci/mutate-script` has `mutate.sh` put, in the directory StrykerJS makes its sandbox in, a symbolic link to each entry of the repository's top but `.git` and the npm directory, so the sandbox stands where `web/` does among the repository's files, and the StrykerJS fixture reads a repository file the same way, so a pass whose sandbox can't reach them fails loudly instead of judging nothing.

## Problem

During the build loop's `clm verify --due`, `mutate.sh` ran StrykerJS for a `web` requirement whose citing test, `web/src/build-assets.test.ts`, reads `../../specs/web/SPEC.md` and calls `buildAssets(new URL('../../', import.meta.url))`. StrykerJS 10.1.1 copies the npm directory into `<tempDirName>/sandbox-<random>/` (`.stryker-tmp-clm-<tag>/` under `web/`, which the script names) and runs the tests there, so `../../` from `src/` resolves to `web/.stryker-tmp-clm-<tag>/`, which holds nothing. The initial test run failed with ENOENT, StrykerJS refused to mutate, the script exited 1 as `ci/mutate-script` says it must for a tool that fails, and the requirement stays due on every verify.

The Rust half of the same requirement already states that a test that asks git about this repository passes in the tool's copy as in the checkout (`20261116-the-mutants-copy-is-a-git-repository`); nothing states the like for StrykerJS's sandbox, so the fix is a requirement of this change (`code-fix-needs-a-written-rule`). No change or code fix since 2026-11-11 touches StrykerJS's sandbox.

## Approaches

- **Links to the repository's top beside the sandbox.** Taken. StrykerJS makes the sandbox one level inside `tempDirName` (`TemporaryDirectory.initialize` in 10.1.1: `mkdtemp(path.join(tempDirName, 'sandbox-'))`), and the script already names `tempDirName` and keeps it out of the copy (`ignorePatterns`). Links there to every top-level entry but `.git` and the npm directory make any path a test resolves relative to its own file reach the repository's file, whatever the test, with no test or config of `web` or `plugin.panel` changed. StrykerJS removes only the sandbox and an empty `tempDirName`, and `fs.rm` follows no link; the script's cleanup already removes `.stryker-tmp-clm-*` without following links. `.git` is left out because a link named `.git` would make the temp directory look like a repository's top to git.
- **Run StrykerJS in place** (`inPlace: true`). Not taken: it writes mutants into the verify tree, which a stopped run would leave changed, and two blocks at once in one directory would see each other's mutants; the Rust pass rejected `--in-place` for the same reason.
- **Run StrykerJS from the repository's top**, so the sandbox copies the whole repository. Not taken: it copies `target/` and every other npm directory unless ignored by hand, and the `node_modules` link StrykerJS makes would no longer be the npm directory's.
- **Have each test find the repository's top** (by git or an environment variable). Not taken: it puts a rule on every test author of `web` and `plugin.panel`, in files `ci` doesn't own (`path-ownership`), to make up for one tool's copy.

## Outcome

`ci/mutate-script` states the links and that the StrykerJS fixture's noticing test reads a repository file by a relative path, with two scenarios proven with the fake `npx`. `ci`'s build adds both to `.github/scripts/mutate.sh`; the requirement's next pass with the real tool proves them through the fixture.

Edge cases and workarounds known now: a test that reads a file of its own npm directory through the repository's top (`../web/...` from `web/`) reaches the unmutated checkout, not the sandbox; nothing does that today, and its land check reads the same file, so the link for the npm directory itself is left out rather than pointed at the sandbox.

From the builder check, for the build: the links are made on every StrykerJS run and may already exist (the fixture's two runs and a block's two runs share a tag), so made so as to replace one; "each entry" takes dot entries and untracked ones (`.github`, `target/`), so a plain `*` glob is not enough; the temp directory is removed as soon as StrykerJS returns, since StrykerJS leaves a non-empty one and `coverage.sh` runs vitest in `plugin-panel/`, whose default include could find tests through the links; the fixture's noticing test reads the file with `fs` (an import would resolve the link to its real path), under `// @vitest-environment node` or with a plain string path, as `web/src/build-assets.test.ts` does under `web`'s `jsdom`; and the blind test reads nothing.

## What it removes

Nothing.

## Constraints

- No file, key or format an adopter holds changes: the script is this repository's own.
- `clm land` gains no check; the pass runs after the land (`mutation-after-land`).
- No test or configuration of `web` or `plugin.panel` changes.

## Open questions

None.
