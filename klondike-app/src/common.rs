use anyhow::{Context, Result};
use egui::{Color32, Pos2};

const SUITS: [char; 5] = ['♦', '♣', '♥', '♠', '?'];
const RANKS: [char; 14] = [
    'A', '2', '3', '4', '5', '6', '7', '8', '9', 'T', 'J', 'Q', 'K', '?',
];
const MAX_RANK: u8 = 13;

#[derive(Debug, Clone)]
pub struct CardAnimation {
    pub card: Card,
    pub start_pos: Pos2,
    pub end_pos: Pos2,
    pub start_time: f64,
    pub duration: f64,
    pub source: PileId,
    pub destination: PileId,
    pub reverse: bool, // Whether it is a reverse animation (undo)
}

#[derive(Debug, Clone)]
pub struct GameMove {
    pub source: PileId,
    pub destination: PileId,
    pub count: usize,
    pub source_flip: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Autofinish {
    #[default]
    Idle,
    Asking,
    Rejected,
    InProgress,
    Succeed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PileId {
    Stock,
    Waste,
    Foundation(usize),
    Tableau(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    pub stock: Vec<Card>,
    pub waste: Vec<Card>,
    pub foundations: [Vec<Card>; 4],
    pub tableaus: [Vec<Card>; 7],
    pub draw_count: usize,
}

impl Board {
    pub fn new(seed: u32, draw_count: usize) -> Self {
        println!("GameId: {seed}");
        let mut current_seed = seed;
        let mut rnd = || {
            current_seed = ((current_seed as u64 * 16807) % 0x7fffffff) as u32;
            current_seed
        };
        let mut deck: [Card; 52] = std::array::from_fn(|i| {
            let card = Card::new_with_id(i as u8);
            if card.suit() == 0 {
                Card::new_with_rank_suit(card.rank(), 1)
            } else if card.suit() == 1 {
                Card::new_with_rank_suit(card.rank(), 0)
            } else {
                card
            }
        });

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

        let mut tableaus: [Vec<Card>; 7] = Default::default();

        let mut m = 0;
        for (i, tableau) in tableaus.iter_mut().enumerate() {
            for _ in 0..=i {
                tableau.push(deck[m]);
                m += 1;
            }
            tableau.last_mut().unwrap().face_up = true;
        }

        Self {
            stock: deck[m..].into(),
            waste: Vec::new(),
            foundations: Default::default(),
            tableaus,
            draw_count,
        }
    }

    pub fn parse(content: &str) -> Result<Self> {
        let mut board = Self {
            stock: Vec::new(),
            waste: Vec::new(),
            foundations: Default::default(),
            tableaus: Default::default(),
            draw_count: 1,
        };

        for line in content
            .split('\n')
            .map(|v| v.trim())
            .filter(|l| !l.is_empty())
        {
            let line_context = || format!("Failed to parse at '{line}'");
            if let Some(rest) = line.strip_prefix("Stock:") {
                let mut cards = Self::parse_cards(rest.trim()).with_context(line_context)?;
                for card in &mut cards {
                    card.face_up = false;
                }
                board.stock = cards;
            } else if let Some(rest) = line.strip_prefix("Waste:") {
                let (before, after) = if let Some(idx) = rest.find('|') {
                    let (b, a) = rest.split_at(idx);
                    (b, &a[1..])
                } else {
                    (rest, "")
                };
                let mut cards = Self::parse_cards(before.trim()).with_context(line_context)?;
                cards.extend(Self::parse_cards(after.trim()).with_context(line_context)?);
                for card in &mut cards {
                    card.face_up = true;
                }
                board.waste = cards;
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
                if let Some(top_card) = cards.last() {
                    for rank in 0..=top_card.rank() {
                        let mut card = Card::new_with_rank_suit(rank, top_card.suit());
                        card.face_up = true;
                        board.foundations[idx].push(card);
                    }
                }
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
                let (before, after) = if let Some(split_idx) = cards_str.find('|') {
                    let (b, a) = cards_str.split_at(split_idx);
                    (b, &a[1..])
                } else {
                    (cards_str, "")
                };

                let mut face_down_cards =
                    Self::parse_cards(before.trim()).with_context(line_context)?;
                for card in &mut face_down_cards {
                    card.face_up = false;
                }

                let mut face_up_cards =
                    Self::parse_cards(after.trim()).with_context(line_context)?;
                for card in &mut face_up_cards {
                    card.face_up = true;
                }

                board.tableaus[idx].extend(face_down_cards);
                board.tableaus[idx].extend(face_up_cards);
            } else if let Some(rest) = line.strip_prefix("DrawCount:") {
                let value = rest
                    .trim()
                    .parse::<usize>()
                    .context("Invalid DrawCount")
                    .with_context(line_context)?;
                board.draw_count = value;
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

    pub fn score(&self) -> u8 {
        self.foundations.iter().map(|f| f.len() as u8).sum()
    }

    pub fn can_autofinish(&self) -> bool {
        self.stock.is_empty()
            && (self.waste.len() <= self.draw_count && self.waste.is_sorted())
            && self.score() < 51
            && self
                .tableaus
                .iter()
                .all(|pile| pile.iter().all(|card| card.face_up))
    }
}

pub type SolutionMove = (PileId, PileId, usize);

pub fn parse_moves(s: &str) -> Result<Vec<SolutionMove>> {
    let mut moves = Vec::new();
    for part in s.split_whitespace().filter(|s| !s.is_empty()) {
        let part_ctx = || format!("Failed to parse move part: '{part}'");
        if part == "R" {
            moves.push((PileId::Waste, PileId::Stock, 0));
        } else if let Some(num_str) = part.strip_suffix('D') {
            let num = if num_str.is_empty() {
                1
            } else {
                num_str.parse::<usize>().with_context(part_ctx)?
            };
            for _ in 0..num {
                moves.push((PileId::Stock, PileId::Waste, 0));
            }
        } else if let Some(colon_idx) = part.find(':') {
            let (from_str, to_part) = part.split_at(colon_idx);
            let to_part = &to_part[1..];

            let from = parse_pile_id(from_str).with_context(part_ctx)?;

            let (to_str, count) = if let Some(at_idx) = to_part.find('@') {
                let (to_s, count_s) = to_part.split_at(at_idx);
                (to_s, count_s[1..].parse::<usize>().with_context(part_ctx)?)
            } else {
                (to_part, 1)
            };
            let to = parse_pile_id(to_str).with_context(part_ctx)?;
            moves.push((from, to, count));
        } else {
            anyhow::bail!("Unknown move format: {}", part);
        }
    }
    Ok(moves)
}

fn parse_pile_id(s: &str) -> Result<PileId> {
    if s == "W" {
        return Ok(PileId::Waste);
    }
    if let Some(stripped) = s.strip_prefix('T') {
        let num = stripped
            .parse::<usize>()
            .with_context(|| format!("Invalid tableau index: {stripped}"))?
            - 1;
        Ok(PileId::Tableau(num))
    } else if let Some(stripped) = s.strip_prefix('F') {
        let num = stripped
            .parse::<usize>()
            .with_context(|| format!("Invalid foundation index: {stripped}"))?
            - 1;
        Ok(PileId::Foundation(num))
    } else {
        anyhow::bail!("Invalid pile identifier: {}", s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card {
    pub id: u8,
    pub face_up: bool,
}

impl Card {
    pub fn new_with_id(id: u8) -> Self {
        Self { id, face_up: false }
    }

    pub fn new_with_rank_suit(rank: u8, suit: u8) -> Self {
        Card {
            id: suit * MAX_RANK + rank,
            face_up: false,
        }
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

    pub fn rank(&self) -> u8 {
        self.id % MAX_RANK
    }

    pub fn suit(&self) -> u8 {
        self.id / MAX_RANK
    }

    pub fn symbols(&self) -> (char, char) {
        (RANKS[self.rank() as usize], SUITS[self.suit() as usize])
    }

    pub fn color(&self) -> Color32 {
        match self.suit() {
            0 | 2 => Color32::RED,
            _ => Color32::BLACK,
        }
    }

    pub fn is_ace(&self) -> bool {
        self.rank() == 0
    }

    pub fn is_king(&self) -> bool {
        self.rank() == MAX_RANK - 1
    }
}

impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(
            self.rank()
                .cmp(&other.rank())
                .then_with(|| self.suit().cmp(&other.suit())),
        )
    }
}
