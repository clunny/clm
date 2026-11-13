# Glossary

Every term CLM uses, in plain words. Where the wider software world has a usual name for the same thing, it's given as *Usually:*. CLM keeps a usual term's usual meaning, and names what it coins so a newcomer can guess it (`plain-terms`).

## What a project declares

- **Capability:** something a person can do with the product, and why, written in `CAPABILITIES.md`. The product manager owns it. *Usually:* a feature, user story or use case.
- **Architectural constraint:** something that must always hold of the product, whatever is built, written in `ARCHITECTURE.md`. The architect owns it. *Usually:* a non-functional requirement.
- **Carried constraint:** one of CLM's own architectural constraints that every project built with CLM follows, supplied by the `clm` the project runs and listed by `clm guide constraints`. Its tier says whether a project may change it: `method` (no), `replaceable` (yes, by a constraint that keeps the guarantee CLM relies on) or `default` (yes, it may be dropped).
- **Component:** one unit of the product with its own `specs/<path>/SPEC.md`: its rules, its Contract, its Design and the files it owns (`paths`). Components form the **spec graph**, a tree under the **root component** (`specs/SPEC.md`), with `depends_on` edges between them. A component's ID is its path with dots (`check.coverage`). A delta names its component with `component:`, a backlog item lists its components under `components:`, and `clm defect` and `clm proposals` take `--component`. *Usually:* a component or module.
- **Contract:** what the outside can rely on from a component: its commands, files and interfaces. *Usually:* the public interface or API.
- **Design:** how a component works inside, visible to whoever builds it and to its parent. *Usually:* the design notes or internal design.
- **Requirement:** one rule a component must meet, with an ID `<component>/<slug>` and at least one scenario. *Usually:* a requirement, or an acceptance criterion.
- **Scenario:** a GIVEN / WHEN / THEN example of a requirement, which a test can follow. *Usually:* an acceptance test case, as in Gherkin.
- **Ancestors:** a component's ancestors are the path from the root component down to it: the context a session building that component reads (`clm context`).
- **Placeholder component:** a component that exists only to hold a place, `unbuilt`, naming in `successor` the work that will fill it in.
- **Needs:** a requirement's list of other components' requirements that must be built before it can be (`#### Needs`). *Usually:* a dependency between tasks.
- **Serves:** which capabilities or architectural constraints a rule exists for (`serves`, `#### Serves`). *Usually:* traceability.
- **Enforced by / Untested:** the blocks of an architectural constraint naming the rules that check it by program (`#### Enforced by`), and what no program can check and why (`#### Untested`).
- **Carries / Guarantee / Replaces:** blocks of an architectural constraint about CLM's carried constraints. In CLM's own `ARCHITECTURE.md`, `#### Carries` names a carried constraint's tier and `#### Guarantee` states what CLM relies on for a `replaceable` one. In a project's, `#### Replaces` names the carried constraints an entry replaces or drops.
- **Decision:** a significant choice and its reasons, one file in `docs/decisions/`. *Usually:* an architecture decision record (ADR).
- **Settings:** `clm.toml` for the project, `clm.local.toml` for one clone, `~/.config/clm/config.toml` for you (`clm config`).

## How the spec changes

- **Change:** one proposed edit to the spec, in `changes/<id>/`: a `change.md` saying why, and its deltas. *Usually:* a change request or proposal.
- **Delta:** one file in a change, `deltas/<component>.md`, that adds, modifies or removes a component's requirements, Contract or Design, each modified part recording the hash it was written against (`clm hashes`). *Usually:* a patch to the spec.
- **`change.md` keys:** `status`; `human_burdens`, the change's **human burdens**: what it spends of a human's money or time, or can't undo, which make its branch need review; `author`; `from`, the backlog item it grew from; `resolves`, the backlog items it settles; `priority`; `frontmatter_keys`, the component frontmatter keys it adds; and `renames`, the `old → new` pairs a rename change carries out (`clm rename`).
- **Change status:** `proposed` (written, not yet in `specs/`), `approved`, `applied` (its deltas are in `specs/`), `implementing`, `done` (built) and `abandoned`.
- **Apply:** folding a change's deltas into `specs/`, all or nothing (`clm apply`).
- **Archive:** moving a change that is done or abandoned to `changes/archive/` (`clm archive`).
- **Rename:** renaming a component, a requirement or a term in spec text from the pairs a change records in `renames`, re-pointing everything that cites it, test markers included (`clm rename`). A branch that holds exactly what the pairs produce lands on checks, unless it swaps a term, which a human reviews. *Usually:* a rename refactoring, applied to the spec.
- **Spec branch:** a branch `spec/<change-id>` that proposes one change. **Impl branch:** a branch `impl/<component>` that builds one component. *Usually:* feature branches.
- **Autosync:** a snapshot of unfinished spec work pushed after each turn to `autosync/<branch>`, so it survives a lost machine (`clm autosync`).

## How work is built and proven

- **Marker:** the text `req:<component>/<slug>@<version>` in a test, saying which requirement the test proves, at which version. Two other markers use the word too: `clm:remove-by <change-id>` on code a contract change will remove (see Expand-contract), and `clm:reads <glob>` in a test that reads the project's own specs, docs or `clm.toml`, so a land that changes only them still runs it. *Usually:* requirement traceability in tests.
- **Version:** the first 8 hex digits of a requirement's hash; rewording the requirement changes it.
- **Requirement states:** `current` (a passing test cites it at its current version), `behind` (cited only at an older version), `unbuilt` (specified, but no test cites it yet) and `obsolete` (a test cites it, but a change removed it). **Pending** means any state but `current`: work the code still owes the spec (`clm pending`, `clm coverage`).
- **Component status:** `unbuilt` (specified, not built yet), `active` (built and kept current) and `deprecated`.
- **Queue:** the components with pending requirements, in priority order (`clm next`). A component in it is `ready` (work no one holds), `claimed` (a build holds it) or `blocked` (its pending requirements wait on a `#### Needs` or a backlog item); one with nothing pending is `converged`.
- **Claim:** pushing a branch that tells other sessions you're working on a component (`impl/<component>`) or a backlog item (`propose/<slug>`); it lapses when no one pushes to it for a while (`clm claim`). *Usually:* assigning a ticket to yourself.
- **Small change:** a code change, or an edit to `docs/` or `CLAUDE.md`, that leaves every requirement, scenario and design as written true, landed from a branch of its own with no claim and no change (`small-change-no-claim`).
- **Agent harness:** the coding agent a session runs in, such as Claude Code or Codex, which CLM supports through an adapter in `clm` that recognises it, translates its hook events and declares what it can do (`agent-harness-adapters`); `clm agent-harness` prints it. Any other agent that reads `AGENTS.md` and runs shell commands is the **generic tier**: it needs no adapter, and there no guard runs and `clm approve` can't refuse, so the human approves in their own terminal. Not the test harness below.
- **Harness:** the plumbing the capability tests run on (runner settings, the script that starts the system under test, ports, worker counts), held in the Design of the component that builds those tests. **Harness-only work** changes only it and is never a requirement; it lands as a small change (`human-layer-proof`). *Usually:* the test harness. Not an agent harness.
- **Test plumbing:** how a test suite runs rather than what it asserts: the harness, or a component's own test-suite plumbing (its runner settings, the ports and servers its tests start, the paths they write). Work that changes only test plumbing is never a requirement and lands as a small change, a `bug` item naming it deleted by a `Resolves: backlog/<slug>` trailer with no claim (`code-fix-needs-a-written-rule`).
- **Run / runner:** `clm run` prepares one component's build for a session; `clm runner` builds everything in the queue unattended, one session per component.
- **Build loop:** the clone's one continuous loop that `/clm:build` runs (`clm build`): `clm` decides each step (what to build or draft next, in priority order, how to handle a run that stops, when to fold, archive and verify) and the session launches what it says. Its state is in git and a loop file beside the build places, so a new session resumes it. What needs you waits in its queue (`clm view queue`) until you deal with it. *Usually:* a work queue with a scheduler.
- **Build place:** one of the clone's `max_builds` places for a build or draft running at once.
- **Slot:** the number a build place holds, from 1 up, distinct among the clone's builds; `clm land` passes it to the check as `CLM_SLOT` (0 outside a place), so the project's tests can key a port or path on it and run beside other builds' tests.
- **Draft:** an unattended session that designs one change from backlog items (`clm draft`).
- **Intake:** matching the host's open issues, pull requests and dependency alerts to the backlog, saying of each issue and alert whether it is new, taken in by an item (whose `issues` or `alerts` names it) or resolved (`clm intake`).
- **Oversized:** a component whose spec or code is larger than its size budget (`clm size`).
- **Expand-contract:** changing a contract in two steps: add the new one beside the old, move everyone over, then remove the old, marked meanwhile with `clm:remove-by <change-id>`. *Usually:* parallel change.

## How work reaches `main`

- **Gate:** `clm gate`'s judgement of a branch before it lands: whether it is code, spec or `mixed` (both, which it blocks: split the branch), and whether a spec branch needs a human's review (`spec review`) or lands on checks (`spec`). *Usually:* a merge check or quality gate.
- **Pull request:** a branch's request on the host to merge into `main` (GitLab: merge request), where a human reads the diff; nothing waits on it, and without a host there is none (`clm pr`).
- **Approve:** a human's recorded yes to a branch that needs review (`clm approve`). *Usually:* an approving review.
- **Land:** merging a branch into `main` through `clm land`, which runs the checks on the merged result on your machine and pushes `main`. *Usually:* merge, or land.
- **Overlap re-check:** what a land runs when the branch's head and `main`'s head were each checked whole and `main` has moved since: only the tests both can affect (`clm affected --overlap`), and none when there are none. The land prints a `re-check:` line, and its ledger record keeps `recheck` (`checked`, `none`, `overlap` or `selected`). See `ci-and-deployment`.
- **`CLM_LAND_DIR`:** the directory, lasting from run to run, that `clm land` gives a check it runs in the clone's land tree (`.clm/land`), and `clm verify` gives `[verify] check` (`.clm/verify-build`), for the check's build output.
- **Rebase:** `clm rebase`: rebase a branch onto `origin/main`, regenerate `specs/` from its changes and run the checks.
- **Wrap-up:** `clm wrap-up`: clean up landed work and say whether the session can end.
- **Ledger:** the record of approvals, checks and lands, kept on one branch per writer (`clm ledger`); `clm ledger verify` audits `main` against it, checking every commit on `main` is a recorded land. *Usually:* an audit log.
- **Writer:** the name your ledger records are made under.
- **Fingerprint:** a hash of a branch's diff, which an approval is tied to (`clm fingerprint`).
- **Signed repository:** one whose `docs/allowed_signers` lists the keys whose ledger records count; only a signed approval lets a branch that needs review land there.
- **Host:** where the remote lives, `github`, `azure` (Azure DevOps), `git` or `none` (`host`); pull requests, issues and branch deletion go through it (`clm pr`, `clm issue`, `clm branch`, `clm contribution` for a fork's pull requests and issues).
- **Reconnect:** pushing `main` and the ledgers to a newly set `origin`, such as a real remote for a repository that had `host = "none"` (`clm reconnect`).
- **Verify:** running the tests too slow for every land after it, at moments the project sets in `[verify]` (a change finishing, the queue draining, the end of a build, every N lands or days, a release), on your machine or the host's pipeline, and recording the result in the ledger (`clm verify`). A red result names the land that broke it; once you take it in as a bug, `clm land` holds code lands on that land's components until the fix lands. *Usually:* a nightly or post-merge test stage, as Maven's `verify` phase runs integration tests. Not `clm ledger verify`, which audits `main` against the ledger.

## The backlog and the project's health

- **Backlog item:** a file in `backlog/` holding work not yet in a change (`clm backlog`). *Usually:* a ticket or issue.
- **Backlog kinds:** `idea` (a possible change), `bug` (behaviour that's wrong), `question` (a fact a human must give), `human-task` (a step only a human can take, such as creating an account), `deferred` (decided work left for later) and `workaround` (a build's flag that a requirement needed a workaround, `clm workaround`).
- **Priority:** `P1` to `P5` on items and changes, with few slots at the top; it orders work and never makes work ready or unready; in the land queue, higher-priority lands take their turns first (`P1` before everything).
- **Triage:** grouping the backlog into units of design work, most urgent first (`clm triage`).
- **Proposals:** the changes that hold backlog items, and the live claims on items (`clm proposals`).
- **Defect:** a problem with CLM itself, reported with `clm defect`.
- **Health:** one advisory report on the project: checks, pending work, sizes, stale questions and what to reconsider (`clm health`).
- **Effort:** the work sessions spent, recorded in the ledger (`clm effort`), which `clm capabilities` turns into each capability's cost and `clm architecture` into each architectural constraint's.
- **Migration:** one step that brings a project set up by an older `clm` up to date, `required` when a newer `clm` would misread the project without it and `advised` when the project's files only differ from what `clm init` writes today; `clm upgrade` lists and applies them, `/clm:upgrade` walks you through them, and `upgraded` in `clm.toml` records the last one the project took. An *apply* migration is made by `clm upgrade --apply` itself, and every required one is one; a *note* migration is an advised edit to a file the project owns, which you take or decline (`declined_migrations`), or advice that asks for no edit, which you read and the upgrade's stamp marks taken. `skip_migrations`, in one clone's `clm.local.toml`, lets that clone run past a broken required migration.
- **Detectors:** the dead-code and unused-dependency tools `clm check` runs (`detectors`).

## Commands

Every `clm` command, by what it's for. `clm help <command>` says more.

- Seeing the state: `clm status`, `clm view`, `clm backlog`, `clm proposals`, `clm pending`, `clm coverage`, `clm next`, `clm health`, `clm size`, `clm capabilities`, `clm architecture`, `clm effort`, `clm ledger`, `clm agent-harness` (which agent harness a command runs under and what it can do), `clm machine` (the machine's CPU load, available memory and free disk, judged against what the clone's builds use).
- Writing specs: `clm context`, `clm hashes`, `clm apply`, `clm archive`, `clm rename`, `clm survey`, `clm guide`.
- Checking: `clm check`, `clm gate`, `clm affected`, `clm unreferenced`, `clm fingerprint`.
- Working: `clm build` (the build loop), `clm claim`, `clm run`, `clm setup`, `clm runner`, `clm triage`, `clm draft`, `clm intake`, `clm workaround`, `clm defect`, `clm superpowers`.
- Landing: `clm rebase`, `clm autosync`, `clm pr`, `clm approve`, `clm land`, `clm verify`, `clm wrap-up`, `clm reconnect`.
- Setting up: `clm init`, `clm upgrade`, `clm config`, `clm branch`, `clm issue`, `clm contribution`, `clm hook` (run by the plugin's hooks).
