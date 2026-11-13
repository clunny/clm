---
status: applied
resolves:
- the-app-never-draws-answer-buttons-or-posts-an-answer
---

## Intent

Wire the answer buttons into the running app: `web` builds the callback that posts an answer and hands it, with the open answers and the offline flag, to Home and to a backlog item's page, so a person can answer a question or mark a human task done from the browser.

## Problem

`web.views` draws a backlog item's answer buttons, and its open answers, when it is given an `Answering` (`web/src/views/answers.ts`: the implementation state's `answers`, the offline flag and `onAnswer`), and `web.worker/answers` takes the post. Between them nothing is wired: `web/src/main.ts` calls `renderBacklogItem` with no `Answering` and never calls `renderHome`, so the running app shows no button and never posts to `POST .../backlog/{slug}/answer`. No requirement of `web` states the wiring: `web.views/answer-buttons` says what a tap does, but its tests observe only the callback, and `web/home-first` routes to Home and an item's page without saying what they are given. `capability/follow-from-browser`'s scenario **answer on a phone** has no test, which is how the gap went unseen.

## Approaches

- **(a) A requirement of `web` that builds the `Answering` (chosen).** The shell owns the writes (`whileWriting`, `reloadImpl`, `fresh`), as it does for proposals, withdrawals and decisions; the views stay free of the network. One added requirement, `web/answer-from-app`, and `fresh-after-write` names the answer among the writes whose reload carries `fresh`.
- **(b) Let `web.views` post the answer itself.** It would need the shell's write lock and reload, which `web.views` imports nothing of by design (`web`'s **Size**). Not chosen.

## Outcome

- `web/answer-from-app`: Home and a backlog item's page get `{answers: impl.answers ?? [], offline: offline(), onAnswer}`. `onAnswer` posts `{"answer": ...}` through `whileWriting`; on 200 it reloads the implementation state with `fresh` and redraws; 422, 404, other statuses and a failed send each give a message under the buttons. Its build adds the capability test of **answer on a phone**.
- `web/fresh-after-write`: lists an answer among the writes (a wording change; the scenarios are unchanged, so its build re-cites its tests).

## Constraints

- Home is drawn only once `web/home-first` is built (pending, unbuilt); `answer-from-app` relies on it for its Home scenario, in the same component, and is best built with it or after it.
- Edge cases and workarounds known: none.
