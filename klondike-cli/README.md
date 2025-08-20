# Klondike CLI

This crate provides a command-line interface for the Klondike Solitaire solver.

It has two binaries:
-   `klondike-solver`: A blazing-fast cross-platform klondike solver.
-   `klondike-win`: A Windows-only tool that can inspect a running game and autoplay the solution.

## `klondike-solver`

A cross-platform command-line tool to solve Klondike Solitaire.

### Usage

```sh
klondike-solver [OPTIONS] [FILE]
```

![klondike-solver](https://github.com/user-attachments/assets/4fea9336-a17a-4b9e-a501-dece921038b0)

### Options

-   `--greenfelt <SEED>`: Game ID from greenfelt.net/klondike.
-   `--draw <NUM>`: Cards drawn per turn (1 or 3).
-   `--max-states <NUM>`: Max states to explore.
-   `--fast`: Stop at first found solution.
-   `--preview`: Preview initial game state without solving.
-   `FILE`: Path to a game state file to solve.

## `klondike-win`

A Windows-only tool to inspect and autoplay a running `Solitaire.exe` game.

### Usage

```sh
klondike-win [OPTIONS]
```

![klondike-win](https://github.com/user-attachments/assets/2239f0ce-0f73-41a0-9b5b-51ed34e54147)

### Options

-   `--max-states <NUM>`: Max states to explore.
-   `--fast`: Stop at first found solution.
-   `--play`: Play the game automatically.
-   `--interval <MS>`: Delay between moves in milliseconds.

> `klondike-win` is designed to work with the [classic version of Solitaire](https://win7games.com/#games) for Windows. If you use a different Solitaire version or edition, the tool may fail to read the game state correctly, as the memory layout and offsets can vary between versions.
