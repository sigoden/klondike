//! This crate provides utilities for solving Solitaire games using the A* search algorithm.
//!
/// Migrated from the https://github.com/ShootMe/MinimalKlondike/blob/8983a1375aa15c5ca7f8c3df054aef37218f85c8/Entities/Board.cs
mod card;
mod helper;
mod move_;
mod pile;
mod solver;

use crate::card::*;
use crate::helper::*;
use crate::move_::*;
use crate::pile::*;

pub use crate::solver::{SolveResult, Solver, solve};
