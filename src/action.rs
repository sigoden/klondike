use crate::board::{Board, Card};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Action {
    WasteToFoundation(usize),
    WasteToTableau(usize),
    TableauToFoundation(usize, usize),
    FoundationToTableau(usize, usize),
    TableauToTableau(usize, usize, usize), // (from_index, to_index, count)
    Draw,
    Redeal,
}

impl Action {
    pub fn is_redeal(&self) -> bool {
        matches!(self, Action::Redeal)
    }
}

pub fn format_actions(actions: &[Action]) -> String {
    let mut list = vec![];
    let mut i = 0;
    while i < actions.len() {
        match actions[i] {
            Action::Draw => {
                let mut count = 1;
                while i + count < actions.len() && matches!(actions[i + count], Action::Draw) {
                    count += 1;
                }
                let str = if count == 1 {
                    "D".into()
                } else {
                    format!("{count}D")
                };
                list.push(str);
                i += count;
                continue;
            }
            Action::WasteToFoundation(idx) => {
                list.push(format!("W:F{}", idx + 1));
            }
            Action::WasteToTableau(idx) => {
                list.push(format!("W:T{}", idx + 1));
            }
            Action::TableauToFoundation(from_idx, to_idx) => {
                list.push(format!("T{}:F{}", from_idx + 1, to_idx + 1));
            }
            Action::FoundationToTableau(from_idx, to_idx) => {
                list.push(format!("F{}:T{}", from_idx + 1, to_idx + 1));
            }
            Action::TableauToTableau(from_idx, to_idx, count) => {
                let mut str = format!("T{}:T{}", from_idx + 1, to_idx + 1);
                if count > 1 {
                    str.push_str(&format!("@{count}"));
                };
                list.push(str);
            }
            Action::Redeal => {
                list.push("R".into());
            }
        }
        i += 1;
    }

    let mut output = String::new();
    let max_width = list.iter().map(|s| s.len()).max().unwrap_or_default() + 1;
    for chunk in list.chunks(10) {
        for cmd in chunk {
            output.push_str(&format!("{cmd:<width$}", width = max_width));
        }
        output.push('\n');
    }

    output
}

pub fn apply_action(board: &mut Board, action: &Action) {
    match action {
        Action::WasteToFoundation(foundation_index) => {
            board.move_waste_to_foundation(*foundation_index);
        }
        Action::WasteToTableau(tableau_index) => {
            board.move_waste_to_tableau(*tableau_index);
        }
        Action::TableauToFoundation(tableau_index, foundation_index) => {
            board.move_tableau_to_foundation(*tableau_index, *foundation_index);
        }
        Action::FoundationToTableau(foundation_index, tableau_index) => {
            board.move_foundation_to_tableau(*foundation_index, *tableau_index);
        }
        Action::TableauToTableau(from_index, to_index, count) => {
            board.move_tableau_to_tableau(*from_index, *to_index, *count);
        }
        Action::Draw | Action::Redeal => {
            board.draw();
        }
    }
}

pub fn describe_action(board: &Board, action: &Action) -> String {
    let format_card =
        |card: Option<&Card>| -> String { card.map(|c| c.to_pretty_string()).unwrap_or_default() };

    match action {
        Action::WasteToFoundation(foundation_index) => {
            let from_card = format_card(board.waste.peek_top());
            let to_card = format_card(board.foundations[*foundation_index].as_ref());
            format!(
                "(Waste) {from_card} -> (Foundation{}) {to_card}",
                foundation_index + 1
            )
        }
        Action::WasteToTableau(tableau_index) => {
            let from_card = format_card(board.waste.peek_top());
            let to_card = format_card(board.tableaus[*tableau_index].peek_top());
            format!(
                "(Waste) {from_card} -> (Tableau{}) {to_card}",
                tableau_index + 1
            )
        }
        Action::TableauToFoundation(tableau_index, foundation_index) => {
            let from_card = format_card(board.tableaus[*tableau_index].peek_top());
            let to_card = format_card(board.foundations[*foundation_index].as_ref());
            format!(
                "(Tableau{}) {from_card} -> (Foundation{}) {to_card}",
                tableau_index + 1,
                foundation_index + 1
            )
        }
        Action::FoundationToTableau(foundation_index, tableau_index) => {
            let from_card = format_card(board.foundations[*foundation_index].as_ref());
            let to_card = format_card(board.tableaus[*tableau_index].peek_top());

            format!(
                "(Foundation{}) {from_card} -> (Tableau{}) {to_card}",
                foundation_index + 1,
                tableau_index + 1
            )
        }
        Action::TableauToTableau(from_index, to_index, count) => {
            let from_tableau_cards = &board.tableaus[*from_index].cards;
            let from_cards = from_tableau_cards
                .iter()
                .skip(from_tableau_cards.len() - count)
                .map(|c| c.to_pretty_string())
                .collect::<Vec<_>>()
                .join("");
            let to_card = format_card(board.tableaus[*to_index].peek_top());
            format!(
                "(Tableau{}) {from_cards} -> (Tableau{}) {to_card}",
                from_index + 1,
                to_index + 1
            )
        }
        Action::Draw => {
            let mut board = board.clone();
            board.draw();
            let card = format_card(board.waste.peek_top());
            format!("Draw {card}",)
        }
        Action::Redeal => "Redeal".to_string(),
    }
}
