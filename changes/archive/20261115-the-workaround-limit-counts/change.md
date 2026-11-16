---
status: done
resolves: [the-workaround-limit-counts-one-answered-cause-once-per-build]
---

## Intent

Accept the behaviour the bug `the-workaround-limit-counts-one-answered-cause-once-per-build` reports, delete the item, and design nothing. No spec, code or skill text changes.

The item reports that builds of several components met one cause the human had already answered, each flagged a workaround, the third flag reached `capability/prove-by-tests`'s limit, and `clm workaround` exited 3, so a build stopped and asked again. It asks that a flag name the question item or decision that settles its cause, so it doesn't count, or that the limit count distinct causes.

The instance it reports is already answered where builds read it. The same session that filed the item landed `20261115-tests-that-stay-are-the-design`: the root's Design, which every build's bundle holds, now says that leaving such a test with a `question` item "is this design and not a workaround: the build flags none with `clm workaround`", and the four flags those builds raised were accepted and deleted. No build flags that cause again, so nothing counts it again.

That is also the general answer, and why the behaviour is accepted rather than changed:

- **An answered cause belongs in the spec, not on a backlog item.** A flag records that a build had to work around its requirement (`workaround-flags`). Once the human answers that the situation is the design, the answer goes into the Design or the requirement a build reads, as this one did, and a build that meets it has no workaround to flag. A cause answered only in a decision file or on a `question` item, which no build's bundle holds, is an answer builds can't see; the limit stopping the next build is how that shows, and the fix is to write the answer into the spec, as `redesign-before-workaround` asks first.
- **Flags already raised are settled by the human.** `clm capabilities decide` records a `decided` record that settles a capability's unsettled flags (`effort/unsettled-flags`), and a spec change can accept and delete them, as `20261115-tests-that-stay-are-the-design` did. Either takes the count back down.
- **A flag naming its own settlement would let a session lift a checked limit by its own judgement.** Whether a new flag's cause is the one an earlier answer settled is a judgement; `workaround-flags` rejected exactly that ("the session judging whether to file a question itself", since the threshold is the program's, `deterministic-enforcement`), and `redesign-before-workaround` is a checked constraint. Counting distinct causes needs the same judgement to say which flags share a cause.
- **The freeze is on.** The plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md` gives a defect the default answer "accept" until it is done, and an exception is the human's to make.

Backlog: deletes `the-workaround-limit-counts-one-answered-cause-once-per-build`.

## Edge cases and workarounds

None.
