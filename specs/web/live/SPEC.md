---
title: Live
status: active
owners: ["@clunny"]
depends_on: []
paths: ["crates/clm-web-live/**"]
successor: ""
serves: [capability/follow-from-browser]
---

## Contract
Crate `clm-web-live`, the two ways `web` reaches a browser without being asked (`web-live-channel`, `push-subscriptions-in-ledger`), built for `wasm32-unknown-unknown` and linked into the Worker by `clm-web`:
- **`Live`**, a Durable Object class, one object per repo. It accepts the WebSockets the Worker forwards to it; on `POST /notify` it sends the body, unchanged, to every socket it holds; on `POST /send` it delivers Web Push messages. It stores nothing, and decides nothing about who may connect or be notified: the Worker checks the session and the repo's access before it forwards a socket, and the repo's `Replica` object (`web.worker.replica`) chooses the recipients before it posts a send.
- **`web_push(subscription, payload, vapid, now, random)`**, which builds the HTTP request that delivers `payload` to one browser's push subscription (`endpoint`, `p256dh`, `auth`): the body encrypted as RFC 8291 says, and an RFC 8292 VAPID `Authorization` header signed with the App's key. It sends nothing.
- **`waiting_notice(repo, previous, new)`** and **`review_notice(repo, event)`**, the notification payloads the repo's `Replica` object sends after it applies a webhook's change: what a push to the default branch, given the backlog items before and after, and a `pull_request` event announce, if anything.

## Design
**The object.** `Live` uses the WebSocket hibernation API (`state.accept_web_socket`, and `state.get_websockets()` to broadcast), so an object whose sockets are idle is evicted from memory and costs no duration, and its sockets survive the eviction. It implements `websocket_message`, `websocket_close` and `websocket_error`, whose defaults in `worker` 0.8.7 panic: a message is ignored, and a close or an error closes the socket (`ws.close`). It sets the hibernation auto-response `ping` → `pong` (`set_websocket_auto_response`), which the runtime answers without waking the object, so a page can tell a dead socket from a quiet one (`web`'s Design). The Free plan offers only SQLite-backed Durable Objects, so `web/wrangler.toml` declares the class under `new_sqlite_classes` (`web`'s Design); the object writes nothing to its storage. A `GET` with `Upgrade: websocket` answers 101 with the client end of a new pair, after accepting the server end; any other `GET` answers 426. `POST /notify` answers 204 once every open socket has been sent the body; a send that fails (the socket closed meanwhile) is skipped. The Worker addresses an object by `id_from_name("<owner>/<repo>")`, so every tab of a repo, from any data centre, reaches the same one. The rules (what to answer for each request, what to broadcast) sit in a core generic over a trait for the set of sockets, the secrets, the clock, the randomness and `fetch`, which native tests drive with fakes; the `durable` feature only binds that core to `worker`'s types. The trait gives the core no storage, so its rules can't write any: that the object stores nothing holds by construction, and no test checks it.

**The notice.** What the repo's `Replica` object posts after it recomputes, and the sockets receive, is `{"delivery": "<X-GitHub-Delivery>"}`: the latest webhook delivery whose change the new views hold. It names no branch, record or item, so a socket opened by someone who has since lost access learns only that the repo changed, and the page's refetch goes through the routes that check access.

**Sending.** `POST /send` takes `{"messages": [{"subscription": {"endpoint", "p256dh", "auth"}, "payload": <JSON>}, ...]}`, at most 10, builds each with `web_push`, sends them with `worker::Fetch`, and answers 200 with `{"gone": [<each endpoint answered 404 or 410>]}`, which the `Replica` object logs. More than 10 messages, or a body that doesn't parse, gets 422 and sends nothing. Without the `VAPID_PUBLIC_KEY` and `VAPID_PRIVATE_KEY` secrets it answers 503 and sends nothing. Sending runs in this object rather than in the `Replica` object's alarm, so its crypto (three P-256 operations per message) stays apart from the recompute (`web-on-free-plan`). The VAPID JWT depends only on the endpoint's origin, so one send signs one JWT per origin and reuses it. The random bytes come from `crypto.getRandomValues`, as `clm-web`'s `Secrets::random` gets them.

**Web Push.** `web_push` follows RFC 8291: the sender's P-256 key pair from the first 32 bytes of `random` and the 16-byte salt from the next 16, ECDH with the subscription's `p256dh`, HKDF-SHA-256 with its `auth` secret to the content key and nonce, one AES-128-GCM record with the `0x02` padding delimiter, and the header block (salt, record size 4096, the sender's public key) in front. The VAPID header is `vapid t=<jwt>, k=<public key>`, the JWT ES256-signed over `{"aud": <the endpoint's origin>, "exp": now + 12 h, "sub": "https://clm-app.dev"}`. The request also sets `Content-Encoding: aes128gcm`, `Content-Type: application/octet-stream`, `TTL: 86400` and `Urgency: normal`. Its signature is `web_push(&Subscription, &[u8], &Vapid, now_secs: u64, random: [u8; 48]) -> Result<PushRequest, PushError>`, where `PushRequest` is `{url, headers: Vec<(String, String)>, body: Vec<u8>}`, this crate's own type, and `PushError` says which input is bad: a `p256dh` that isn't a 65-byte uncompressed point, an `auth` that isn't 16 bytes, an endpoint that isn't an `https` URL, or a VAPID key that doesn't parse. The keys are the secrets `VAPID_PUBLIC_KEY` (the uncompressed point) and `VAPID_PRIVATE_KEY` (the 32-byte scalar), both base64url without padding, as `npx web-push generate-vapid-keys` prints them. The crypto is pure Rust, `p256` 0.13 (with its `ecdh` and `ecdsa` features), `hkdf` 0.12, `aes-gcm` 0.10 and `sha2` 0.10, which build for wasm without `getrandom`, since the randomness is a parameter; so it tests natively.

**Payloads.** Each is `{"title", "body", "url"}`.
- `waiting_notice(repo, previous, new)`, over the snapshot's backlog items (`slug`, `kind`, `title`): the `question` and `human-task` items of `new` whose slug `previous` doesn't hold. None gives no notice; one gives `{"title": "<o>/<r>: waiting on you", "body": <its title>, "url": "/app/#/<o>/<r>/backlog"}`; more give the first's title followed by ` and <n> more`. One push is one notice, so a subscriber gets one notification however many items it adds.
- `review_notice(repo, event)`, over a `pull_request` webhook's payload: for action `opened`, `reopened` or `ready_for_review`, not a draft, whose head is a `spec/*` branch of this repository (`head.repo.full_name` is the repo), `{"title": "<o>/<r>: open for review", "body": "<branch>: <the pull request's title>", "url": "/app/#/<o>/<r>/prs/<the branch, percent-encoded as one segment>"}`; otherwise none.

**Building.** The crate depends on `worker` only under its `durable` feature, which `clm-web` enables in its `wasm32` target dependencies, beside its own `worker` dependency, so native builds and tests compile neither. `clm-web` re-exports `Live` (`pub use clm_web_live::Live`) so worker-build exports the class from the Worker's module. The root `Cargo.toml` gains the crate as a workspace dependency, as the root's Design has a component that adds a crate do. `.github/scripts/ci-check.sh`'s wasm build of `clm-web` compiles this crate with `durable`, so it needs no line of its own. Its `pub` items are what `Live` and its tests use: the tests live in `crates/clm-web-live/tests/`, outside the library, so `clm unreferenced` (`config/unreferenced-detector`) counts their uses, and an item that only the crate itself uses is `pub(crate)`, never an `--allow` entry in `clm.toml` (`unreferenced-allow-then-narrow`).

## Requirements

### Requirement: hub
The `Live` object SHALL accept each WebSocket upgrade it is sent with the hibernation API, and on `POST /notify` SHALL send the request's body, unchanged, as one text message to every socket it holds and answer 204. It SHALL ignore messages a socket sends, SHALL close a socket the browser closes or that errors, without panicking, and SHALL answer 426 to a `GET` without `Upgrade: websocket`.

#### Scenario: two pages
- GIVEN the object's core holding fake sockets `s1` and `s2`, and `s1` then closed by its browser
- WHEN `POST /notify` arrives with `{"delivery": "d1"}`
- THEN `s2` receives exactly that text, `s1` receives nothing, and the answer is 204

#### Scenario: not a socket
- GIVEN the object's core holding a fake socket
- WHEN a `GET` without `Upgrade: websocket` arrives, and the socket sends `hello` and then errors
- THEN the `GET` gets 426, nothing was sent to the socket, and it is closed

### Requirement: push-send
On `POST /send` with at most 10 messages, the `Live` object SHALL send each message's `web_push` request and answer 200 with the endpoints answered 404 or 410 under `gone`; with more than 10, or a body that doesn't parse, it SHALL answer 422, and without the VAPID secrets 503, sending nothing. It SHALL sign one VAPID JWT per endpoint origin per send.

#### Scenario: one gone
- GIVEN the object's core with a fake fetch, VAPID keys, and messages for `https://fcm.googleapis.com/fcm/send/a` and `https://fcm.googleapis.com/fcm/send/b`, where `b` answers 410
- WHEN they are posted to `/send`
- THEN two requests were sent, both carrying the same JWT, and the answer's `gone` is `["https://fcm.googleapis.com/fcm/send/b"]`

#### Scenario: too many
- GIVEN 11 messages
- WHEN they are posted to `/send`
- THEN the answer is 422 and nothing was sent

#### Serves
- capability/follow-from-browser

### Requirement: web-push-request
`web_push(subscription, payload, vapid, now, random)` SHALL return a `POST` to the subscription's `endpoint` whose body is `payload` encrypted with `aes128gcm` as RFC 8291 says for the subscription's `p256dh` and `auth`, using `random`'s first 32 bytes as the sender's private key and the next 16 as the salt, with `Content-Encoding: aes128gcm`, `TTL: 86400` and an `Authorization` header `vapid t=<jwt>, k=<public key>` whose JWT is ES256-signed by the VAPID key over `aud`, the endpoint's origin, `exp`, at most 12 hours after `now`, and `sub` `https://clm-app.dev`. It SHALL return an error naming the input for a `p256dh` that isn't a 65-byte uncompressed point, an `auth` that isn't 16 bytes, an endpoint that isn't `https`, or a VAPID key that doesn't parse.

#### Scenario: the browser can read it
- GIVEN a subscription made from a P-256 key pair and auth secret the test holds, the endpoint `https://push.example/s/1`, and a VAPID key pair
- WHEN `web_push` builds the request for the payload `{"title":"t"}`
- THEN decrypting its body with the test's private key and auth secret gives `{"title":"t"}`, and the JWT verifies with the VAPID public key and has `aud` `https://push.example`

#### Scenario: RFC 8291's example
- GIVEN the sender's private key, salt, subscription and plaintext of RFC 8291's Appendix A, the key and salt as `random`
- WHEN `web_push` builds the request
- THEN its body is the appendix's message, byte for byte

#### Scenario: a bad key
- GIVEN a subscription whose `auth` decodes to 15 bytes
- WHEN `web_push` builds the request
- THEN it returns the error naming `auth`

### Requirement: notices
`waiting_notice(repo, previous, new)` SHALL give one payload naming the `question` and `human-task` items of `new` whose slug `previous` doesn't hold, the first by title and the rest counted, or none when there are none. Each payload's `url` SHALL be the app's page for it under `/app/` (`public-site`); this build also moves the fixtures of other components that assert the old URL (`crates/clm-web/tests/push_notify.rs`) and names them in its pull request (`path-ownership`). `review_notice(repo, event)` SHALL give a payload naming the branch and the pull request's title for a `pull_request` event with action `opened`, `reopened` or `ready_for_review`, not a draft, whose head is a `spec/*` branch of the repo itself, and none for any other.

#### Scenario: two questions land
- GIVEN `previous` holding the `idea` item `i`, and `new` holding `i`, the `question` item "Which region?" and the `human-task` item "Set the key"
- WHEN `waiting_notice("o/a", previous, new)` runs
- THEN it gives `{"title": "o/a: waiting on you", "body": "Which region? and 1 more", "url": "/app/#/o/a/backlog"}`

#### Scenario: nothing new waits
- GIVEN `new` adding only an `idea` item
- WHEN `waiting_notice` runs
- THEN it gives none

#### Scenario: opened for review
- GIVEN a `pull_request` event with action `opened` for head `spec/x` of `o/a`, titled "Add x", and the same event with action `closed`, with head `impl/y`, and with a fork's head
- WHEN `review_notice("o/a", event)` runs on each
- THEN the first gives `{"title": "o/a: open for review", "body": "spec/x: Add x", "url": "/app/#/o/a/prs/spec%2Fx"}` and the others none

#### Needs
- model/parse-backlog

### Requirement: builds-for-wasm
`clm-web-live` SHALL build for `wasm32-unknown-unknown` with its `durable` feature, and natively without it, where its tests run.

#### Scenario: both builds
- GIVEN the workspace
- WHEN `cargo build -p clm-web-live --target wasm32-unknown-unknown --features durable` and `cargo test -p clm-web-live` run
- THEN both succeed

#### Needs
- web.worker/cache-is-disposable
