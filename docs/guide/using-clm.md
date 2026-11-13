# Using CLM

Day to day you do two things: say what should change, and say yes to it. Claude does the rest through a handful of workflows.

## The workflows

| Run | When |
|---|---|
| `/clm:init` | once, to bring a repository under CLM (see the [Quickstart](quickstart.md)) |
| `/clm:propose` | you want the system to do something new or different |
| `/clm:build` | there is approved work to build |
| `/clm:triage` | the backlog has piled up: approvals, questions for you, bugs, ideas |
| `/clm:help` | you want to know how CLM works or how to do something |
| `/clm:health` | you want to know what to tidy, cut or rethink |
| `/clm:verify` | you want to know now whether `main` passes its slow tests, whatever the triggers say |

A typical loop is: propose a change, approve it, build it. Triage whenever the backlog grows.

## Propose a change: `/clm:propose`

Run `/clm:propose` and say what you want, in your own words, or name a backlog item. Claude:

1. talks it through with you until it understands what you want and why;
2. lays out two or three ways to do it, recommends one, and asks you to pick;
3. writes the design: which parts of the system change, the rules they must now meet, and any choices worth recording;
4. lands it, at once when it needs no review from you, or once you approve it when it does.

The design changes; the code doesn't yet. `/clm:build` builds it.

## Approve a change

Some changes need your yes before they land: those that change what a person can do with the product, what must always hold, or that cost you money or time or can't be undone. Everything else lands on its own once the checks pass.

When Claude asks for your approval it shows you the change's intent, what it asks of you and any open questions. You're judging intent: is this what the system should do? You never review code. Say yes in the conversation and confirm the command Claude Code shows you; that confirmation is your approval. If you say no, say what to change, and Claude reworks it and asks again.

## Build: `/clm:build`

Run `/clm:build` and Claude builds every part of the system whose code is behind its design, each in its own background session, in parallel, writing the tests first. Each part lands on `main` once its checks pass. It also drafts fixes for the backlog's bugs beside the builds, most urgent work first, whether build or fix; fixes that need your review are gathered into one change, which it asks you to approve as soon as no fix is still being drafted. It runs your project's slow tests when your `[verify]` settings say.

The workflow keeps going: when nothing is left, it waits for new work, until you ask it to drain (finish what's running, then stop) or stop now, in the chat or with the panel's buttons. If the session ends or crashes, run `/clm:build` again in a new one and it carries on where it was: within 15 minutes of the old session's last step it shows you the queue and waits, unless you tell it the old session is gone. Anything waiting on you (an approval, a question a build raised, a build that keeps failing) stays in its queue, shown in the panel, until you deal with it.

Builds run under your session's permission mode: in manual mode they pause for your answers, in auto mode they run alone. Auto mode lets them push and land only through the allow rules that setting up CLM puts in `.claude/settings.json`; if your repository was set up before those rules existed, Claude tells you which to add before the first build.

If a build finds the design is wrong or incomplete, it doesn't guess: it raises it with you as a question or a proposed change.

## Work the backlog: `/clm:triage`

The backlog is where everything not yet done waits: ideas, bugs, questions only you can answer, steps only you can take (such as creating an account), and work deferred until later. Builds and proposals add to it as they go.

Run `/clm:triage` and Claude shows you a menu of what the backlog holds. Pick what to work on:

- **Approvals:** changes already designed and waiting for your yes.
- **Needs your input:** questions and steps only you can take, each with the answer Claude recommends. Work that depends on them waits until you answer.
- **Bugs:** Claude designs the fixes in the background, in parallel. `/clm:build` drafts them too, so after a build there may be none left.
- **Blockers:** whatever is holding up the most work, first.
- **Ideas:** Claude brainstorms each one with you, then proposes it, keeps it for later, or drops it.

Whatever needs your review comes back as one batch, with one approval to give.

## Check health: `/clm:health`

Run `/clm:health` for a report on the project's shape. It changes nothing; it points at what to reconsider:

- parts of the system that have grown too big;
- questions left open and claims on work that lapsed;
- rules the builds keep having to work around, which may be wrong;
- specs that are bloated or code no test runs.

Claude summarises it and proposes cuts. Act on any of it with `/clm:propose`. [How it works](how-it-works.md#health) says what each signal means.

## See where things stand

- Every Claude Code session starts with the project's status: what's built, what's in flight, what waits on you.
- `/clm:view` opens a panel beside the chat with the same picture, kept up to date. With the `clm-panel` plugin installed, `/clm:build` opens it for you on the Queue tab: what's running, what runs next, and what waits on you, with buttons to drain or stop.
- The web app at [clm-app.dev/app/](https://clm-app.dev/app/), if you've been invited, shows the design, the work in flight and the backlog, and lets you approve changes and suggest new ones from any browser, a phone included.

## Decide what's worth keeping

Ask Claude what a capability costs, or what an architectural rule costs, and it shows you its page: what it took to build, how much of your attention it needed, and what it would take to drop. Tell Claude whether to keep it, cut it, redesign it, or accept it as it is, and confirm the command. Cuts and redesigns go through `/clm:propose`.

## Working with others

- Several people and sessions can work on one project at once: see [Teams](teams.md).
- Suggestions from people outside the project come in through `/clm:contribution`: see [Contributions](contributions.md).
