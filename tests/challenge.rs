use nvimkata::challenge::{BufferContent, Category, Challenge, Medal};

fn sample_challenge() -> Challenge {
    Challenge {
        id: "motion_001".to_string(),
        version: "1.0.0".to_string(),
        title: "Test Challenge".to_string(),
        topic: "motions".to_string(),
        difficulty: 1,
        hint: "Use f to find".to_string(),
        detailed_hint: Some("Try 3fw".to_string()),
        par_keystrokes: 10,
        perfect_moves: None,
        focused_actions: None,
        start: BufferContent {
            content: "hello world".to_string(),
        },
        target: BufferContent {
            content: "hello rust".to_string(),
        },
    }
}

#[test]
fn test_perfect_at_par() {
    let c = sample_challenge();
    assert_eq!(c.score(10), Some(Medal::Perfect));
}

#[test]
fn test_perfect_under_par() {
    let c = sample_challenge();
    assert_eq!(c.score(7), Some(Medal::Perfect));
}

#[test]
fn test_gold() {
    let c = sample_challenge();
    // par=10, gold threshold = 10 * 3 / 2 = 15
    assert_eq!(c.score(11), Some(Medal::Gold));
    assert_eq!(c.score(15), Some(Medal::Gold));
}

#[test]
fn test_silver() {
    let c = sample_challenge();
    // par=10, silver threshold = 20
    assert_eq!(c.score(16), Some(Medal::Silver));
    assert_eq!(c.score(20), Some(Medal::Silver));
}

#[test]
fn test_bronze() {
    let c = sample_challenge();
    // par=10, bronze threshold = 30
    assert_eq!(c.score(21), Some(Medal::Bronze));
    assert_eq!(c.score(30), Some(Medal::Bronze));
}

#[test]
fn test_fail() {
    let c = sample_challenge();
    assert_eq!(c.score(31), None);
    assert_eq!(c.score(100), None);
}

#[test]
fn test_thresholds() {
    let c = sample_challenge();
    // par=10
    assert_eq!(c.threshold(Medal::Perfect), 10);
    assert_eq!(c.threshold(Medal::Gold), 15); // 10 * 3 / 2
    assert_eq!(c.threshold(Medal::Silver), 20); // 10 * 2
    assert_eq!(c.threshold(Medal::Bronze), 30); // 10 * 3
}

#[test]
fn test_category_for_topic() {
    assert_eq!(Category::for_topic(1), Category::Beginner);
    assert_eq!(Category::for_topic(2), Category::Beginner);
    assert_eq!(Category::for_topic(3), Category::Intermediate);
    assert_eq!(Category::for_topic(4), Category::Intermediate);
    assert_eq!(Category::for_topic(5), Category::Advanced);
    assert_eq!(Category::for_topic(6), Category::Advanced);
    assert_eq!(Category::for_topic(7), Category::Advanced);
    assert_eq!(Category::for_topic(8), Category::Legendary);
    assert_eq!(Category::for_topic(100), Category::Freestyle);
    assert_eq!(Category::for_topic(107), Category::Freestyle);
}

#[test]
fn test_category_freestyle() {
    assert_eq!(Category::Freestyle.name(), "FREESTYLE");
    assert_eq!(Category::Freestyle.color(), ratatui::style::Color::Cyan);
    assert_eq!(Category::ALL.len(), 5);
    assert!(Category::ALL.contains(&Category::Freestyle));
}

#[test]
fn test_is_freestyle() {
    let mut c = sample_challenge();
    // Has par_keystrokes=10, no perfect_moves → not freestyle
    assert!(!c.is_freestyle());

    // Has par_keystrokes=0, no perfect_moves → freestyle
    c.par_keystrokes = 0;
    assert!(c.is_freestyle());

    // Has par_keystrokes=0 but has perfect_moves → not freestyle (auto-calculated par)
    c.perfect_moves = Some(vec!["jj".to_string()]);
    assert!(!c.is_freestyle());
}

#[test]
fn test_deserialize_from_toml() {
    let toml_str = r#"
id = "motion_001"
version = "1.0.0"
title = "Seek and Replace"
topic = "motions"
difficulty = 1
hint = "Use f/F to jump to characters"
detailed_hint = "Try 3fw to jump to the 3rd w"
par_keystrokes = 8

[start]
content = "The quick brown fox"

[target]
content = "The quick brown cat"
"#;
    let challenge: Challenge = toml::from_str(toml_str).unwrap();
    assert_eq!(challenge.id, "motion_001");
    assert_eq!(challenge.par_keystrokes, 8);
    assert_eq!(challenge.target.content, "The quick brown cat");
}
