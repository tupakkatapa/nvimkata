use std::fs;
use std::path::{Path, PathBuf};

use crate::challenge::{Challenge, Topic, count_keystrokes};

/// Topic metadata. Challenge TOML files live in subdirectories.
const TOPICS: &[(u8, &str, &str, &str)] = &[
    (
        1,
        "01_motions",
        "Advanced Motions",
        "f/t/;, %, [{, ]m, H/M/L, g;/g,",
    ),
    (
        2,
        "02_text_objects",
        "Text Objects",
        "ci\", da(, vit, ciw, cip",
    ),
    (
        3,
        "03_registers",
        "Registers",
        "\"a-z, \"0-9, \"+, \"., \"_",
    ),
    (
        4,
        "04_marks_jumps",
        "Marks & Jumps",
        "ma, `a, '', g;, Ctrl-O/I",
    ),
    (
        5,
        "05_macros",
        "Macros",
        "qa, @a, @@, recursive macros, macro editing",
    ),
    (
        6,
        "06_ex_commands",
        "Ex Commands",
        ":g, :s, :norm, ranges, :sort, :!",
    ),
    (
        7,
        "07_advanced_combos",
        "Advanced Combos",
        "Combining all techniques",
    ),
    (
        8,
        "08_legendary",
        "Legendary Combos",
        "The ultimate vim challenges",
    ),
];

/// Freestyle topic metadata — no par, no grades, personal-best tracking.
const FREESTYLE_TOPICS: &[(u8, &str, &str, &str)] = &[
    (
        100,
        "f01_refactoring",
        "Code Refactoring",
        "Rename, restructure, and clean up code",
    ),
    (
        101,
        "f02_data_wrangling",
        "Data Wrangling",
        "Transform CSV, JSON, and tabular data",
    ),
    (
        102,
        "f03_bug_fixing",
        "Bug Fixing",
        "Find and fix multiple bugs in code",
    ),
    (
        103,
        "f04_pattern_power",
        "Pattern Power",
        "Repetitive transformations at scale",
    ),
    (
        104,
        "f05_format_alchemy",
        "Format Alchemy",
        "Convert between data formats",
    ),
    (
        105,
        "f06_legacy_cleanup",
        "Legacy Cleanup",
        "Modernize and clean messy legacy code",
    ),
    (
        106,
        "f07_multi_edit",
        "Multi-Edit Mastery",
        "Complex edits across many locations",
    ),
    (
        107,
        "f08_grand",
        "Grand Challenges",
        "Long, complex mixed-skill challenges",
    ),
];

/// Load all topics from a challenges directory.
pub fn load_curriculum(challenges_dir: &Path) -> Vec<Topic> {
    TOPICS
        .iter()
        .chain(FREESTYLE_TOPICS.iter())
        .map(|(id, dir_name, name, description)| {
            let dir = challenges_dir.join(dir_name);
            let challenges = load_challenges_from_dir(&dir);
            Topic {
                id: *id,
                name: (*name).to_string(),
                description: (*description).to_string(),
                challenges,
            }
        })
        .collect()
}

/// Load all .toml challenge files from a directory.
fn load_challenges_from_dir(dir: &Path) -> Vec<Challenge> {
    let mut challenges = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return challenges;
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(std::result::Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    paths.sort();
    for path in paths {
        match fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<Challenge>(&content) {
                Ok(mut challenge) => {
                    if let Some(moves) = &challenge.perfect_moves {
                        challenge.par_keystrokes =
                            u32::try_from(moves.iter().map(|m| count_keystrokes(m)).sum::<usize>())
                                .expect("keystroke count exceeds u32");
                    }
                    challenges.push(challenge);
                }
                Err(e) => eprintln!("Warning: failed to parse {}: {}", path.display(), e),
            },
            Err(e) => eprintln!("Warning: failed to read {}: {}", path.display(), e),
        }
    }
    challenges
}
