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

- **Marker:** the text `req:<component>/<slug>@<version>` in a test, saying which requirement the test proves, at which version. *Usually:* requirement traceability in tests.
- **Version:** the first 8 hex digits of a requirement's hash; rewording the requirement changes it.
- **Requirement states:** `current` (a passing test cites it at its current version), `behind` (cited only at an older version), `unbuilt` (specified, but no test cites it yet) and `obsolete` (a test cites it, but a change removed it). **Pending** means any state but `current`: work the code still owes the spec (`clm pending`, `clm coverage`).
- **Component status:** `unbuilt` (specified, not built yet), `active` (built and kept current) and `deprecated`.
- **Queue:** the components with pending requirements, in priority order (`clm next`). A component in it is `ready` (work no one holds), `claimed` (a build holds it) or `blocked` (its pending requirements wait on a `#### Needs` or a backlog item); one with nothing pending is `converged`.
- **Claim:** pushing a branch that tells other sessions you're working on a component (`impl/<component>`) or a backlog item (`propose/<slug>`); it lapses when no one pushes to it for a while (`clm claim`). *Usually:* assigning a ticket to yourself.
- **Run / runner:** `clm run` prepares one component's build for a session; `clm runner` builds everything in the queue unattended, one session per component.
- **Build place:** one of the clone's slots for a build running at once (`max_builds`).
- **Draft:** an unattended session that designs one change from backlog items (`clm draft`).
- **Intake:** matching the host's open issues and pull requests to the backlog, saying of each issue whether it is new, taken in by an item or resolved (`clm intake`).
- **Oversized:** a component whose spec or code is larger than its size budget (`clm size`).
- **Expand-contract:** changing a contract in two steps: add the new one beside the old, move everyone over, then remove the old, marked meanwhile with `clm:remove-by <change-id>`. *Usually:* parallel change.

## How work reaches `main`

- **Gate:** `clm gate`'s judgement of a branch before it lands: whether it is code, spec or `mixed` (both, which it blocks: split the branch), and whether a spec branch needs a human's review (`spec review`) or lands on checks (`spec`). *Usually:* a merge check or quality gate.
- **Pull request:** a branch's request on the host to merge into `main` (GitLab: merge request), where a human reads the diff; nothing waits on it, and without a host there is none (`clm pr`).
- **Approve:** a human's recorded yes to a branch that needs review (`clm approve`). *Usually:* an approving review.
- **Land:** merging a branch into `main` through `clm land`, which runs the checks on the merged result on your machine and pushes `main`. *Usually:* merge, or land.
- **Rebase:** `clm rebase`: rebase a branch onto `origin/main`, regenerate `specs/` from its changes and run the checks.
- **Wrap-up:** `clm wrap-up`: clean up landed work and say whether the session can end.
- **Ledger:** the record of approvals, checks and lands, kept on one branch per writer (`clm ledger`). *Usually:* an audit log.
- **Writer:** the name your ledger records are made under.
- **Fingerprint:** a hash of a branch's diff, which an approval is tied to (`clm fingerprint`).
- **Signed repository:** one whose `docs/allowed_signers` lists the keys whose ledger records count; only a signed approval lets a branch that needs review land there.
- **Host:** where the remote lives, `github`, `git` or `none` (`host`); pull requests, issues and branch deletion go through it (`clm pr`, `clm issue`, `clm branch`, `clm contribution` for a fork's pull requests and issues).
- **Reconnect:** pushing a `main` that was landed with no remote to a newly added `origin` (`clm reconnect`).

## The backlog and the project's health

- **Backlog item:** a file in `backlog/` holding work not yet in a change (`clm backlog`). *Usually:* a ticket or issue.
- **Backlog kinds:** `idea` (a possible change), `bug` (behaviour that's wrong), `question` (a fact a human must give), `human-task` (a step only a human can take, such as creating an account), `deferred` (decided work left for later) and `workaround` (a build's flag that a requirement needed a workaround, `clm workaround`).
- **Priority:** `P1` to `P5` on items and changes, with few slots at the top; it orders work and never holds it back.
- **Triage:** grouping the backlog into units of design work, most urgent first (`clm triage`).
- **Proposals:** the changes that hold backlog items, and the live claims on items (`clm proposals`).
- **Defect:** a problem with CLM itself, reported with `clm defect`.
- **Health:** one advisory report on the project: checks, pending work, sizes, stale questions and what to reconsider (`clm health`).
- **Effort:** the work sessions spent, recorded in the ledger (`clm effort`), which `clm capabilities` turns into each capability's cost and `clm architecture` into each architectural constraint's.
- **Detectors:** the dead-code and unused-dependency tools `clm check` runs (`detectors`).

## Commands

Every `clm` command, by what it's for. `clm help <command>` says more.

- Seeing the state: `clm status`, `clm view`, `clm backlog`, `clm proposals`, `clm pending`, `clm coverage`, `clm next`, `clm health`, `clm size`, `clm capabilities`, `clm architecture`, `clm effort`, `clm ledger`.
- Writing specs: `clm context`, `clm hashes`, `clm apply`, `clm archive`, `clm rename`, `clm survey`, `clm guide`.
- Checking: `clm check`, `clm gate`, `clm affected`, `clm unreferenced`, `clm fingerprint`.
- Working: `clm claim`, `clm run`, `clm runner`, `clm triage`, `clm draft`, `clm intake`, `clm workaround`, `clm defect`, `clm superpowers`.
- Landing: `clm rebase`, `clm autosync`, `clm pr`, `clm approve`, `clm land`, `clm wrap-up`, `clm reconnect`.
- Setting up: `clm init`, `clm config`, `clm branch`, `clm issue`, `clm contribution`, `clm hook` (run by the plugin's hooks).
