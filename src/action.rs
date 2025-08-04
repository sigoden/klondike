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
    list.join(" ")
}
