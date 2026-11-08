//! What a land says of its check's time (land/check-time-report).

mod common;

use common::Fixture;

/// The lines a land at `target` seconds says, for a check that runs `script`.
fn land_saying(target: u64, script: &str, history: usize) -> Vec<String> {
    let f = Fixture::new("");
    f.land_setting(&format!("check_target_seconds = {target}"));
    f.write("check-prints", script);
    f.write(
        "why-out",
        "a/src/lib.rs\ncomponent a\ncomponent b\nb/src/lib.rs\n",
    );
    for i in 0..history {
        f.branch(&format!("impl/h{i}"), &[("h.txt", &format!("{i}\n"))]);
        clm_land::land(&f.work(), Some(&format!("impl/h{i}")), &f.env(&[])).unwrap();
    }
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
    out.warnings
}

#[test]
fn under_target() {
    // req:land/check-time-report@ddbc0e19
    let said = land_saying(60, "echo ::group::fmt\n", 0);
    assert!(
        said.iter()
            .any(|l| l.starts_with("check ") && l.ends_with("s, target 60s")),
        "{said:?}"
    );
    assert!(
        !said.iter().any(|l| l.contains("ci-and-deployment")),
        "{said:?}"
    );
}

#[test]
fn over_target() {
    // req:land/check-time-report@ddbc0e19
    let script =
        "echo ::group::fmt\necho ::group::slow\necho ::clm-selected::a/src/lib.rs\nsleep 2\n";
    let said = land_saying(1, script, 0);
    let at = |needle: &str| {
        said.iter()
            .position(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("no `{needle}` in {said:?}"))
    };
    assert!(said[at("target 1s")].contains("check "));
    let slow = at("slow 2s");
    assert!(at("target 1s") < slow, "{said:?}");
    let chain: Vec<&str> = ["a/src/lib.rs", "component a", "component b", "b/src/lib.rs"]
        .into_iter()
        .collect();
    for (i, link) in chain.iter().enumerate() {
        assert!(said[slow + 1 + i].ends_with(link), "{said:?}");
    }
    assert!(
        said[at("fmt 0s")].ends_with("selected by no file"),
        "{said:?}"
    );
    assert!(slow < at("fmt 0s"), "{said:?}");
    let history = said[at("last 1 timed lands")].clone();
    assert!(history.contains("1 over target"), "{history}");
    assert!(history.contains("most time in slow 2s"), "{history}");
    assert!(
        said.last().unwrap().contains("clm guide ci-and-deployment"),
        "{said:?}"
    );
}

#[test]
fn the_last_timed_lands_count() {
    // req:land/check-time-report@ddbc0e19
    let script = "echo ::group::slow\nsleep 2\n";
    let said = land_saying(1, script, 1);
    assert!(
        said.iter()
            .any(|l| l.contains("last 2 timed lands: 2 over target; most time in slow 4s")),
        "{said:?}"
    );
}
