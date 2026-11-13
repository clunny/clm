//! The `queue` model and its charts, as library calls.

use clm_view::{
    Checkout, Format, Head, Loop, Next, QueueView, Running, Stop, View, WaitingOnWork,
    WaitingOnYou, render,
};

fn head() -> Head {
    Head::Checkout(Checkout {
        commit: "abc1234".into(),
        behind: None,
    })
}

fn model() -> QueueView {
    QueueView {
        view: "queue",
        loop_state: Loop::Held { renewed_minutes: 3 },
        running: vec![Running {
            name: "build:a".into(),
            kind: "build".into(),
            model: "sonnet".into(),
            round: 1,
            minutes: 12,
            taken_over: false,
        }],
        next: vec![Next {
            name: "build:b".into(),
            kind: "build".into(),
            priority: "P2".into(),
            reason: "waits for a place".into(),
        }],
        waiting_on_you: vec![WaitingOnYou {
            name: "fold:x".into(),
            kind: "fold".into(),
            what: "fold batch x".into(),
            since_minutes: 90,
            action: "say \"approve x\"".into(),
        }],
        waiting_on_work: vec![],
    }
}

fn text(m: QueueView, format: Format) -> String {
    render(&View::Queue(m), format, 100, Some(&head()))
}

#[test]
fn scenario_chart_sections() {
    // req:view/queue-view@ba511898
    let out = text(model(), Format::Text);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "commit abc1234");
    assert_eq!(lines[1], "loop held, renewed 3m ago");
    assert_eq!(lines[2], "running");
    assert!(lines[3].starts_with("  build:a"), "{}", lines[3]);
    assert!(lines[3].contains("sonnet") && lines[3].contains("12m"));
    assert_eq!(lines[4], "next up");
    assert!(lines[5].starts_with("  build:b"), "{}", lines[5]);
    assert!(lines[5].contains("P2") && lines[5].contains("waits for a place"));
    assert_eq!(lines[6], "waiting on you");
    assert!(lines[7].starts_with("  fold:x"), "{}", lines[7]);
    assert!(lines[7].contains("say \"approve x\""));
    assert_eq!(lines[8], "waiting on work");
    assert_eq!(lines[9], "  none");
    assert_eq!(lines.len(), 10);
}

#[test]
fn scenario_waiting_on_you_highlighted() {
    // req:view/queue-view@ba511898
    const AMBER: &str = "\x1b[33m";
    let ansi = text(model(), Format::Ansi);
    let fold = ansi.lines().find(|l| l.contains("fold:x")).unwrap();
    let start = fold.find("fold:x").unwrap();
    assert!(fold[..start].ends_with(AMBER), "{fold:?}");
    // From the name to the end, every character lies between amber and a reset.
    let mut amber = false;
    let mut chars = fold[start - AMBER.len()..].chars();
    let mut shown = String::new();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            let mut esc = String::from("\x1b");
            for e in chars.by_ref() {
                esc.push(e);
                if e == 'm' {
                    break;
                }
            }
            assert!(esc == AMBER || esc == "\x1b[0m", "{esc:?}");
            amber = esc == AMBER;
        } else {
            assert!(amber, "{c:?} outside amber in {fold:?}");
            shown.push(c);
        }
    }
    assert!(
        shown.starts_with("fold:x") && shown.contains("say"),
        "{shown}"
    );
    for name in ["build:a", "build:b"] {
        let l = ansi.lines().find(|l| l.contains(name)).unwrap();
        assert!(!l[..l.find(name).unwrap()].contains(AMBER), "{l:?}");
    }
    let heading = ansi.lines().find(|l| l.contains("waiting on you")).unwrap();
    assert!(!heading.contains(AMBER), "{heading:?}");
    assert!(fold.ends_with("\x1b[0m"), "{fold:?}");
    let mut empty = model();
    empty.waiting_on_you.clear();
    let ansi = text(empty, Format::Ansi);
    assert!(!ansi.contains(AMBER), "{ansi:?}");
}

#[test]
fn scenario_ansi_is_the_text_with_colour() {
    // req:view/queue-view@ba511898
    let ansi = text(model(), Format::Ansi);
    let mut plain = String::new();
    let mut chars = ansi.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for e in chars.by_ref() {
                if e == 'm' {
                    break;
                }
            }
        } else {
            plain.push(c);
        }
    }
    assert!(ansi.contains('\x1b'));
    assert_eq!(plain, text(model(), Format::Text));
}

#[test]
fn scenario_loop_line_and_empty_sections() {
    // req:view/queue-view@ba511898
    let empty = |loop_state| QueueView {
        view: "queue",
        loop_state,
        running: vec![],
        next: vec![],
        waiting_on_you: vec![],
        waiting_on_work: vec![],
    };
    let line = |loop_state| {
        let out = render(&View::Queue(empty(loop_state)), Format::Text, 100, None);
        out.lines().next().unwrap().to_string()
    };
    assert_eq!(line(Loop::NotHeld), "loop not held");
    assert_eq!(
        line(Loop::Stop { mode: Stop::Drain }),
        "stop requested: drain"
    );
    assert_eq!(line(Loop::Stop { mode: Stop::Now }), "stop requested: now");
    let out = render(&View::Queue(empty(Loop::NotHeld)), Format::Text, 100, None);
    assert_eq!(
        out.lines().skip(1).collect::<Vec<_>>(),
        [
            "running",
            "  none",
            "next up",
            "  none",
            "waiting on you",
            "  none",
            "waiting on work",
            "  none"
        ]
    );
}

#[test]
fn scenario_running_entry_held_by_a_taken_over_session() {
    // req:view/queue-view@ba511898
    let mut m = model();
    m.running[0].taken_over = true;
    m.waiting_on_work.push(WaitingOnWork {
        name: "build:c".into(),
        kind: "build".into(),
        waits_on: "b/y".into(),
    });
    let out = text(m, Format::Text);
    let a = out.lines().find(|l| l.starts_with("  build:a")).unwrap();
    assert!(a.contains("taken over"), "{a}");
    let c = out.lines().find(|l| l.starts_with("  build:c")).unwrap();
    assert!(c.contains("b/y"), "{c}");
}

#[test]
fn scenario_json_is_the_model() {
    // req:view/queue-view@ba511898
    let out = text(model(), Format::Json);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["view"], "queue");
    assert_eq!(v["checkout"]["commit"], "abc1234");
    assert_eq!(v["running"][0]["name"], "build:a");
    assert_eq!(v["running"][0]["model"], "sonnet");
    assert_eq!(v["next"][0]["priority"], "P2");
    assert_eq!(v["waiting_on_you"][0]["action"], "say \"approve x\"");
    assert_eq!(v["waiting_on_work"], serde_json::json!([]));
}

#[test]
fn scenario_entries_keep_the_models_order_and_show_their_fields() {
    // req:view/queue-view@ba511898
    let mut m = model();
    m.running.insert(
        0,
        Running {
            name: "build:z".into(),
            kind: "build".into(),
            model: "opus".into(),
            round: 2,
            minutes: 5,
            taken_over: false,
        },
    );
    let out = text(m.clone(), Format::Text);
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines[3].starts_with("  build:z") && lines[3].contains("round 2"));
    assert!(lines[4].starts_with("  build:a") && lines[4].contains("round 1"));
    let fold = lines.iter().find(|l| l.starts_with("  fold:x")).unwrap();
    assert!(
        fold.contains("fold batch x") && fold.contains("since 1h"),
        "{fold}"
    );
    let v: serde_json::Value = serde_json::from_str(&text(m, Format::Json)).unwrap();
    assert_eq!(v["loop"]["state"], "held");
    assert_eq!(v["running"][0]["round"], 2);
    assert_eq!(v["waiting_on_you"][0]["since_minutes"], 90);
    assert_eq!(v["next"][0]["reason"], "waits for a place");
}

#[test]
fn scenario_lines_fit_the_width() {
    // req:view/queue-view@ba511898
    let out = render(&View::Queue(model()), Format::Text, 20, None);
    assert!(out.lines().all(|l| l.chars().count() <= 20), "{out}");
}
