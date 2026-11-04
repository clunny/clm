//! A fake `Host`: sockets, secrets and fetch the tests can watch.
// Each test binary uses a part of the fake.
#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use clm_web_live::{Host, PushRequest, Vapid};

/// Runs a future whose every await is ready at once, as the fake's are.
pub fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut cx) {
        Poll::Ready(out) => out,
        Poll::Pending => panic!("the fake host never waits"),
    }
}

#[derive(Default)]
pub struct Fake {
    /// Each socket's received texts, and whether it is open.
    pub sockets: RefCell<Vec<(Vec<String>, bool)>>,
    pub vapid: Option<Vapid>,
    /// Status per endpoint; 201 otherwise.
    pub statuses: BTreeMap<String, u16>,
    pub sent: RefCell<Vec<PushRequest>>,
    /// `crypto.getRandomValues` is missing.
    pub no_random: bool,
}

impl Fake {
    /// A socket the object accepted, as the runtime would hold it.
    pub fn connect(&self) -> usize {
        let mut sockets = self.sockets.borrow_mut();
        sockets.push((Vec::new(), true));
        sockets.len() - 1
    }

    pub fn received(&self, socket: usize) -> Vec<String> {
        self.sockets.borrow()[socket].0.clone()
    }

    pub fn is_open(&self, socket: usize) -> bool {
        self.sockets.borrow()[socket].1
    }

    /// The browser closes its end: sends to it fail from now on.
    pub fn browser_closes(&self, socket: usize) {
        self.sockets.borrow_mut()[socket].1 = false;
    }
}

impl Host for Fake {
    type Socket = usize;

    fn accept(&self) -> Option<usize> {
        Some(self.connect())
    }

    fn sockets(&self) -> Vec<usize> {
        (0..self.sockets.borrow().len()).collect()
    }

    fn send(&self, socket: &usize, text: &str) -> bool {
        let mut sockets = self.sockets.borrow_mut();
        let (received, open) = &mut sockets[*socket];
        if *open {
            received.push(text.into());
        }
        *open
    }

    fn close(&self, socket: &usize) {
        self.sockets.borrow_mut()[*socket].1 = false;
    }

    fn vapid(&self) -> Option<Vapid> {
        self.vapid.clone()
    }

    fn now(&self) -> u64 {
        1_800_000_000
    }

    fn random(&self) -> Option<[u8; 48]> {
        (!self.no_random).then_some([5; 48])
    }

    async fn fetch(&self, request: PushRequest) -> Option<u16> {
        let status = self.statuses.get(&request.url).copied().unwrap_or(201);
        self.sent.borrow_mut().push(request);
        Some(status)
    }
}
