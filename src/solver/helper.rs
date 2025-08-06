use super::*;

use crate::board::TALON_SIZE;

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub struct Estimate {
    pub current: u8,
    pub remaining: u8,
}

impl Estimate {
    pub fn total(&self) -> u8 {
        let total = self.current as u16 + self.remaining as u16;
        if total > 255 { 255_u8 } else { total as u8 }
    }
}

#[derive(Debug, Clone)]
pub struct TalonHelper {
    pub stock_waste: [CardExt; TALON_SIZE],
    pub cards_drawn: [i32; TALON_SIZE],
    stock_used: [bool; TALON_SIZE],
}

impl TalonHelper {
    pub fn new() -> Self {
        TalonHelper {
            stock_waste: std::array::from_fn(|_| CardExt::UNKNOWN),
            cards_drawn: [0; TALON_SIZE],
            stock_used: [false; TALON_SIZE],
        }
    }
    pub fn calculate(&mut self, draw_count: usize, waste_pile: &Pile, stock_pile: &Pile) -> usize {
        let mut size = 0;
        self.stock_used.fill(false);

        // Check waste
        let waste_size = waste_pile.size;
        if waste_size > 0 {
            self.stock_waste[size] = waste_pile.peek_top_unchecked();
            self.cards_drawn[size] = 0;
            size += 1;
        }

        // Check cards waiting to be turned over from stock
        let stock_size = stock_pile.size;
        let mut position = stock_size as i32 - draw_count as i32;
        if position < 0 {
            position = if stock_size > 0 { 0 } else { -1 };
        }

        let mut i = position;
        while i >= 0 {
            let i_usize = i as usize;
            self.stock_waste[size] = stock_pile.get(i_usize);
            self.cards_drawn[size] = stock_size as i32 - i;
            self.stock_used[i_usize] = true;
            size += 1;
            i -= draw_count as i32;
        }

        // Check cards already turned over in the waste, meaning we have to "redeal" the deck to get to it
        let mut amount_to_draw = stock_size as i32 + 1;
        let waste_size_index = waste_size as i32 - 1; // Use a signed index for the loop condition

        let mut position_waste = draw_count as i32 - 1;
        while position_waste < waste_size_index {
            let position_waste_usize = position_waste as usize;
            self.stock_waste[size] = waste_pile.get(position_waste_usize);
            self.cards_drawn[size] = -amount_to_draw - position_waste;
            size += 1;
            position_waste += draw_count as i32;
        }

        // Check cards in stock after a "redeal". Only happens when draw count > 1 and you have access to more cards in the talon
        if position_waste > waste_size_index && waste_size_index >= 0 {
            amount_to_draw += stock_size as i32 + waste_size_index;
            position = stock_size as i32 - position_waste + waste_size_index;

            let mut i = position;
            while i > 0 {
                let i_usize = i as usize;
                if self.stock_used[i_usize] {
                    break;
                }
                self.stock_waste[size] = stock_pile.get(i_usize);
                self.cards_drawn[size] = i - amount_to_draw;
                size += 1;
                i -= draw_count as i32;
            }
        }

        size
    }
}

pub fn find_prime(start: usize) -> usize {
    fn is_prime(n: usize) -> bool {
        if n < 2 {
            return false;
        }
        if n == 2 {
            return true;
        }
        if n % 2 == 0 {
            return false;
        }
        let sqrt_n = (n as f64).sqrt() as usize;
        for i in (3..=sqrt_n).step_by(2) {
            if n % i == 0 {
                return false;
            }
        }
        true
    }

    let mut n = start.max(2);
    while !is_prime(n) {
        n += 1;
    }
    n
}
