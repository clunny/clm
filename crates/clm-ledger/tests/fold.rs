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
        recheck: None,
        recheck_files: None,
    }
}

#[test]
fn two_writers() {
    // req:ledger/fold@44978b52
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
    // req:ledger/fold@44978b52
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
    // req:ledger/fold@44978b52
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

/// Commits `files` alone, a tree of their own, in `hand` and stores it as `refs` in `into`.
fn ledger_ref(hand: &std::path::Path, files: &[(&str, &str)], into: &std::path::Path, refs: &str) {
    sh(hand, &["rm", "-rq", "--cached", "--ignore-unmatch", "."]);
    for (path, text) in files {
        std::fs::write(hand.join(path), text).unwrap();
        sh(hand, &["add", path]);
    }
    sh(hand, &["commit", "-q", "--allow-empty", "-m", "ledger"]);
    let tip = sh(hand, &["rev-parse", "HEAD"]);
    sh(
        into,
        &[
            "fetch",
            "-q",
            hand.to_str().unwrap(),
            &format!("{tip}:{refs}"),
        ],
    );
}

#[test]
fn odd_lines() {
    // req:ledger/fold@44978b52
    let dir = tempfile::tempdir().unwrap();
    let reader = clone_with_origin(dir.path(), "reader", "R");
    let clone = "1a2b3c4d";
    std::fs::create_dir_all(reader.join(".git/clm")).unwrap();
    std::fs::write(reader.join(".git/clm/writer"), format!("{clone}\n")).unwrap();
    let hand = dir.path().join("hand");
    std::fs::create_dir_all(&hand).unwrap();
    repo(&hand, "H");
    let approved = |fp: &str| {
        format!(
            r#"{{"id":"{fp}","kind":"approved","writer":"x/1","at":"2026-01-01T00:00:00.000Z","fingerprint":"{fp}","branch":"b","change":null"#
        )
    };
    let x = [
        r#"{"id":"1","kind":"nope","writer":"x/1","at":"2026-01-01T00:00:00.000Z"}"#.to_string(),
        r#"{"id":"2","kind":"commented","writer":"x/1","at":"2026-01-01T00:00:00.000Z","branch":"b","fingerprint":"f","text":"t"}"#.to_string(),
        "[1,2]".to_string(),
        String::new(),
        approved("f") + r#","x":1}"#,
    ]
    .join("\n");
    let origin = dir.path().join("origin.git");
    ledger_ref(
        &hand,
        &[("records.jsonl", &x)],
        &origin,
        "refs/heads/clm/ledger/x/1",
    );
    ledger_ref(
        &hand,
        &[("other.txt", "x\n")],
        &origin,
        "refs/heads/clm/ledger/y/2",
    );
    sh(&reader, &["fetch", "-q", "origin"]);
    let old = format!("refs/heads/clm/ledger/old/{clone}");
    ledger_ref(
        &hand,
        &[("records.jsonl", &(approved("g") + "}"))],
        &reader,
        &old,
    );

    let fold = clm_ledger::fold(&reader).unwrap();
    let approvals: Vec<&str> = fold.approvals.keys().map(String::as_str).collect();
    assert_eq!(approvals, ["f", "g"], "{fold:?}");
    assert_eq!(fold.lines["f"]["x"], 1);
    let w = &fold.warnings;
    assert_eq!(w.len(), 4, "{w:?}");
    let missing = "origin/clm/ledger/y/2: skipped, no records.jsonl: ";
    assert!(
        w.iter()
            .any(|l| l.starts_with(missing) && l.len() > missing.len()),
        "git's message: {w:?}"
    );
    for n in 1..=3 {
        let start = format!("origin/clm/ledger/x/1 line {n}: skipped, not a record: ");
        assert!(w.iter().any(|l| l.starts_with(&start)), "{start}: {w:?}");
    }
}

#[test]
fn times_out_of_order_or_equal() {
    // req:ledger/fold@44978b52
    let line = |id: &str, at: &str| {
        format!(
            r#"{{"id":"{id}","kind":"approved","writer":"x/1","at":"{at}","fingerprint":"f{id}","branch":"b","change":null}}"#
        )
    };
    let a = [
        line("2", "2026-01-02T00:00:00.000Z"),
        line("3", "2026-01-01T00:00:00.000Z"),
    ]
    .join("\n");
    let b = line("1", "2026-01-02T00:00:00.000Z");
    for sources in [[("a", a.as_str()), ("b", &b)], [("b", &b), ("a", &a)]] {
        let fold = clm_ledger::fold_records(&sources);
        let ids: Vec<&str> = fold.records.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["3", "1", "2"]);
    }
}

#[test]
fn fold_records_counts_a_duplicate_id_once_and_skips_blank_lines() {
    // req:ledger/fold@44978b52
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
        recheck: None,
        recheck_files: None,
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
    // req:ledger/subscription-records@94d71184
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
    // req:ledger/fold@44978b52
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
    // req:ledger/effort-records@828c1a9a
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
    // req:ledger/effort-records@828c1a9a
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
    // req:ledger/effort-records@828c1a9a
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
    // req:ledger/effort-records@828c1a9a
    let approved = r#"{"id":"e","kind":"approved","writer":"x/1","at":"2026-01-01T00:00:00.000Z","fingerprint":"f","branch":"spec/x","change":null}"#;
    let effort = effort_line("2026-01-02T00:00:00.000Z", "y/2", 5);
    let fold = clm_ledger::fold_records(&[("a", approved), ("b", &effort)]);
    assert_eq!(fold.approvals["f"].len(), 1, "{fold:?}");
    assert_eq!(fold.lines["e"]["kind"], "approved");
    assert!(fold.efforts.is_empty(), "{fold:?}");
}

#[test]
fn huge_effort_shown_without_overflow() {
    // req:ledger/effort-records@828c1a9a
    let line = effort_line("2026-01-01T00:00:00.000Z", "x/1", u64::MAX);
    let fold = clm_ledger::fold_records(&[("a", &line)]);
    let shown = clm_ledger::show(&fold);
    assert_eq!(shown.len(), 1);
    assert!(
        shown[0].contains(&format!(" {} 120s ", u64::MAX)),
        "{shown:?}"
    );
}

#[test]
fn show_a_check() {
    // req:ledger/show@6e2fd27f
    let line = r#"{"id":"1","kind":"checked","writer":"x/1","at":"2026-01-01T00:00:00.000Z","code_key":"k","command":"check"}"#;
    let fold = clm_ledger::fold_records(&[("origin/clm/ledger/x/1", line)]);
    assert_eq!(
        clm_ledger::show(&fold),
        ["checked k x/1 2026-01-01T00:00:00.000Z"]
    );
}

#[test]
fn show_odd_endpoints() {
    // req:ledger/subscription-records@94d71184
    let subscribed = |id: &str, endpoint: &str| {
        format!(
            r#"{{"id":"{id}","kind":"subscribed","writer":"alice/web","at":"2026-01-01T00:00:00.000Z","endpoint":"{endpoint}","p256dh":"p","auth":"a"}}"#
        )
    };
    let text = [
        subscribed("s1", "https://push.example:8443/s?x=1"),
        subscribed("s2", "https://user:token@push.example/s/1"),
        subscribed("s3", "not a url"),
        subscribed("s4", "foo?x=https://a/b"),
    ]
    .join("\n");
    let fold = clm_ledger::fold_records(&[("origin/clm/ledger/alice/web", &text)]);
    let at = "alice/web 2026-01-01T00:00:00.000Z";
    assert_eq!(
        clm_ledger::show(&fold),
        [
            format!("subscribed s1 https://push.example:8443 {at}"),
            format!("subscribed s2 https://push.example {at}"),
            format!("subscribed s3 - {at}"),
            format!("subscribed s4 - {at}"),
        ]
    );
}

/// Every order of `lines`.
fn every_order(lines: &[String]) -> Vec<Vec<String>> {
    if lines.len() <= 1 {
        return vec![lines.to_vec()];
    }
    let mut orders = Vec::new();
    for n in 0..lines.len() {
        let mut rest = lines.to_vec();
        let first = rest.remove(n);
        for mut order in every_order(&rest) {
            order.insert(0, first.clone());
            orders.push(order);
        }
    }
    orders
}

/// The fold of `lines` in every order, each line read as a ledger of its own and again all in
/// one ledger, each fold holding one record and checked by `check`.
fn fold_every_way(lines: &[String], check: impl Fn(&clm_ledger::Fold)) {
    for order in every_order(lines) {
        let names: Vec<String> = (0..order.len()).map(|n| format!("l{n}")).collect();
        let apart: Vec<(&str, &str)> = names
            .iter()
            .zip(&order)
            .map(|(n, l)| (n.as_str(), l.as_str()))
            .collect();
        let together = order.join("\n");
        for fold in [
            clm_ledger::fold_records(&apart),
            clm_ledger::fold_records(&[("one", &together)]),
        ] {
            assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
            assert_eq!(fold.records.len(), 1, "{order:?}: {:?}", fold.records);
            assert_eq!(fold.lines.len(), 1, "{order:?}");
            check(&fold);
        }
    }
}

/// An `approved` line of `id` at `at` by `x/1`.
fn approved_line(id: &str, at: &str) -> String {
    format!(
        r#"{{"id":"{id}","kind":"approved","writer":"x/1","at":"{at}","fingerprint":"f","branch":"spec/x","change":null}}"#
    )
}

#[test]
fn effort_ties_and_collisions() {
    // req:ledger/effort-records@828c1a9a
    let (t1, t2, t3) = (
        "2026-01-01T00:00:00.000Z",
        "2026-01-02T00:00:00.000Z",
        "2026-01-03T00:00:00.000Z",
    );
    // Two `effort` lines at one `at` by two writers: the greater writer's.
    fold_every_way(
        &[effort_line(t1, "a/1", 5), effort_line(t1, "b/2", 50)],
        |fold| {
            assert_eq!(fold.efforts["e"].writer, "b/2");
            assert_eq!(fold.lines["e"]["writer"], "b/2");
        },
    );
    // `d`: an approval at t2 among `effort` lines at t1 and t3: the least of all, the
    // `effort` line at t1.
    let effort_d = |at: &str| effort_line(at, "x/1", 5).replace(r#""id":"e""#, r#""id":"d""#);
    fold_every_way(
        &[approved_line("d", t2), effort_d(t1), effort_d(t3)],
        |fold| {
            assert_eq!(fold.records[0].at, t1, "{:?}", fold.records);
            assert!(matches!(fold.records[0].body, Body::Effort { .. }));
            assert_eq!(fold.lines["d"]["at"], t1);
        },
    );
    // `s`: two approvals equal but for `sig`: the one with it, whose sorted text is the lesser.
    let unsigned = approved_line("s", t1);
    let signed = unsigned.replace(r#","change":null}"#, r#","change":null,"sig":"S"}"#);
    fold_every_way(&[unsigned, signed], |fold| {
        assert_eq!(fold.lines["s"]["sig"], "S");
    });
    // One `at` and one writer, written with `models` before `active_seconds`: the sorted text
    // decides, by `active_seconds` before `models`, whatever order the line's keys are in.
    let effort_written = |active: u64, output: u64| {
        format!(
            r#"{{"id":"e","kind":"effort","writer":"x/1","at":"{t1}","branch":"impl/a","session":"s1","models":{{"m":{{"input":1,"output":{output},"cache_read":0,"cache_write":0}}}},"active_seconds":{active}}}"#
        )
    };
    fold_every_way(&[effort_written(20, 1), effort_written(10, 9)], |fold| {
        let Body::Effort { active_seconds, .. } = &fold.efforts["e"].body else {
            panic!("{:?}", fold.efforts);
        };
        assert_eq!(*active_seconds, 20);
        assert_eq!(fold.lines["e"]["active_seconds"], 20);
    });
    // An approval of `e` and a later `effort` line of `e`: the approval, whichever is read
    // first.
    fold_every_way(
        &[effort_line(t2, "y/2", 5), approved_line("e", t1)],
        |fold| {
            assert_eq!(fold.approvals["f"].len(), 1, "{fold:?}");
            assert!(fold.efforts.is_empty(), "{fold:?}");
            assert_eq!(fold.lines["e"]["kind"], "approved");
        },
    );
}
