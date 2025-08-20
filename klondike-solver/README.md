# Klondike Solver

This crate provides a blazing-fast solver for Klondike Solitaire, using an A* search algorithm to find the optimal solution.

## Features

-   **A* Search Algorithm:** Efficiently finds the shortest sequence of moves to solve a game.
-   **Configurable:** The solver can be configured to prioritize speed vs. optimality.
-   **Game State Analysis:** The solver can determine if a game is lost.

## Usage

The main entry point is the `solve` function, which takes a `Board` from the `klondike-common` crate and returns a `SolveResult`.

```rust
use klondike_common::board::Board;
use klondike_solver::{solve, SolveResult};

let board = Board::new_from_seed(12345);
let SolveResult {
    actions,
    elapsed,
    states,
    minimal,
} = solve(board, 100_000_000, true).unwrap();
```
