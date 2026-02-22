use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub id: String,
    pub version: String,
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
pub enum Grade {
    #[serde(alias = "Perfect")]
    A,
    #[serde(alias = "Gold")]
    B,
    #[serde(alias = "Silver")]
    C,
    #[serde(alias = "Bronze")]
    D,
    E,
    F,
}

impl Grade {
    pub fn color(self) -> Color {
        match self {
            Self::A => Color::Rgb(255, 165, 0), // Orange (same as Legendary)
            Self::B | Self::C | Self::D | Self::E => Color::Cyan,
            Self::F => Color::Red,
        }
    }

    pub fn style(self) -> Style {
        let s = Style::new().fg(self.color());
        match self {
            Self::A => s.add_modifier(Modifier::BOLD),
            _ => s,
        }
    }

    pub fn display_char(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
            Self::F => "F",
        }
    }
}

/// Display string and style for an optional grade. Returns "-" in `Gray` for None.
pub fn grade_display(grade: Option<Grade>) -> (&'static str, Style) {
    match grade {
        Some(g) => (g.display_char(), g.style()),
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
            Self::Beginner => Color::Cyan,
            Self::Intermediate => Color::Blue,
            Self::Advanced => Color::Magenta,
            Self::Legendary => Color::Rgb(255, 165, 0),
            Self::Freestyle => Color::Red,
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
    /// Always returns a grade (F for anything above E threshold).
    pub fn score(&self, keystrokes: u32) -> Grade {
        let par = self.par_keystrokes;
        if keystrokes <= par {
            Grade::A
        } else if keystrokes <= par * 14 / 10 {
            Grade::B
        } else if keystrokes <= par * 18 / 10 {
            Grade::C
        } else if keystrokes <= par * 24 / 10 {
            Grade::D
        } else if keystrokes <= par * 28 / 10 {
            Grade::E
        } else {
            Grade::F
        }
    }

    /// Get the keystroke threshold for a given grade.
    pub fn threshold(&self, grade: Grade) -> u32 {
        let par = self.par_keystrokes;
        match grade {
            Grade::A => par,
            Grade::B => par * 14 / 10,
            Grade::C => par * 18 / 10,
            Grade::D => par * 24 / 10,
            Grade::E => par * 28 / 10,
            Grade::F => par * 32 / 10,
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
