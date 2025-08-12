#[cfg(windows)]
mod utils;

use clap::Parser;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// Max states to explore (~1 GB per 64 million states)
    #[arg(short, long, default_value_t = 100_000_000, value_name = "NUM")]
    max_states: u32,
    /// Stop at first found solution (may not be minimal)
    #[arg(short, long)]
    fast: bool,
    /// Play the game automatically
    #[arg(short, long)]
    play: bool,
    /// Delay between moves in milliseconds
    #[arg(short, long, default_value_t = 3000, value_name = "MS")]
    interval: u64,
}

#[cfg(windows)]
fn main() -> anyhow::Result<()> {
    let Cli {
        max_states,
        fast,
        play,
        interval,
    } = Cli::parse();
    let board = klondike_win::inspect()?;
    let actions = crate::utils::do_solve(board.clone(), max_states, !fast)?;
    if play {
        klondike_win::autoplay(board, actions, interval)?;
    } else {
        println!("{}", klondike_common::action::format_actions(&actions));
    }
    Ok(())
}

#[cfg(not(windows))]
fn main() -> anyhow::Result<()> {
    Cli::parse();
    anyhow::bail!("OS not supported");
}
