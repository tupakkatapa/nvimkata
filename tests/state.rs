use nvimkata::challenge::{BufferContent, Challenge, Grade};
use nvimkata::state::GameState;

fn test_challenge(id: &str, version: &str) -> Challenge {
    Challenge {
        id: id.to_string(),
        version: version.to_string(),
        title: format!("Test {id}"),
        topic: "motions".to_string(),
        difficulty: 1,
        hint: "hint".to_string(),
        detailed_hint: None,
        par_keystrokes: 10,
        perfect_moves: None,
        focused_actions: None,
        start: BufferContent {
            content: "a".to_string(),
        },
        target: BufferContent {
            content: "b".to_string(),
        },
    }
}

#[test]
fn test_default_state() {
    let state = GameState::default();
    assert!(state.challenges.is_empty());
    assert_eq!(state.stats.challenges_attempted, 0);
}

#[test]
fn test_record_result_stores_grade() {
    let mut state = GameState::default();
    state.record_result("motion_001", Grade::C, 12, 30, "jf8cw3000", "1.0.0");
    assert_eq!(state.best_grade("motion_001"), Some(Grade::C));
}

#[test]
fn test_record_result_keeps_better_grade() {
    let mut state = GameState::default();
    state.record_result("motion_001", Grade::C, 12, 30, "jf8cw3000", "1.0.0");
    state.record_result("motion_001", Grade::B, 7, 15, "jcw3000", "1.0.0");
    assert_eq!(state.best_grade("motion_001"), Some(Grade::B));
}

#[test]
fn test_record_result_does_not_downgrade() {
    let mut state = GameState::default();
    state.record_result("motion_001", Grade::B, 7, 15, "jcw3000", "1.0.0");
    state.record_result("motion_001", Grade::D, 30, 60, "jjjjcw3000", "1.0.0");
    assert_eq!(state.best_grade("motion_001"), Some(Grade::B));
}

#[test]
fn test_record_result_updates_on_fewer_keystrokes() {
    let mut state = GameState::default();
    state.record_result("motion_001", Grade::B, 12, 30, "jf8cw3000", "1.0.0");
    state.record_result("motion_001", Grade::B, 9, 20, "jcw3000", "1.0.0");
    assert_eq!(state.challenges["motion_001"].keystrokes, 9);
}

#[test]
fn test_stats_accumulate() {
    let mut state = GameState::default();
    state.record_result("m001", Grade::B, 10, 20, "keys1", "1.0.0");
    state.record_result("m002", Grade::C, 15, 25, "keys2", "1.0.0");
    assert_eq!(state.stats.total_keystrokes, 25);
    assert_eq!(state.stats.challenges_attempted, 2);
}

#[test]
fn test_save_load_roundtrip() {
    let mut state = GameState::default();
    state.record_result("m001", Grade::A, 5, 10, "jcw", "1.0.0");
    state.stats.challenges_attempted = 3;
    let json = serde_json::to_string_pretty(&state).unwrap();
    let loaded: GameState = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.stats.challenges_attempted, 3);
    assert_eq!(loaded.best_grade("m001"), Some(Grade::A));
}

#[test]
fn test_mark_stale_matching_version_not_stale() {
    let mut state = GameState::default();
    state.record_result("m001", Grade::B, 10, 20, "keys", "1.0.0");
    let challenges = [test_challenge("m001", "1.0.0")];
    state.mark_stale(&challenges);
    assert!(!state.is_stale("m001"));
    assert_eq!(state.stale_count(), 0);
    assert_eq!(state.best_grade("m001"), Some(Grade::B));
}

#[test]
fn test_mark_stale_mismatched_version_marked() {
    let mut state = GameState::default();
    state.record_result("m001", Grade::B, 10, 20, "keys", "1.0.0");
    let challenges = [test_challenge("m001", "1.0.1")];
    state.mark_stale(&challenges);
    assert!(state.is_stale("m001"));
    assert_eq!(state.stale_count(), 1);
    // Score and history preserved while stale
    assert_eq!(state.best_grade("m001"), Some(Grade::B));
    assert!(state.history.get("m001").is_some());
}

#[test]
fn test_mark_stale_empty_version_treated_as_mismatch() {
    // Old save format using "medal" key and old variant name — backward compat
    let json = r#"{"challenges":{"m001":{"medal":"Gold","keystrokes":10,"time_secs":20}},"stats":{"total_keystrokes":10,"challenges_attempted":1},"history":{}}"#;
    let mut state: GameState = serde_json::from_str(json).unwrap();
    assert_eq!(state.best_grade("m001"), Some(Grade::B)); // Gold → B
    let challenges = [test_challenge("m001", "1.0.0")];
    state.mark_stale(&challenges);
    assert!(state.is_stale("m001"));
}

#[test]
fn test_mark_stale_unknown_challenge_not_touched() {
    let mut state = GameState::default();
    state.record_result("m001", Grade::B, 10, 20, "keys", "1.0.0");
    state.record_result("deleted", Grade::C, 15, 25, "keys2", "1.0.0");
    // Only m001 exists in current challenges; "deleted" is not in the list
    let challenges = [test_challenge("m001", "1.0.0")];
    state.mark_stale(&challenges);
    assert!(!state.is_stale("m001"));
    assert!(!state.is_stale("deleted"));
    assert_eq!(state.best_grade("deleted"), Some(Grade::C));
}

#[test]
fn test_stale_cleared_on_new_result() {
    let mut state = GameState::default();
    state.record_result("m001", Grade::B, 10, 20, "keys", "1.0.0");
    let challenges = [test_challenge("m001", "1.0.1")];
    state.mark_stale(&challenges);
    assert!(state.is_stale("m001"));
    // Old history visible while stale
    assert_eq!(state.history["m001"].len(), 1);
    // Re-completing clears stale and old history, starts fresh
    state.record_result("m001", Grade::D, 30, 60, "long_keys", "1.0.1");
    assert!(!state.is_stale("m001"));
    assert_eq!(state.best_grade("m001"), Some(Grade::D));
    // History has only the new attempt
    assert_eq!(state.history["m001"].len(), 1);
    assert_eq!(state.history["m001"][0].keystrokes, 30);
}

#[test]
fn test_stale_persists_in_json_roundtrip() {
    let mut state = GameState::default();
    state.record_result("m001", Grade::B, 10, 20, "keys", "1.0.0");
    let challenges = [test_challenge("m001", "1.0.1")];
    state.mark_stale(&challenges);
    let json = serde_json::to_string(&state).unwrap();
    let loaded: GameState = serde_json::from_str(&json).unwrap();
    assert!(loaded.is_stale("m001"));
}

#[test]
fn test_backward_compat_old_medal_key() {
    // Old save format with "medal" field name and old variant names
    let json = r#"{"challenges":{"m001":{"medal":"Gold","keystrokes":10,"time_secs":20}},"stats":{"total_keystrokes":10,"challenges_attempted":1},"history":{"m001":[{"medal":"Gold","keystrokes":10,"time_secs":20,"keys":"jcw"}]}}"#;
    let state: GameState = serde_json::from_str(json).unwrap();
    assert_eq!(state.best_grade("m001"), Some(Grade::B));
    assert_eq!(state.history["m001"][0].grade, Grade::B);
}

#[test]
fn test_new_format_serializes_as_grade() {
    let mut state = GameState::default();
    state.record_result("m001", Grade::B, 10, 20, "jcw", "1.0.0");
    let json = serde_json::to_string(&state).unwrap();
    // New format uses "grade" key and variant "B"
    assert!(json.contains(r#""grade":"B""#));
    assert!(!json.contains(r#""medal""#));
}
