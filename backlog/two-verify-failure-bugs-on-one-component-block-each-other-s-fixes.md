---
title: "Two verify-failure bugs on one component block each other's fixes in clm land"
kind: bug
components: [land]
issues: [486]
---

When two `kind: bug` items with `verify_failures` both hold the same component, `land/verify-stop` admits a code branch on that component only if its commits resolve every item that holds it. A branch that fixes one item and carries `Resolves:` for that item alone is refused because the other item still holds the component. A second branch that fixes only the other item is refused for the same reason. Neither branch can land, and two drafts working on the items separately wait for each other until they give up. The only way out is to merge the fixes onto one branch by hand.

Expected: separate fixes for items that hold the same component can reach `main`. Either the refusal names the other holding items and says that one branch must resolve them all, or the rule admits a branch that resolves at least one holding item. The second option has to be weighed against `verify-after-land`, which keeps a component held until each item's own fix lands. Triage could also put every `verify_failures` bug on a component into one draft, which would stop the split from happening at all (`orchestrator.triage`, `plugin.triage`). `land/verify-stop` has no scenario with two items holding one component, so whichever way is chosen needs a requirement or scenario that states it.
