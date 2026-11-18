---
component: web
---

## MODIFIED Requirements

### Requirement: push-in-the-browser <!-- base: b9d56673da12c865 -->
The built app, opened in a browser, SHALL wire `web.client-live`'s Notify me switch and push handlers into the page and its service worker (`web.client-live/notify-switch`, `push-shown`): the switch shows in the sidebar once the Worker answers a push key, and is hidden when it answers 404; and the service worker the app registers, `/app/sw.js` with scope `/app/`, shows a push and handles a click on it through the handlers it imports.

#### Scenario: no key
- GIVEN a 1280 px wide window, and a Worker answering 404 for the push key
- WHEN `#/o/a/graph` is opened and the key has been asked for
- THEN the sidebar shows no switch

#### Scenario: a key and no subscription
- GIVEN a Worker answering a push key, and a browser with no subscription
- WHEN `#/o/a/graph` is opened
- THEN the sidebar shows the switch, off and enabled, and no request went to the repo's push route

#### Scenario: turn on
- GIVEN a browser with notification permission granted, and a Worker answering the key `k` and `{"subscribed": false}` for `o/a`
- WHEN the switch is turned on, and then another view is chosen
- THEN the push manager was subscribed with `userVisibleOnly` and `applicationServerKey` `k`, one `POST /api/repos/o/a/push` carried that subscription, the switch reads on, and the redraw asked the push route for the subscription's endpoint and still reads on

#### Scenario: the service worker shows a push
- GIVEN the app opened at `/app/`, its service worker `/app/sw.js` active, and no app window known to it
- WHEN the worker receives a push `{"title": "o/a: waiting on you", "body": "Which region?", "url": "/app/#/o/a/backlog"}`, and then a click on its notification
- THEN a notification with that title and body, tagged `/app/#/o/a/backlog`, with the icon `/app/icon-192.png`, was shown, and a window was opened at `/app/#/o/a/backlog` on the worker's origin

#### Serves
- capability/follow-from-browser
