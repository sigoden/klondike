use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use solitaire_solver::{
    action::{Action, format_actions},
    board::Board,
    solver::{SolveResult, solve},
};
use std::io::{IsTerminal, Write, stderr};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Inspect the game state
    #[cfg(windows)]
    Inspect,
    /// Solve the game
    Solve {
        /// Max states to try to find a solution
        #[arg(long, default_value_t = 50_000_000, value_name = "NUM")]
        max_states: usize,
        /// Whether to find the solution with minimal steps
        #[arg(long, default_value_t = true)]
        minimal: bool,
        /// Optional file to load the game state from
        file: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        #[cfg(windows)]
        Commands::Inspect => {
            let board = solitaire_solver::inspect::inspect()?;
            println!("{}", board.pretty_print());
        }
        Commands::Solve {
            max_states,
            minimal,
            file,
        } => {
            let board = if let Some(file) = file {
                let content = std::fs::read_to_string(file)?;
                Board::parse(&content).map_err(|err| anyhow!("Failed to parse board; {err}"))?
            } else {
                #[cfg(windows)]
                {
                    solitaire_solver::inspect::inspect()?
                }
                #[cfg(not(windows))]
                {
                    anyhow::bail!("The 'solver' requires a file to load the game state from.");
                }
            };
            let actions = do_solve(board, *max_states, *minimal)?;
            println!("{}", format_actions(&actions));
        }
    }

    Ok(())
}

fn do_solve(board: Board, max_states: usize, minimal: bool) -> Result<Vec<Action>> {
    let state = board.pretty_print();
    let SolveResult {
        actions,
        elapsed,
        states,
        minimal,
    } = with_spinner("Solving the game...", move || {
        solve(board, max_states, minimal)
    })?;
    let actions_len = actions.len();
    let elapsed = format_elapsed(elapsed);
    println!(
        r#"✓ Solved the game. Steps: {actions_len}, Elapsed: {elapsed}, States: {states}, Minimal: {minimal}

===== STATE =====
{state}

===== STEPS ====="#
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
