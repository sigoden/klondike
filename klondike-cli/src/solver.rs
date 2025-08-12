mod utils;

use crate::utils::*;

use anyhow::{Context, Result, bail};
use clap::Parser;
use klondike_common::{action::format_actions, board::Board};

use std::{
    io::{IsTerminal, Read, stdin},
    path::PathBuf,
};

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
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
}

fn main() -> Result<()> {
    let Cli {
        max_states,
        fast,
        preview,
        greenfelt,
        draw,
        file,
    } = Cli::parse();

    let mut board = if let Some(file) = file {
        let content = std::fs::read_to_string(file)?;
        Board::parse(&content).context("Failed to parse board")?
    } else if let Some(seed) = greenfelt {
        Board::new_from_seed(seed)
    } else if !stdin().is_terminal() {
        let mut content = String::new();
        stdin()
            .read_to_string(&mut content)
            .context("Failed to read from stdin")?;
        Board::parse(&content).context("Failed to parse board")?
    } else {
        bail!("No game state `file` or `--greenfelt` provided.");
    };
    if let Some(draw_count) = draw {
        if draw_count != 1 && draw_count != 3 {
            bail!("Draw count must be 1 or 3.");
        }
        board.set_draw_count(draw_count);
    }
    if preview {
        println!("{}", board.to_pretty_string());
        return Ok(());
    }
    let actions = do_solve(board, max_states, !fast)?;
    println!("{}", format_actions(&actions));

    Ok(())
}
