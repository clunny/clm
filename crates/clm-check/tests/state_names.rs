use clm_check::{STATE_NAMES, State};

#[test]
fn scenario_list_reaches_clm_check() {
    // req:check.coverage/state-names@63dc5b5d
    for (state, word) in STATE_NAMES {
        assert_eq!(state.to_string(), word);
    }
    assert!(STATE_NAMES.contains(&(State::Obsolete, "obsolete")));
}
