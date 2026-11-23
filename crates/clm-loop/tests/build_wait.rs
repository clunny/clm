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
    // req:orchestrator.loop/build-wait@9341226a
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
    // `wait` reads the places it starts with before its first `clm wrap-up`, so the place is
    // freed only once that call is seen: a fixed sleep let a loaded host free it first.
    let started = Instant::now();
    while fx.ran("wrap-up") == 0 {
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "wait never ran clm wrap-up"
        );
        thread::sleep(Duration::from_millis(20));
    }
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
fn the_disk_frees() {
    // req:orchestrator.loop/build-wait@9341226a
    let fx = Fixture::new();
    let mut options = fx.options();
    options.max_builds = 2;
    fx.set("next", "a\n");
    fx.disk(Some(0), "warn");
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(wait(&options, 0.1));
    });
    thread::sleep(Duration::from_millis(1500));
    assert!(rx.try_recv().is_err(), "the disk holds build:a back");
    let freed = Instant::now();
    fx.disk(Some(1), "warn");

    let why = rx.recv_timeout(Duration::from_secs(20)).unwrap().unwrap();
    assert!(freed.elapsed() < Duration::from_secs(20));
    assert!(why.contains("disk"), "{why}");
    assert!(why.contains("build:a"), "{why}");
}

#[test]
fn other_work_waking_it_while_the_disk_holds_doesnt_name_the_disk() {
    // req:orchestrator.loop/build-wait@9341226a
    let fx = Fixture::new();
    fx.set("next", "a\n");
    fx.disk(Some(0), "warn");
    let rx = waiting(&fx, 5.0);
    thread::sleep(Duration::from_millis(1500));
    assert!(rx.try_recv().is_err(), "the disk holds build:a back");
    // A fold falls due, which the disk doesn't hold back.
    clm_loop::update(&fx.work, |f| {
        f.review.push("x".to_string());
        Ok(())
    })
    .unwrap();
    let why = rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
    assert!(why.contains("fold:x"), "{why}");
    assert!(!why.contains("disk"), "{why}");
}

#[test]
fn a_stop_request() {
    // req:orchestrator.loop/build-wait@9341226a
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
    // req:orchestrator.loop/build-wait@9341226a
    let fx = Fixture::new();
    fx.set("next", "a\n");
    let why = wait(&fx.options(), 5.0).unwrap();
    assert!(why.contains("build:a"), "{why}");
}

#[test]
fn an_interval_that_isnt_allowed() {
    // req:orchestrator.loop/build-wait@9341226a
    let fx = Fixture::new();
    let started = Instant::now();
    for bad in [10.0, 0.0, -1.0, f64::NAN, 7.1] {
        // On a thread, so a wait that wrongly starts fails the test instead of hanging it.
        let rx = waiting(&fx, bad);
        let got = rx.recv_timeout(Duration::from_secs(2));
        assert!(matches!(got, Ok(Err(Failure::Refused(_)))), "{bad}");
    }
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "it doesn't wait"
    );
    assert!(fx.calls().is_empty(), "nothing is read before the check");
}

#[test]
fn a_failed_fetch_is_reported_and_waiting_goes_on() {
    // req:orchestrator.loop/build-wait@9341226a
    let fx = Fixture::new();
    // A 0.05-minute interval fetches at the first poll; the remote is gone.
    clm_git::git(&fx.work, &["remote", "set-url", "origin", "/nonexistent"]).unwrap();
    let rx = waiting(&fx, 0.05);
    thread::sleep(Duration::from_secs(7));
    assert!(rx.try_recv().is_err(), "it still waits");
    stop(&fx.options(), StopMode::Now).unwrap();
    rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
}

#[test]
fn a_stop_asked_for_earlier_wakes_once_nothing_runs() {
    // req:orchestrator.loop/build-wait@9341226a
    let fx = Fixture::new();
    stop(&fx.options(), StopMode::Drain).unwrap();
    let why = wait(&fx.options(), 5.0).unwrap();
    assert!(why.contains("drain"), "{why}");
}

#[test]
fn a_held_orphan_whose_branch_stayed_still() {
    // req:orchestrator.loop/build-wait@9341226a
    let fx = Fixture::new();
    let mut run = common::run(
        "build:a",
        clm_loop::Kind::Build,
        "sonnet",
        clm_loop::RunState::Running,
        Some("t-1"),
    );
    run.orphan = true;
    run.hold = Some(clm_loop::Hold {
        tip: "0".repeat(40),
        since: clm_git::now() - 15 * 60,
    });
    fx.add_run(run);
    let rx = waiting(&fx, 5.0);
    let why = rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
    assert!(why.contains("build:a"), "{why}");
}

fn running_build_a(fx: &Fixture) {
    fx.add_run(common::run(
        "build:a",
        clm_loop::Kind::Build,
        "sonnet",
        clm_loop::RunState::Running,
        Some("t-1"),
    ));
}

#[test]
fn a_build_lands() {
    // req:orchestrator.loop/build-wait@9341226a
    let fx = Fixture::new();
    running_build_a(&fx);
    let rx = waiting(&fx, 5.0);
    thread::sleep(Duration::from_millis(700));
    assert!(rx.try_recv().is_err(), "the claim is live, so it sleeps");

    // As `clm land impl/a` run in the clone leaves `origin/main` and `origin/impl/a`.
    let landed = std::process::Command::new(fx.fake.join("clm"))
        .args(["land", "impl/a"])
        .current_dir(&fx.work)
        .status()
        .unwrap();
    assert!(landed.success());

    let why = rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
    assert!(why.contains("build:a"), "{why}");
}

#[test]
fn a_claim_already_gone_wakes_before_any_housekeeping() {
    // req:orchestrator.loop/build-wait@9341226a
    let fx = Fixture::new();
    running_build_a(&fx);
    fx.drop_claim("a");

    let why = wait(&fx.options(), 5.0).unwrap();

    assert!(why.contains("build:a"), "{why}");
    assert_eq!(fx.ran("wrap-up"), 0, "the wrap-up would remove the branch");
}
