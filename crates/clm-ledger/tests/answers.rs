use clm_ledger::{Body, fold_records};

fn line(writer: &str, id: &str, item: &str, answer: &str, at: &str) -> String {
    format!(
        r#"{{"id":"{id}","kind":"answered","writer":"{writer}","at":"2026-01-01T{at}.000Z","item":"{item}","answer":"{answer}"}}"#
    )
}

#[test]
fn a_later_answer_replaces_an_earlier_one() {
    // req:ledger/answer-records@96b5bcca
    let alice = [
        line("alice/web", "a2", "q", "no", "10:00:00"),
        line("alice/web", "a1", "q", "yes", "09:00:00"),
    ]
    .join("\n");
    let bob = line("bob/web", "b1", "q", "yes", "08:00:00");
    // Either read order folds alike.
    for sources in [
        [("alice/web", alice.as_str()), ("bob/web", bob.as_str())],
        [("bob/web", bob.as_str()), ("alice/web", alice.as_str())],
    ] {
        let fold = fold_records(&sources);
        assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
        let open: Vec<(&str, &str, &str)> = fold.answers["q"]
            .iter()
            .map(|r| {
                let Body::Answered { answer, .. } = &r.body else {
                    panic!("not answered");
                };
                (r.id.as_str(), r.writer.as_str(), answer.as_str())
            })
            .collect();
        assert_eq!(open, [("b1", "bob/web", "yes"), ("a2", "alice/web", "no")]);
    }
}

#[test]
fn a_tie_goes_to_the_greater_id() {
    // req:ledger/answer-records@96b5bcca
    let text = [
        line("alice/web", "a9", "q", "no", "10:00:00"),
        line("alice/web", "a3", "q", "yes", "10:00:00"),
    ]
    .join("\n");
    let fold = fold_records(&[("alice/web", &text)]);
    let ids: Vec<&str> = fold.answers["q"].iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["a9"]);
}
