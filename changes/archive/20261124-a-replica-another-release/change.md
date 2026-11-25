---
status: done
resolves: [a-replica-another-release-saved-waits-for-its-idle-alarm]
---

## Intent

A Worker's replica object that wakes after a deploy, holding a replica the older release saved, starts its re-index at once even when the store already holds a far-off idle alarm: `web.worker.replica/refetches-what-a-release-forgot` now has each read bring such an alarm forward to now, and each event to one minute ahead, where it said only to set one when none was set.

## The problem

Since `replica/markers-of-this-release`, `Replica::load` of a state another release saved forgets each scanned file whose text it doesn't keep, so `pending_fetch` lists them again and the replica is `stale()`. `refetches-what-a-release-forgot` (unbuilt) asks the first read or event to set the alarm to now *unless one is set*. A Durable Object's alarm outlives a deploy, so a repo whose pages were open before the deploy wakes holding the idle alarm `view()` set for the next resync or lapse, up to ten minutes off. Under the rule as written, nothing brings it forward: the re-fetch and the recompute wait for it, and until the alarm saves the replica, every wake loads the old state and runs the rescan again, while reads answer the views the older release computed. The requirement as written allows the defect, so the fix is a spec change (`code-fix-needs-a-written-rule`).

## Approaches

1. **Bring the alarm forward on each read or event (chosen).** For a read, the rule's "unless one is set" becomes "unless one is set to fire within one minute": an alarm set for later is brought forward to now. An event sets the alarm as `alarm-batches` states, one minute ahead unless one fires within that minute, which brings a later alarm forward too, so the two rules agree. It reuses the object's existing alarm and fetch path (`reuse-first`) and adds nothing to the store.
2. **Arm it in `Object::load`.** What the item suggests. `load` holds no clock and is also run by the alarm's own wake, where the alarm is already running; the rule would then be stated of a step no route shows. A read or event is what the requirement already names, so the change keeps that place.
3. **Save the replica on load, so the rescan is not repeated.** Saves at every wake after a deploy cost rows written and still leave the files unfetched until the idle alarm; it fixes the symptom, not the wait. Not taken.

## Outcome

- `web.worker.replica/refetches-what-a-release-forgot` is modified: each read sets the alarm to now unless one is set to fire within one minute, and each event sets it as `alarm-batches` states, so a later alarm is brought forward. Its scenario *a newer release* is unchanged; *an idle alarm already set* is added. The requirement is unbuilt, so its build writes both tests.

## Constraints and edge cases

- An object with files still to fetch for another reason (a push whose files outran one alarm's budget) is covered by the same rule; its alarm is already set within a minute by `on_alarm` (one second, or one batch), so a read changes nothing there.
- The rule holds for every read or event while files are listed, not only a wake's first: checking each is no dearer, and "first" couldn't be tested apart from it.
- A `fresh` read of such an object recomputes before the forgotten files are fetched, as `answers-views` states, so until the alarms fetch them its coverage leaves out their markers. Accepted: it lasts only until the alarm this rule brings forward has run, which is now, and holding a fresh read's recompute back would add a rule for a window of seconds after a deploy (`yagni`).
- No workaround is known to be needed.
