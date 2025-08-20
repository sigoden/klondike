# Klondike Common

This crate provides common data structures and logic for the Klondike Solitaire game.

## Core Data Structures

-   `Card`: Represents a playing card with a suit and rank.
-   `Tableau`: Represents one of the seven tableau piles.
-   `Board`: Represents the entire game state, including the stock, waste, foundations, and tableaus.
-   `Action`: Represents a possible move in the game, like moving a card between piles.

## Functionality

-   **Game State Management:** The `Board` struct can be used to manage the state of a Klondike game.
-   **Parsing:** Parse a game state from a string representation.
-   **Seeding:** Generate a new game from a seed, compatible with [greenfelt.net](https://greenfelt.net/klondike).
-   **Action Formatting:** Format a sequence of actions into a human-readable string.
