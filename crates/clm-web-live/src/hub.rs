//! The `Live` object's rules, generic over the runtime it runs in: what to answer for each
//! request, and what to broadcast. The `durable` module binds them to `worker`'s types.

use crate::push::{PushRequest, Vapid};

/// What the object reaches: the sockets the runtime holds for it, the VAPID secrets, the
/// clock, randomness and `fetch`. It has no storage: the object stores nothing.
#[allow(async_fn_in_trait)]
pub trait Host {
    type Socket;
    /// A new socket pair whose server end is accepted with the hibernation API; its client
    /// end, for the 101 answer. None when the runtime can't make one.
    fn accept(&self) -> Option<Self::Socket>;
    /// Every socket the runtime holds for the object.
    fn sockets(&self) -> Vec<Self::Socket>;
    /// Sends `text` as one text message; false when the send fails.
    fn send(&self, socket: &Self::Socket, text: &str) -> bool;
    fn close(&self, socket: &Self::Socket);
    /// `VAPID_PUBLIC_KEY` and `VAPID_PRIVATE_KEY`, when both are set.
    fn vapid(&self) -> Option<Vapid>;
    /// Unix seconds.
    fn now(&self) -> u64;
    /// From `crypto.getRandomValues`; None when it is unavailable.
    fn random(&self) -> Option<[u8; 48]>;
    /// Sends `request`; its status, or None when it fails to send.
    async fn fetch(&self, request: PushRequest) -> Option<u16>;
}

/// A request the Worker forwards or posts to the object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    pub path: String,
    /// Whether it carries `Upgrade: websocket`.
    pub websocket: bool,
    pub body: String,
}

/// The object's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer<S> {
    /// 101 with the client end of a new socket.
    Upgrade(S),
    Status(u16),
    /// A status with a JSON body.
    Json(u16, String),
}

/// The answer to `request`: a `GET` with the upgrade accepts a socket, one without gets
/// 426; `POST /notify` broadcasts its body and `POST /send` sends Web Push messages.
pub async fn fetch<H: Host>(host: &H, request: Request) -> Answer<H::Socket> {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", _) if request.websocket => match host.accept() {
            Some(client) => Answer::Upgrade(client),
            None => Answer::Status(500),
        },
        ("GET", _) => Answer::Status(426),
        ("POST", "/notify") => {
            for socket in host.sockets() {
                // A socket closed meanwhile is skipped.
                host.send(&socket, &request.body);
            }
            Answer::Status(204)
        }
        ("POST", "/send") => crate::send::send(host, &request.body).await,
        _ => Answer::Status(404),
    }
}

/// A message a page sends: ignored, since pages only listen.
pub fn on_message<H: Host>(_host: &H, _socket: &H::Socket, _text: &str) {}

/// A socket its browser closed, or that errored: closed.
pub fn on_close<H: Host>(host: &H, socket: &H::Socket) {
    host.close(socket);
}
