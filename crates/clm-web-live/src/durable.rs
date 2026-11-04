//! The `Live` Durable Object: the core in `hub` bound to `worker`'s types. It decides
//! nothing.

use worker::js_sys::{Function, Reflect, Uint8Array};
use worker::wasm_bindgen::{self, JsCast, JsValue};
use worker::worker_sys::WebSocketRequestResponsePair;
use worker::{
    DurableObject, Env, Fetch, Headers, Method, Request, RequestInit, Response, Result, State,
    WebSocket, WebSocketIncomingMessage, WebSocketPair, durable_object,
};

use crate::hub::{self, Answer, Host};
use crate::push::{PushRequest, Vapid};

/// One object per repo, addressed by `id_from_name("<owner>/<repo>")`. It holds the repo's
/// open pages' sockets with the hibernation API, and stores nothing.
#[durable_object]
pub struct Live {
    state: State,
    env: Env,
}

impl DurableObject for Live {
    fn new(state: State, env: Env) -> Self {
        // Answered by the runtime without waking the object, so a page can tell a dead
        // socket from a quiet one.
        match WebSocketRequestResponsePair::new("ping", "pong") {
            Ok(pair) => state.set_websocket_auto_response(&pair),
            Err(_) => worker::console_warn!("no ping/pong auto-response: pages will reconnect"),
        }
        Live { state, env }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        let method = req.method();
        let websocket = req
            .headers()
            .get("upgrade")?
            .is_some_and(|u| u.eq_ignore_ascii_case("websocket"));
        let body = match method {
            // A body that isn't UTF-8 doesn't parse: 422, as `/send` answers one that isn't JSON.
            Method::Post => match req.text().await {
                Ok(body) => body,
                Err(_) => return Ok(Response::empty()?.with_status(422)),
            },
            _ => String::new(),
        };
        let request = hub::Request {
            method: method.to_string(),
            path: req.path(),
            websocket,
            body,
        };
        match hub::fetch(self, request).await {
            Answer::Upgrade(client) => Response::from_websocket(client),
            Answer::Status(status) => Ok(Response::empty()?.with_status(status)),
            Answer::Json(status, body) => {
                let headers = Headers::new();
                headers.set("content-type", "application/json")?;
                Ok(Response::ok(body)?
                    .with_status(status)
                    .with_headers(headers))
            }
        }
    }

    async fn websocket_message(
        &self,
        ws: WebSocket,
        _message: WebSocketIncomingMessage,
    ) -> Result<()> {
        // Pages only listen: the core ignores what they send, so the payload isn't read.
        hub::on_message(self, &ws, "");
        Ok(())
    }

    async fn websocket_close(
        &self,
        ws: WebSocket,
        _code: usize,
        _reason: String,
        _was_clean: bool,
    ) -> Result<()> {
        hub::on_close(self, &ws);
        Ok(())
    }

    async fn websocket_error(&self, ws: WebSocket, _error: worker::Error) -> Result<()> {
        hub::on_close(self, &ws);
        Ok(())
    }
}

impl Host for Live {
    type Socket = WebSocket;

    fn accept(&self) -> Option<WebSocket> {
        let pair = WebSocketPair::new().ok()?;
        self.state.accept_web_socket(&pair.server);
        Some(pair.client)
    }

    fn sockets(&self) -> Vec<WebSocket> {
        self.state.get_websockets()
    }

    fn send(&self, socket: &WebSocket, text: &str) -> bool {
        socket.send_with_str(text).is_ok()
    }

    fn close(&self, socket: &WebSocket) {
        // Throws when the socket is already closed, which is the outcome wanted.
        let _ = socket.close(Some(1000), None::<&str>);
    }

    fn vapid(&self) -> Option<Vapid> {
        let secret = |name| self.env.secret(name).ok().map(|s| s.to_string());
        Some(Vapid {
            public_key: secret("VAPID_PUBLIC_KEY")?,
            private_key: secret("VAPID_PRIVATE_KEY")?,
        })
    }

    fn now(&self) -> u64 {
        worker::Date::now().as_millis() / 1000
    }

    /// From `crypto.getRandomValues`, as `clm-web`'s `Secrets::random` gets it. None rather
    /// than a panic, which would break the object's sockets too: the send answers 503.
    fn random(&self) -> Option<[u8; 48]> {
        let array = Uint8Array::new_with_length(48);
        let crypto = Reflect::get(&worker::js_sys::global(), &"crypto".into()).ok()?;
        let f: Function = Reflect::get(&crypto, &"getRandomValues".into())
            .ok()?
            .dyn_into()
            .ok()?;
        f.call1(&crypto, &array).ok()?;
        let mut out = [0; 48];
        array.copy_to(&mut out);
        Some(out)
    }

    /// Logs an answer that is neither a success nor `gone`, such as a VAPID key the push
    /// service refuses, since the Worker sees only `gone`.
    async fn fetch(&self, request: PushRequest) -> Option<u16> {
        let status = self.post(&request).await;
        match status {
            Some(200..=299 | 404 | 410) => {}
            Some(other) => worker::console_warn!("push to {} answered {other}", request.url),
            None => worker::console_warn!("push to {} failed", request.url),
        }
        status
    }
}

impl Live {
    async fn post(&self, request: &PushRequest) -> Option<u16> {
        let headers = Headers::new();
        for (name, value) in &request.headers {
            headers.set(name, value).ok()?;
        }
        let body = Uint8Array::from(request.body.as_slice());
        let mut init = RequestInit::new();
        init.with_method(Method::Post)
            .with_headers(headers)
            .with_body(Some(JsValue::from(body)));
        let request = Request::new_with_init(&request.url, &init).ok()?;
        let response = Fetch::Request(request).send().await.ok()?;
        Some(response.status_code())
    }
}
