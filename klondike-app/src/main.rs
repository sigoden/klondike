#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod common;

use crate::common::Board;
use crate::{
    app::KlondikeApp,
    common::{SolutionMove, parse_moves},
};

use anyhow::Context;
use clap::Parser;
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
    #[arg(short, long, value_name = "NUM", default_value_t = 1)]
    draw: usize,
    /// Path to a game state file to load
    file: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let seed = cli.greenfelt.unwrap_or(rand::random());
    let draw_count = cli.draw;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([710.0, 775.0]),
        ..Default::default()
    };

    let (board, solution) = match cli.file {
        Some(path) => {
            let content = std::fs::read_to_string(path)?;
            parse(content)?
        }
        None => {
            if !stdin().is_terminal() {
                let mut content = String::new();
                stdin()
                    .read_to_string(&mut content)
                    .context("Failed to read from stdin")?;
                parse(content)?
            } else {
                (Board::new(seed, draw_count), None)
            }
        }
    };
    let mut app = KlondikeApp::new(board);

    if let Some(moves) = solution {
        app.solve(moves);
    }

    eframe::run_native(
        "Klondike Solitaire",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run app; {e}"))?;

    Ok(())
}

fn parse(content: String) -> anyhow::Result<(Board, Option<Vec<SolutionMove>>)> {
    let (board_str, moves_str) = if let Some(idx) = content.find("✓ Solved in") {
        let (board_part, rest) = content.split_at(idx);
        let moves_part = rest.lines().skip(2).collect::<Vec<_>>().join(" ");
        (board_part, Some(moves_part))
    } else {
        (content.as_str(), None)
    };
    let board = Board::parse(board_str).context("Failed to parse board")?;
    let moves = if let Some(s) = moves_str {
        Some(parse_moves(&s).context("Failed to parse moves")?)
    } else {
        None
    };
    Ok((board, moves))
}
