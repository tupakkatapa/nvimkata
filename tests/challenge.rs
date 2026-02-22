use nvimkata::challenge::{BufferContent, Category, Challenge, Grade};

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
fn test_grade_a_at_par() {
    let c = sample_challenge();
    assert_eq!(c.score(10), Grade::A);
}

#[test]
fn test_grade_a_under_par() {
    let c = sample_challenge();
    assert_eq!(c.score(7), Grade::A);
}

#[test]
fn test_grade_b() {
    let c = sample_challenge();
    // par=10, B threshold = 10 * 14 / 10 = 14
    assert_eq!(c.score(11), Grade::B);
    assert_eq!(c.score(14), Grade::B);
}

#[test]
fn test_grade_c() {
    let c = sample_challenge();
    // par=10, C threshold = 10 * 18 / 10 = 18
    assert_eq!(c.score(15), Grade::C);
    assert_eq!(c.score(18), Grade::C);
}

#[test]
fn test_grade_d() {
    let c = sample_challenge();
    // par=10, D threshold = 10 * 24 / 10 = 24
    assert_eq!(c.score(19), Grade::D);
    assert_eq!(c.score(24), Grade::D);
}

#[test]
fn test_grade_e() {
    let c = sample_challenge();
    // par=10, E threshold = 10 * 28 / 10 = 28
    assert_eq!(c.score(25), Grade::E);
    assert_eq!(c.score(28), Grade::E);
}

#[test]
fn test_grade_f() {
    let c = sample_challenge();
    // par=10, anything above E threshold = F
    assert_eq!(c.score(29), Grade::F);
    assert_eq!(c.score(100), Grade::F);
}

#[test]
fn test_thresholds() {
    let c = sample_challenge();
    // par=10
    assert_eq!(c.threshold(Grade::A), 10);
    assert_eq!(c.threshold(Grade::B), 14); // 10 * 14 / 10
    assert_eq!(c.threshold(Grade::C), 18); // 10 * 18 / 10
    assert_eq!(c.threshold(Grade::D), 24); // 10 * 24 / 10
    assert_eq!(c.threshold(Grade::E), 28); // 10 * 28 / 10
    assert_eq!(c.threshold(Grade::F), 32); // 10 * 32 / 10
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
    assert_eq!(Category::Freestyle.color(), ratatui::style::Color::Red);
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
