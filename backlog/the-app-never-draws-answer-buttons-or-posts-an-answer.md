---
title: "The app never draws the answer buttons or posts an answer, since its shell passes the views no answerer"
kind: bug
components: [web, web.views]
---

`web.views` draws answer buttons on a backlog item's page and on Home, and the open answers on the page, when it is given an `Answering` (`web/src/views/answers.ts`: the implementation state's `answers`, the offline flag and `onAnswer`). `web/src/main.ts`, which is `web`'s file, still calls `renderBacklogItem` without one and never calls `renderHome`, so in the running app no button shows and nothing posts to `POST .../backlog/{slug}/answer`.

`web` needs a requirement that builds the `onAnswer` (post `/answer` through `whileWriting`, answer 422 and 404 with a message, then `reloadImpl`) and passes `{answers: impl.answers ?? [], offline: offline(), onAnswer}` to the item page and to Home, with the end-to-end test of the scenarios that name the route and the second request. The unit tests of `web.views` observe the callback only.
