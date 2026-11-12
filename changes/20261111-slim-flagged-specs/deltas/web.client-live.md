---
component: web.client-live
base: { design: "585cc4fdf5e51373" }
---

## MODIFIED Design
The modules import nothing of `web`'s outside `web/src/live/`, since an import of `web`'s code from here would need `depends_on: [web]`, a cycle with `web`'s dependency on this component (`check/undeclared-imports`). What they need of the app comes in through their constructors and parameters, the repo's API base among them.

**The socket** (`live-updates`). A browser reports no status for a failed handshake, so every close or failure while a URL is wanted is retried the same way. `web.live`'s object answers the ping without waking, so a socket a phone's network silently dropped is noticed by the missing `pong` at no cost to the object.

**Notifications** (`push-subscriptions-in-ledger`, `notify-switch`, `push-renewal`). The browser holds one push subscription for the app, shared by every repo's switch, so the app remembers which repos it posted that subscription to: the browser may renew the subscription under a new endpoint, which the Worker's ledger doesn't know, and unsubscribing the browser for one repo would silence the others. Without storage the app can't tell that no other switch is on, so it keeps the subscription.

**Push handlers** (`push-shown`). The tag makes a repeat replace the notification. `navigate` rejects for a window the worker doesn't control (one hard-reloaded, or opened before the worker took over), hence the new window.
