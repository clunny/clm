//! How the fold chooses among lines sharing an `id` (`ledger/effort-records`). Built with
//! the default features off as well: `tests/wasm.rs` runs this file so, since the choice is
//! the same in both builds.

use clm_ledger::Body;

/// An `effort` line for session `s1` on `impl/a`, with `id` `e` and two models.
fn effort_line(at: &str, writer: &str, output: u64) -> String {
    format!(
        r#"{{"id":"e","kind":"effort","writer":"{writer}","at":"{at}","branch":"impl/a","session":"s1","models":{{"claude-opus-5-5":{{"input":10,"output":{output},"cache_read":100,"cache_write":20}},"claude-sonnet-5-5":{{"input":1,"output":2,"cache_read":0,"cache_write":3}}}},"active_seconds":120}}"#
    )
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
