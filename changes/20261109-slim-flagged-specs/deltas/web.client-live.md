---
component: web.client-live
base: { contract: "6af14d29106f0a1e", design: "49a868a5ef71cdde" }
---

## MODIFIED Contract
Three modules of `web`'s browser app under `web/src/live/`, each with its Vitest tests beside it, which `web/src/main.ts` imports and wires to the page:
- `live.ts`: `LiveSocket`, the repo's live socket (`web-live-channel`), made with a function that opens a socket for a URL and a callback for notices, and steered with `want(url)` and `want(null)`. It decides nothing about when to hold a socket or what to do with a notice: that is `web`'s (`web/notice-refresh`).
- `notify.ts`: `Notify`, the Notify me switch (`push-subscriptions-in-ledger`), given what the browser and the Worker offer through `NotifyEnv`; `switchFor(owner, repo)` draws one and `refresh(fresh)` asks again for those on screen.
- `push-sw.ts`: `onPush` and `onNotificationClick`, the service worker's push handlers, which `web/vite.config.ts` builds into `sw-push.js` and `web/sw.js` imports.

## MODIFIED Design
The modules import nothing of `web`'s outside `web/src/live/`, since an import of `web`'s code from here would need `depends_on: [web]`, a cycle with `web`'s dependency on this component (`check/undeclared-imports`). What they need of the app comes in through their constructors and parameters: the repo's API base through `NotifyEnv` or as a parameter, and the switch's few elements are built with `document.createElement` or a builder passed in, the builder's choice.

**The socket** (`live-updates`). A browser reports no status for a failed handshake, so every close or failure while a URL is wanted is retried the same way. `web.live`'s object answers the ping with `pong` without waking (its hibernation auto-response), so a socket a phone's network silently dropped is noticed by the missing `pong`.

**Notifications** (`push-subscriptions-in-ledger`, `notify-switch`, `push-renewal`). The browser holds one push subscription for the app, shared by every repo's switch, so the app remembers which repos it posted that subscription to: the browser may renew the subscription under a new endpoint, which the Worker's ledger doesn't know, and unsubscribing the browser for one repo would silence the others. Without storage the app can't tell that no other switch is on, so it keeps the subscription. Where the switch sits on the page is `web`'s.

**Push handlers** (`push-shown`). The notification is tagged with its `url` so a repeat replaces it. A click only ever opens the app's own origin, whatever the message names. `navigate` rejects for a window the worker doesn't control (one hard-reloaded, or opened before the worker took over), hence the new window.

## MODIFIED Requirements
### Requirement: notify-switch <!-- base: 59a9bf958dfd7fa6 -->
The app SHALL show a Notify me switch for the repo on screen once a service worker is registered, the browser has `Notification` and `PushManager`, and the Worker answers a push key, and none otherwise. Its state SHALL be what `GET /api/repos/<o>/<r>/push?endpoint=<endpoint>` answers for the browser's subscription's endpoint, off without asking when the browser has no subscription, asked again with `fresh` when `refresh(fresh)` is called, and it SHALL be disabled offline. Turning it on SHALL ask for notification permission when it isn't granted, and when refused leave the switch off with a message beside it, outside its label so tapping the message doesn't toggle it; otherwise it SHALL take the browser's subscription, or subscribe the push manager with `userVisibleOnly` and the Worker's key as `applicationServerKey` when there is none, and post it to `POST /api/repos/<o>/<r>/push`. Turning it off SHALL send `DELETE /api/repos/<o>/<r>/push` with that endpoint. A post or delete the Worker refuses SHALL leave the switch as it was and say so.

#### Scenario: turn on
- GIVEN a browser with notification permission granted, and a Worker answering the key `k` and `{"subscribed": false}` for `o/a`
- WHEN the switch for `o/a` is turned on
- THEN the push manager was subscribed with `applicationServerKey` `k`, one `POST /api/repos/o/a/push` carried that subscription, and the switch reads on

#### Scenario: no key or no Web Push
- GIVEN a Worker answering 404 for the push key, and, apart, a browser with no push manager or no registered service worker
- WHEN the switch for `o/a` is drawn in each
- THEN it is hidden

#### Needs
- web.worker/push-subscriptions

### Requirement: push-shown <!-- base: 28764ae85563c81b -->
The service worker SHALL show each push message's `title` and `body` as a notification tagged with its `url`, with the app's icon from under `/app/`, titled `CLM` when the message has no title or its payload is missing or not JSON, and taking `/app/` as the `url` when the message has none. A click on it SHALL focus an open app window, one whose URL is under the worker's origin's `/app/`, and navigate it to that `url`, and SHALL open a window at that `url` instead when no app window is open or the window rejects the navigation. A `url` on another origin than the worker's SHALL be replaced by the app's root, `/app/` on the worker's origin.

#### Scenario: notification click
- GIVEN a fake service-worker `self` with no open app window
- WHEN the push handler receives `{"title": "o/a: waiting on you", "body": "Which region?", "url": "/app/#/o/a/backlog"}` and the click handler gets its notification
- THEN a notification with that title and body, tagged `/app/#/o/a/backlog`, was shown, and a window was opened at `/app/#/o/a/backlog`

#### Scenario: a site page is open
- GIVEN a fake service-worker `self` whose only open window is at `https://clm-app.dev/guide/quickstart`
- WHEN the click handler gets a notification for `/app/#/o/a/backlog`
- THEN that window isn't navigated, and a window is opened at `/app/#/o/a/backlog`

#### Scenario: a push with no payload
- GIVEN a fake service-worker `self`
- WHEN the push handler receives a message with no payload, and another whose payload isn't JSON
- THEN each shows a notification titled `CLM`

#### Scenario: a window that can't be navigated
- GIVEN a fake service-worker `self` at `https://clm-app.dev` whose only open window is at `https://clm-app.dev/app/#/o/a/graph`, and whose `navigate` rejects
- WHEN the click handler gets a notification for `/app/#/o/a/backlog`
- THEN a window is opened at `https://clm-app.dev/app/#/o/a/backlog`

#### Scenario: a url on another origin
- GIVEN a fake service-worker `self` at `https://clm-app.dev` with no open window
- WHEN the click handler gets a notification for `https://evil.example/x`
- THEN a window is opened at `https://clm-app.dev/app/`

## ADDED Requirements
### Requirement: push-renewal
The app SHALL remember in `localStorage`, under `clm-push`, the endpoint it posted for each repo `<owner>/<repo>` whose switch it turned on, and forget the repo when its switch is turned off. When it next updates a switch and the browser's subscription's endpoint differs from a repo's remembered one, it SHALL post the subscription to that repo's push route, remember the new endpoint, and delete the old one there; a repo whose post fails SHALL keep its old endpoint and be tried again at the next update. Turning a switch off SHALL unsubscribe the browser only when no repo is left remembered, and never once a write to storage has failed since the app loaded.

#### Scenario: a renewed browser subscription
- GIVEN a browser subscription at `https://fcm.googleapis.com/fcm/send/renewed`, with `o/a` remembered at `.../old` and `o/b` at `.../renewed`
- WHEN the switch for `o/a` is drawn
- THEN `POST /api/repos/o/a/push` carried the subscription and `DELETE /api/repos/o/a/push` named `.../old`, nothing was sent for `o/b`, and both repos are remembered at `.../renewed`

#### Scenario: a renewal whose post fails is tried again
- GIVEN that renewed browser subscription with `o/a` remembered at `.../old`, and a Worker answering 500 to every write
- WHEN the switch for `o/a` is drawn, then the Worker answers 200 and the switches are refreshed
- THEN after the draw `o/a` is still remembered at `.../old`, and after the refresh a `POST` then a `DELETE` were sent and `o/a` is remembered at `.../renewed`

#### Scenario: the last switch turned off
- GIVEN a browser subscription remembered for `o/a` and `o/b`, both switches on
- WHEN the switch for `o/a` is turned off, then the one for `o/b`
- THEN each sent its `DELETE`, the browser was not unsubscribed after the first, and was unsubscribed after the second

#### Scenario: without storage
- GIVEN storage whose writes throw, and a browser subscription with the switch for `o/a` on
- WHEN the switch is turned off, on and off again
- THEN `DELETE`, `POST` and `DELETE` were sent, and the browser was never unsubscribed

#### Needs
- web.worker/push-subscriptions
