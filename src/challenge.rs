use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

const GOLD_MULTIPLIER_NUM: u32 = 3;
const GOLD_MULTIPLIER_DEN: u32 = 2; // 1.5x
const SILVER_MULTIPLIER: u32 = 2; // 2x
const BRONZE_MULTIPLIER: u32 = 3; // 3x

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub id: String,
    pub title: String,
    pub topic: String,
    pub difficulty: u8,
    pub hint: String,
    #[serde(default)]
    pub detailed_hint: Option<String>,
    #[serde(default)]
    pub par_keystrokes: u32,
    #[serde(default)]
    pub perfect_moves: Option<Vec<String>>,
    #[serde(default)]
    pub focused_actions: Option<Vec<String>>,
    pub start: BufferContent,
    pub target: BufferContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferContent {
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Medal {
    Perfect,
    Gold,
    Silver,
    Bronze,
}

impl Medal {
    pub fn color(self) -> Color {
        match self {
            Self::Perfect => Color::Magenta,
            Self::Gold => Color::Yellow,
            Self::Silver => Color::White,
            Self::Bronze => Color::Rgb(205, 127, 50),
        }
    }

    pub fn style(self) -> Style {
        let s = Style::new().fg(self.color());
        match self {
            Self::Perfect => s.add_modifier(Modifier::BOLD),
            _ => s,
        }
    }

    pub fn display_char(self) -> &'static str {
        match self {
            Self::Perfect => "P",
            Self::Gold => "G",
            Self::Silver => "S",
            Self::Bronze => "B",
        }
    }
}

/// Display string and style for an optional medal. Returns "-" in `Gray` for None.
pub fn medal_display(medal: Option<Medal>) -> (&'static str, Style) {
    match medal {
        Some(m) => (m.display_char(), m.style()),
        None => ("-", Style::new().fg(Color::Gray)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Beginner,
    Intermediate,
    Advanced,
    Legendary,
    Freestyle,
}

impl Category {
    pub const ALL: [Category; 5] = [
        Self::Beginner,
        Self::Intermediate,
        Self::Advanced,
        Self::Legendary,
        Self::Freestyle,
    ];

    pub fn for_topic(id: u8) -> Self {
        match id {
            1 | 2 => Self::Beginner,
            3 | 4 => Self::Intermediate,
            5..=7 => Self::Advanced,
            100..=107 => Self::Freestyle,
            _ => Self::Legendary,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Beginner => "BEGINNER",
            Self::Intermediate => "INTERMEDIATE",
            Self::Advanced => "ADVANCED",
            Self::Legendary => "LEGENDARY",
            Self::Freestyle => "FREESTYLE",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Self::Beginner => Color::Green,
            Self::Intermediate => Color::Blue,
            Self::Advanced => Color::Magenta,
            Self::Legendary => Color::Rgb(255, 165, 0),
            Self::Freestyle => Color::Cyan,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    pub id: u8,
    pub name: String,
    pub description: String,
    pub challenges: Vec<Challenge>,
}

impl Challenge {
    /// Returns true if this is a freestyle challenge (no par, no `perfect_moves`).
    pub fn is_freestyle(&self) -> bool {
        self.par_keystrokes == 0 && self.perfect_moves.is_none()
    }

    /// Score a completed challenge based on keystroke count vs par.
    /// Returns None if the player failed (exceeded bronze threshold).
    pub fn score(&self, keystrokes: u32) -> Option<Medal> {
        let par = self.par_keystrokes;
        if keystrokes <= par {
            Some(Medal::Perfect)
        } else if keystrokes <= par * GOLD_MULTIPLIER_NUM / GOLD_MULTIPLIER_DEN {
            Some(Medal::Gold)
        } else if keystrokes <= par * SILVER_MULTIPLIER {
            Some(Medal::Silver)
        } else if keystrokes <= par * BRONZE_MULTIPLIER {
            Some(Medal::Bronze)
        } else {
            None
        }
    }

    /// Get the keystroke threshold for a given medal.
    pub fn threshold(&self, medal: Medal) -> u32 {
        let par = self.par_keystrokes;
        match medal {
            Medal::Perfect => par,
            Medal::Gold => par * GOLD_MULTIPLIER_NUM / GOLD_MULTIPLIER_DEN,
            Medal::Silver => par * SILVER_MULTIPLIER,
            Medal::Bronze => par * BRONZE_MULTIPLIER,
        }
    }
}

/// Count keystrokes in a vim key notation string.
/// Regular characters count as 1. `<...>` sequences (e.g., `<Esc>`, `<C-r>`) count as 1.
///
/// **Convention for challenge authors:** Literal `<` in typed text (e.g., `Vec<String>`)
/// must be written as `<lt>` in `perfect_moves` to avoid being parsed as a vim key name.
/// For example, `ciw<lt>Esc>` types the literal text `<Esc>` rather than pressing Escape.
pub fn count_keystrokes(s: &str) -> usize {
    let mut count = 0;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '<' {
            for c2 in chars.by_ref() {
                if c2 == '>' {
                    break;
                }
            }
        }
        count += 1;
    }
    count
}
