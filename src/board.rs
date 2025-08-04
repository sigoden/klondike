use anyhow::{Context, Result};
use smallvec::SmallVec;

const SUITS: [char; 5] = ['?', '♣', '♦', '♠', '♥'];
const RANKS: [char; 14] = [
    '?', 'A', '2', '3', '4', '5', '6', '7', '8', '9', 'T', 'J', 'Q', 'K',
];
const TOTAL_FOUNDATIONS: usize = 4;
const TOTAL_TABLEAUS: usize = 7;
const TALON_SIZE: usize = 24;
const TABLEAU_SIZE: usize = 19;

#[derive(Debug, Clone, Default)]
pub struct Board {
    pub stock: SmallVec<[Card; TALON_SIZE]>,
    pub waste: WastePile,
    pub foundations: [Card; TOTAL_FOUNDATIONS],
    pub tableaus: [Tableau; TOTAL_TABLEAUS],
    pub draw_count: Option<usize>,
}

impl Board {
    pub fn draw_count(&self) -> usize {
        self.draw_count.unwrap_or(1)
    }

    pub fn parse(content: &str) -> Result<Self> {
        let mut board: Self = Default::default();

        for line in content
            .split('\n')
            .map(|v| v.trim())
            .filter(|l| !l.is_empty())
        {
            let line_context = || format!("Invalid line {line}");
            if let Some(rest) = line.strip_prefix("Stock:") {
                for card in Self::parse_cards(rest.trim()).with_context(line_context)? {
                    board.stock.push(card);
                }
            } else if let Some(rest) = line.strip_prefix("Waste:") {
                let (before, after) = if let Some(idx) = rest.find('|') {
                    let (b, a) = rest.split_at(idx);
                    (b, &a[1..])
                } else {
                    (rest, "")
                };
                let cards = Self::parse_cards(before.trim()).with_context(line_context)?;
                let visible_cards = Self::parse_cards(after.trim()).with_context(line_context)?;
                board.waste.visible_count = visible_cards.len();
                for c in [cards, visible_cards].concat() {
                    board.waste.cards.push(c);
                }
            } else if let Some(rest) = line.strip_prefix("Foundation") {
                let mut parts = rest.splitn(2, ':');
                let idx = parts
                    .next()
                    .unwrap_or("")
                    .trim()
                    .parse::<usize>()
                    .context("Invalid foundation index")
                    .with_context(line_context)?;
                let cards = Self::parse_cards(parts.next().unwrap_or("").trim())
                    .with_context(line_context)?;
                board.foundations[idx] = cards.last().cloned().unwrap_or(Card::EMPTY);
            } else if let Some(rest) = line.strip_prefix("Tableau") {
                let mut parts = rest.splitn(2, ':');
                let idx = parts
                    .next()
                    .unwrap_or("")
                    .trim()
                    .parse::<usize>()
                    .context("Invalid tableau index")
                    .with_context(line_context)?;
                let cards_str = parts.next().unwrap_or("").trim();
                let (before, after) = if let Some(idx) = cards_str.find('|') {
                    let (b, a) = cards_str.split_at(idx);
                    (b, &a[1..])
                } else {
                    (cards_str, "")
                };
                let cards = Self::parse_cards(before.trim()).with_context(line_context)?;
                let face_up_cards = Self::parse_cards(after.trim()).with_context(line_context)?;
                board.tableaus[idx].face_up_count = face_up_cards.len();
                for c in [cards, face_up_cards].concat() {
                    board.tableaus[idx].cards.push(c);
                }
            } else if let Some(rest) = line.strip_prefix("DrawCount:") {
                board.draw_count = Some(
                    rest.trim()
                        .parse::<usize>()
                        .context("Invalid DrawCount")
                        .with_context(line_context)?,
                );
            }
        }

        Ok(board)
    }

    fn parse_cards(s: &str) -> Result<Vec<Card>> {
        let mut cards = Vec::new();
        let mut chars = s.chars().peekable();
        while let Some(&c1) = chars.peek() {
            if c1.is_whitespace() || c1 == '|' {
                chars.next();
                continue;
            }
            let rank = c1;
            chars.next();
            let suit = match chars.next() {
                Some(s) => s,
                None => break,
            };
            cards.push(Card::parse(rank, suit)?);
        }
        Ok(cards)
    }

    pub fn pretty_print(&self) -> String {
        let mut output = String::new();

        // Stock
        if !self.stock.is_empty() {
            output.push_str("Stock: ");
            for card in &self.stock {
                output.push_str(&card.pretty_print());
            }
            output.push('\n');
        }

        // Waste
        if !self.waste.is_empty() {
            output.push_str("Waste: ");
            let waste_len = self.waste.cards.len();
            let vis = self.waste.visible_count.min(waste_len);
            let sep = waste_len.saturating_sub(vis);
            for (i, card) in self.waste.cards.iter().enumerate() {
                if i == sep && vis > 0 {
                    output.push('|');
                }
                output.push_str(&card.pretty_print());
            }
            output.push('\n');
        }

        // Foundations
        for (i, card) in self.foundations.iter().enumerate() {
            if !card.is_empty() {
                output.push_str(&format!("Foundation{i}: {}\n", card.pretty_print()));
            }
        }

        // Tableaus
        for (i, tableau) in self.tableaus.iter().enumerate() {
            if tableau.is_empty() {
                continue;
            }
            output.push_str(&format!("Tableau{i}: "));
            let len = tableau.cards.len();
            let face_up = tableau.face_up_count.min(len);
            let sep = len.saturating_sub(face_up);
            for (j, card) in tableau.cards.iter().enumerate() {
                if j == sep && face_up > 0 {
                    output.push('|');
                }
                output.push_str(&card.pretty_print());
            }
            output.push('\n');
        }

        // DrawCount
        output.push_str(&format!("DrawCount: {}\n", self.draw_count()));

        output
    }
}

#[derive(Debug, Clone, Default)]
pub struct WastePile {
    pub cards: SmallVec<[Card; TALON_SIZE]>,
    pub visible_count: usize,
}

impl WastePile {
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Tableau {
    pub cards: SmallVec<[Card; TABLEAU_SIZE]>,
    pub face_up_count: usize,
}

impl Tableau {
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Card(u8);

impl Card {
    pub const EMPTY: Card = Card(0);

    pub fn new(rank: u8, suit: u8) -> Self {
        assert!(rank < 14, "Rank must be less than 14");
        assert!(suit < 5, "Suit must be less than 5");
        Self((suit << 4) | rank)
    }

    pub fn parse(rank: char, suit: char) -> Result<Self> {
        let rank = RANKS
            .iter()
            .position(|&r| r == rank)
            .with_context(|| format!("Invalid rank of card {rank}{suit}"))?;
        let suit = SUITS
            .iter()
            .position(|&s| s == suit)
            .with_context(|| format!("Invalid suit of card {rank}{suit}"))?;
        Ok(Card::new(rank as u8, suit as u8))
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0 == Self::EMPTY.0
    }

    #[inline]
    pub fn reset(&mut self) {
        self.0 = Self::EMPTY.0;
    }

    #[inline]
    pub fn rank(&self) -> u8 {
        self.0 & 0x0F
    }

    #[inline]
    pub fn suit(&self) -> u8 {
        self.0 >> 4
    }

    pub fn pretty_print(&self) -> String {
        format!(
            "{}{}",
            RANKS[self.rank() as usize],
            SUITS[self.suit() as usize]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_board() {
        const TEST_DATA: &str = r#"Stock: 5♦2♥8♦K♣7♥J♣
Waste: 7♦Q♥K♥T♦6♣9♥K♦J♠T♣Q♣3♣2♦Q♦8♥6♥|7♠8♠
Foundation0: A♠
Foundation2: 2♣
Tableau0: |5♣
Tableau1: J♥|6♠
Tableau2: T♠5♥|Q♠
Tableau3: 9♠T♥2♠|9♣
Tableau4: 7♣4♥3♠|A♦
Tableau5: 3♥3♦4♣5♠4♦|8♣
Tableau6: 6♦4♠A♥9♦K♠|J♦
DrawCount: 3
"#;

        let board = Board::parse(TEST_DATA).unwrap();
        assert_eq!(TEST_DATA, board.pretty_print());
    }
}
