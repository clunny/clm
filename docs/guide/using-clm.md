# Using CLM

Day to day, you do two things: say what should change, and review it. Claude does the rest.

## Where to look

- `clm status`: components with requirements proven, changes in flight, the backlog by kind, and live claims. Claude Code shows it at the start of every session.
- `clm backlog`: ideas, deferred work, questions, human steps and bugs.
- `clm next`: work ready to be built.
- The web app, if you've been invited: the spec graph, spec branches, implementation progress and the backlog. It also takes a new proposal for the backlog, which the next build's `clm rebase` files as an `idea` item.

## The spec

`specs/` is a tree of components, one `SPEC.md` each. A component has:

- a **Contract**: what other components may rely on;
- a **Design**: how it works inside, for whoever builds it;
- **Requirements**, each with an ID such as `billing/refund-window` and at least one scenario (GIVEN, WHEN, THEN);
- frontmatter: the `paths` of code it owns, and what it `depends_on`.

Significant choices are recorded one per file in `docs/decisions/`. The format is in the guide's [spec format](../spec-format.md) page (`clm guide spec-format`).

## The human layers

Above the component specs sit two short files at the repository's root, written for people rather than for builds:

- **`CAPABILITIES.md`** holds the capabilities: what a person can do with the product, each with a `#### Why` and a scenario. The product manager owns it.
- **`ARCHITECTURE.md`** holds the architectural constraints: what must always hold, each with SHALL or MUST and a scenario, and the rules that enforce it under `#### Enforced by`, why no program can check it under `#### Untested`, or both. The architect owns it, and every build reads it.

Each entry has an ID, `capability/<slug>` or `architecture/<slug>`, and a version, as a component's requirement does: `clm hashes capability` and `clm hashes architecture` print them. They are proven differently. A capability is proven by a black-box test at the product's public boundary that cites it, `req:capability/<slug>@<version>`, unless an `#### Untested` block says why no program can check it. An architectural constraint is never cited by a test: it names under `#### Enforced by` the component rules that enforce it, which their own components build and prove, and is current when every rule it names is current; it says what no program can check under `#### Untested`. `clm check` fails an architectural constraint with neither block, and a test that cites one. Any edit to either file needs your review before it lands; everything below them, the component specs included, is the AI's to change. A component says which entries its rules serve with `serves` in its frontmatter, and a requirement can override that with a `#### Serves` block. The format is in [spec format](../spec-format.md).

## Proposing a change

Run `/clm:propose` and say what you want, or name a backlog item. Claude:

1. talks it through with you, using superpowers' brainstorming, until it understands what you want and why, then lays out two or three ways to build it and asks you to pick;
2. writes the design into `changes/<id>/change.md` and asks you to approve it;
3. writes one delta per component it touches, the decisions it needs and any backlog items, and applies them, so the branch shows the spec as it will be;
4. runs `clm gate`, which says whether the branch needs your review, pushes one spec branch and opens its pull request (on GitHub), for reading the diff;
5. lands it through `clm land`: at once when it needs no review, and once you approve when it does.

Every piece of design goes on that one branch, so a change that needs your review asks for it once.

## Review

A spec branch either needs your review or lands on checks. It needs your review, and `clm gate`'s first line says `spec review`, when:

- it edits `ARCHITECTURE.md` or `CAPABILITIES.md`;
- it changes the `approval` setting in `clm.toml`;
- a change it adds or edits declares `human_burdens`: what it spends of your money or time, or can't undo, one short line each, such as a paid service or a step only you can take.

Everything else lands through `clm land` on checks, and nothing is reviewed after it lands. The pull request's `## Intent` says what the change does and why, and the change's `human_burdens` say what it asks of you. Approve in the conversation where Claude asks you: on your yes it runs `clm approve <branch>` and Claude Code asks you to confirm the command, so the approval is yours. It's a record in your ledger, pinned to the files the branch changes: a rebase or a merge of `main` that touched only other files keeps it, and any change to the branch's own files drops it, so you review again.

`clm land` is the only way anything reaches `main`. It gates the branch, merges it with `main`, runs the project's checks and `clm check` on the result on the machine that runs it, and pushes `main`. No hosted CI and no labels are involved.

## Building

Run `/clm:build`. CLM works per component: `clm next` lists the components whose requirements the code hasn't caught up with, leaving out requirements that wait on a backlog item or on another component's requirement they `Needs`, and capabilities whose serving rules haven't been built yet, and each is built in its own session and worktree, test-first, landing its own `impl/<component>` branch through `clm land`. Work whose spec changed under it is built again against the new spec rather than landed stale. A change is complete once every requirement it added or changed is cited by a passing test at its current version and no test cites one it removed; the next build's `clm rebase` then archives it.

You don't review code. If a build finds the spec is wrong or incomplete, it stops and raises a spec change, which comes to you as intent.

## The backlog

`backlog/` holds one file per item:

- `idea`: future work, which `/clm:triage` drafts.
- `deferred`: future work that waits on a condition its body names, which you judge. `/clm:triage` leaves it; take it up with `/clm:propose`, or set its kind to `idea`.
- `bug`: something broken, including problems with CLM itself (`clm defect`).
- `question`: a fact only you can supply. The requirements it `blocks` wait for it.
- `human-task`: a step only you can do, such as installing a tool or creating an account. The requirements it `blocks` wait for it too.

Answer a question or confirm a human step by editing or deleting the item in a spec change, and the requirements waiting on it can be built.

## Health

`clm health` prints one advisory report of the project's shape, and `/clm:health` reads it for you. It never fails anything: it shows the check violations, what is pending, oversized components, lapsed claims, changes nothing has built, questions left open, decisions nothing names, capabilities and architectural constraints no rule serves, spec text that narrates history, Designs that dwarf their requirements, components far off the median in size per requirement, code no test runs (from an lcov file named under `[health] lcov` in `clm.toml`), and the signals that a requirement is fighting its implementation.

Those signals come from builds. When a build needs a workaround to satisfy a scenario, fakes something the requirement doesn't mention, changes another component's Design to get it built, or stops to raise a spec change, it flags the requirement with `clm workaround`, a `workaround` backlog item, and lists it under `## Workarounds` in its pull request. Flags, rewrites in the archive and oversized scenario or exception counts add up to a score, and when it reaches the `reconsider` threshold the next build's `clm rebase` files `backlog/reconsider-<component>-<slug>.md`, a question for you: drop the requirement, change it, or redesign around it, through `/clm:propose`. The spec change that rewrites or drops the requirement deletes its flags. Thresholds live under `[health]` in `clm.toml`.

## Reviewing what capabilities and architecture cost

Every land records the work its Claude Code sessions spent, read from this machine's transcripts, as `effort` records in the ledger. `clm capabilities` lists each capability with what it cost: an estimate in dollars and work tokens, the commits on its components, the reviews you gave, its unsettled workarounds, and your latest decision on it. A component's cost is split over the capabilities its rules serve, so the numbers are estimates, and the work no branch names shows on an `unattributed` line.

`clm capabilities <slug>` prints one capability's page: why you need it; what it cost and what it took of your attention (approvals, questions answered, flags settled, items that held it up); its size and declared burdens; the workarounds builds flagged on its rules; whether you could live without it (how many of its rules serve only it, and what the rest also serve, computed from the specs); its rules; and your decisions so far.

Decide on a capability in the conversation where Claude shows you its page: on your word it runs `clm capabilities decide <slug> --value high|medium|low --decision keep|cut|redesign|accept`, and Claude Code asks you to confirm the command, so the decision is yours. The decision is a record in your ledger, and it settles the workarounds open on the capability then; a flag filed later is unsettled again. Keep, cut and redesign say what comes next, which you take up through `/clm:propose`; accept says the cost is worth it as it is.

The web app shows the same list and pages under **Capabilities**, on a phone too, and decides with a tap: choose a value, then Keep, Cut, Redesign or Accept as is. It has no git history, so for the commits, questions answered, blocking and size it says to run `clm capabilities <slug>`; in a repository that signs its records, it gives the `clm capabilities decide` command to run in your clone instead.

`clm architecture` gives the same review for the requirements in `ARCHITECTURE.md`, the rules every change pays for. It lists each with whether it is held (how many of the rules that enforce it are built and current, and what it leaves untested) and what it costs: its enforcing rules' share, counted apart from the capabilities', so the two lists never add up to one total. `clm architecture <slug>` prints one requirement's page, and you decide on it as on a capability: on your word Claude runs `clm architecture decide <slug> --value high|medium|low --decision keep|cut|redesign|accept`, and Claude Code asks you to confirm the command. The web app shows the same under **Architecture**.

A build that flags a requirement whose capability already has `workaround_flag` unsettled workarounds (3 unless `[effort]` in `clm.toml` says otherwise) stops working on that requirement and files a question for you: whether to redesign the capability rather than work around it again.

## When something goes wrong

- A branch conflicts with `main`, or the gate says it's out of date: `clm rebase`, push, and `clm land` again.
- A check fails when landing: Claude reads the failure, fixes it, pushes and lands again.
- CLM itself got in the way: `clm defect` files it.
- Before ending a session: `clm wrap-up` reports anything not yet safe on `origin`.
