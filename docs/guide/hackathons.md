# Hackathons

A small team, an idea and a few hours. With CLM, the humans spend the time on the idea, and AI builds while they think.

## The shape of the day

**Hour one: set up and set intent.** One person runs `/clm:init` on a new repository and does the bootstrap steps. The team agrees the idea in a sentence and the few constraints that matter, and the first spec change comes out of that conversation: a root component and two or three first components. Keep it coarse; you'll reshape it all day. Approve it and start `/clm:build` straight away.

**The rest of the day: intent and building overlap.** While the first components are being built, the team is already fleshing out the next ones with `/clm:propose`. Each change that lands leaves its components with requirements to build, and the builders pick up each component as soon as it has work ready, holding back only a requirement whose `#### Needs` aren't built yet. Nobody waits for a sprint to end, because there are no sprints.

## No plan to maintain

In sprint planning someone decides the order of the work up front and the plan goes stale as soon as the idea moves. CLM doesn't keep a plan. It works the order out from the spec every time anyone asks what's ready:

- work is per component: `clm next` lists the components with unblocked requirements to build, and a requirement that relies on another component's unbuilt work names it in `#### Needs`, so that work builds first;
- different components build in parallel, one session each, and a session claims its component by pushing `impl/<component>`;
- several changes to one component need no ordering: each build works through the component's whole spec, highest priority first;
- when new intent moves a requirement that is already built, only that requirement goes `behind`, and the component's next build brings it up to date. Everything the change didn't touch stays current.

So the idea can change direction mid-afternoon. You change the spec, and the build follows, keeping all the work that's still right.

## Roles for a team of three to five

- **One or two people shape intent:** talk to the users or the judges, write proposals, approve spec changes fast. Approval is the main brake on a hackathon, so someone should always be watching for changes waiting for `clm approve`.
- **One person keeps the builders busy:** runs `/clm:build` (more than one machine is fine), answers the `question` and `human-task` items that block requirements, and keeps an eye on builds that stop without landing.
- **Everyone tests the result** as it lands on `main`, and turns what they find into the next proposal.

## Tips

- Lock in only the architectural constraints you're sure of at `/clm:init`. Changing one later always needs an approval.
- Prefer several small changes on separate components over one big one: they build in parallel. Changes to the components land on checks without waiting for anyone; only an edit to the architectural constraints or the capabilities, or a change that declares a burden, waits for an approval.
- Use the backlog for ideas you won't get to today. It's tomorrow's starting point.
- The web app's implementation view shows what's building, what's blocked and what's waiting on a human, on a phone.
