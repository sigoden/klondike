# Klondike

A Klondike (Solitaire) game, solver, and auto-player implemented in Rust.

## Features

-   **Playable GUI:** A graphical interface to play Klondike Solitaire.
-   **Blazing-Fast Solver:** An efficient A* search algorithm to find the optimal solution.
-   **Windows Auto-player:** Inspect a running `Solitaire.exe` game on Windows and autoplay the solution.
-   **Game Loading:** Load games from a file or a random seed compatible with [greenfelt.net](https://greenfelt.net/klondike).
-   **Solution Visualization:** Visualize the steps of a solution in the GUI.

## Crates

This repository is a workspace containing several crates:

-   `klondike-app`: A GUI application for playing and visualizing Klondike Solitaire.
-   `klondike-cli`: A command-line interface with two binaries:
    -   `klondike-solver`: A cross-platform solver.
    -   `klondike-win`: A Windows-only tool that can inspect a running game and autoplay the solution.
-   `klondike-common`: Common data structures and logic for the Klondike Solitaire game.
-   `klondike-solver`: A blazing-fast solver for Klondike Solitaire, using an A* search algorithm.
-   `klondike-win`: Windows-specific functionality for the Klondike Solitaire solver, including game state inspection and autoplay.

## Installation

### Pre-built Binaries

Download ready-to-use binaries for major platforms from the [GitHub Releases](https://github.com/sigoden/klondike/releases) page, extract, and add `klondike-*` binaries to your system's `PATH`.

### Build from source

1.  Install Rust: https://www.rust-lang.org/tools/install
2.  Build the project:
    ```sh
    cargo build --release
    ```
    The binaries will be located in the `target/release` directory.

## Usage

### `klondike-solver`

The `klondike-solver` is a cross-platform command-line tool to solve Klondike Solitaire.

![klondike-solver](https://github.com/user-attachments/assets/4fea9336-a17a-4b9e-a501-dece921038b0)

```sh
klondike-solver [OPTIONS] [FILE]
```

**Options:**

-   `--greenfelt <SEED>`: Game ID from greenfelt.net/klondike.
-   `--draw <NUM>`: Cards drawn per turn (1 or 3).
-   `--max-states <NUM>`: Max states to explore.
-   `--fast`: Stop at first found solution.
-   `--preview`: Preview initial game state without solving.
-   `FILE`: Path to a game state file to solve.

### `klondike-app`

The `klondike-app` provides a GUI for playing and visualizing Klondike Solitaire.

![klondike-app](https://github.com/user-attachments/assets/b87e8374-bcdd-4a8b-985b-da421f4fa0db)

```sh
# Run the application
klondike-app

# Change cards drawn per turn to 3
klondike-app --draw 3

# Load a game from random seed that compatible with greenfelt.net/klondike?game=283409412
klondike-app --greenfelt 283409412

# Load a game from a file
klondike-app game.txt

# Solve the game and visualize the solution
klondike-solver --greenfelt 283409412 | klondike-app
```

### `klondike-win`

The `klondike-win` is a Windows-only tool to inspect and autoplay a running `Solitaire.exe` game.

![klondike-win](https://github.com/user-attachments/assets/2239f0ce-0f73-41a0-9b5b-51ed34e54147)

```sh
klondike-win [OPTIONS]
```

**Options:**

-   `--max-states <NUM>`: Max states to explore.
-   `--fast`: Stop at first found solution.
-   `--play`: Play the game automatically.
-   `--interval <MS>`: Delay between moves in milliseconds.

## License

This project is licensed under the MIT License. See the [LICENSE-MIT](LICENSE-MIT) file for details.
