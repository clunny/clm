---
title: Client live
status: active
owners: ["@clunny"]
depends_on: []
paths: ["web/src/live/**"]
successor: ""
serves: [capability/follow-from-browser]
---

## Contract
Three modules of `web`'s browser app under `web/src/live/`, each with its Vitest tests beside it, which `web/src/main.ts` imports and wires to the page:
- `live.ts`: `LiveSocket`, the repo's live socket (`web-live-channel`). It is made with a function that opens a socket for a URL and a callback for notices; `want(url)` holds one socket to `url`, and `want(null)` holds none. It hands the callback each notice's `delivery`, and decides nothing about when to hold a socket or what to do with a notice: that is `web`'s (`web/notice-refresh`).
- `notify.ts`: `Notify`, the Notify me switch (`push-subscriptions-in-ledger`), given what the browser and the Worker offer through `NotifyEnv`.
- `push-sw.ts`: `onPush` and `onNotificationClick`, the service worker's push handlers, which `web/vite.config.ts` builds into `sw-push.js` and `web/sw.js` imports.

These modules import no module of `web`'s outside `web/src/live/`: what they need of the app comes in through their constructors and parameters, so `web` depends on this component and not the reverse (`check/undeclared-imports`).

## Design
The modules import nothing of `web`'s outside `web/src/live/`, since an import of `web`'s code from here would need `depends_on: [web]`, a cycle with `web`'s dependency on this component (`check/undeclared-imports`). The repo's API base comes in through `NotifyEnv` or as a parameter, and the switch's few elements are built with `document.createElement` or a builder passed in, the builder's choice.

**The socket.** `LiveSocket` holds at most one WebSocket, to the URL its last `want` named (`live-updates`). A browser reports no status for a failed handshake, so every close or failure while a URL is wanted is retried the same way. The ping is answered by `web.live`'s object with `pong` without waking (its hibernation auto-response), so a socket a phone's network silently dropped is noticed by the missing `pong`. Any other message is a notice, `{"delivery"}`, and its `delivery` goes to the callback (undefined for one that names none). `want` with another URL closes the socket held and opens one to the new URL.

**Notifications** (`push-subscriptions-in-ledger`, `notify-switch`). `Notify` draws the switch only once the service worker is registered as well as where the browser has `Notification` and `PushManager` and the Worker answered a key. Its state is `GET /api/repos/<o>/<r>/push?endpoint=<the browser's subscription's endpoint>`, off when the browser has no subscription. Turning it on reuses the existing subscription and subscribes `pushManager` with the Worker's key as `applicationServerKey`; a refused permission leaves it off and says so. Turning it off unsubscribes the browser only when no other repo's switch is on, as the app remembers them. The app remembers in `localStorage` the endpoint it posted for each repo; when the browser's subscription has a different endpoint (the browser renewed it), it deletes the remembered one and posts the new one for each repo whose switch was on. Where the switch sits on the page is `web`'s.

**Push handlers** (`push-shown`). Both handlers are functions in `push-sw.ts`; the notification is tagged with its `url` so a repeat replaces it.

## Requirements

### Requirement: live-updates
`LiveSocket` SHALL hold one WebSocket to the URL its last `want` named and none after `want(null)`, closing the one it holds when the URL changes, and opening it again after any close or failure with a delay starting at 1 second and doubling to at most 60, back to 1 once a socket stays open for a minute. It SHALL send `ping` every 50 seconds and reopen a socket that gives no `pong` within 10. It SHALL hand every other message's `delivery` to its notice callback, once per message.

#### Scenario: a notice
- GIVEN a `LiveSocket` wanting `wss://h/api/repos/o/a/live` with its socket open
- WHEN the socket receives `{"delivery": "d1"}`
- THEN the notice callback was called once, with `d1`

#### Scenario: a silent drop
- GIVEN a `LiveSocket` with its socket open, and a socket that stops answering `ping`
- WHEN 60 seconds pass
- THEN it closed that socket and opened a new one

#### Scenario: backoff
- GIVEN a `LiveSocket` whose sockets each close as soon as they are opened
- WHEN time passes
- THEN it opens them again after 1, 2 and 4 seconds, and never waits more than 60

#### Scenario: none wanted
- GIVEN a `LiveSocket` with its socket open
- WHEN `want(null)` is called and 5 minutes pass
- THEN the socket was closed and no other was opened

#### Needs
- web.worker/live-socket

### Requirement: notify-switch
The app SHALL show a Notify me switch for the repo on screen where the browser supports Web Push and the Worker answers a push key, and none elsewhere. Turning it on SHALL ask for notification permission when needed, subscribe the service worker with `userVisibleOnly` and the Worker's key, and post the subscription to the repo's push route; turning it off SHALL delete that endpoint's subscription there. Its state SHALL be what the push route answers for the browser's endpoint, and it SHALL be disabled offline.

#### Scenario: turn on
- GIVEN a browser with notification permission granted, and a Worker answering the key `k` and `{"subscribed": false}` for `o/a`
- WHEN the switch for `o/a` is turned on
- THEN the push manager was subscribed with `applicationServerKey` `k`, one `POST /api/repos/o/a/push` carried that subscription, and the switch reads on

#### Scenario: no key
- GIVEN a Worker answering 404 for the push key
- WHEN the sidebar is shown
- THEN it shows no Notify me switch

#### Needs
- web.worker/push-subscriptions

### Requirement: push-shown
The service worker SHALL show each push message's `title` and `body` as a notification tagged with its `url`, with the app's icon from under `/app/`, taking `/app/` as the `url` when the message has none, and a click on it SHALL focus an open app window, one whose URL is under `/app/`, and navigate it to that `url`, or open a window there when none is open.

#### Scenario: notification click
- GIVEN a fake service-worker `self` with no open app window
- WHEN the push handler receives `{"title": "o/a: waiting on you", "body": "Which region?", "url": "/app/#/o/a/backlog"}` and the click handler gets its notification
- THEN a notification with that title and body, tagged `/app/#/o/a/backlog`, was shown, and a window was opened at `/app/#/o/a/backlog`

#### Scenario: a site page is open
- GIVEN a fake service-worker `self` whose only open window is at `https://clm-app.dev/guide/quickstart`
- WHEN the click handler gets a notification for `/app/#/o/a/backlog`
- THEN that window isn't navigated, and a window is opened at `/app/#/o/a/backlog`
