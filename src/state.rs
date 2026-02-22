use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::challenge::{Challenge, Grade};

#[derive(Debug)]
pub struct SaveError {
    pub path: PathBuf,
    pub source: String,
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Failed to load save file '{}': {}",
            self.path.display(),
            self.source
        )
    }
}

impl std::error::Error for SaveError {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameState {
    pub challenges: HashMap<String, BestResult>,
    pub stats: Stats,
    #[serde(default)]
    pub history: HashMap<String, Vec<AttemptRecord>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptRecord {
    #[serde(alias = "medal")]
    pub grade: Grade,
    pub keystrokes: u32,
    pub time_secs: u32,
    #[serde(default)]
    pub keys: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestResult {
    #[serde(alias = "medal")]
    pub grade: Grade,
    pub keystrokes: u32,
    #[serde(default)]
    pub time_secs: u32,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub stale: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Stats {
    pub total_keystrokes: u64,
    pub challenges_attempted: u32,
}

impl GameState {
    pub fn record_result(
        &mut self,
        challenge_id: &str,
        grade: Grade,
        keystrokes: u32,
        time_secs: u32,
        keys: &str,
        version: &str,
    ) {
        let was_stale = self.challenges.get(challenge_id).is_some_and(|b| b.stale);
        let is_improvement = self.challenges.get(challenge_id).is_none_or(|best| {
            best.stale
                || grade_rank(grade) < grade_rank(best.grade)
                || (grade == best.grade && keystrokes < best.keystrokes)
        });
        if is_improvement {
            self.challenges.insert(
                challenge_id.to_string(),
                BestResult {
                    grade,
                    keystrokes,
                    time_secs,
                    version: version.to_string(),
                    stale: false,
                },
            );
            if was_stale {
                self.history.remove(challenge_id);
            }
        }
        self.stats.total_keystrokes += u64::from(keystrokes);
        self.stats.challenges_attempted += 1;

        // Store in history (keep top 10 by keystrokes)
        let history = self.history.entry(challenge_id.to_string()).or_default();
        history.push(AttemptRecord {
            grade,
            keystrokes,
            time_secs,
            keys: keys.to_string(),
        });
        history.sort_by_key(|a| a.keystrokes);
        history.truncate(10);
    }

    /// Record a freestyle result — improves on fewer keystrokes only, no grade comparison.
    pub fn record_freestyle_result(
        &mut self,
        challenge_id: &str,
        keystrokes: u32,
        time_secs: u32,
        keys: &str,
        version: &str,
    ) {
        let was_stale = self.challenges.get(challenge_id).is_some_and(|b| b.stale);
        let is_improvement = self
            .challenges
            .get(challenge_id)
            .is_none_or(|best| best.stale || keystrokes < best.keystrokes);
        if is_improvement {
            self.challenges.insert(
                challenge_id.to_string(),
                BestResult {
                    grade: Grade::F, // placeholder, never displayed for freestyle
                    keystrokes,
                    time_secs,
                    version: version.to_string(),
                    stale: false,
                },
            );
            if was_stale {
                self.history.remove(challenge_id);
            }
        }
        self.stats.total_keystrokes += u64::from(keystrokes);
        self.stats.challenges_attempted += 1;

        // Store in history (keep top 10 by keystrokes)
        let history = self.history.entry(challenge_id.to_string()).or_default();
        history.push(AttemptRecord {
            grade: Grade::F,
            keystrokes,
            time_secs,
            keys: keys.to_string(),
        });
        history.sort_by_key(|a| a.keystrokes);
        history.truncate(10);
    }

    /// Mark saved results as stale when their version doesn't match the current challenge.
    pub fn mark_stale(&mut self, challenges: &[Challenge]) {
        let challenge_map: HashMap<&str, &Challenge> =
            challenges.iter().map(|c| (c.id.as_str(), c)).collect();
        for (id, best) in &mut self.challenges {
            if let Some(c) = challenge_map.get(id.as_str())
                && best.version != c.version
            {
                best.stale = true;
            }
        }
    }

    /// Count challenges with stale scores.
    pub fn stale_count(&self) -> usize {
        self.challenges.values().filter(|b| b.stale).count()
    }

    /// Check if a specific challenge has a stale score.
    pub fn is_stale(&self, challenge_id: &str) -> bool {
        self.challenges.get(challenge_id).is_some_and(|b| b.stale)
    }

    /// Get the best keystroke count for a challenge, if attempted.
    pub fn best_keystrokes(&self, challenge_id: &str) -> Option<u32> {
        self.challenges.get(challenge_id).map(|r| r.keystrokes)
    }

    pub fn best_grade(&self, challenge_id: &str) -> Option<Grade> {
        self.challenges.get(challenge_id).map(|r| r.grade)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = save_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json)?;
        Ok(())
    }

    pub fn load() -> Result<Self, SaveError> {
        let path = save_path();
        match fs::read_to_string(&path) {
            Ok(json) => serde_json::from_str(&json).map_err(|e| SaveError {
                path,
                source: e.to_string(),
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(SaveError {
                path,
                source: e.to_string(),
            }),
        }
    }
}

fn save_path() -> PathBuf {
    let local = PathBuf::from("save.json");
    if local.exists() {
        return local;
    }
    let data_dir = if let Ok(dir) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(dir)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".local/share")
    };
    data_dir.join("nvimkata/save.json")
}

fn grade_rank(grade: Grade) -> u8 {
    match grade {
        Grade::A => 0,
        Grade::B => 1,
        Grade::C => 2,
        Grade::D => 3,
        Grade::E => 4,
        Grade::F => 5,
    }
}
