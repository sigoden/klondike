# Klondike App

This crate provides a GUI application for playing and visualizing Klondike Solitaire.

![klondike-app](https://github.com/user-attachments/assets/b87e8374-bcdd-4a8b-985b-da421f4fa0db)

## Features

-   **Graphical Interface:** A user-friendly interface for playing Klondike Solitaire, built with `egui`.
-   **Game Loading:** Load games from:
    -   A file.
    -   A randome seed, compatible with [greenfelt.net](https://greenfelt.net/klondike).
-   **Solution Visualization:** If a solution is provided, the application can visualize the steps.

## Usage

To run the application:

```sh
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
