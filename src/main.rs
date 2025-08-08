use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use solitaire_solver::{
    action::{Action, format_actions},
    board::Board,
    solver::{SolveResult, solve},
};
use std::{
    io::{IsTerminal, Read, Write, stderr, stdin},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Solve the game
    Solve {
        /// Game ID from greenfelt.net/klondike (e.g. 283409412)
        #[arg(short, long, value_name = "SEED")]
        greenfelt: Option<u32>,
        /// Cards drawn per turn (1 or 3)
        #[arg(short, long, value_name = "NUM")]
        draw: Option<usize>,
        /// Max states to explore (~1 GB per 64 million states)
        #[arg(short = 's', long, default_value_t = 100_000_000, value_name = "NUM")]
        max_states: u32,
        /// Stop at first found solution (may not be minimal)
        #[arg(short, long)]
        fast: bool,
        /// Preview initial game state without solving
        #[arg(short, long)]
        preview: bool,
        /// Path to a game state file to solve
        file: Option<PathBuf>,
    },
    /// Automatically play the game
    #[cfg(windows)]
    Autoplay {
        /// Max states to explore (~1 GB per 64 million states)
        #[arg(short, long, default_value_t = 100_000_000, value_name = "NUM")]
        max_states: u32,
        /// Stop at first found solution (may not be minimal)
        #[arg(short, long)]
        fast: bool,
        /// Delay between moves in milliseconds
        #[arg(short, long, default_value_t = 3000, value_name = "MS")]
        interval: u64,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Solve {
            max_states,
            fast,
            preview,
            greenfelt,
            draw,
            file,
        } => {
            let mut board = if let Some(file) = file {
                let content = std::fs::read_to_string(file)?;
                Board::parse(&content).context("Failed to parse board")?
            } else if let Some(seed) = greenfelt {
                Board::new_from_seed(*seed)
            } else if !stdin().is_terminal() {
                let mut content = String::new();
                stdin()
                    .read_to_string(&mut content)
                    .context("Failed to read from stdin")?;
                Board::parse(&content).context("Failed to parse board")?
            } else {
                #[cfg(windows)]
                {
                    solitaire_solver::inspect::inspect()?
                }
                #[cfg(not(windows))]
                {
                    bail!("No game state `file` or `--greenfelt` provided.");
                }
            };
            if let Some(draw_count) = draw {
                if *draw_count != 1 && *draw_count != 3 {
                    bail!("Draw count must be 1 or 3.");
                }
                board.set_draw_count(*draw_count);
            }
            if *preview {
                println!("{}", board.to_pretty_string());
                return Ok(());
            }
            let actions = do_solve(board, *max_states, !fast)?;
            println!("{}", format_actions(&actions));
        }
        #[cfg(windows)]
        Commands::Autoplay {
            max_states,
            fast,
            interval,
        } => {
            let board = solitaire_solver::inspect::inspect()?;
            let actions = do_solve(board.clone(), *max_states, !fast)?;
            solitaire_solver::autoplay::autoplay(board, actions, *interval)
                .context("Failed to autoplay the game")?;
        }
    }

    Ok(())
}

fn do_solve(board: Board, max_states: u32, minimal: bool) -> Result<Vec<Action>> {
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
