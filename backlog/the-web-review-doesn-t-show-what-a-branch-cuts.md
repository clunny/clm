---
title: "The web review doesn't show what a branch cuts"
kind: idea
components: [web.views]
origin: 20261118-the-pass-s-tooling
---

The Worker's review of a spec branch carries `cuts`, the lines `clm gate` prints for what the branch removes (`web.worker.review/pr-review`), and marks the branch as needing review. No view draws the list: a person approving from the phone sees removed requirements as cards, and reads the removed scenarios, the lost sentences and the tests to delete only in the pull request the review links. A requirement of `web.views` would show the lines above the cards.
