use crate::{board::TALON_SIZE, solver::card::CardExt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pile {
    pub size: usize,
    pub first: i8,
    cards: [CardExt; TALON_SIZE],
}

impl Pile {
    #[inline]
    pub fn reset(&mut self) {
        self.size = 0;
        self.first = -1;
        self.cards.fill(CardExt::UNKNOWN);
    }

    #[inline]
    pub fn set_face_up_count(&mut self, count: usize) {
        self.first = (self.size as i8) - (count as i8);
    }

    #[inline]
    pub fn push_card(&mut self, card: CardExt) {
        self.cards[self.size] = card;
        self.size += 1;
    }

    #[inline]
    pub fn pop_card_to(&mut self, to: &mut Pile) {
        self.size -= 1;
        let card = self.cards[self.size];
        to.push_card(card);
    }

    #[inline]
    pub fn move_n_cards_to(&mut self, to: &mut Pile, count: usize) {
        let from_idx = self.size - count;
        let to_idx = to.size;

        for i in 0..count {
            to.cards[to_idx + i] = self.cards[from_idx + i];
        }

        self.size -= count;
        to.size += count;
    }

    #[inline]
    pub fn move_n_cards_reversed_to(&mut self, to: &mut Pile, count: usize) {
        let from_idx = self.size - count;
        let to_idx = to.size;

        for i in 0..count {
            to.cards[to_idx + i] = self.cards[from_idx + i];
        }

        // Reverse the copied cards in the destination pile
        for i in 0..(count / 2) {
            to.cards.swap(to_idx + i, to_idx + count - 1 - i);
        }

        self.size -= count;
        to.size += count;
    }

    #[inline]
    pub fn get(&self, index: usize) -> CardExt {
        self.cards[index]
    }

    #[inline]
    pub fn peek_top(&self) -> CardExt {
        if self.size > 0 {
            self.cards[self.size - 1]
        } else {
            CardExt::UNKNOWN
        }
    }

    #[inline]
    pub fn peek_top_unchecked(&self) -> CardExt {
        self.cards[self.size - 1]
    }

    #[inline]
    pub fn peek_first_face_up(&self) -> CardExt {
        if self.size > 0 && self.first > -1 {
            self.cards[self.first as usize]
        } else {
            CardExt::UNKNOWN
        }
    }

    #[inline]
    pub fn peek_first_face_up_unchecked(&self) -> CardExt {
        self.cards[self.first as usize]
    }

    #[inline]
    pub fn peek_nth_from_top_unchecked(&self, offset: usize) -> CardExt {
        self.cards[self.size - offset - 1]
    }

    #[inline]
    pub fn face_up_count(&self) -> usize {
        if self.first > -1 {
            self.size - self.first as usize
        } else {
            0
        }
    }
}

impl Default for Pile {
    fn default() -> Self {
        Pile {
            size: 0,
            first: -1,
            cards: [CardExt::UNKNOWN; TALON_SIZE],
        }
    }
}
