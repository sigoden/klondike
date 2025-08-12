use klondike_common::board::{Card, MAX_CARD, MAX_RANK, MAX_SUIT};

// CardExt is an extended representation of Card that includes computed properties for performance optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardExt {
    pub id: u8,
    pub id2: u8,
    pub suit: u8,
    pub rank: u8,
    pub is_red: u8,
    pub is_even: u8,
    pub red_even: u8,
    pub order: u8,
}

impl CardExt {
    pub const UNKNOWN: CardExt = CardExt {
        id: MAX_CARD,
        id2: 0,
        suit: MAX_SUIT,
        rank: MAX_RANK,
        is_even: 1,
        is_red: 2,
        red_even: 2,
        order: 0,
    };

    pub fn new_with_id(id: u8) -> Self {
        if id >= MAX_CARD {
            return Self::UNKNOWN;
        }
        let rank = id % MAX_RANK;
        let suit = id / MAX_RANK;
        let id2 = (rank << 2) | suit;
        let is_red = suit & 1;
        let is_even = rank & 1;
        let red_even = is_red ^ is_even;
        let order = suit >> 1;

        CardExt {
            id,
            id2,
            suit,
            rank,
            is_red,
            is_even,
            red_even,
            order,
        }
    }

    pub fn new_with_rank_suit(rank: u8, suit: u8) -> Self {
        Self::new_with_id((suit * MAX_RANK) + rank)
    }

    pub fn is_unknown(&self) -> bool {
        self.id >= MAX_CARD
    }

    pub fn is_king(&self) -> bool {
        self.rank == MAX_RANK - 1
    }
}

impl From<&Card> for CardExt {
    fn from(card: &Card) -> Self {
        Self::new_with_id(card.id())
    }
}

impl Default for CardExt {
    fn default() -> Self {
        Self::UNKNOWN
    }
}
