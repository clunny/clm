//! Appends to a web ledger that isn't there yet, that ends without a newline, or that
//! GitHub won't read or move (`web-ledger-append`).

use clm_ledger::{Body, Record};
use clm_web_platform::access::{self, Access};
use clm_web_platform::fake::*;
use clm_web_platform::ledger::{self, Appended};
use serde_json::json;

const BRANCH: &str = "clm/ledger/alice/web";

/// The five repos of the scenario, in turn.
#[derive(Clone, Copy, Debug)]
enum Case {
    First,
    Unended,
    TipFails,
    NullRepository,
    Refused,
}

const OLD: &str = r#"{"id":"r0","kind":"unsubscribed","writer":"alice/web","at":"2027-01-15T08:00:00.000Z","subscription":"s"}"#;

fn setup(case: Case) -> (Fake, Access) {
    let fake = Fake::default();
    let mut repo = FakeRepo::new("111", clm_repo());
    if !matches!(case, Case::First) {
        let text = match case {
            Case::Unended => OLD.to_string(),
            _ => format!("{OLD}\n"),
        };
        repo.branches.insert(BRANCH.into(), "old".into());
        repo.commits
            .insert("old".into(), files(&[("records.jsonl", &text)]));
    }
    match case {
        Case::TipFails => {
            repo.ref_fails.insert(BRANCH.into());
        }
        Case::Refused => {
            repo.refused_refs.insert(BRANCH.into());
        }
        Case::NullRepository => {
            fake.gh.graphql_null.borrow_mut().insert("Ledgers".into());
        }
        _ => {}
    }
    fake.gh.add_repo("o/a", repo);
    fake.gh.grant("u", "o/a");
    let access = block_on(access::check(&fake, "u", "o", "a")).unwrap();
    fake.gh.calls.borrow_mut().clear();
    (fake, access)
}

fn record(fake: &Fake) -> Record {
    ledger::record(
        fake,
        "alice",
        Body::Unsubscribed {
            subscription: "s1".into(),
        },
    )
}

fn tip(fake: &Fake) -> Option<String> {
    fake.gh.repos.borrow()["o/a"].branches.get(BRANCH).cloned()
}

fn text_at(fake: &Fake, sha: &str) -> String {
    let repos = fake.gh.repos.borrow();
    String::from_utf8(repos["o/a"].commits[sha]["records.jsonl"].clone()).unwrap()
}

#[test]
fn a_first_an_unended_and_a_refused_append() {
    // req:web.worker.platform/web-ledger-append@debff849
    for case in [
        Case::First,
        Case::Unended,
        Case::TipFails,
        Case::NullRepository,
        Case::Refused,
    ] {
        let (fake, acc) = setup(case);
        let rec = record(&fake);
        assert_eq!(rec.at, "2027-01-15T08:00:00.000Z", "{case:?}");
        assert_eq!(rec.writer, "alice/web");
        assert!(
            rec.id.len() == 32
                && rec
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "{case:?}: {}",
            rec.id
        );
        let line = serde_json::to_string(&rec).unwrap();
        let given = block_on(ledger::append(&fake, &acc, &rec));
        match case {
            Case::First => {
                assert_eq!(given, Some(()));
                let tip = tip(&fake).unwrap();
                assert!(fake.gh.repos.borrow()["o/a"].parents[&tip].is_empty());
                assert_eq!(text_at(&fake, &tip), format!("{line}\n"));
                let created = fake.gh.writes().last().unwrap().clone();
                assert_eq!(created.method, "POST");
                assert!(created.url.ends_with("/git/refs"), "the ref is created");
            }
            Case::Unended => {
                assert_eq!(given, Some(()));
                let tip = tip(&fake).unwrap();
                assert_eq!(text_at(&fake, &tip), format!("{OLD}\n{line}\n"));
            }
            Case::TipFails | Case::NullRepository => {
                assert_eq!(given, None, "{case:?}");
                assert!(fake.gh.writes().is_empty(), "{case:?}: no write made");
            }
            Case::Refused => {
                assert_eq!(given, None);
                assert_eq!(tip(&fake).as_deref(), Some("old"), "the ref where it was");
            }
        }
        // `appended` makes the same append on a fresh repo, and gives what it wrote.
        let (fake, acc) = setup(case);
        let rec = record(&fake);
        let given = block_on(ledger::appended(&fake, &acc, &rec));
        match case {
            Case::First | Case::Unended => {
                let Appended { tip: at, text } = given.expect("appended");
                assert_eq!(Some(&at), tip(&fake).as_ref(), "the commit the ref names");
                assert_eq!(text, text_at(&fake, &at), "the file at that commit");
            }
            _ => assert!(given.is_none(), "{case:?}"),
        }
    }
}

#[test]
fn the_append_reads_its_old_line() {
    // req:web.worker.platform/web-ledger-append@debff849
    let (fake, acc) = setup(Case::Unended);
    let rec = record(&fake);
    block_on(ledger::append(&fake, &acc, &rec)).unwrap();
    let writes = fake.gh.writes();
    assert_eq!(writes.len(), 4, "a blob, a tree, a commit and the ref");
    let commit: serde_json::Value =
        serde_json::from_str(writes[2].body.as_deref().unwrap()).unwrap();
    assert_eq!(commit["parents"], json!(["old"]));
}
