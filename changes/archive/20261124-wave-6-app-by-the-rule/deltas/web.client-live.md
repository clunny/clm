---
component: web.client-live
base: { contract: "7e9a8ea5213f0dde", design: "7d0a9a0e0ec6fdbb" }
---

## MODIFIED Contract
Three modules of `web`'s browser app under `web/src/live/`, which `web` imports and wires to the page and its service worker:
- `live.ts`: `LiveSocket`, the repo's live socket (`web-live-channel`). `new LiveSocket(open, onNotice)` takes `open(url)`, which returns a socket: something with `send(text)`, `close()` and the handlers `onopen`, `onmessage`, `onclose` and `onerror` (`SocketLike`); and `onNotice(delivery)`, called with a notice's `delivery`, or with `undefined` for a message that names none. `want(url)` names the URL to hold a socket to, and `want(null)` none. It decides nothing about when to hold a socket or what to do with a notice: that is `web`'s (`web/notice-refresh`).
- `notify.ts`: `Notify`, the Notify me switch (`push-subscriptions-in-ledger`). `new Notify(env)` takes a `NotifyEnv`, through which everything of the browser and the Worker comes in:
  - `pushManager()`: the push manager, or null where the browser can't take push messages. A push manager offers `getSubscription()`, the subscription it holds or null, and `subscribe({userVisibleOnly, applicationServerKey})`; a subscription offers `endpoint`, `toJSON()` and `unsubscribe()`.
  - `key()`: the Worker's push key, or null when it has none; it rejects when the key can't be fetched.
  - `permission()` and `requestPermission()`: the notification permission (`granted`, `denied` or `default`), and asking the person for it.
  - `subscribed(base, endpoint, fresh)`: whether the Worker holds `endpoint` for the repo whose API base is `base`, asked with `fresh` when one is given.
  - `send(method, path, body)`: a `POST` or `DELETE` to the Worker, resolving to the Worker's `Response`, or rejecting.
  - `apiBase(owner, repo)`: the repo's API base, `/api/repos/<o>/<r>`.
  - `online()`: whether the app is online.
  - `storage`: `getItem(key)` and `setItem(key, value)`; a write that fails throws.

  `switchFor(owner, repo)` draws the switch for a repo and returns its element; `refresh(fresh)`, `fresh` optional, updates the switches drawn that are still in the document, and resolves once they are updated.
- `push-sw.ts`: `onPush(self, event)` and `onNotificationClick(self, event)`, the push handlers for the app's service worker to call with its scope, a `PushScope` (`location.origin`, `registration.showNotification`, `clients.matchAll` and `clients.openWindow`), and the event, whose `waitUntil` they hand their work.

## MODIFIED Design
The modules import nothing of `web`'s outside `web/src/live/`, since an import of `web`'s code from here would need `depends_on: [web]`, a cycle with `web`'s dependency on this component (`check/undeclared-imports`). What they need of the app comes in through their constructors and parameters, the repo's API base among them; which browser features, Worker routes and storage stand behind them is `web`'s to state (`web/push-in-the-browser`), so the tests here fake that boundary.

**The socket** (`live-updates`). A browser reports no status for a failed handshake, so every close or failure while a URL is wanted is retried the same way, an `open` that throws (a URL `WebSocket` refuses) among them. `web.live`'s object answers the ping without waking (`web.live/hub`), so a socket a phone's network silently dropped is noticed by the missing `pong` at no cost to the object. A message that isn't `pong` is handed over even when it names no delivery: a notice only says the repo changed, so a refresh it starts does no harm.

**Notifications** (`push-subscriptions-in-ledger`, `notify-switch`, `notify-toggle`, `push-renewal`). The browser holds one push subscription for the app, shared by every repo's switch, so the app remembers which repos it posted that subscription to: the browser may renew the subscription under a new endpoint, which the Worker's ledger doesn't know, and unsubscribing the browser for one repo would silence the others. Without storage the app can't tell that no other switch is on, so it keeps the subscription. A renewal's `DELETE` of the old endpoint is sent once and never again: the endpoint is no longer the browser's, and one left in the ledger receives no further push once the push service reports it gone (`web.worker.replica/push-waiting`). Once the Worker has dropped a repo's endpoint the switch reads off, even when the browser's unsubscribe then fails: the browser keeps a subscription no repo uses, which the next turn-on takes. A key or push manager the env answers null for is kept for the session; one that fails to come is asked for again.

**Push handlers** (`push-shown`). The tag makes a repeat replace the notification. `navigate` rejects for a window the worker doesn't control (one hard-reloaded, or opened before the worker took over), hence the new window; a window that can't be focused gets one too, so a click always shows the page.

## ADDED Requirements

### Requirement: notify-toggle
Turning a `Notify` switch on SHALL ask the env for notification permission when `permission()` isn't `granted`, and when the answer isn't `granted` leave the switch off, with a message beside it whose tap doesn't toggle the switch. Otherwise it SHALL take the push manager's subscription, or subscribe the push manager with `userVisibleOnly` true and the key as `applicationServerKey` when it holds none, `send` `POST <base>/push`, `<base>` the repo's API base, with the subscription's `toJSON()` as its body, and read on once the Worker answers it with a 2xx status. Turning it off SHALL `send` `DELETE <base>/push` with `{"endpoint"}` naming the push manager's subscription's endpoint, or the endpoint remembered for the repo (`push-renewal`) when the push manager holds none, and send nothing when there is neither; once the Worker answers it with a 2xx status, or when nothing was sent, the switch SHALL read off, whatever the browser's unsubscribe (`push-renewal`) then does. A `POST` or `DELETE` the Worker answers with any other status SHALL leave the switch as it was, with a message naming that status; a turn-on or turn-off that fails otherwise (`send` rejecting, or the push manager or the permission ask rejecting) SHALL leave it as it was, with a message saying to try again. While a turn-on or turn-off is in flight the switch SHALL be disabled, another change of it SHALL do nothing, and an update SHALL draw nothing over it.

#### Scenario: turn on
- GIVEN a browser with notification permission granted, and a Worker answering the key `k` and `{"subscribed": false}` for `o/a`
- WHEN the switch for `o/a` is turned on
- THEN the push manager was subscribed with `userVisibleOnly` true and `applicationServerKey` `k`, one `POST /api/repos/o/a/push` carried that subscription's `toJSON()`, and the switch reads on

#### Scenario: permission refused
- GIVEN an env whose permission is `default` and whose permission ask answers `denied`
- WHEN the switch for `o/a` is turned on, and then its message is tapped
- THEN permission was asked for once, nothing was subscribed or sent, a message shows beside the switch, and the switch reads off after the tap

#### Scenario: a write that fails
- GIVEN, each apart, a Worker answering 403 to the `POST`; a `send` that rejects; a push manager whose `subscribe` rejects; and the switch for `o/a` on with a Worker answering 500 to the `DELETE`
- WHEN the switch is turned on in the first three, and off in the last
- THEN it reads off in the first three and on in the last, and the message names 403 in the first, says to try again in the second and third, and names 500 in the last

#### Scenario: off with no browser subscription
- GIVEN the switch for `o/a` drawn reading on, and then a push manager holding no subscription, with `o/a` remembered at `e1`; and, apart, the same with nothing remembered
- WHEN the switch is turned off in each
- THEN in the first one `DELETE /api/repos/o/a/push` named `e1` and `o/a` is no longer remembered, in the second nothing was sent, and the switch reads off in both

#### Scenario: an unsubscribe that fails
- GIVEN the switch for `o/a` on, `o/a` the only repo remembered, and a browser subscription whose `unsubscribe` rejects
- WHEN the switch is turned off
- THEN one `DELETE` was sent, the switch reads off with no message, and `o/a` is no longer remembered

#### Scenario: busy
- GIVEN a switch for `o/a`, off, and a Worker that holds its answer to the `POST`
- WHEN the switch is turned on, then changed again, then `refresh()` is called, and then the Worker answers 200
- THEN the switch is disabled until the `POST` is answered, one `POST` was sent, and the switch then reads on and is enabled

## MODIFIED Requirements

### Requirement: live-updates <!-- base: 468e272c8c8030d6 -->
`LiveSocket` SHALL hold one socket, made by the `open` it was given, for the URL its last `want` named, and none after `want(null)`: a different URL SHALL close the socket it holds and open one for the new URL, and the same URL again SHALL change nothing. While a URL is wanted it SHALL open a socket again after any close or failure of the one it holds, an `open` that throws included, with a delay starting at 1 second and doubling to at most 60, back to 1 once a socket stays open for a minute; a close or failure of a socket it no longer holds SHALL open nothing. Once a socket opens it SHALL send `ping` every 50 seconds, and close and reopen, as for a failure, a socket that gives no `pong` within 10 seconds of a `ping`. It SHALL hand every other message to its notice callback once: with the message's `delivery` when the message is text that parses as JSON whose `delivery` is a string, and with none otherwise.

#### Scenario: a notice
- GIVEN a `LiveSocket` wanting `wss://h/api/repos/o/a/live` with its socket open
- WHEN the socket receives `{"delivery": "d1"}`
- THEN the notice callback was called once, with `d1`

#### Scenario: a message naming no delivery
- GIVEN a `LiveSocket` with its socket open
- WHEN the socket receives `not json`, then `{"x": 1}`, then `{"delivery": 5}`, then `pong`
- THEN the notice callback was called three times, each with no delivery

#### Scenario: a silent drop
- GIVEN a `LiveSocket` with its socket open, and a socket that stops answering `ping`
- WHEN 61 seconds pass
- THEN it closed that socket after 60 seconds, and opened a new one a second later

#### Scenario: backoff
- GIVEN a `LiveSocket` whose sockets each close as soon as they are opened
- WHEN time passes
- THEN it opens them again after 1, 2 and 4 seconds, and never waits more than 60

#### Scenario: a socket that can't be opened
- GIVEN an `open` that throws on its first call and then returns a socket
- WHEN `want(url)` is called, and then 1 second passes
- THEN `want` returned without throwing, and a socket for `url` was opened after that second

#### Scenario: none wanted
- GIVEN a `LiveSocket` with its socket open
- WHEN `want(null)` is called and 5 minutes pass
- THEN the socket was closed and no other was opened

### Requirement: notify-switch <!-- base: bc04aecc59cd9a96 -->
`Notify`'s switch for a repo SHALL be hidden until the env gives both a push manager and a key, and stay hidden, asking for neither again, when either is null; when either fails to come (its call rejects) the switch SHALL stay hidden and both be asked for again at the next update. Drawing a switch updates it, and `refresh(fresh)` SHALL update every switch drawn that is still in the document and none other. An update SHALL set the switch to what the env's `subscribed` answers for the repo's API base and the endpoint of the push manager's subscription, asked with `fresh` when `refresh` was given one, and to off without asking when the push manager holds no subscription or rejects the request for it. Updates that ask for the same base, endpoint and `fresh` while such an ask is in flight SHALL share it. A switch drawn for a repo whose state this `Notify` has already shown SHALL show that state at once while it asks, and keep it when the ask fails; a first ask that fails SHALL read off. The switch SHALL be disabled when `online()` answers false as it is drawn or updated.

#### Scenario: no key or no Web Push
- GIVEN an env whose key is null, and, apart, an env whose push manager is null
- WHEN the switch for `o/a` is drawn in each, and then `refresh()` is called
- THEN it is hidden, and the env was asked for the key at most once

#### Scenario: the key can't be fetched
- GIVEN an env whose `key()` rejects, and then answers `k`
- WHEN the switch for `o/a` is drawn into the document, and then `refresh()` is called
- THEN it is hidden after the draw and shown after the refresh

#### Scenario: nothing to go on
- GIVEN a push manager whose `getSubscription` rejects, and, apart, a browser subscription whose first `subscribed` ask rejects
- WHEN the switch for `o/a` is drawn in each
- THEN it is shown off, and in the first nothing was asked

#### Scenario: redrawn with a known state
- GIVEN a switch for `o/a` that read on, and a Worker whose push route then fails
- WHEN the switch for `o/a` is drawn again
- THEN it shows on before the push route answers, and still reads on after the request fails

#### Scenario: two switches at once
- GIVEN a browser subscription at `e1`
- WHEN two switches for `o/a` are drawn together
- THEN `subscribed` was asked once, for `/api/repos/o/a` and `e1`, and both switches read its answer

#### Scenario: refreshed
- GIVEN a browser subscription, two switches for `o/a` and `o/b` drawn and in the document, and then the one for `o/b` removed from it
- WHEN `refresh('d1')` is called
- THEN `subscribed` was asked once more, for `/api/repos/o/a` with `fresh` `d1`, and the switch for `o/a` reads its answer

### Requirement: push-shown <!-- base: c8f12d9cabb37145 -->
`onPush` SHALL show each push message as a notification with the payload's `title` and `body`, tagged with its `url`, with the icon `/app/icon-192.png` and the `url` in its data. A `title` that is missing or not a string, as when the message has no payload or one that isn't a JSON object, SHALL give the title `CLM`; a `body` that is missing or not a string, an empty body; and a `url` that is missing or not a string, `/app/`. `onNotificationClick` SHALL close the notification and take its `url` resolved against `/app/` on the worker's origin, replaced by that root, `/app/` on the worker's origin, when it isn't a string, doesn't parse or lies on another origin than the worker's. It SHALL then focus an open app window, one whose URL is under the worker's origin's `/app/`, and navigate it to that url, and SHALL open a window at that url instead when no app window is open or the window rejects the focus or the navigation.

#### Scenario: notification click
- GIVEN a fake service-worker `self` at `https://clm-app.dev` with no open app window
- WHEN the push handler receives `{"title": "o/a: waiting on you", "body": "Which region?", "url": "/app/#/o/a/backlog"}` and the click handler gets its notification
- THEN a notification with that title and body, tagged `/app/#/o/a/backlog`, with the icon `/app/icon-192.png`, was shown, the notification was closed on the click, and a window was opened at `https://clm-app.dev/app/#/o/a/backlog`

#### Scenario: a site page is open
- GIVEN a fake service-worker `self` at `https://clm-app.dev` whose only open window is at `https://clm-app.dev/guide/quickstart`
- WHEN the click handler gets a notification for `/app/#/o/a/backlog`
- THEN that window isn't navigated, and a window is opened at `https://clm-app.dev/app/#/o/a/backlog`

#### Scenario: a push with no payload
- GIVEN a fake service-worker `self`
- WHEN the push handler receives a message with no payload, and another whose payload isn't JSON
- THEN each shows a notification titled `CLM`

#### Scenario: a payload of the wrong shape
- GIVEN a fake service-worker `self`
- WHEN the push handler receives the payloads `"x"`, `{"title": 5, "body": 5, "url": 5}` and `{"title": "t"}`
- THEN the first two are titled `CLM` and the third `t`, every body is empty, and every notification is tagged `/app/`

#### Scenario: a window that can't be navigated
- GIVEN a fake service-worker `self` at `https://clm-app.dev` whose only open window is at `https://clm-app.dev/app/#/o/a/graph`, and whose `navigate` rejects; and, apart, the same window whose `focus` rejects
- WHEN the click handler gets a notification for `/app/#/o/a/backlog` in each
- THEN a window is opened at `https://clm-app.dev/app/#/o/a/backlog` in each

#### Scenario: a url on another origin
- GIVEN a fake service-worker `self` at `https://clm-app.dev` with no open window
- WHEN the click handler gets a notification for `https://evil.example/x`, and another for `http://[`
- THEN a window is opened at `https://clm-app.dev/app/` for each

### Requirement: push-renewal <!-- base: d02c4647b215da8b -->
`Notify` SHALL remember, in the env's storage under `clm-push`, a JSON object from each repo `<owner>/<repo>` whose switch it turned on to the endpoint it posted for it, and forget the repo when its switch is turned off; a stored value that isn't a JSON object, an array included, SHALL count as nothing remembered. When it next updates a switch and the push manager's subscription's endpoint differs from a repo's remembered one, it SHALL post the subscription to that repo's push route, remember the new endpoint, and then send one `DELETE` of the old one there, not sent again whatever it answers; a repo whose post fails SHALL keep its old endpoint and be tried again at the next update, and switches updated at once SHALL move each repo once. Turning a switch off SHALL unsubscribe the browser's subscription only when no repo is left remembered, and never once a write to storage has failed since the `Notify` was made.

#### Scenario: a renewed browser subscription
- GIVEN a browser subscription at `https://fcm.googleapis.com/fcm/send/renewed`, with `o/a` remembered at `.../old` and `o/b` at `.../renewed`
- WHEN the switches for `o/a` and `o/b` are drawn together
- THEN one `POST /api/repos/o/a/push` carried the subscription and one `DELETE /api/repos/o/a/push` named `.../old`, nothing was sent for `o/b`, and both repos are remembered at `.../renewed`

#### Scenario: a renewal whose post fails is tried again
- GIVEN that renewed browser subscription with `o/a` remembered at `.../old`, and a Worker answering 500 to every write; and, apart, a Worker answering 200 to the `POST` and 500 to the `DELETE`
- WHEN the switch for `o/a` is drawn, then the Worker answers 200 to every write and the switches are refreshed
- THEN in the first, after the draw `o/a` is still remembered at `.../old`, and after the refresh a `POST` then a `DELETE` were sent and `o/a` is remembered at `.../renewed`; in the second, one `POST` and one `DELETE` were sent in all, and `o/a` is remembered at `.../renewed` after the draw

#### Scenario: unreadable storage
- GIVEN `clm-push` holding `not json`, and, apart, `5`, and, apart, `["x"]`, with a browser subscription at `e1`
- WHEN the switch for `o/a` is drawn, and then turned on
- THEN nothing was sent at the draw, and `clm-push` then holds `{"o/a": "e1"}`

#### Scenario: the last switch turned off
- GIVEN a browser subscription remembered for `o/a` and `o/b`, both switches on
- WHEN the switch for `o/a` is turned off, then the one for `o/b`
- THEN each sent its `DELETE`, the browser was not unsubscribed after the first, and was unsubscribed after the second

#### Scenario: without storage
- GIVEN storage whose writes throw, and a browser subscription with the switch for `o/a` on
- WHEN the switch is turned off, on and off again
- THEN `DELETE`, `POST` and `DELETE` were sent, and the browser was never unsubscribed
