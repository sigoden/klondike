use anyhow::Result;
use solitaire_solver::{
    action::format_actions,
    board::Board,
    solver::{SolveResult, solve},
};
use std::env;
use std::fs;
use std::time::Duration;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let board = if args.len() < 2 {
        #[cfg(windows)]
        {
            solitaire_solver::inspect::inspect()?
        }
        #[cfg(not(windows))]
        {
            anyhow::bail!("Usage: {} <board_file>", args[0])
        }
    } else {
        let board_str = fs::read_to_string(&args[1])?;
        Board::parse(&board_str)?
    };
    println!("=== Board ===\n{}\n", board.pretty_print());
    let SolveResult {
        actions,
        elapsed,
        states,
        minimal,
    } = solve(board, 50_000_000, true)?;

    let actions_len = actions.len();
    let elapsed = format_elapsed(elapsed);
    println!(
        r#"=== Result ===
Steps: {actions_len}, Elapsed: {elapsed}, States: {states}, Minimal: {minimal}, Actions: ↵
{}
    "#,
        format_actions(&actions)
    );

    Ok(())
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
