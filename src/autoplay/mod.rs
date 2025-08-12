//! This module provides functionality for autoplaying the Solitaire game using mouse movements and clicks.

mod window;

use self::window::*;

use crate::{
    action::{Action, apply_action, describe_action},
    board::Board,
    inspect::get_pid,
};

use anyhow::{Context, Result, anyhow, bail};
use enigo::{Button, Coordinate, Direction, Enigo, Mouse, Settings, set_dpi_awareness};
use std::{thread::sleep, time::Duration};

pub fn autoplay(mut board: Board, actions: Vec<Action>, interval: u64) -> Result<()> {
    let (window_rect, hwnd) = get_window_rect(get_pid()?)?;
    let window = Window::new(window_rect);
    let interval = interval.max(500);

    let mut enigo = Enigo::new(&Settings::default()).context("Failed to init enigo")?;
    set_dpi_awareness().map_err(|_| anyhow!("Failed to set DPI awareness"))?;

    focus_window(hwnd)?;
    sleep(Duration::from_millis(100));

    let actions_count = actions.len();
    for (index, action) in actions.iter().enumerate() {
        sleep(Duration::from_millis(interval));
        if !is_foreground_window(hwnd) {
            bail!("Abort due to lost focus on the game window");
        }
        println!(
            "{:03}/{actions_count:03} {}",
            index + 1,
            describe_action(&board, action)
        );
        play_action(&board, action, &mut enigo, &window)?;
        apply_action(&mut board, action);
    }
    Ok(())
}

fn play_action(
    board: &Board,
    action: &Action,
    enigo: &mut impl Mouse,
    window: &Window,
) -> Result<()> {
    match action {
        Action::WasteToFoundation(foundation_index) => {
            mouse_move(
                enigo,
                window.waste_point(),
                window.foundation_point(*foundation_index),
            )?;
        }
        Action::WasteToTableau(tableau_index) => {
            let tableau = &board.tableaus[*tableau_index];
            mouse_move(
                enigo,
                window.waste_point(),
                window.move_to_tableau_point(
                    *tableau_index,
                    tableau.cards.len(),
                    tableau.face_up_count,
                ),
            )?;
        }
        Action::TableauToFoundation(tableau_index, foundation_index) => {
            let tableau = &board.tableaus[*tableau_index];
            let cards_count = tableau.cards.len();
            mouse_move(
                enigo,
                window.move_from_tableau_point(
                    *tableau_index,
                    cards_count,
                    tableau.face_up_count,
                    1,
                ),
                window.foundation_point(*foundation_index),
            )?;
        }
        Action::FoundationToTableau(foundation_index, tableau_index) => {
            let tableau = &board.tableaus[*tableau_index];
            mouse_move(
                enigo,
                window.foundation_point(*foundation_index),
                window.move_to_tableau_point(
                    *tableau_index,
                    tableau.cards.len(),
                    tableau.face_up_count,
                ),
            )?;
        }
        Action::TableauToTableau(from_index, to_index, moved_count) => {
            let from_tableau = &board.tableaus[*from_index];
            let to_tableau = &board.tableaus[*to_index];
            mouse_move(
                enigo,
                window.move_from_tableau_point(
                    *from_index,
                    from_tableau.cards.len(),
                    from_tableau.face_up_count,
                    *moved_count,
                ),
                window.move_to_tableau_point(
                    *to_index,
                    to_tableau.cards.len(),
                    to_tableau.face_up_count,
                ),
            )?;
        }
        Action::Draw | Action::Redeal => {
            mouse_click(enigo, window.stock_point())?;
        }
    }
    Ok(())
}

fn mouse_click(enigo: &mut impl Mouse, point: Point) -> Result<()> {
    enigo.move_mouse(point.0, point.1, Coordinate::Abs)?;
    sleep(Duration::from_millis(50));
    enigo.button(Button::Left, Direction::Click)?;
    Ok(())
}

fn mouse_move(enigo: &mut impl Mouse, from_point: (i32, i32), to_point: (i32, i32)) -> Result<()> {
    let (from_x, from_y) = from_point;
    let (to_x, to_y) = to_point;

    enigo.move_mouse(from_x, from_y, Coordinate::Abs)?;
    enigo.button(Button::Left, Direction::Press)?;

    sleep(Duration::from_millis(50));

    let steps = 30;
    let dx = (to_x - from_x) as f32 / steps as f32;
    let dy = (to_y - from_y) as f32 / steps as f32;
    for i in 1..=steps {
        let x = from_x as f32 + dx * i as f32;
        let y = from_y as f32 + dy * i as f32;
        enigo.move_mouse(x as i32, y as i32, Coordinate::Abs)?;
        sleep(Duration::from_millis(15));
    }

    enigo.button(Button::Left, Direction::Release)?;
    Ok(())
}
