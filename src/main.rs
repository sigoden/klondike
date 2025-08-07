use anyhow::{Result, anyhow, bail};
use clap::{Parser, Subcommand};
use solitaire_solver::{
    action::{Action, format_actions},
    board::Board,
    solver::{SolveResult, solve},
};
use std::{
    io::{IsTerminal, Write, stderr},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

const INSPECT_EXAMPLES: &str = r#"Examples:
  # Extract game state from memory of Solitaire.exe (Windows only)
  solitaire-solver inspect

  # View https://greenfelt.net/klondike?game=670334786
  solitaire-solver inspect -g 670334786

  # View https://greenfelt.net/klondike3?game=670334786
  solitaire-solver inspect -g 670334786 -d 3
"#;

const SOLVE_EXAMPLES: &str = r#"Examples:
  # Solve the current running Solitaire.exe game (Windows only)
  solitaire-solver solve

  # Release the limit to 500 million states (about 8 GB of memory)
  solitaire-solver solve -s 500000000

  # Solve https://greenfelt.net/klondike?game=670334786
  solitaire-solver solve -g 670334786

  # Solve https://greenfelt.net/klondike3?game=670334786
  solitaire-solver solve -g 670334786 -d 3
"#;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Inspect the game state
    #[command(after_help = INSPECT_EXAMPLES)]
    Inspect {
        /// Game ID from greenfelt.net/klondike
        #[arg(short = 'g', long, value_name = "SEED")]
        greenfelt: Option<u32>,
        /// Cards drawn per turn (1 or 3)
        #[arg(short = 'd', long, value_name = "NUM")]
        draw_count: Option<usize>,
        /// Write game state to a file instead of stdout
        #[arg(short = 'o', long, value_name = "FILE")]
        output: Option<PathBuf>,
    },
    /// Solve the game
    #[command(after_help = SOLVE_EXAMPLES)]
    Solve {
        /// Maximum number of states to explore (~1 GB per 64 million states)
        #[arg(short = 's', long, default_value_t = 100_000_000, value_name = "NUM")]
        max_states: u32,
        /// Return early when any solution is found (not necessarily minimal)
        #[arg(short = 'f', long)]
        fast: bool,
        /// Game ID from greenfelt.net/klondike
        #[arg(short = 'g', long, value_name = "SEED")]
        greenfelt: Option<u32>,
        /// Cards drawn per turn (1 or 3)
        #[arg(short = 'd', long, value_name = "NUM")]
        draw_count: Option<usize>,
        /// Path to a saved game state file, typically generated via `inspect`
        file: Option<PathBuf>,
    },
    /// Automatically play the game
    #[cfg(windows)]
    Autoplay {
        /// Maximum number of states to explore (~1 GB per 64 million states)
        #[arg(short = 's', long, default_value_t = 100_000_000, value_name = "NUM")]
        max_states: u32,
        /// Return early when any solution is found (not necessarily minimal)
        #[arg(short = 'f', long)]
        fast: bool,
        /// Delay between moves in milliseconds
        #[arg(short, long, default_value_t = 3000, value_name = "MILLIS")]
        interval: u64,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Inspect {
            greenfelt,
            draw_count,
            output,
        } => {
            let mut board = if let Some(seed) = greenfelt {
                Board::new_from_seed(*seed)
            } else {
                #[cfg(windows)]
                {
                    solitaire_solver::inspect::inspect()?
                }
                #[cfg(not(windows))]
                {
                    bail!("`--greenfelt` seed must be provided.");
                }
            };
            if let Some(draw_count) = draw_count {
                if *draw_count != 1 && *draw_count != 3 {
                    bail!("Draw count must be 1 or 3.");
                }
                board.set_draw_count(*draw_count);
            }
            if let Some(file) = output {
                std::fs::write(file, board.pretty_print())
                    .map_err(|err| anyhow!("Failed to write board to file; {err}"))?;
                println!("Game state written to '{}'", file.display());
            } else {
                println!("{}", board.pretty_print());
            }
        }
        Commands::Solve {
            max_states,
            fast,
            greenfelt,
            draw_count,
            file,
        } => {
            let mut board = if let Some(file) = file {
                let content = std::fs::read_to_string(file)?;
                Board::parse(&content).map_err(|err| anyhow!("Failed to parse board; {err}"))?
            } else if let Some(seed) = greenfelt {
                Board::new_from_seed(*seed)
            } else {
                #[cfg(windows)]
                {
                    solitaire_solver::inspect::inspect()?
                }
                #[cfg(not(windows))]
                {
                    bail!("Either a game state `file` or `--greenfelt` seed must be provided.");
                }
            };
            if let Some(draw_count) = draw_count {
                if *draw_count != 1 && *draw_count != 3 {
                    bail!("Draw count must be 1 or 3.");
                }
                board.set_draw_count(*draw_count);
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
                .map_err(|err| anyhow!("Autoplay failed; {err}"))?;
        }
    }

    Ok(())
}

fn do_solve(board: Board, max_states: u32, minimal: bool) -> Result<Vec<Action>> {
    let board_str = board.pretty_print();
    println!("{board_str}\n");
    let SolveResult {
        actions,
        elapsed,
        states,
        minimal,
    } = with_spinner("Solving the game...", move || {
        solve(board, max_states, minimal)
    })?;
    let actions_count = actions.len();
    let redeal_count = actions.iter().filter(|a| a.is_redeal()).count();
    let elapsed_str = format_elapsed(elapsed);
    let mut steps_str = format!("{actions_count} Moves");
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
