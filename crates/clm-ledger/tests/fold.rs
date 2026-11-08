mod common;

use clm_ledger::{Body, Decision, LandKind, Rating, Usage};
use common::{clone_with_origin, env, repo, sh};

fn approved(fingerprint: &str) -> Body {
    Body::Approved {
        fingerprint: fingerprint.into(),
        branch: "spec/x".into(),
        change: None,
    }
}

fn landed(commit: &str) -> Body {
    Body::Landed {
        commit: commit.into(),
        branch: "impl/x".into(),
        kind: LandKind::Code,
        review: false,
        fingerprint: "g".into(),
        code_key: None,
        changes: vec![],
        check_seconds: None,
        steps: vec![],
    }
}

#[test]
fn two_writers() {
    // req:ledger/fold@134ea7aa
    let dir = tempfile::tempdir().unwrap();
    let a = clone_with_origin(dir.path(), "a", "A");
    let b = clone_with_origin(dir.path(), "b", "B");
    let reader = clone_with_origin(dir.path(), "reader", "R");
    let home = dir.path();
    clm_ledger::append(&a, approved("f"), &env(home, &[])).unwrap();
    clm_ledger::append(&b, landed("c"), &env(home, &[])).unwrap();
    // `fold` doesn't fetch: before one, the reader sees nothing.
    assert!(clm_ledger::fold(&reader).unwrap().records.is_empty());
    sh(&reader, &["fetch", "-q", "origin"]);
    let fold = clm_ledger::fold(&reader).unwrap();
    assert!(fold.approvals.contains_key("f"), "{fold:?}");
    assert!(fold.lands.contains_key("c"), "{fold:?}");
    assert_eq!(fold.records.len(), 2);
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
}

#[test]
fn local_ahead() {
    // req:ledger/fold@134ea7aa
    let dir = tempfile::tempdir().unwrap();
    let a = clone_with_origin(dir.path(), "a", "A");
    let home = dir.path();
    clm_ledger::append(&a, approved("f"), &env(home, &[])).unwrap();
    let nowhere = dir.path().join("nowhere.git");
    sh(
        &a,
        &["remote", "set-url", "origin", nowhere.to_str().unwrap()],
    );
    assert!(
        !clm_ledger::append(&a, approved("r"), &env(home, &[]))
            .unwrap()
            .pushed
    );
    let fold = clm_ledger::fold(&a).unwrap();
    assert!(fold.approvals.contains_key("r"), "{fold:?}");
    // Read from both the local branch and `origin/...`, the first record counts once.
    assert_eq!(fold.approvals["f"].len(), 1);
    assert_eq!(fold.records.len(), 2);
}

#[test]
fn bad_line() {
    // req:ledger/fold@134ea7aa
    let dir = tempfile::tempdir().unwrap();
    let reader = clone_with_origin(dir.path(), "reader", "R");
    let good = |id: &str, fp: &str| {
        format!(
            r#"{{"id":"{id}","kind":"approved","writer":"x/1","at":"2026-01-01T00:00:00.000Z","fingerprint":"{fp}","branch":"b","change":null}}"#
        )
    };
    // A ledger branch made by hand, its second line not JSON.
    let hand = dir.path().join("hand");
    std::fs::create_dir_all(&hand).unwrap();
    repo(&hand, "H");
    let text = format!("{}\nnot json\n{}\n", good("1", "f1"), good("2", "f2"));
    std::fs::write(hand.join("records.jsonl"), text).unwrap();
    sh(&hand, &["add", "records.jsonl"]);
    sh(&hand, &["commit", "-q", "-m", "ledger"]);
    let origin = dir.path().join("origin.git");
    sh(
        &hand,
        &[
            "push",
            "-q",
            origin.to_str().unwrap(),
            "HEAD:refs/heads/clm/ledger/x/1",
        ],
    );
    sh(&reader, &["fetch", "-q", "origin"]);
    let fold = clm_ledger::fold(&reader).unwrap();
    assert!(fold.approvals.contains_key("f1") && fold.approvals.contains_key("f2"));
    assert_eq!(fold.warnings.len(), 1, "{:?}", fold.warnings);
    let warning = &fold.warnings[0];
    assert!(
        warning.contains("origin/clm/ledger/x/1") && warning.contains("line 2"),
        "{warning}"
    );
}

#[test]
fn fold_records_counts_a_duplicate_id_once_and_skips_blank_lines() {
    // req:ledger/fold@134ea7aa
    let line = r#"{"id":"1","kind":"approved","writer":"x/1","at":"2026-01-01T00:00:00.000Z","fingerprint":"r","branch":"spec/x","change":null}"#;
    let text = format!("{line}\n\n{line}\r\n");
    let fold = clm_ledger::fold_records(&[("one", &text), ("two", line)]);
    assert_eq!(fold.records.len(), 1);
    assert_eq!(fold.approvals["r"].len(), 1);
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
}

#[test]
fn landed_record_written_flat() {
    // req:ledger/landed-record-fields@ab130936
    let dir = tempfile::tempdir().unwrap();
    let a = clone_with_origin(dir.path(), "a", "A");
    let body = Body::Landed {
        commit: "c".into(),
        branch: "impl/x".into(),
        kind: LandKind::Code,
        review: false,
        fingerprint: "f".into(),
        code_key: Some("k".into()),
        changes: vec![],
        check_seconds: None,
        steps: vec![],
    };
    let appended = clm_ledger::append(&a, body, &env(dir.path(), &[])).unwrap();
    let branch = format!("clm/ledger/{}", appended.record.writer);
    let line = sh(&a, &["show", &format!("{branch}:records.jsonl")]);
    for field in [
        r#""kind":"landed""#,
        r#""land_kind":"code""#,
        r#""review":false"#,
        r#""code_key":"k""#,
    ] {
        assert!(line.contains(field), "{field}: {line}");
    }
    assert!(!line.contains("tier"), "{line}");
    assert_eq!(line.matches(r#""kind""#).count(), 1, "{line}");
}

#[test]
fn landed_record_written_before_the_code_key() {
    // req:ledger/landed-record-fields@ab130936
    let line = r#"{"id":"1","writer":"a/1","at":"2026-01-01T00:00:00.000Z","kind":"landed","commit":"c","branch":"impl/x","land_kind":"code","tier":null,"fingerprint":"f","changes":[]}"#;
    let fold = clm_ledger::fold_records(&[("origin/clm/ledger/a/1", line)]);
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    let land = &fold.lands["c"][0];
    assert!(
        matches!(&land.body, Body::Landed { code_key: None, .. }),
        "{land:?}"
    );
    // An unknown key is left out when written, not written `null`.
    let text = serde_json::to_string(land).unwrap();
    assert!(!text.contains("code_key"), "{text}");
}

#[test]
fn subscriptions_ended_in_another_order() {
    // req:ledger/subscription-records@e4cb2df2
    // The end comes first, in the file and in time, and still ends `s1`.
    let text = [
        r#"{"id":"u1","kind":"unsubscribed","writer":"alice/web","at":"2026-01-01T00:00:00.000Z","subscription":"s1"}"#,
        r#"{"id":"s1","kind":"subscribed","writer":"alice/web","at":"2026-01-02T00:00:00.000Z","endpoint":"https://push.example/s/1","p256dh":"p1","auth":"a1"}"#,
        r#"{"id":"s2","kind":"subscribed","writer":"alice/web","at":"2026-01-03T00:00:00.000Z","endpoint":"https://push.example/s/2","p256dh":"p2","auth":"a2"}"#,
    ]
    .join("\n");
    let fold = clm_ledger::fold_records(&[("origin/clm/ledger/alice/web", &text)]);
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    assert_eq!(fold.records.len(), 3);
    let live: Vec<&str> = fold.subscriptions.keys().map(String::as_str).collect();
    assert_eq!(live, ["s2"]);
    let s2 = &fold.subscriptions["s2"];
    assert_eq!(s2.writer, "alice/web");
    assert!(
        matches!(&s2.body, Body::Subscribed { endpoint, p256dh, auth }
            if endpoint == "https://push.example/s/2" && p256dh == "p2" && auth == "a2"),
        "{s2:?}"
    );
    // Read the other way round, the same.
    let reversed: String = text.lines().rev().collect::<Vec<_>>().join("\n");
    let fold = clm_ledger::fold_records(&[("origin/clm/ledger/alice/web", &reversed)]);
    let live: Vec<&str> = fold.subscriptions.keys().map(String::as_str).collect();
    assert_eq!(live, ["s2"]);
}

#[test]
fn proposals_withdrawn_in_another_order() {
    // req:ledger/proposal-records@f2ac7e51
    // The withdrawal comes first, in the file and in time, and still withdraws `p1`.
    let text = [
        r#"{"id":"w1","kind":"withdrawn","writer":"alice/web","at":"2026-01-01T00:00:00.000Z","proposal":"p1"}"#,
        r#"{"id":"p1","kind":"proposed","writer":"alice/web","at":"2026-01-02T00:00:00.000Z","slug":"dark-mode","title":"Dark mode","body":"Make it dark.","nodes":["web"]}"#,
        r#"{"id":"p2","kind":"proposed","writer":"alice/web","at":"2026-01-03T00:00:00.000Z","slug":"export-csv","title":"Export CSV","body":"A CSV export.","nodes":["view","web"]}"#,
    ]
    .join("\n");
    for text in [
        text.clone(),
        text.lines().rev().collect::<Vec<_>>().join("\n"),
    ] {
        let fold = clm_ledger::fold_records(&[("origin/clm/ledger/alice/web", &text)]);
        assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
        assert_eq!(fold.records.len(), 3);
        let open: Vec<&str> = fold.proposals.keys().map(String::as_str).collect();
        assert_eq!(open, ["p2"]);
        let p2 = &fold.proposals["p2"];
        assert_eq!(
            (p2.writer.as_str(), p2.at.as_str()),
            ("alice/web", "2026-01-03T00:00:00.000Z")
        );
        assert_eq!(
            p2.body,
            Body::Proposed {
                slug: "export-csv".into(),
                title: "Export CSV".into(),
                body: "A CSV export.".into(),
                nodes: vec!["view".into(), "web".into()],
            }
        );
    }
    // Written back as read: the proposal's `body` is its own field, not the record's.
    let fold = clm_ledger::fold_records(&[("l", &text)]);
    let line = serde_json::to_value(&fold.proposals["p2"]).unwrap();
    assert_eq!(line["kind"], "proposed");
    assert_eq!(line["body"], "A CSV export.");
}

#[test]
fn feedback_folded_by_branch() {
    // req:ledger/feedback-records@9066f278
    let alice = [
        r#"{"id":"r1","kind":"changes_requested","writer":"alice/web","at":"2026-01-02T00:00:00.000Z","branch":"spec/x","fingerprint":"f","text":"Split it"}"#,
        r#"{"id":"c1","kind":"commented","writer":"alice/web","at":"2026-01-01T00:00:00.000Z","branch":"spec/x","fingerprint":"f","target":"n/r","text":"Why?"}"#,
        // Later in the file, earlier in time: first in the index.
        r#"{"id":"c0","kind":"commented","writer":"alice/web","at":"2025-12-31T00:00:00.000Z","branch":"spec/x","fingerprint":"e","target":null,"text":"Old"}"#,
        r#"{"id":"c2","kind":"commented","writer":"alice/web","at":"2026-01-03T00:00:00.000Z","branch":"spec/z","fingerprint":"h","target":null,"text":"Fine"}"#,
    ]
    .join("\n");
    let bob = r#"{"id":"r2","kind":"changes_requested","writer":"bob/web","at":"2026-01-01T00:00:00.000Z","branch":"spec/y","fingerprint":"g","text":"No"}"#;
    let fold = clm_ledger::fold_records(&[
        ("origin/clm/ledger/alice/web", &alice),
        ("origin/clm/ledger/bob/web", bob),
    ]);
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    let comments = &fold.comments["spec/x"];
    let ids: Vec<&str> = comments.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["c0", "c1"]);
    assert_eq!(
        comments[1].body,
        Body::Commented {
            branch: "spec/x".into(),
            fingerprint: "f".into(),
            target: Some("n/r".into()),
            text: "Why?".into(),
        }
    );
    assert!(matches!(
        &fold.comments["spec/z"][0].body,
        Body::Commented { target: None, .. }
    ));
    let requests = |branch: &str| -> Vec<(String, String)> {
        fold.change_requests[branch]
            .iter()
            .map(|r| match &r.body {
                Body::ChangesRequested { fingerprint, .. } => {
                    (r.writer.clone(), fingerprint.clone())
                }
                other => panic!("{other:?}"),
            })
            .collect()
    };
    let pair = |w: &str, f: &str| (w.to_string(), f.to_string());
    assert_eq!(requests("spec/x"), [pair("alice/web", "f")]);
    assert_eq!(requests("spec/y"), [pair("bob/web", "g")]);
    assert_eq!(fold.change_requests.len(), 2);
    // A null target is written, not left out.
    let line = serde_json::to_string(&fold.comments["spec/z"][0]).unwrap();
    assert!(line.contains(r#""target":null"#), "{line}");
    let line = serde_json::to_string(&fold.change_requests["spec/x"][0]).unwrap();
    assert!(line.contains(r#""kind":"changes_requested""#), "{line}");
    // A comment's `target` is always present: a line without it is no record.
    let bare = r#"{"id":"c9","kind":"commented","writer":"a/1","at":"2026-01-01T00:00:00.000Z","branch":"spec/x","fingerprint":"f","text":"t"}"#;
    let fold = clm_ledger::fold_records(&[("l", bare)]);
    assert!(fold.comments.is_empty(), "{fold:?}");
    assert_eq!(fold.warnings.len(), 1, "{:?}", fold.warnings);
}

fn legacy_land(commit: &str, tier: &str) -> String {
    format!(
        r#"{{"id":"{commit}","writer":"a/1","at":"2026-01-01T00:00:00.000Z","kind":"landed","commit":"{commit}","branch":"spec/x","land_kind":"spec","tier":{tier},"fingerprint":"f","changes":[]}}"#
    )
}

#[test]
fn legacy_tier_reads_as_review() {
    // req:ledger/landed-record-fields@ab130936
    let text = [
        legacy_land("h", r#""HARD""#),
        legacy_land("s", r#""SOFT""#),
        legacy_land("a", r#""AUTO""#),
        legacy_land("n", "null"),
    ]
    .join("\n");
    let fold = clm_ledger::fold_records(&[("l", &text)]);
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    let review = |c: &str| match &fold.lands[c][0].body {
        Body::Landed { review, .. } => *review,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        ["h", "s", "a", "n"].map(review),
        [true, false, false, false]
    );
}

#[test]
fn landed_writes_review() {
    // req:ledger/landed-record-fields@ab130936
    let mut body = landed("c");
    if let Body::Landed { review, kind, .. } = &mut body {
        (*review, *kind) = (true, LandKind::Spec);
    }
    let record = clm_ledger::Record {
        id: "1".into(),
        writer: "a/1".into(),
        at: "2026-01-01T00:00:00.000Z".into(),
        body,
    };
    let line = serde_json::to_string(&record).unwrap();
    assert!(line.contains(r#""review":true"#), "{line}");
    assert!(!line.contains("tier"), "{line}");
}

#[test]
fn reviewed_lines_skipped_quietly() {
    // req:ledger/fold@134ea7aa
    let reviewed = r#"{"id":"2","writer":"a/1","at":"2026-01-01T00:00:01.000Z","kind":"reviewed","fingerprint":"r"}"#;
    let approved = |id: &str, f: &str| {
        format!(
            r#"{{"id":"{id}","writer":"a/1","at":"2026-01-01T00:00:00.000Z","kind":"approved","fingerprint":"{f}","branch":"spec/x","change":null}}"#
        )
    };
    let text = [approved("1", "f"), reviewed.to_string(), approved("3", "g")].join("\n");
    let fold = clm_ledger::fold_records(&[("l", &text)]);
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    assert_eq!(fold.approvals.len(), 2);
    assert_eq!(fold.records.len(), 2);
}

/// An `effort` line for session `s1` on `impl/a`, with `id` `e` and two models.
fn effort_line(at: &str, writer: &str, output: u64) -> String {
    format!(
        r#"{{"id":"e","kind":"effort","writer":"{writer}","at":"{at}","branch":"impl/a","session":"s1","models":{{"claude-opus-5-5":{{"input":10,"output":{output},"cache_read":100,"cache_write":20}},"claude-sonnet-5-5":{{"input":1,"output":2,"cache_read":0,"cache_write":3}}}},"active_seconds":120}}"#
    )
}

#[test]
fn effort_round_trips() {
    // req:ledger/effort-records@2bee0a30
    let line = effort_line("2026-01-01T00:00:00.000Z", "x/1", 5);
    let fold = clm_ledger::fold_records(&[("one", &line)]);
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    let record = &fold.efforts["e"];
    let Body::Effort {
        branch,
        session,
        models,
        active_seconds,
    } = &record.body
    else {
        panic!("{record:?}");
    };
    assert_eq!((branch.as_str(), session.as_str()), ("impl/a", "s1"));
    assert_eq!(*active_seconds, 120);
    let opus = Usage {
        input: 10,
        output: 5,
        cache_read: 100,
        cache_write: 20,
    };
    assert_eq!(models["claude-opus-5-5"], opus);
    assert_eq!(models["claude-sonnet-5-5"].cache_write, 3);
    // Written back, it reads the same.
    let again = serde_json::to_string(record).unwrap();
    let fold = clm_ledger::fold_records(&[("two", &again)]);
    assert_eq!(&fold.efforts["e"], record);
}

#[test]
fn effort_latest_at_wins() {
    // req:ledger/effort-records@2bee0a30
    let early = effort_line("2026-01-01T00:00:00.000Z", "a/1", 5);
    let late = effort_line("2026-01-02T00:00:00.000Z", "b/2", 50);
    for sources in [
        [("a", early.as_str()), ("b", late.as_str())],
        [("b", late.as_str()), ("a", early.as_str())],
    ] {
        let fold = clm_ledger::fold_records(&sources);
        assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
        assert_eq!(fold.efforts["e"].at, "2026-01-02T00:00:00.000Z");
        assert_eq!(fold.records.len(), 1, "{:?}", fold.records);
        assert_eq!(fold.records[0].writer, "b/2");
        assert_eq!(fold.lines["e"]["writer"], "b/2");
    }
}

fn decided_line(id: &str, capability: &str, at: &str, value: &str) -> String {
    format!(
        r#"{{"id":"{id}","kind":"decided","writer":"x/1","at":"{at}","capability":"{capability}","value":"{value}","decision":"keep","settles":["f1"]}}"#
    )
}

#[test]
fn decided_by_capability() {
    // req:ledger/decision-records@9ca8cb93
    let text = [
        decided_line("2", "capability/x", "2026-01-02T00:00:00.000Z", "low"),
        decided_line("1", "capability/x", "2026-01-01T00:00:00.000Z", "high"),
        decided_line("3", "capability/y", "2026-01-01T00:00:00.000Z", "medium"),
    ]
    .join("\n");
    let fold = clm_ledger::fold_records(&[("one", &text)]);
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    let x: Vec<&str> = fold.decisions["capability/x"]
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    assert_eq!(x, ["1", "2"]);
    assert_eq!(fold.decisions["capability/y"].len(), 1);
    let Body::Decided {
        value,
        decision,
        settles,
        ..
    } = &fold.decisions["capability/x"][0].body
    else {
        panic!()
    };
    assert_eq!((*value, *decision), (Rating::High, Decision::Keep));
    assert_eq!(settles, &["f1"]);
}

#[test]
fn decided_architecture_entry_held_and_shown() {
    // req:ledger/decision-records@9ca8cb93
    let line = decided_line("1", "architecture/x", "2026-01-01T00:00:00.000Z", "high");
    let fold = clm_ledger::fold_records(&[("one", &line)]);
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    assert_eq!(fold.decisions["architecture/x"].len(), 1);
    assert_eq!(
        clm_ledger::show(&fold),
        ["decided architecture/x high keep x/1 2026-01-01T00:00:00.000Z"]
    );
}

#[test]
fn decided_shown() {
    // req:ledger/decision-records@9ca8cb93
    let line = decided_line("1", "capability/x", "2026-01-01T00:00:00.000Z", "low")
        .replace("\"keep\"", "\"cut\"");
    let fold = clm_ledger::fold_records(&[("one", &line)]);
    assert_eq!(
        clm_ledger::show(&fold),
        ["decided capability/x low cut x/1 2026-01-01T00:00:00.000Z"]
    );
}

#[test]
fn bad_rating_skipped() {
    // req:ledger/decision-records@9ca8cb93
    let good = decided_line("1", "capability/x", "2026-01-01T00:00:00.000Z", "low");
    let bad = decided_line("2", "capability/x", "2026-01-02T00:00:00.000Z", "huge");
    let fold = clm_ledger::fold_records(&[("origin/clm/ledger/x/1", &format!("{good}\n{bad}"))]);
    assert_eq!(fold.decisions["capability/x"].len(), 1);
    assert_eq!(fold.warnings.len(), 1, "{:?}", fold.warnings);
    assert!(
        fold.warnings[0].contains("origin/clm/ledger/x/1 line 2"),
        "{:?}",
        fold.warnings
    );
}

#[test]
fn effort_tie_folds_alike_in_either_order() {
    // req:ledger/effort-records@2bee0a30
    let at = "2026-01-01T00:00:00.000Z";
    let (a, b) = (effort_line(at, "a/1", 5), effort_line(at, "b/2", 50));
    for sources in [
        [("a", a.as_str()), ("b", b.as_str())],
        [("b", b.as_str()), ("a", a.as_str())],
    ] {
        let fold = clm_ledger::fold_records(&sources);
        assert_eq!(fold.records.len(), 1);
        assert_eq!(fold.efforts["e"].writer, "b/2");
    }
}

#[test]
fn effort_never_replaces_another_kind() {
    // req:ledger/effort-records@2bee0a30
    let approved = r#"{"id":"e","kind":"approved","writer":"x/1","at":"2026-01-01T00:00:00.000Z","fingerprint":"f","branch":"spec/x","change":null}"#;
    let effort = effort_line("2026-01-02T00:00:00.000Z", "y/2", 5);
    let fold = clm_ledger::fold_records(&[("a", approved), ("b", &effort)]);
    assert_eq!(fold.approvals["f"].len(), 1, "{fold:?}");
    assert_eq!(fold.lines["e"]["kind"], "approved");
    assert!(fold.efforts.is_empty(), "{fold:?}");
}

#[test]
fn huge_effort_shown_without_overflow() {
    // req:ledger/effort-records@2bee0a30
    let line = effort_line("2026-01-01T00:00:00.000Z", "x/1", u64::MAX);
    let fold = clm_ledger::fold_records(&[("a", &line)]);
    let shown = clm_ledger::show(&fold);
    assert_eq!(shown.len(), 1);
    assert!(
        shown[0].contains(&format!(" {} 120s ", u64::MAX)),
        "{shown:?}"
    );
}
