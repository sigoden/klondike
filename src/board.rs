use anyhow::{Context, Result};
use smallvec::SmallVec;

pub const TOTAL_FOUNDATIONS: usize = 4;
pub const TOTAL_TABLEAUS: usize = 7;
pub const TALON_SIZE: usize = 24;
pub const MAX_RANK: u8 = 13;
pub const MAX_SUIT: u8 = 4;
pub const MAX_CARD: u8 = MAX_SUIT * MAX_RANK;

const SUITS: [char; 5] = ['♣', '♦', '♠', '♥', '?'];
const RANKS: [char; 14] = [
    'A', '2', '3', '4', '5', '6', '7', '8', '9', 'T', 'J', 'Q', 'K', '?',
];
const TABLEAU_SIZE: usize = 19;

#[derive(Debug, Clone, Default)]
pub struct Board {
    pub stock: SmallVec<[Card; TALON_SIZE]>,
    pub waste: WastePile,
    pub foundations: [Option<Card>; TOTAL_FOUNDATIONS],
    pub tableaus: [Tableau; TOTAL_TABLEAUS],
    draw_count: usize,
}

impl Board {
    pub fn new() -> Self {
        Self {
            draw_count: 1,
            ..Default::default()
        }
    }

    pub fn new_from_seed(seed: u32) -> Self {
        let mut current_seed = seed;
        let mut rnd = || {
            current_seed = ((current_seed as u64 * 16807) % 0x7fffffff) as u32;
            current_seed
        };
        let mut deck = [Card::UNKNOWN; 52];
        for (i, id) in (0..26).chain(39..52).chain(26..39).enumerate() {
            deck[i] = Card::new_with_id(id as u8);
        }

        for _ in 0..7 {
            for j in 0..52 {
                let k = (rnd() % 52) as usize;
                deck.swap(j, k);
            }
        }

        deck.rotate_left(24);

        let mut orig: i32 = 27;
        for i in 0..7_i32 {
            let mut pos = (i + 1) * (i + 2) / 2 - 1;
            for j in (0..=(6 - i)).rev() {
                if j >= i {
                    deck.swap(pos as usize, orig as usize);
                }
                orig -= 1;
                pos += 6 - j + 1;
            }
        }

        let mut board = Board::new();

        let mut m = 0;
        for j in 1..=TOTAL_TABLEAUS {
            let tableau_idx = j - 1;
            for _ in 0..j {
                board.tableaus[tableau_idx].cards.push(deck[m]);
                m += 1;
            }
            board.tableaus[tableau_idx].face_up_count = 1;
        }

        board.stock.extend_from_slice(&deck[m..]);

        board
    }

    pub fn draw_count(&self) -> usize {
        if self.draw_count == 3 { 3 } else { 1 }
    }

    pub fn set_draw_count(&mut self, value: usize) {
        self.draw_count = value;
        self.waste.visible_count = self.waste.visible_count.min(value);
    }

    pub fn foundation_score(&self) -> u8 {
        self.foundations
            .iter()
            .map(|card| match card {
                Some(card) => card.rank() + 1,
                None => 0,
            })
            .sum()
    }

    pub fn is_valid(&self) -> bool {
        let draw_count = self.draw_count();
        if draw_count != 1 && draw_count != 3 {
            return false;
        }

        let mut seen = [false; MAX_CARD as usize];
        let mut count = 0;
        let mut check_cards = |cards: &[Card]| -> bool {
            for &card in cards {
                if card.is_unknown() {
                    return false;
                }
                let id = card.id() as usize;
                if seen[id] {
                    return false;
                }
                seen[id] = true;
                count += 1;
            }
            true
        };

        if !check_cards(&self.stock) {
            return false;
        }
        if !check_cards(&self.waste.cards) {
            return false;
        }
        for &card in &self.foundations {
            let Some(card) = card else {
                continue;
            };
            let cards: Vec<_> = (0..=card.rank())
                .map(|r| Card::new_with_rank_suit(r, card.suit()))
                .collect();
            if !check_cards(&cards) {
                return false;
            }
        }
        for tableau in &self.tableaus {
            if !check_cards(&tableau.cards) {
                return false;
            }
        }
        count == MAX_CARD as usize
    }

    pub fn need_redeal(&self) -> bool {
        self.stock.is_empty() && !self.waste.is_empty()
    }

    pub fn draw(&mut self) {
        let stock_len = self.stock.len();
        if stock_len == 0 {
            if !self.waste.is_empty() {
                self.stock.extend(self.waste.cards.drain(..).rev());
                self.waste.visible_count = 0;
            }
        } else {
            let draw_count = self.draw_count();
            let num = draw_count.min(stock_len);
            let iter = self.stock.drain(self.stock.len() - num..).rev();
            self.waste.cards.extend(iter);
            self.waste.visible_count = num.max(1);
        }
    }

    pub fn move_waste_to_foundation(&mut self, idx: usize) {
        let card = self.waste.pop_unchecked();
        self.foundations[idx] = Some(card);
    }

    pub fn move_waste_to_tableau(&mut self, idx: usize) {
        let card = self.waste.pop_unchecked();
        self.tableaus[idx].push(card);
    }

    pub fn move_tableau_to_foundation(&mut self, tableau_idx: usize, foundation_idx: usize) {
        let card = self.tableaus[tableau_idx].pop_unchecked();
        self.foundations[foundation_idx] = Some(card);
    }

    pub fn move_tableau_to_tableau(&mut self, from_idx: usize, to_idx: usize, count: usize) {
        let cards = self.tableaus[from_idx].drain_unchecked(count);
        self.tableaus[to_idx].face_up_count += cards.len();
        self.tableaus[to_idx].cards.extend(cards);
    }

    pub fn move_foundation_to_tableau(&mut self, foundation_idx: usize, tableau_idx: usize) {
        let card = self.foundations[foundation_idx].expect("Foundation must have a card");
        let rank = card.rank();
        self.foundations[foundation_idx] = match rank {
            0 => None,
            _ => Some(Card::new_with_rank_suit(rank - 1, card.suit())),
        };
        self.tableaus[tableau_idx].push(card);
    }

    pub fn copy_from(&mut self, other: &Self) {
        self.stock.clone_from(&other.stock);
        self.waste.clone_from(&other.waste);
        for (dst, src) in self.foundations.iter_mut().zip(other.foundations.iter()) {
            *dst = *src;
        }
        for (dst, src) in self.tableaus.iter_mut().zip(other.tableaus.iter()) {
            dst.clone_from(src);
        }
        self.draw_count = other.draw_count;
    }

    pub fn parse(content: &str) -> Result<Self> {
        let mut board: Self = Default::default();

        for line in content
            .split('\n')
            .map(|v| v.trim())
            .filter(|l| !l.is_empty())
        {
            let line_context = || format!("Failed to parse at '{line}'");
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
                let idx = idx - 1;
                let cards = Self::parse_cards(parts.next().unwrap_or("").trim())
                    .with_context(line_context)?;
                board.foundations[idx] = cards.last().cloned();
            } else if let Some(rest) = line.strip_prefix("Tableau") {
                let mut parts = rest.splitn(2, ':');
                let idx = parts
                    .next()
                    .unwrap_or("")
                    .trim()
                    .parse::<usize>()
                    .context("Invalid tableau index")
                    .with_context(line_context)?;
                let idx = idx - 1;
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
                let value = rest
                    .trim()
                    .parse::<usize>()
                    .context("Invalid DrawCount")
                    .with_context(line_context)?;
                board.set_draw_count(value);
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
            if let Some(card) = card {
                output.push_str(&format!("Foundation{}: {}\n", i + 1, card.pretty_print()));
            }
        }

        // Tableaus
        for (i, tableau) in self.tableaus.iter().enumerate() {
            if tableau.is_empty() {
                continue;
            }
            output.push_str(&format!("Tableau{}: ", i + 1));
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
        output.push_str(&format!("DrawCount: {}", self.draw_count()));

        output
    }
}

#[derive(Debug, Clone, Default)]
pub struct WastePile {
    pub cards: SmallVec<[Card; TALON_SIZE]>,
    pub visible_count: usize,
}

impl WastePile {
    pub fn new(cards: Vec<Card>, visible_count: usize) -> Self {
        Self {
            cards: cards.into_iter().collect(),
            visible_count,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn peek_top(&self) -> Option<&Card> {
        self.cards.last()
    }

    pub fn pop_unchecked(&mut self) -> Card {
        match self.cards.pop() {
            Some(card) => {
                if self.cards.is_empty() {
                    self.visible_count = 0;
                } else {
                    self.visible_count = 1.max(self.visible_count - 1);
                }
                card
            }
            None => Card::UNKNOWN,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Tableau {
    pub cards: SmallVec<[Card; TABLEAU_SIZE]>,
    pub face_up_count: usize,
}

impl Tableau {
    pub fn new(cards: Vec<Card>, face_up_count: usize) -> Self {
        Self {
            cards: cards.into_iter().collect(),
            face_up_count,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn peek_top(&self) -> Option<&Card> {
        self.cards.last()
    }

    pub fn pop_unchecked(&mut self) -> Card {
        match self.cards.pop() {
            Some(card) => {
                if self.cards.is_empty() {
                    self.face_up_count = 0;
                } else {
                    self.face_up_count = 1.max(self.face_up_count - 1);
                }
                card
            }
            None => Card::UNKNOWN,
        }
    }

    pub fn drain_unchecked(&mut self, count: usize) -> Vec<Card> {
        let len = self.cards.len();
        let cards = self.cards.drain(len - count..).collect();
        if self.cards.is_empty() {
            self.face_up_count = 0;
        } else {
            self.face_up_count = 1.max(self.face_up_count - count);
        }
        cards
    }

    pub fn push(&mut self, card: Card) {
        self.face_up_count += 1;
        self.cards.push(card);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Card(u8);

impl Card {
    pub const UNKNOWN: Self = Self(MAX_CARD);

    pub fn new_with_id(id: u8) -> Self {
        if id >= MAX_CARD {
            Self::UNKNOWN
        } else {
            Self(id)
        }
    }

    pub fn new_with_rank_suit(rank: u8, suit: u8) -> Self {
        Self(suit * MAX_RANK + rank)
    }

    pub fn parse(rank: char, suit: char) -> Result<Self> {
        let rank = RANKS
            .iter()
            .position(|&r| r == rank)
            .with_context(|| format!("Invalid rank at card {rank}{suit}"))?;
        let suit = SUITS
            .iter()
            .position(|&s| s == suit)
            .with_context(|| format!("Invalid suit at card {rank}{suit}"))?;
        Ok(Card::new_with_rank_suit(rank as u8, suit as u8))
    }

    pub fn id(&self) -> u8 {
        self.0
    }

    pub fn is_unknown(&self) -> bool {
        self.0 >= Card::UNKNOWN.0
    }

    pub fn rank(&self) -> u8 {
        self.0 % MAX_RANK
    }

    pub fn suit(&self) -> u8 {
        self.0 / MAX_RANK
    }

    pub fn pretty_print(&self) -> String {
        format!(
            "{}{}",
            RANKS[self.rank() as usize],
            SUITS[self.suit() as usize]
        )
    }
}

impl Default for Card {
    fn default() -> Self {
        Card::UNKNOWN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_board() {
        const BOARD_STR: &str = r#"Stock: 5♦2♥8♦K♣7♥J♣
Waste: 7♦Q♥K♥T♦6♣9♥K♦J♠T♣Q♣3♣2♦Q♦8♥6♥|7♠8♠
Foundation1: 2♣
Foundation3: A♠
Tableau1: |5♣
Tableau2: J♥|6♠
Tableau3: T♠5♥|Q♠
Tableau4: 9♠T♥2♠|9♣
Tableau5: 7♣4♥3♠|A♦
Tableau6: 3♥3♦4♣5♠4♦|8♣
Tableau7: 6♦4♠A♥9♦K♠|J♦
DrawCount: 3"#;

        let board = Board::parse(BOARD_STR).unwrap();
        assert!(board.is_valid());
        assert_eq!(BOARD_STR, board.pretty_print());
    }

    #[test]
    fn test_new_board() {
        let board = Board::new();
        assert_eq!(board.draw_count(), 1);
        assert_eq!(board.foundation_score(), 0);
        assert!(!board.is_valid());
    }

    #[test]
    fn test_new_from_seed() {
        let board = Board::new_from_seed(670334786);
        assert_eq!(board.draw_count(), 1);
        assert!(board.is_valid());
        println!("{}", board.pretty_print());
    }
}
