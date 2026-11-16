---
title: "The workaround limit counts one answered cause once per build that meets it"
kind: bug
components: [check.health]
---

Step: a build flags a requirement with `clm workaround` when it must leave part of it undone.

What happened: several builds of different components met the same cause, one the human had already answered in a decision and on the backlog items. Each build flagged a workaround of its own; the third flag reached the capability's limit, `clm workaround` exited 3, the build stopped work on its requirement and filed a question asking whether to redesign. A later build of another component did exactly the same and filed the same question again.

Expected: an answered cause is not counted again. Either a flag can name the question item or decision that already settles its cause, so it does not count toward the limit, or the limit counts distinct causes and not flags.

Rule or decision involved: `redesign-before-workaround`, `workaround-flags`.
