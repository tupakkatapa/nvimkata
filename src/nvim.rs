use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::challenge::{Challenge, Medal};

/// Result of running a challenge in neovim.
pub struct ChallengeResult {
    pub buffer_matches: bool,
    pub keystrokes: u32,
    pub elapsed_secs: u32,
    pub keys: String,
}

/// Temporary file paths for a challenge session.
struct SessionFiles {
    buffer: PathBuf,
    target: PathBuf,
    results: PathBuf,
    start: PathBuf,
    lua: PathBuf,
}

impl SessionFiles {
    fn new() -> Self {
        let dir = std::env::temp_dir().join("nvimkata");
        Self {
            buffer: dir.join("challenge_buffer"),
            target: dir.join("challenge_target"),
            results: dir.join("results"),
            start: dir.join("challenge_start"),
            lua: dir.join("runtime.lua"),
        }
    }

    fn ensure_dir(&self) -> io::Result<()> {
        if let Some(parent) = self.buffer.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

/// Launch neovim with a challenge. Returns the result after nvim exits.
pub fn run_challenge(challenge: &Challenge, number: usize) -> io::Result<ChallengeResult> {
    let files = SessionFiles::new();
    files.ensure_dir()?;

    // Write start content, target content, and start backup to temp files
    fs::write(&files.buffer, &challenge.start.content)?;
    fs::write(&files.target, &challenge.target.content)?;
    fs::write(&files.start, &challenge.start.content)?;

    // Remove old results file if exists
    let _ = fs::remove_file(&files.results);

    let freestyle = challenge.is_freestyle();
    let limit = if freestyle {
        9999
    } else {
        challenge.threshold(Medal::Bronze)
    };

    // Build and write the Lua runtime script
    let lua_script = build_lua_script(challenge, number, limit, freestyle, &files);
    fs::write(&files.lua, &lua_script)?;

    // Build nvim command
    let status = Command::new("nvim")
        // Disable swap files and viminfo to avoid noise
        .arg("--cmd")
        .arg("set noswapfile noundofile nobackup nowritebackup")
        // Open target in a horizontal split (top, read-only, labeled)
        .arg("-c")
        .arg(format!(
            "split {} | setlocal readonly nomodifiable noswapfile buftype=nofile | \
             let &l:winbar = '  [TARGET]' | \
             diffthis | set diffopt+=context:99999 | setlocal wrap nocursorbind | \
             wincmd j | diffthis | set diffopt+=context:99999 | setlocal wrap nocursorbind",
            files.target.display()
        ))
        // Load the Lua runtime
        .arg("-c")
        .arg(format!("luafile {}", files.lua.display()))
        // Stop counting keystrokes and quit on :w
        .arg("-c")
        .arg(format!(
            "autocmd BufWritePost {} lua _G._ks_stop(); vim.cmd('qall!')",
            files.buffer.display()
        ))
        // Open the challenge buffer
        .arg(&files.buffer)
        .status()?;

    if !status.success() {
        return Err(io::Error::other(format!(
            "nvim exited with status: {status}"
        )));
    }

    // Read results
    let result_content = fs::read_to_string(&files.buffer)?;
    let (keystrokes, elapsed_secs, keys) = read_results(&files.results);
    let buffer_matches = normalize(&result_content) == normalize(&challenge.target.content);

    Ok(ChallengeResult {
        buffer_matches,
        keystrokes,
        elapsed_secs,
        keys,
    })
}

/// Escape a string for use in a Lua single-quoted string literal.
pub fn escape_for_lua_sq(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Build the full Lua script by prepending variable definitions to the template.
fn build_lua_script(
    challenge: &Challenge,
    number: usize,
    limit: u32,
    freestyle: bool,
    files: &SessionFiles,
) -> String {
    let title = escape_for_lua_sq(&challenge.title);
    let hint = escape_for_lua_sq(&challenge.hint);
    let detailed_hint = challenge
        .detailed_hint
        .as_deref()
        .map_or_else(String::new, escape_for_lua_sq);
    let results_path = files.results.display();
    let target_path = files.target.display();
    let start_path = files.start.display();

    let preamble = format!(
        "_VK_NUMBER = {number}\n\
         _VK_TITLE = '{title}'\n\
         _VK_PAR = {par}\n\
         _VK_HINT = '{hint}'\n\
         _VK_DETAILED_HINT = '{detailed_hint}'\n\
         _VK_LIMIT = {limit}\n\
         _VK_FREESTYLE = {freestyle}\n\
         _VK_RESULTS_PATH = '{results_path}'\n\
         _VK_TARGET_PATH = '{target_path}'\n\
         _VK_START_PATH = '{start_path}'\n\
         _VK_THRESHOLD_P = {tp}\n\
         _VK_THRESHOLD_G = {tg}\n\
         _VK_THRESHOLD_S = {ts}\n\
         _VK_THRESHOLD_B = {tb}\n",
        par = challenge.par_keystrokes,
        tp = challenge.threshold(Medal::Perfect),
        tg = challenge.threshold(Medal::Gold),
        ts = challenge.threshold(Medal::Silver),
        tb = challenge.threshold(Medal::Bronze),
    );

    let template = include_str!("challenge_runtime.lua");
    format!("{preamble}\n{template}")
}

/// Read keystroke count, elapsed seconds, and key log from the results file.
/// Format: three lines — keystroke count, elapsed seconds, key presses.
fn read_results(path: &Path) -> (u32, u32, String) {
    let contents = fs::read_to_string(path).unwrap_or_default();
    let mut lines = contents.lines();
    let keystrokes = lines
        .next()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let elapsed = lines
        .next()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let keys = lines.next().unwrap_or("").to_string();
    (keystrokes, elapsed, keys)
}

/// Normalize content for comparison: trim trailing whitespace per line,
/// strip trailing empty lines.
pub fn normalize(s: &str) -> String {
    s.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end_matches('\n')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_results_valid() {
        let tmp = std::env::temp_dir().join("rlv_test_results");
        fs::write(&tmp, "42\n15\njf8cw3000").unwrap();
        assert_eq!(read_results(&tmp), (42, 15, "jf8cw3000".to_string()));
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_read_results_missing_file() {
        let tmp = std::env::temp_dir().join("rlv_nonexistent_results");
        assert_eq!(read_results(&tmp), (0, 0, String::new()));
    }

    #[test]
    fn test_read_results_partial() {
        let tmp = std::env::temp_dir().join("rlv_test_results_partial");
        fs::write(&tmp, "35\n").unwrap();
        assert_eq!(read_results(&tmp), (35, 0, String::new()));
        let _ = fs::remove_file(&tmp);
    }
}
