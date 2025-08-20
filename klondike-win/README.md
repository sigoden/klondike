# Klondike Win

This crate provides Windows-specific functionality for the Klondike Solitaire solver.

## Features

-   **Game State Inspection:** Inspect the memory of a running `Solitaire.exe` process to extract the current game state.
-   **Autoplay:** Simulate mouse clicks to automatically play a solved game in the `Solitaire.exe` window.

## Usage

This crate is intended to be used on Windows. It provides two main functions:

-   `inspect()`: Finds the `Solitaire.exe` process and returns a `Board` representing the current game state.
-   `autoplay(board, actions, interval)`: Takes a `Board`, a slice of `Action`s, and an interval, and then simulates mouse clicks to play the game.

```rust
use klondike_win::{inspect, autoplay};

// Inspect the running game
let board = inspect().unwrap();

// Solve the game (using klondike-solver)
let result = klondike_solver::solve(board.clone(), 100_000_000, true).unwrap();

// Autoplay the solution
autoplay(board, result.actions, 1000);
```
