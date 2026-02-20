use std::fs;
use std::path::PathBuf;

use nvimkata::challenge::count_keystrokes;
use nvimkata::curriculum::load_curriculum;

#[test]
fn test_load_curriculum_from_fixture() {
    let tmp = std::env::temp_dir().join("rlv_test_curriculum");
    let _ = fs::remove_dir_all(&tmp);
    let motions_dir = tmp.join("01_motions");
    fs::create_dir_all(&motions_dir).unwrap();

    fs::write(
        motions_dir.join("motion_001.toml"),
        r#"
id = "motion_001"
title = "Test"
topic = "motions"
difficulty = 1
hint = "hint"
detailed_hint = "detailed"
par_keystrokes = 8

[start]
content = "hello"

[target]
content = "world"
"#,
    )
    .unwrap();

    let topics = load_curriculum(&tmp);
    assert_eq!(topics.len(), 16);
    assert_eq!(topics[0].name, "Advanced Motions");
    assert_eq!(topics[0].challenges.len(), 1);
    assert_eq!(topics[0].challenges[0].id, "motion_001");

    // Topics without dirs have empty challenge lists
    assert!(topics[1].challenges.is_empty());

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_all_challenge_files_parse() {
    let challenges_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("challenges");
    let topics = load_curriculum(&challenges_dir);
    let mut total = 0;
    let mut errors = Vec::new();
    for topic in &topics {
        for challenge in &topic.challenges {
            total += 1;
            if challenge.par_keystrokes == 0 && !challenge.is_freestyle() {
                errors.push(format!("{}: par_keystrokes is 0", challenge.id));
            }
            if challenge.start.content.is_empty() {
                errors.push(format!("{}: start content is empty", challenge.id));
            }
            if challenge.target.content.is_empty() {
                errors.push(format!("{}: target content is empty", challenge.id));
            }
            if challenge.start.content == challenge.target.content {
                errors.push(format!(
                    "{}: start and target content are identical",
                    challenge.id
                ));
            }
        }
    }
    assert!(total > 0, "No challenges found");
    assert!(
        errors.is_empty(),
        "Challenge validation errors:\n{}",
        errors.join("\n")
    );
}

#[test]
fn test_empty_dir_returns_empty_challenges() {
    let tmp = std::env::temp_dir().join("rlv_test_empty");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join("01_motions")).unwrap();

    let topics = load_curriculum(&tmp);
    assert!(topics[0].challenges.is_empty());

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_count_keystrokes() {
    assert_eq!(count_keystrokes("jf8cw3000"), 9);
    assert_eq!(count_keystrokes("jf8cw3000<Esc>"), 10);
    assert_eq!(count_keystrokes("<C-r>a"), 2);
    assert_eq!(count_keystrokes(":%s/^/- [x] <Enter>"), 13);
    assert_eq!(count_keystrokes(""), 0);
}

#[test]
fn test_par_auto_calculated_from_perfect_moves() {
    let challenges_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("challenges");
    let topics = load_curriculum(&challenges_dir);
    let mut with_moves = 0;
    let mut without_moves = 0;
    let mut errors = Vec::new();
    for topic in &topics {
        for challenge in &topic.challenges {
            if challenge.is_freestyle() {
                continue;
            }
            if let Some(moves) = &challenge.perfect_moves {
                let expected: usize = moves.iter().map(|m| count_keystrokes(m)).sum();
                if challenge.par_keystrokes != expected as u32 {
                    errors.push(format!(
                        "{}: par {} != computed {}",
                        challenge.id, challenge.par_keystrokes, expected
                    ));
                }
                with_moves += 1;
            } else {
                if challenge.par_keystrokes == 0 {
                    errors.push(format!(
                        "{}: no perfect_moves and par_keystrokes is 0",
                        challenge.id
                    ));
                }
                without_moves += 1;
            }
        }
    }
    assert!(with_moves > 0, "No challenges with perfect_moves found");
    assert!(
        errors.is_empty(),
        "Par validation errors ({with_moves} with moves, {without_moves} without):\n{}",
        errors.join("\n")
    );
}

#[test]
fn test_perfect_moves_produce_target() {
    use std::time::Duration;

    let challenges_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("challenges");
    let topics = load_curriculum(&challenges_dir);
    let mut errors = Vec::new();
    let mut checked = 0;
    let timeout = Duration::from_secs(5);

    let tmp = std::env::temp_dir().join("nvimkata_test");
    let _ = fs::create_dir_all(&tmp);

    for topic in &topics {
        for challenge in &topic.challenges {
            let Some(moves) = &challenge.perfect_moves else {
                continue;
            };

            let buffer = tmp.join(format!("test_{}", challenge.id));
            fs::write(&buffer, &challenge.start.content).unwrap();

            let moves_lua: String = moves
                .iter()
                .map(|m| {
                    let escaped = nvimkata::nvim::escape_for_lua_sq(m);
                    format!("'{escaped}'")
                })
                .collect::<Vec<_>>()
                .join(", ");

            // Concatenate all moves and feed at once so insert-mode
            // sequences that span adjacent moves work correctly.
            // do_lt (3rd arg) is true so <lt> converts to literal '<'.
            // Write/quit is a separate -c command to avoid timeouts.
            let lua = format!(
                "lua local ms = {{{}}}; \
                 local all = ''; \
                 for _, m in ipairs(ms) do \
                   all = all .. vim.api.nvim_replace_termcodes(m, true, true, true) \
                 end; \
                 vim.api.nvim_feedkeys( \
                   all .. vim.api.nvim_replace_termcodes('<Esc>', true, true, true), \
                   'ntx', false)",
                moves_lua
            );

            let result = std::process::Command::new("nvim")
                .arg("--headless")
                .arg("-u")
                .arg("NONE")
                .arg("-i")
                .arg("NONE")
                .arg("--cmd")
                .arg("set noswapfile noundofile nobackup nowritebackup")
                .arg("-c")
                .arg(&lua)
                .arg("-c")
                .arg("silent! write | qall!")
                .arg(&buffer)
                .spawn()
                .and_then(|mut child| {
                    let start = std::time::Instant::now();
                    loop {
                        match child.try_wait()? {
                            Some(status) => return Ok(status),
                            None if start.elapsed() > timeout => {
                                let _ = child.kill();
                                let _ = child.wait();
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::TimedOut,
                                    "nvim timed out",
                                ));
                            }
                            None => std::thread::sleep(Duration::from_millis(50)),
                        }
                    }
                });

            match result {
                Ok(status) if status.success() => {
                    let content = fs::read_to_string(&buffer).unwrap_or_default();
                    let result_norm = nvimkata::nvim::normalize(&content);
                    let target_norm = nvimkata::nvim::normalize(&challenge.target.content);
                    if result_norm != target_norm {
                        errors.push(format!("{}: buffer does not match target", challenge.id));
                    }
                }
                Ok(status) => {
                    errors.push(format!("{}: nvim exited with {status}", challenge.id));
                }
                Err(e) => {
                    errors.push(format!("{}: {e}", challenge.id));
                }
            }

            let _ = fs::remove_file(&buffer);
            checked += 1;
        }
    }

    let _ = fs::remove_dir_all(&tmp);

    assert!(checked > 0, "No challenges with perfect_moves found");
    if !errors.is_empty() {
        panic!(
            "{}/{} challenges failed:\n{}",
            errors.len(),
            checked,
            errors.join("\n")
        );
    }
}
