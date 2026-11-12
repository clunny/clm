//! `clm verify` follows the settings committed at `origin/main`
//! (verify/settings-from-main).

mod common;

use clm_verify::Request;
use common::Fixture;

const LOCAL: &str = "[verify]\ncheck = \"true\"\nmode = \"local\"\ntriggers = [\"work-done\"]\n";

#[test]
fn a_checkout_behind_main() {
    // req:verify/settings-from-main@bbaa1ef1
    let f = Fixture::new(None);
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    f.land(&[("clm.toml", LOCAL)], "turn verify on");
    // The checkout falls behind: its own clm.toml sets no [verify].
    f.git(&f.work(), &["reset", "-q", "--hard", &green]);
    assert_eq!(f.git(&f.work(), &["rev-parse", "HEAD"]), green);
    let out = f.run(&Request::Due {
        work_done: true,
        dry_run: true,
    });
    assert_eq!((out.line.as_str(), out.code), ("verify due work-done", 0));
}

#[test]
fn turned_off_on_main() {
    // req:verify/settings-from-main@bbaa1ef1
    let f = Fixture::new(None);
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    f.land(&[("a.txt", "2\n")], "two");
    let wrote = f.path("wrote");
    let local = format!(
        "[verify]\ncheck = \"touch {}\"\nmode = \"local\"\ntriggers = [\"work-done\"]\n",
        wrote.display()
    );
    f.write("work/clm.toml", &local);
    let out = f.run(&Request::Due {
        work_done: true,
        dry_run: false,
    });
    assert_eq!(
        (out.line.as_str(), out.code),
        ("verify off: no [verify] in origin/main:clm.toml", 0)
    );
    assert!(!wrote.exists());
    assert!(f.records().len() == 1);
}

#[test]
fn an_invalid_setting_on_main() {
    // req:verify/settings-from-main@bbaa1ef1
    let f = Fixture::new(None);
    let before_land = f.back(0);
    f.land(
        &[("clm.toml", "[verify]\nmode = \"local\"\n")],
        "verify with no check",
    );
    f.git(&f.work(), &["reset", "-q", "--hard", &before_land]);
    let before = f.records().len();
    let err = clm_verify::verify(
        &f.work(),
        &Request::Due {
            work_done: false,
            dry_run: false,
        },
        &f.env(),
    )
    .unwrap_err();
    assert!(matches!(err, clm_verify::Error::Refused(_)), "{err}");
    let message = err.to_string();
    assert!(message.contains("origin/main:clm.toml"), "{message}");
    assert!(message.contains("verify"), "{message}");
    assert_eq!(f.records().len(), before);
}

#[test]
fn a_fetch_that_fails_is_a_git_failure() {
    // req:verify/settings-from-main@bbaa1ef1
    let f = Fixture::new(Some("mode = \"local\"\ntriggers = [\"work-done\"]"));
    f.git(
        &f.work(),
        &["remote", "set-url", "origin", "/nonexistent/x"],
    );
    let err = clm_verify::verify(
        &f.work(),
        &Request::Due {
            work_done: false,
            dry_run: true,
        },
        &f.env(),
    )
    .unwrap_err();
    assert!(matches!(err, clm_verify::Error::Git(_)), "{err}");
}
