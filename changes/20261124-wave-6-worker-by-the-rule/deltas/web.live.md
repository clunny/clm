---
component: web.live
base: { contract: "46c4520df34fe3ac", design: "fcba23432f2a1200" }
---

## MODIFIED Contract
Crate `clm-web-live`, the two ways `web` reaches a browser without being asked (`web-live-channel`, `push-subscriptions-in-ledger`), built for `wasm32-unknown-unknown` and linked into the Worker by `clm-web`:
- **`Live`**, a Durable Object class, one object per repo. A `GET` with `Upgrade: websocket` opens a socket it holds; any other `GET` answers 426; `POST /notify` sends its body, unchanged, to every socket it holds, and answers 422 to a body that isn't UTF-8 text; `POST /send` takes `{"messages": [{"subscription": {"endpoint", "p256dh", "auth"}, "payload": <JSON>}, ...]}` and delivers each as a Web Push message, answering `{"gone": [<endpoint>, ...]}`; anything else answers 404. A socket that sends `ping` is answered `pong`. It stores nothing, and decides nothing about who may connect or be notified: the Worker checks the session and the repo's access before it forwards a socket, and the repo's `Replica` object (`web.worker.replica`) chooses the recipients before it posts a send.
- **`MAX_MESSAGES`** (`pub const MAX_MESSAGES: usize`, 10), the most messages one `/send` takes, which the `Replica` object cuts its batch to.
- **`web_push(&Subscription, payload: &[u8], &Vapid, now_secs: u64, random: [u8; 48]) -> Result<PushRequest, PushError>`**, which builds the HTTP request that delivers `payload` to one browser's push subscription: the body encrypted as RFC 8291 says, and an RFC 8292 VAPID `Authorization` header signed with the App's key. It sends nothing. `Subscription` is `{endpoint, p256dh, auth}`, as `PushSubscription.toJSON()` gives them; `Vapid` is `{public_key, private_key}`; `PushRequest` is `{url, headers: Vec<(String, String)>, body: Vec<u8>}`, to be `POST`ed; `PushError` is one of `P256dh`, `Auth`, `Endpoint`, `Vapid`, `Random` and `Payload`, and its text names the input. It is offered for its proof, the one call that takes `random` as a parameter: `/send` builds its requests the same way.
- **`Item {slug, kind, title}`**, a backlog item as the snapshot holds it, which the `Replica` object builds by literal, and **`Notice {title, body, url}`**, a notification's payload, which serialises as those three JSON keys and which the `Replica` object posts to `/send` and the app's service worker reads. A field added to either is a change in steps (`expand-contract`).
- **`waiting_notice(repo: &str, previous: &[Item], new: &[Item]) -> Option<Notice>`** and **`review_notice(repo: &str, event: &serde_json::Value) -> Option<Notice>`**, the notices the repo's `Replica` object sends after it applies a webhook's change: what a push to the default branch, given the backlog items before and after, and a `pull_request` event announce, if anything.

## MODIFIED Design
**The object.** `Live` uses the WebSocket hibernation API (`state.accept_web_socket`, and `state.get_websockets()` to broadcast), so an object whose sockets are idle is evicted from memory and costs no duration, and its sockets survive the eviction (`web-on-free-plan`). It implements `websocket_message`, `websocket_close` and `websocket_error`, whose defaults in `worker` 0.8.7 panic. The `ping` → `pong` answer is the runtime's hibernation auto-response (`set_websocket_auto_response`), which the runtime gives without waking the object, so a page can tell a dead socket from a quiet one; a `ping` therefore never reaches `websocket_message`. The object writes nothing to its storage. The rules (what to answer for each request, the `Upgrade` header's match and a body's decoding included, what to broadcast, which auto-response to ask for) sit in a core generic over a trait `Host` for the set of sockets, the auto-response, the secrets, the clock, the randomness and `fetch`, which native tests drive with a fake Host; the fake answers an auto-response as the runtime's documentation says, so a test of `ping` shows what the core asks for and the fake's reading of the runtime, not the runtime. The core's `start(host)` asks the host for the `ping` → `pong` pair, and `Live::new` calls it. The `durable` feature only binds that core to `worker`'s types: it hands the core a request's method, path, its `Upgrade` header's value, and its body as bytes, and decides nothing. The trait gives the core no storage, so its rules can't write any: that the object stores nothing holds by construction, and no test checks it.

**Sending.** `/send` builds each message with the same steps as `web_push`, and sends them together with `worker::Fetch`. Sending runs in this object rather than in the `Replica` object's alarm, so its crypto (three P-256 operations per message) stays apart from the recompute (`web-on-free-plan`). The VAPID JWT depends only on the endpoint's origin, so one send signs one JWT per origin and reuses it. The object keeps nothing between sends, so it can't queue a retry or honour `Retry-After`: the next change sends a fresh notice. The random bytes come from `crypto.getRandomValues`, as `clm-web`'s `Secrets::random` gets them.

**Web Push.** `web_push` follows RFC 8291: the sender's P-256 key pair from the first 32 bytes of `random` and the 16-byte salt from the next 16, ECDH with the subscription's `p256dh`, HKDF-SHA-256 with its `auth` secret to the content key and nonce, one AES-128-GCM record with the `0x02` padding delimiter, and the header block (salt, record size 4096, the sender's public key) in front; one record keeps the body within the 4096 bytes RFC 8030 lets a push service refuse beyond. The JWT is `{"aud", "exp": now + 12 h, "sub"}` under the header `{"typ":"JWT","alg":"ES256"}`. The keys are the secrets `VAPID_PUBLIC_KEY` (the uncompressed point) and `VAPID_PRIVATE_KEY` (the 32-byte scalar), base64url as `npx web-push generate-vapid-keys` prints them. The crypto is pure Rust, `p256` 0.13 (with its `ecdh` and `ecdsa` features), `hkdf` 0.12, `aes-gcm` 0.10 and `sha2` 0.10; the randomness is a parameter, so the crypto needs no `getrandom` and tests natively.

**Payloads.** A review notice's url encodes the branch as one path segment with the `percent-encoding` crate, declared in this crate's own `Cargo.toml` as its crypto crates are, as `web.worker.platform`'s `http` encodes a segment; neither crate depends on the other. One push is one notice, so a subscriber gets one notification however many items it adds.

**Building.** The crate depends on `worker` only under its `durable` feature, which `clm-web` enables in its `wasm32` target dependencies, so native builds and tests compile neither. Its `pub` items are what `Live`, its dependants and its tests use, the core (`Host`, `Request`, `Answer`, `fetch`, `start`, `on_message`, `on_close`) for its tests; an item only the crate itself uses is `pub(crate)`, never an `--allow` entry in `clm.toml` (`unreferenced-allow-then-narrow`).

## MODIFIED Requirements

### Requirement: hub <!-- base: 688e05696360e127 -->
The `Live` object SHALL answer a `GET` with `Upgrade: websocket` (the header's value in any case) with 101 and a new socket it then holds, which stays open, and keeps receiving notices, while the object is evicted from memory; or, when the runtime can't make a socket pair, with 500, holding no new socket; and SHALL answer 426 to any other `GET`. On `POST /notify` it SHALL send the request's body, unchanged, as one text message to every socket it holds, skipping any whose send fails, and answer 204; to a `POST /notify` whose body isn't UTF-8 text it SHALL answer 422, sending nothing. A socket that sends the text `ping` SHALL receive the text `pong`, each time, whether or not the object has been evicted since, and the `ping` SHALL reach no other socket. It SHALL ignore any other message a socket sends, leaving the socket open, SHALL close a socket the browser closes or that errors, and SHALL answer 404, sending nothing, to any request other than a `GET`, `POST /notify` and `POST /send`, whatever its body.

#### Scenario: two pages
- GIVEN the `Live` object holding sockets `s1` and `s2`, each opened by a `GET` with `Upgrade: websocket` (`s2`'s written `WebSocket`) that was answered 101, and `s1` then closed by its browser
- WHEN `POST /notify` arrives with `{"delivery": "d1"}`, and, with no socket held, again
- THEN `s2` receives exactly that text, `s1` receives nothing, and each answer is 204

#### Scenario: not a socket
- GIVEN the `Live` object holding a socket
- WHEN a `GET` without `Upgrade: websocket` arrives, and the socket sends `hello` and then errors
- THEN the `GET` gets 426, nothing was sent to the socket, it was open after `hello`, and it is closed after the error

#### Scenario: anything else
- GIVEN the `Live` object holding a socket
- WHEN `POST /other`, `PUT /notify` and `DELETE /` arrive, each with body `x`
- THEN each gets 404 and the socket received nothing

#### Scenario: a body that isn't text
- GIVEN the `Live` object holding a socket
- WHEN `POST /notify`, `POST /send` and `POST /other` arrive, each with the body bytes `ff fe`
- THEN they get 422, 422 and 404, and the socket received nothing

#### Scenario: no socket pair
- GIVEN a runtime that can't make a socket pair
- WHEN a `GET` with `Upgrade: websocket` arrives
- THEN it gets 500 and the object holds no socket

#### Scenario: ping
- GIVEN the `Live` object started, holding sockets `s1` and `s2`
- WHEN `s1` sends `ping`, twice
- THEN `s1` receives `pong` each time, `s2` receives nothing, and both are open

### Requirement: push-send <!-- base: 50c3e1cbd6cba542 -->
On `POST /send` with a body `{"messages": [{"subscription": {"endpoint", "p256dh", "auth"}, "payload": <JSON>}, ...]}` of at most `MAX_MESSAGES` (10) messages, the `Live` object SHALL post each message's `web_push` request, built for the payload serialised as JSON, signing one VAPID JWT per endpoint origin per send, and SHALL answer 200 with `{"gone": [...]}` listing each endpoint the push service answered 404 or 410. An endpoint answered anything else, or whose post fails, SHALL be left out of `gone` and not posted again, and a message whose subscription or payload `web_push` refuses SHALL be skipped, neither posted nor in `gone`, the others still posted. With more than `MAX_MESSAGES` messages, or a body that doesn't parse, it SHALL answer 422 whatever its secrets; otherwise without both secrets `VAPID_PUBLIC_KEY` and `VAPID_PRIVATE_KEY`, with one that doesn't parse, or with no randomness from the runtime for its messages, it SHALL answer 503. Neither a 422 nor a 503 posts anything.

#### Scenario: one gone
- GIVEN the `Live` object with its VAPID secrets set, messages for `https://fcm.googleapis.com/fcm/send/a` and `https://fcm.googleapis.com/fcm/send/b`, and a push service that answers 201 for `a` and 410 for `b`; and the same with `b` answered 404 and `a` 500
- WHEN they are posted to `/send`
- THEN two requests were sent each time, both carrying the same JWT, and the answer's `gone` is `["https://fcm.googleapis.com/fcm/send/b"]` each time

#### Scenario: too many
- GIVEN the `Live` object with its VAPID secrets set, and `MAX_MESSAGES` + 1 (11) messages, and `MAX_MESSAGES` (10)
- WHEN each set is posted to `/send`
- THEN the 11 get 422 and nothing was sent, and the 10 get 200 with all 10 sent

#### Scenario: a bad subscription
- GIVEN the `Live` object with its VAPID secrets set, and messages for `b` and for `a`, where `a` is in turn: a subscription whose `auth` decodes to 3 bytes, one whose `p256dh` isn't a point, one whose endpoint is `http://`, and a payload too long for one record
- WHEN each set is posted to `/send`
- THEN each time one request was sent, to `b`, and the answer is 200 with `gone` `[]`

#### Scenario: not gone
- GIVEN the `Live` object with its VAPID secrets set, and messages for four endpoints, answered 201, 429 and 500, and one whose post fails
- WHEN they are posted to `/send`
- THEN the answer is 200 with `gone` `[]`, and each endpoint was posted once

#### Scenario: can't sign
- GIVEN 2 messages, and 11
- WHEN the 2 are posted with no VAPID secrets, with a `VAPID_PRIVATE_KEY` that is 32 zero bytes, and with no randomness, and the 11 with no VAPID secrets
- THEN the answers are 503, 503, 503 and 422, and nothing was sent

#### Serves
- capability/follow-from-browser

### Requirement: web-push-request <!-- base: 81a115ff75242451 -->
`web_push(subscription, payload, vapid, now, random)` SHALL return the request to be `POST`ed to the subscription's `endpoint`, unchanged, whose body is `payload` encrypted as one `aes128gcm` record as RFC 8291 says for the subscription's `p256dh` and `auth`, using `random`'s first 32 bytes as the sender's private key and the next 16 as the salt, and whose headers are `Content-Encoding: aes128gcm`, `Content-Type: application/octet-stream`, `TTL: 86400`, `Urgency: normal` and `Authorization: vapid t=<jwt>, k=<the VAPID public key, base64url without padding>`, the JWT ES256-signed by the VAPID private key over `aud`, the endpoint's origin (`https://`, its host in lower case, and its port unless it is 443, without path or query), `exp`, later than `now` by at most 12 hours, and `sub` `https://clm-app.dev`. It SHALL read `p256dh`, `auth` and the VAPID keys as base64url with or without padding. It SHALL return an error whose text names the input: `p256dh` for one that isn't a 65-byte uncompressed P-256 point; `auth` for one that isn't 16 bytes; `endpoint` for one that isn't an `https` URL with a non-empty host before any port, or whose authority holds user info or whitespace; `VAPID` for a key that doesn't parse; `payload` for one over 3993 bytes, whose record wouldn't fit a 4096-byte body; and `random` for one whose first 32 bytes aren't a P-256 private key.

#### Scenario: the browser can read it
- GIVEN a subscription made from the browser's P-256 key pair and auth secret, the endpoint `https://push.example/s/1`, and a VAPID key pair
- WHEN `web_push` builds the request for the payload `{"title":"t"}`
- THEN its url is `https://push.example/s/1`, decrypting its body with the browser's private key and auth secret gives `{"title":"t"}`, its headers are `Content-Encoding: aes128gcm`, `Content-Type: application/octet-stream`, `TTL: 86400` and `Urgency: normal`, its `Authorization` header's `k` is the VAPID public key in base64url without padding, and its JWT verifies with the VAPID public key and has `aud` `https://push.example`, `sub` `https://clm-app.dev` and an `exp` later than `now` by at most 12 hours

#### Scenario: RFC 8291's example
- GIVEN the sender's private key, salt, subscription and plaintext of RFC 8291's Appendix A, the key and salt as `random`
- WHEN `web_push` builds the request
- THEN its body is the appendix's message, byte for byte

#### Scenario: a bad key
- GIVEN a subscription whose `auth` decodes to 15 bytes; one whose `p256dh` is 65 bytes but not a point, and one whose `p256dh` is a compressed point; a VAPID private key of 32 zero bytes, and a VAPID public key that isn't base64url
- WHEN `web_push` builds each request
- THEN each returns the error, naming `auth`, `p256dh`, `p256dh`, `VAPID` and `VAPID`

#### Scenario: the audience
- GIVEN endpoints `https://Push.Example:443/s/1`, `https://push.example:8443/s?x=1` and `https://push.example`
- WHEN `web_push` builds each request
- THEN the JWT's `aud` is `https://push.example`, `https://push.example:8443` and `https://push.example`, and each request's url is its endpoint unchanged

#### Scenario: a bad endpoint
- GIVEN endpoints `http://push.example/s/1`, `https:///s/1`, `push.example/s/1`, `https://user@push.example/s/1`, `https://push .example/s` and `https://:8443/s`
- WHEN `web_push` builds each request
- THEN each returns the error naming `endpoint`

#### Scenario: too long for one record
- GIVEN payloads of 3993 and 3994 bytes
- WHEN `web_push` builds each request
- THEN the first's body is 4096 bytes, and the second returns the error naming `payload`

#### Scenario: random that isn't a key
- GIVEN a `random` whose first 32 bytes are zero, and one whose first 32 bytes are `0xff`
- WHEN `web_push` builds each request
- THEN each returns the error naming `random`

#### Scenario: padded keys
- GIVEN a subscription and a VAPID key pair written in base64url with `=` padding, and the same written without
- WHEN `web_push` builds the request from each, with the same `now` and `random`
- THEN the two requests' url, body and `Authorization` `k` are the same, and decrypting either with the browser's private key gives the payload

### Requirement: notices <!-- base: 30857aa931296a5c -->
Each notice SHALL be a JSON object of the strings `title`, `body` and `url`, `url` the path of the app's page for it under `/app/`, relative to the app's origin (`public-site`). `waiting_notice(repo, previous, new)` SHALL give none when no `question` or `human-task` item of `new` has a slug `previous` doesn't hold; otherwise one notice with title `<repo>: waiting on you`, body the first such item's title in `new`'s order, followed by ` and <n> more` when `n` more follow, and url `/app/#/<repo>/backlog`. `review_notice(repo, event)` SHALL give the notice with title `<repo>: open for review`, body `<branch>: <the pull request's title>` and url `/app/#/<repo>/prs/<branch>`, the branch percent-encoded as one path segment (each byte other than RFC 3986's unreserved characters as `%` and two upper-case hex digits), for a `pull_request` event whose action is `opened`, `reopened` or `ready_for_review`, whose `draft` is `false`, whose head is a `spec/*` branch, and whose head repo's `full_name` is `repo`; and none for any other event, one missing any of those keys included.

#### Scenario: two questions land
- GIVEN `previous` holding the `idea` item `i`, and `new` holding `i`, the `question` item "Which region?" and the `human-task` item "Set the key"; and `new` holding only the question beside `i`
- WHEN `waiting_notice("o/a", previous, new)` runs on each
- THEN the first gives `{"title": "o/a: waiting on you", "body": "Which region? and 1 more", "url": "/app/#/o/a/backlog"}`, and the second the same with body `Which region?`

#### Scenario: nothing new waits
- GIVEN `new` adding only an `idea` item
- WHEN `waiting_notice` runs
- THEN it gives none

#### Scenario: opened for review
- GIVEN a `pull_request` event with action `opened` for head `spec/x` of `o/a`, `draft` `false`, titled "Add x"; the same with action `reopened`, with `ready_for_review`, and with head `spec/a b/ü`; and the same with action `closed`, with head `impl/y`, with a fork's head, with `draft` `true`, and `{"action": "opened"}` alone
- WHEN `review_notice("o/a", event)` runs on each
- THEN the first gives `{"title": "o/a: open for review", "body": "spec/x: Add x", "url": "/app/#/o/a/prs/spec%2Fx"}`, the next two a notice, the fourth the url `/app/#/o/a/prs/spec%2Fa%20b%2F%C3%BC`, and the others none

### Requirement: builds-for-wasm <!-- base: 7c0fd37dbb72cb86 -->
`clm-web-live` SHALL build for `wasm32-unknown-unknown` with its `durable` feature, and for the host without it.

#### Scenario: both builds
- GIVEN the workspace
- WHEN `cargo build -p clm-web-live --target wasm32-unknown-unknown --features durable` and `cargo build -p clm-web-live` run
- THEN both succeed
