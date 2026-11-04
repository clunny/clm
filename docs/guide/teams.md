# Teams

CLM is built for several people and several Claude sessions working on one project at once. Nobody coordinates in chat: everything goes through the repository.

## Who does what

- **Intent owners** (product, tech leads, anyone who decides what the system does) propose changes with `/clm:propose` and review spec changes. Each component lists `owners`, so the right person reviews changes to it.
- **Reviewers** approve the spec changes that need review with `clm approve`. The question is always "is this what we want?", never "is this code right?". Programs answer that one.
- **Builders** run `/clm:build`, which drains the queue with unattended sessions. Anyone can run it, on any machine. Several people running it at once share the work rather than duplicate it.
- **Claude** writes all the code, keeps the backlog, and reports problems with CLM itself.

One person often plays every role. A team splits them so intent never waits on building, or building on intent.

## How work is shared

**Claims are branches.** `clm claim <component>` takes a component's build by pushing its `impl/<component>` branch. A second session sees the branch and moves on. A claim with no push for four hours lapses and someone else can take it, so a laptop closing mid-build blocks no one.

**Each piece of work is its own branch, based on `main`.** There are no stacked branches and no long-lived branches. `clm land` is the only way onto `main`: it merges the branch with the current `main`, runs the project's checks and `clm check` on the result on the machine that runs it, and pushes `main` only when they pass, so `main` always holds checked work. No hosted CI runs checks or merges; a team that wants one shared lander runs `clm land` there.

**Order comes from the spec.** Work is per component, not per change. A component is ready when it has unblocked requirements still to build; `depends_on` says what a component's code may use, not when it is built. `clm next` lists the ready components, most urgent first. Different components build in parallel, one session each. Several changes to one component need no ordering between them: each build works through the component's whole spec, highest priority first. A requirement that needs another component's requirement first says so in a `#### Needs` block, and waits until that one is current.

**Stale work is caught, not merged.** Each test cites the requirement it proves at the version it was built against. When someone's spec change lands and moves a requirement that is already built, its tests still cite the old version, so the requirement is `behind` and its component is pending again; the next build of that component brings it up to date. `clm pending <component>` lists what's left. Requirements the change didn't touch stay current.

**People outside the team.** Someone without write access can't claim or land, so they suggest intent instead: see [Contributions](contributions.md).

## Reviewing as a team

- Approve in a Claude Code session, where Claude runs `clm approve <branch>` and you confirm the command, or with Approve in the web app. Either way it's an `approved` record in your own ledger branch, naming you.
- In the web app you can also comment on a requirement in a branch's diff, or ask for changes. A change request holds the branch: `clm land` prints what you asked and lands nothing until the author pushes a change or you approve. Both are records in your ledger branch, like an approval.
- Approval is pinned to the branch's diff. A rebase or a merge of `main` that touched only other files keeps it, and any change to a file the branch changes drops it.
- Who may approve is the project's choice: `approval = "any"` in `clm.toml` (the default) counts anyone's approval, the author's included, and `approval = "other"` counts only someone who wrote none of the branch's commits.
- A change that needs no review lands on checks, and nothing is reviewed after it lands. If you disagree with one, propose a follow-up change; nothing is reverted behind anyone's back.
- Questions and human steps in the backlog are the team's to-do list for people: the web app shows them first, with the requirements each one blocks. A blocked requirement waits; the rest of its component builds.

## Habits that help

- Keep changes small and focused, so reviews are quick. Put each piece of design on one spec branch, so there's one review per idea.
- Write architectural constraints into `ARCHITECTURE.md` early. Every session reads them.
- Let Claude file `clm defect` when CLM gets in the way, so the tool improves for everyone.
- End sessions with `clm wrap-up`, so nothing is left only on one machine.
