# Quickstart

From nothing to your first change built, for a new project or an existing one.

## You need

- [Claude Code](https://claude.com/claude-code).
- The GitHub CLI, `gh`, signed in (`gh auth login`) with access to `clunny/clm`, where CLM's releases live.
- A git repository for your project, where you can push to `main`. A host (GitHub today) is optional: it holds the remote and shows a pull request per branch, but runs no checks and makes no decisions.

## 1. Install the plugin

In Claude Code:

```
/plugin marketplace add clunny/clm
/plugin install clm@clm
```

If you installed it before, update it first with `claude plugin update clm`. The plugin puts `clm` on Claude's path and downloads the matching binary on first use.

## 2. Run `/clm:init`

Open Claude Code in your repository and run `/clm:init`. It starts a short conversation:

- what the project is about, in a sentence or two;
- what a person can do with it: its first capabilities;
- the architectural constraints to lock in from the start, what must always hold ("all money is integer pence", "no service talks to the database of another");
- the language and stack.

Then it runs `clm init`, which writes CLM's tooling:

- `clm.toml`: settings: where the remote lives (`host`), whose approval counts (`approval`), what `clm land` runs (`[land] check`), and the dead-code detectors for your languages.
- `scripts/clm-check.sh`: builds and tests your project. `clm land` runs it before `clm check`, on your machine, in the tree it is about to land. It's yours to edit.
- `ARCHITECTURE.md`, `CAPABILITIES.md`, `specs/SPEC.md` and `CLAUDE.md`, if you don't have them yet.

It writes nothing under `.github/` and creates no labels, and it never changes a file that already exists, so running it again is safe. It prints `wrote` or `kept` for each file.

## 3. Do the steps only you can do

`clm init` ends with a list headed `Next steps (yours):`. Typically:

1. **Push the bootstrap commit to `main` yourself.** Claude can't push to `main`. This is the only time anyone pushes to `main` directly: from then on every branch lands through `clm land`.
2. **If your host protects `main`,** let the identity that runs `clm land` push it, with a bypass or as an allowed pusher. There are no secrets, workflows or required status checks to set up.

## 4. Review your first design

Claude then writes your first spec graph on a `spec/` branch, a change that needs your approval, and opens its pull request with `clm pr open` for you to read.

**A new project:** your architectural constraints in `ARCHITECTURE.md` and your first capabilities in `CAPABILITIES.md`, a root component and a few first components, drafted from the conversation.

**An existing project:** Claude maps the codebase with `clm survey`: directories, languages, manifests, tests, docs, and which parts import which. From that it proposes a component map, a set of components that cover all the code, and shows it to you before writing anything else. Fix the boundaries now; they're the expensive thing to change later. Once you agree, one subagent per component writes its contract, design and requirements, describing what the code does today. Bugs it notices become backlog items, not requirements. Existing ADRs become decisions.

**All of it, or one part first.** Before it maps anything, Claude asks whether to adopt the whole codebase now or one service or module first. One part makes a smaller first review, and the first change finishes as soon as that part's tests cite its requirements. The price: the rest sits `outside` every component, listed in the root's `outside` in `specs/SPEC.md`. It has no requirements, so no test proves it to CLM and nothing reports when it drifts; it still lands only through `clm land`, whose check script runs its existing tests, and a build of an adopted component gets no contract for the outside code it calls. Each later change widens the graph: it adds a component for the next part and takes that part's globs out of `outside`. `clm check` fails on code that is in no component and not declared outside, and on an `outside` glob that no longer matches anything, so the list only shrinks.

On an existing project, CLM moves the build and test steps of your existing CI workflows into `scripts/clm-check.sh` and removes those workflows in the bootstrap commit, because `clm land` lands on CLM's own checks. It tells you before it does this, and the first design lists every workflow it removed. Deploy, release and scheduled workflows are left alone.

Read the design as intent: is this what the system is, or should be? When you're happy, say yes in the conversation. Claude runs `clm approve` and asks you to confirm, then `clm land` checks the branch on your machine and pushes it to `main`.

## 5. Build

Run `/clm:build`. Claude works through the queue (`clm next`), one unattended build per ready component, in parallel, each landing itself with `clm land` once its checks pass.

For an existing project, the components' builds link your existing tests to the requirements they prove (`req:<component>/<slug>@<version>`), write tests for the rest, and remove dead code; the root component's build turns the dead-code detectors on.

## Next

- Propose your next change with `/clm:propose`: see [Using CLM](using-clm.md).
- Working with others: [Teams](teams.md).
