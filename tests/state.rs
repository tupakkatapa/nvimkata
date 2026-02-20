use nvimkata::challenge::Medal;
use nvimkata::state::GameState;

#[test]
fn test_default_state() {
    let state = GameState::default();
    assert!(state.challenges.is_empty());
    assert_eq!(state.stats.challenges_attempted, 0);
}

#[test]
fn test_record_result_stores_medal() {
    let mut state = GameState::default();
    state.record_result("motion_001", Medal::Silver, 12, 30, "jf8cw3000");
    assert_eq!(state.best_medal("motion_001"), Some(Medal::Silver));
}

#[test]
fn test_record_result_keeps_better_medal() {
    let mut state = GameState::default();
    state.record_result("motion_001", Medal::Silver, 12, 30, "jf8cw3000");
    state.record_result("motion_001", Medal::Gold, 7, 15, "jcw3000");
    assert_eq!(state.best_medal("motion_001"), Some(Medal::Gold));
}

#[test]
fn test_record_result_does_not_downgrade() {
    let mut state = GameState::default();
    state.record_result("motion_001", Medal::Gold, 7, 15, "jcw3000");
    state.record_result("motion_001", Medal::Bronze, 30, 60, "jjjjcw3000");
    assert_eq!(state.best_medal("motion_001"), Some(Medal::Gold));
}

#[test]
fn test_record_result_updates_on_fewer_keystrokes() {
    let mut state = GameState::default();
    state.record_result("motion_001", Medal::Gold, 12, 30, "jf8cw3000");
    state.record_result("motion_001", Medal::Gold, 9, 20, "jcw3000");
    assert_eq!(state.challenges["motion_001"].keystrokes, 9);
}

#[test]
fn test_stats_accumulate() {
    let mut state = GameState::default();
    state.record_result("m001", Medal::Gold, 10, 20, "keys1");
    state.record_result("m002", Medal::Silver, 15, 25, "keys2");
    assert_eq!(state.stats.total_keystrokes, 25);
    assert_eq!(state.stats.challenges_attempted, 2);
}

#[test]
fn test_save_load_roundtrip() {
    let mut state = GameState::default();
    state.record_result("m001", Medal::Perfect, 5, 10, "jcw");
    state.stats.challenges_attempted = 3;
    let json = serde_json::to_string_pretty(&state).unwrap();
    let loaded: GameState = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.stats.challenges_attempted, 3);
    assert_eq!(loaded.best_medal("m001"), Some(Medal::Perfect));
}
