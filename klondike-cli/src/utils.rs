use anyhow::Result;
use klondike_common::{action::Action, board::Board};
use klondike_solver::{SolveResult, solve};

use std::{
    io::{IsTerminal, Write, stderr},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub fn do_solve(board: Board, max_states: u32, minimal: bool) -> Result<Vec<Action>> {
    let board_str = board.to_pretty_string();
    println!("{board_str}\n");
    let SolveResult {
        actions,
        elapsed,
        states,
        minimal,
    } = with_spinner("Solving the game...", move || {
        solve(board, max_states, minimal)
    })?;
    let total_actions = actions.len();
    let redeal_count = actions.iter().filter(|a| a.is_redeal()).count();
    let elapsed_str = format_elapsed(elapsed);
    let mut steps_str = format!("{} Moves", total_actions - redeal_count);
    if redeal_count > 0 {
        steps_str.push_str(&format!(", {redeal_count} Redeal"));
        if redeal_count > 1 {
            steps_str.push('s');
        }
    };
    println!(
        "✓ Solved in {steps_str} — Minimal: {minimal}, Time: {elapsed_str}, States: {states}\n"
    );
    Ok(actions)
}

fn with_spinner<T, F: FnOnce() -> T>(message: &str, f: F) -> T {
    if stderr().is_terminal() {
        let spinning = Arc::new(AtomicBool::new(true));
        let spinning_clone = Arc::clone(&spinning);
        let message = message.to_string();

        let handle = std::thread::spawn(move || {
            let spinner_chars = ['|', '/', '-', '\\'];
            let mut i = 0;
            let stderr = stderr();
            let mut handle = stderr.lock();

            let _ = write!(handle, "\x1b[?25l"); // hide cursor
            let _ = handle.flush();

            while spinning_clone.load(Ordering::Relaxed) {
                let spinner_char = spinner_chars[i % spinner_chars.len()];
                let _ = write!(handle, "\r{spinner_char} {message}",);
                let _ = handle.flush();
                std::thread::sleep(Duration::from_millis(100));
                i += 1;
            }

            let _ = write!(handle, "\r\x1b[2K\r\x1b[?25h"); // clear line and show cursor
            let _ = handle.flush();
        });

        let result = f();
        spinning.store(false, Ordering::Relaxed);
        let _ = handle.join();
        result
    } else {
        f()
    }
}

fn format_elapsed(elapsed: Duration) -> String {
    let secs = elapsed.as_secs();
    if secs < 90 {
        let ms = elapsed.subsec_millis();
        format!("{secs}.{ms:03}s")
    } else {
        let minutes = secs / 60;
        let secs = secs % 60;
        format!("{minutes}m {secs}s")
    }
}
