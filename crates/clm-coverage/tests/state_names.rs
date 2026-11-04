use clm_coverage::{STATE_NAMES, State};

/// Exhaustive on purpose: a new `State` variant stops this compiling until the list holds it.
fn index(state: State) -> usize {
    match state {
        State::Current => 0,
        State::Behind => 1,
        State::Unbuilt => 2,
        State::Obsolete => 3,
    }
}

#[test]
fn scenario_list_matches_the_type() {
    // req:check.coverage/state-names@63dc5b5d
    let mut seen = [0; 4];
    for (state, word) in STATE_NAMES {
        seen[index(state)] += 1;
        assert_eq!(state.to_string(), word);
    }
    assert_eq!(seen, [1; 4], "every variant appears once");
    let words: Vec<_> = STATE_NAMES.iter().map(|(_, w)| *w).collect();
    assert_eq!(words, ["current", "behind", "unbuilt", "obsolete"]);
}
