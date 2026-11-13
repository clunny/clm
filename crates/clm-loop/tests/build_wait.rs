//! `clm build wait` (`build-wait`).

mod common;

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use clm_git::Failure;
use clm_loop::{StopMode, stop, wait};
use common::Fixture;

/// Runs `wait` on a thread, so a test can act while it sleeps.
fn waiting(fx: &Fixture, minutes: f64) -> mpsc::Receiver<Result<String, Failure>> {
    let options = fx.options();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(wait(&options, minutes));
    });
    rx
}

#[test]
fn a_place_frees() {
    // req:orchestrator.loop/build-wait@838b1759
    let fx = Fixture::new();
    let mut options = fx.options();
    options.max_builds = 1;
    fx.set("next", "a\n");
    clm_orchestrator::take_place(
        &fx.work,
        "b",
        clm_orchestrator::PlaceKind::Build,
        &["impl/b".to_string()],
        1,
        4,
    )
    .unwrap();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(wait(&options, 1.0));
    });
    thread::sleep(Duration::from_millis(700));
    assert!(rx.try_recv().is_err(), "no place is free yet");
    let freed = Instant::now();
    clm_orchestrator::free_place(&fx.work, "b").unwrap();

    let why = rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
    assert!(freed.elapsed() < Duration::from_secs(10));
    assert!(why.contains("place b freed"), "{why}");
    assert!(why.contains("build:a"), "{why}");
    assert!(fx.file().lease.is_some(), "the lease is renewed");
}

#[test]
fn a_stop_request() {
    // req:orchestrator.loop/build-wait@838b1759
    let fx = Fixture::new();
    let rx = waiting(&fx, 5.0);
    thread::sleep(Duration::from_millis(700));
    assert!(rx.try_recv().is_err(), "nothing to do, so it sleeps");
    stop(&fx.options(), StopMode::Drain).unwrap();
    let why = rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
    assert!(why.contains("drain"), "{why}");
}

#[test]
fn work_already_ready_wakes_at_once() {
    // req:orchestrator.loop/build-wait@838b1759
    let fx = Fixture::new();
    fx.set("next", "a\n");
    let why = wait(&fx.options(), 5.0).unwrap();
    assert!(why.contains("build:a"), "{why}");
}

#[test]
fn an_interval_that_isnt_allowed() {
    // req:orchestrator.loop/build-wait@838b1759
    let fx = Fixture::new();
    let started = Instant::now();
    for bad in [10.0, 0.0, -1.0, f64::NAN, 7.1] {
        assert!(
            matches!(wait(&fx.options(), bad), Err(Failure::Refused(_))),
            "{bad}"
        );
    }
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "it doesn't wait"
    );
    assert!(fx.calls().is_empty(), "nothing is read before the check");
}

#[test]
fn a_failed_fetch_is_reported_and_waiting_goes_on() {
    // req:orchestrator.loop/build-wait@838b1759
    let fx = Fixture::new();
    // A 0.05-minute interval fetches at the first poll; the remote is gone.
    clm_git::git(&fx.work, &["remote", "set-url", "origin", "/nonexistent"]).unwrap();
    let rx = waiting(&fx, 0.05);
    thread::sleep(Duration::from_secs(7));
    assert!(rx.try_recv().is_err(), "it still waits");
    stop(&fx.options(), StopMode::Now).unwrap();
    rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
}
