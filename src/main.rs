use std::path::PathBuf;

use nvimkata::{challenge, curriculum, game, hub, state};

fn challenges_dir() -> PathBuf {
    // Check for bundled challenges next to the binary first,
    // then fall back to the current directory.
    if let Ok(exe) = std::env::current_exe() {
        let dir = exe
            .parent()
            .unwrap_or(&exe)
            .join("../share/nvimkata/challenges");
        if dir.exists() {
            return dir;
        }
    }
    PathBuf::from("challenges")
}

fn print_help() {
    let version = env!("CARGO_PKG_VERSION");
    println!("nvimkata {version} — practice efficient editing in Neovim");
    println!();
    println!("Usage: nvimkata [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --unlock-all  Unlock all categories (skip progression)");
    println!("  -h, --help    Show this help message");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut unlock_all = false;

    for arg in &args {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            "--unlock-all" => unlock_all = true,
            other => {
                eprintln!("Unknown option: {other}");
                eprintln!("Run with --help for usage.");
                std::process::exit(1);
            }
        }
    }

    // Check neovim is available
    if std::process::Command::new("nvim")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("Error: neovim (nvim) is required but not found in PATH.");
        std::process::exit(1);
    }

    let challenges_path = challenges_dir();
    let topics = curriculum::load_curriculum(&challenges_path);

    if topics.iter().all(|t| t.challenges.is_empty()) {
        eprintln!("No challenges found. Make sure the 'challenges/' directory exists.");
        eprintln!("Looked in: {}", challenges_path.display());
        std::process::exit(1);
    }

    let mut state = state::GameState::load();
    let all_challenges: Vec<challenge::Challenge> =
        topics.iter().flat_map(|t| t.challenges.clone()).collect();
    state.mark_stale(&all_challenges);
    let mut terminal = ratatui::init();

    let result = run(&mut terminal, &mut state, &topics, unlock_all);

    ratatui::restore();
    state.save()?;

    result?;
    Ok(())
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    state: &mut state::GameState,
    topics: &[challenge::Topic],
    unlock_all: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut hub = hub::Hub::new(topics.to_vec(), unlock_all);

    loop {
        match hub.run(terminal, state)? {
            hub::HubAction::Quit => return Ok(()),
            hub::HubAction::SelectTopic(topic_id) => {
                if let Some(topic) = topics.iter().find(|t| t.id == topic_id) {
                    let offset: usize = topics
                        .iter()
                        .filter(|t| t.id < topic_id)
                        .map(|t| t.challenges.len())
                        .sum();
                    game::run_challenge_picker(terminal, state, topic, offset)?;
                    state.save()?;
                }
            }
        }
    }
}
