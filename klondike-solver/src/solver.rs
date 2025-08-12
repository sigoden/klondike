use super::*;

use klondike_common::action::Action;
use klondike_common::board::{Board, Card, MAX_CARD, MAX_SUIT, TOTAL_FOUNDATIONS, TOTAL_TABLEAUS};

use ahash::AHasher;
use anyhow::{Result, bail};
use smallvec::SmallVec;
use std::{
    collections::BinaryHeap,
    hash::Hasher,
    time::{Duration, Instant},
};

const MAX_ROUNDS: usize = 15;
const MAX_MOVES: usize = 255;
const PILE_STOCK: usize = 0;
const PILE_WASTE: usize = 1;
const PILE_FOUNDATION_START: usize = 2;
const PILE_FOUNDATION_END: usize = PILE_FOUNDATION_START + TOTAL_FOUNDATIONS - 1;
const PILE_TABLEAU_START: usize = PILE_FOUNDATION_END + 1;
const PILE_TABLEAU_END: usize = PILE_TABLEAU_START + TOTAL_TABLEAUS - 1;
const PILE_SIZE: usize = TOTAL_FOUNDATIONS + TOTAL_TABLEAUS + 2;

type PossibleMoves = SmallVec<[Move; 64]>;

pub fn solve(board: Board, max_states: u32, minimal: bool) -> Result<SolveResult> {
    let mut solver = Solver::new();
    solver.set_board(board);
    solver.solve(max_states, minimal)
}

/// A struct representing the solver for the Solitaire game.
#[derive(Debug, Clone)]
pub struct Solver {
    helper: TalonHelper,
    initial_board: Board,
    initial_piles: [Pile; PILE_SIZE],
    initial_foundation_score: u8,
    piles: [Pile; PILE_SIZE],
    moves: [Move; MAX_MOVES],
    suits_to_foundations: [usize; TOTAL_FOUNDATIONS],
    foundation_score: u8,
    foundation_minimum: u8,
    last_move: Move,
    moves_total: usize,
    round_count: usize,
}

impl Default for Solver {
    fn default() -> Self {
        Self::new()
    }
}

impl Solver {
    pub fn new() -> Self {
        Self {
            helper: TalonHelper::new(),
            initial_board: Board::default(),
            initial_piles: std::array::from_fn(|_| Default::default()),
            initial_foundation_score: 0,
            piles: std::array::from_fn(|_| Default::default()),
            moves: std::array::from_fn(|_| Default::default()),
            foundation_score: 0,
            foundation_minimum: 0,
            suits_to_foundations: [MAX_SUIT as usize; TOTAL_FOUNDATIONS],
            last_move: Default::default(),
            moves_total: 0,
            round_count: 1,
        }
    }

    pub fn draw_count(&self) -> usize {
        self.initial_board.draw_count()
    }

    pub fn solve(&mut self, max_nodes: u32, minimal: bool) -> Result<SolveResult> {
        if !self.initial_board.is_valid() {
            bail!("Invalid initial board state.");
        }
        let mut open = BinaryHeap::with_capacity((max_nodes as usize) / 10);
        let mut closed = StateMap::with_capacity(max_nodes as usize + 1);
        let mut node_storage: Vec<MoveNode> = vec![MoveNode::default(); max_nodes as usize + 1];

        let mut node_count = 1;
        let mut max_foundation_score = 0;
        let mut possible_moves = PossibleMoves::new();
        let mut moves_storage = [Move::default(); MAX_MOVES];

        let estimate = Estimate {
            current: 0,
            remaining: self.minimum_moves_remaining(false),
        };
        closed.insert(self.get_state(), estimate);
        open.push(MoveIndex::new(node_count - 1, 0, estimate));

        let mut best_solution_move_count = MAX_MOVES as u8;
        let mut solution_node_index = None;
        let timer = Instant::now();

        while let Some(node) = open.pop() {
            if node_count >= max_nodes {
                break;
            }

            let estimate = node.estimate;
            if estimate.total() >= best_solution_move_count {
                continue;
            }

            let moves_to_make =
                node_storage[node.index as usize].copy(&mut moves_storage, &node_storage);
            self.reset();
            for i in (0..moves_to_make).rev() {
                self.make_move(moves_storage[i]);
            }

            possible_moves.clear();
            self.compute_possible_moves(&mut possible_moves);

            for &mov in possible_moves.iter() {
                let additional_moves = self.calculate_additional_moves(mov);
                self.make_move(mov);

                let new_current = estimate.current.saturating_add(additional_moves);
                let new_estimate = Estimate {
                    current: new_current,
                    remaining: self.minimum_moves_remaining(self.round_count == MAX_ROUNDS),
                };

                if new_estimate.total() < best_solution_move_count && self.round_count <= MAX_ROUNDS
                {
                    let mut skip = false;

                    let key = self.get_state();
                    match closed.get(key) {
                        Some((estimate, bucket_index)) => {
                            if estimate.total() > new_estimate.total() {
                                closed.estimate_mut(bucket_index).clone_from(&new_estimate);
                            } else {
                                skip = true
                            }
                        }
                        None => {
                            closed.insert(key, new_estimate);
                        }
                    }
                    if !skip {
                        node_storage[node_count as usize] = MoveNode {
                            mov,
                            parent: node.index,
                        };

                        let solved = self.foundation_score == MAX_CARD;
                        if self.foundation_score > max_foundation_score || solved {
                            solution_node_index = Some(node_count);
                            max_foundation_score = self.foundation_score;
                        }
                        if solved {
                            best_solution_move_count = new_estimate.total();
                            node_count += 1;
                            if !minimal {
                                open.clear();
                                break;
                            }
                        } else {
                            let heuristic = ((new_estimate.total() as i16) << 1)
                                + additional_moves as i16
                                + (MAX_CARD - self.foundation_score) as i16
                                + ((self.round_count as i16) << 1);
                            open.push(MoveIndex::new(node_count, heuristic, new_estimate));
                            node_count += 1;
                            if node_count >= max_nodes {
                                break;
                            }
                        }
                    }
                }

                self.undo_move();
            }
        }

        if let Some(node_index) = solution_node_index {
            let moves_to_make =
                node_storage[node_index as usize].copy(&mut moves_storage, &node_storage);
            self.reset();
            for i in (0..moves_to_make).rev() {
                self.make_move(moves_storage[i]);
            }
        }

        if max_foundation_score != MAX_CARD {
            if node_count < max_nodes {
                bail!("No solution found.");
            } else {
                bail!("Unable to solve the game; reached max states {max_nodes}.");
            }
        }

        Ok(SolveResult {
            minimal: minimal && node_count < max_nodes,
            states: node_count as i32,
            elapsed: timer.elapsed(),
            actions: self.export_actions(),
        })
    }

    fn minimum_moves_remaining(&self, is_last_round: bool) -> u8 {
        let waste_pile = &self.piles[PILE_WASTE];
        let waste_size = waste_pile.size;
        let stock_size = self.piles[PILE_STOCK].size;
        let draw_count = self.draw_count();

        let mut num: usize = stock_size + stock_size.div_ceil(draw_count) + waste_size;
        let mut mins = [u8::MAX; 4];

        if draw_count == 1 || is_last_round {
            for i in 0..waste_size {
                let card = waste_pile.get(i);
                let suit_idx = card.suit as usize;
                if card.rank < mins[suit_idx] {
                    mins[suit_idx] = card.rank;
                } else {
                    num += 1;
                }
            }
        }

        for i in PILE_TABLEAU_START..=PILE_TABLEAU_END {
            mins.fill(u8::MAX);
            let pile = &self.piles[i];
            num += pile.size;

            for j in 0..pile.size {
                let card = pile.get(j);
                let suit_idx = card.suit as usize;
                if card.rank < mins[suit_idx] {
                    if let Some(first) = pile.first
                        && (j as u8) < first
                    {
                        mins[suit_idx] = card.rank;
                    }
                } else {
                    num += 1;
                    if let Some(first) = pile.first
                        && (j as u8) >= first
                    {
                        break;
                    }
                }
            }
        }

        num as u8
    }

    fn get_state(&self) -> u64 {
        let mut state = [0; 32];

        state[0] = self.piles[PILE_WASTE].size as u8;

        state[1] = ((self.piles[PILE_FOUNDATION_START].size << 4)
            | self.piles[PILE_FOUNDATION_START + 2].size) as u8;
        state[2] = ((self.piles[PILE_FOUNDATION_START + 1].size << 4)
            | self.piles[PILE_FOUNDATION_START + 3].size) as u8;

        let mut tableau_idxs: [usize; TOTAL_TABLEAUS] =
            std::array::from_fn(|i| PILE_TABLEAU_START + i);
        tableau_idxs.sort_by(|&a, &b| {
            let pile_a = &self.piles[a];
            let pile_b = &self.piles[b];
            pile_b
                .peek_first_face_up()
                .id2
                .cmp(&pile_a.peek_first_face_up().id2)
        });

        for (i, &tableau_idx) in tableau_idxs.iter().enumerate() {
            let state_idx = 4 * (i + 1);
            let pile = &self.piles[tableau_idx];
            let face_up_count = pile.face_up_count();
            state[state_idx] = face_up_count as u8;
            if face_up_count > 0 {
                state[state_idx + 1] = pile.peek_first_face_up_unchecked().id;
                let mut flags: u16 = 0;
                for card_offset in 0..(face_up_count - 1) {
                    let order = pile.peek_nth_from_top_unchecked(card_offset).order as u16;
                    flags |= order << card_offset;
                }
                let flag_bytes = flags.to_be_bytes();
                state[state_idx + 2] = flag_bytes[0];
                state[state_idx + 3] = flag_bytes[1];
            }
        }

        let mut hasher = AHasher::default();
        hasher.write(&state);
        hasher.finish()
    }

    fn calculate_additional_moves(&self, mov: Move) -> u8 {
        let mut count = 1;
        let mov_count = mov.count() as u8;
        if mov.from() == PILE_WASTE as u8 && mov_count != 0 {
            let draw_count = self.draw_count() as u8;
            if !mov.flip() {
                count += mov_count.div_ceil(draw_count);
            } else {
                let stock_size = self.piles[PILE_STOCK].size as u8;
                count += stock_size.div_ceil(draw_count);
                count += (mov_count - stock_size).div_ceil(draw_count);
            }
        }
        count
    }

    fn compute_possible_moves(&mut self, possible_moves: &mut PossibleMoves) {
        self.foundation_minimum = (PILE_FOUNDATION_START..=PILE_FOUNDATION_END)
            .map(|i| self.piles[i].size)
            .min()
            .unwrap_or(0) as u8
            + 1;

        if self.compute_with_last_move(possible_moves) {
            return;
        }
        if self.compute_move_from_tableau(possible_moves) {
            return;
        }
        if self.compute_move_from_waste(possible_moves) {
            return;
        }
        self.compute_move_from_foundation(possible_moves);
    }

    fn compute_with_last_move(&mut self, possible_moves: &mut PossibleMoves) -> bool {
        let (move_from, move_to, _, move_flip) = self.last_move.values();

        if (PILE_TABLEAU_START..=PILE_TABLEAU_END).contains(&move_from)
            && (PILE_TABLEAU_START..=PILE_TABLEAU_END).contains(&move_to)
            && !move_flip
        {
            let src_pile = &self.piles[move_from];
            if src_pile.size > 0 {
                let src_top_card = src_pile.peek_top_unchecked();
                if let Some(foundation_idx) = self.can_move_to_foundation(src_top_card) {
                    possible_moves.push(Move::new(
                        move_from as u8,
                        foundation_idx,
                        1,
                        src_pile.size > 1 && src_pile.face_up_count() == 1,
                    ));
                    return true;
                }
            }
        }
        false
    }

    fn compute_move_from_tableau(&mut self, possible_moves: &mut PossibleMoves) -> bool {
        let mut non_empty_tableaus: SmallVec<[u8; TOTAL_TABLEAUS]> = SmallVec::new();
        let mut empty_tableaus_count = 0;
        for idx in PILE_TABLEAU_START..=PILE_TABLEAU_END {
            if self.piles[idx].size > 0 {
                non_empty_tableaus.push(idx as u8);
            } else {
                empty_tableaus_count += 1;
            }
        }

        for &src_idx in &non_empty_tableaus {
            let src_pile = &self.piles[src_idx as usize];
            let src_pile_size = src_pile.size;

            let src_top_card = src_pile.peek_top_unchecked();
            if let Some(foundation_idx) = self.can_move_to_foundation(src_top_card) {
                let mov = Move::new(
                    src_idx,
                    foundation_idx,
                    1,
                    src_pile_size > 1 && src_pile.face_up_count() == 1,
                );
                if src_top_card.rank <= self.foundation_minimum {
                    possible_moves.clear();
                    possible_moves.push(mov);
                    return true;
                } else {
                    possible_moves.push(mov);
                }
            }

            let src_first_face_up_card = src_pile.peek_first_face_up_unchecked();
            let src_face_up_count =
                src_first_face_up_card.rank as i32 - src_top_card.rank as i32 + 1;
            let mut king_moved = !src_first_face_up_card.is_king();

            for dest_idx in PILE_TABLEAU_START..=PILE_TABLEAU_END {
                if src_idx == dest_idx as u8 {
                    continue;
                }
                let dest_pile = &self.piles[dest_idx];
                if dest_pile.size == 0 {
                    if !king_moved && (src_pile_size as i32) != src_face_up_count {
                        possible_moves.push(Move::new(
                            src_idx,
                            dest_idx as u8,
                            src_face_up_count as u8,
                            true,
                        ));
                        king_moved = true;
                    }
                    continue;
                }

                let dest_top_card = dest_pile.peek_top_unchecked();
                if dest_top_card.rank as i32 - src_first_face_up_card.rank as i32 > 1
                    || src_top_card.red_even != dest_top_card.red_even
                    || src_top_card.rank >= dest_top_card.rank
                {
                    continue;
                }
                let src_moved_count = dest_top_card.rank as i32 - src_top_card.rank as i32;
                if (src_moved_count == src_face_up_count
                    && (src_moved_count != src_pile_size as i32 || empty_tableaus_count == 0))
                    || (src_moved_count < src_face_up_count
                        && self
                            .can_move_to_foundation(
                                src_pile.peek_nth_from_top_unchecked(src_moved_count as usize),
                            )
                            .is_some())
                {
                    possible_moves.push(Move::new(
                        src_idx,
                        dest_idx as u8,
                        src_moved_count as u8,
                        src_pile_size as i32 > src_moved_count
                            && src_moved_count == src_face_up_count,
                    ));
                }
            }
        }

        false
    }

    fn compute_move_from_waste(&mut self, possible_moves: &mut PossibleMoves) -> bool {
        let draw_count = self.draw_count();
        let talon_count = self.helper.calculate(
            self.draw_count(),
            &self.piles[PILE_WASTE],
            &self.piles[PILE_STOCK],
        );
        for idx in 0..talon_count {
            let talon_card = self.helper.stock_waste[idx];
            let mut cards_to_draw = self.helper.cards_drawn[idx];
            let flip = cards_to_draw < 0;
            if flip {
                cards_to_draw = -cards_to_draw;
            }

            if let Some(foundation_idx) = self.can_move_to_foundation(talon_card) {
                possible_moves.push(Move::new(
                    PILE_WASTE as u8,
                    foundation_idx,
                    cards_to_draw as u8,
                    flip,
                ));
                if talon_card.rank <= self.foundation_minimum {
                    if draw_count > 1 {
                        continue;
                    }
                    if cards_to_draw == 0 || possible_moves.len() == 1 {
                        return true;
                    }
                    break;
                }
            }
            for tableau_idx in PILE_TABLEAU_START..=PILE_TABLEAU_END {
                let tableau_top_card = self.piles[tableau_idx].peek_top();
                if tableau_top_card.rank as i32 - talon_card.rank as i32 == 1
                    && talon_card.is_red != tableau_top_card.is_red
                {
                    possible_moves.push(Move::new(
                        PILE_WASTE as u8,
                        tableau_idx as u8,
                        cards_to_draw as u8,
                        flip,
                    ));
                    if talon_card.is_king() {
                        break;
                    }
                }
            }
        }
        false
    }

    fn compute_move_from_foundation(&mut self, possible_moves: &mut PossibleMoves) -> bool {
        for foundation_idx in PILE_FOUNDATION_START..=PILE_FOUNDATION_END {
            let foundation_pile = &self.piles[foundation_idx];
            if foundation_pile.size <= self.foundation_minimum as usize {
                continue;
            }
            let foundation_card = foundation_pile.peek_top_unchecked();
            for tableau_idx in PILE_TABLEAU_START..=PILE_TABLEAU_END {
                let tableau_top_card = &self.piles[tableau_idx].peek_top();
                if tableau_top_card.rank as i32 - foundation_card.rank as i32 == 1
                    && tableau_top_card.is_red != foundation_card.is_red
                {
                    possible_moves.push(Move::new(
                        foundation_idx as u8,
                        tableau_idx as u8,
                        1,
                        false,
                    ));
                    if foundation_card.is_king() {
                        break;
                    }
                }
            }
        }
        false
    }

    fn can_move_to_foundation(&self, card: CardExt) -> Option<u8> {
        let idx = if card.is_unknown() {
            return None;
        } else {
            self.suits_to_foundations[card.suit as usize]
        };
        match self.piles[idx].size == card.rank as usize {
            true => Some(idx as u8),
            false => None,
        }
    }

    fn make_move(&mut self, mov: Move) {
        self.moves[self.moves_total] = mov;
        self.moves_total += 1;
        self.last_move = mov;

        let (move_from, move_to, move_count, move_flip) = mov.values();

        if move_from == PILE_WASTE && move_count != 0 {
            if !move_flip {
                let (from_pile, to_pile) = self.get_mut_piles(PILE_STOCK, PILE_WASTE);
                from_pile.move_n_cards_reversed_to(to_pile, move_count);
            } else {
                self.round_count += 1;
                let size = self.piles[PILE_STOCK].size as isize
                    + self.piles[PILE_WASTE].size as isize
                    - move_count as isize;
                if size >= 1 {
                    let (from_pile, to_pile) = self.get_mut_piles(PILE_WASTE, PILE_STOCK);
                    from_pile.move_n_cards_reversed_to(to_pile, size as usize);
                } else {
                    let (from_pile, to_pile) = self.get_mut_piles(PILE_STOCK, PILE_WASTE);
                    from_pile.move_n_cards_reversed_to(to_pile, -size as usize);
                }
            }
        }

        if move_from == PILE_WASTE || move_count == 1 {
            let (from_pile, to_pile) = self.get_mut_piles(move_from, move_to);
            from_pile.pop_card_to(to_pile);

            if (PILE_FOUNDATION_START..=PILE_FOUNDATION_END).contains(&move_to) {
                self.foundation_score += 1;
            } else if (PILE_FOUNDATION_START..=PILE_FOUNDATION_END).contains(&move_from) {
                self.foundation_score -= 1;
            }
        } else {
            let (from_pile, to_pile) = self.get_mut_piles(move_from, move_to);
            from_pile.move_n_cards_to(to_pile, move_count);
        }

        if move_flip && (PILE_TABLEAU_START..=PILE_TABLEAU_END).contains(&move_from) {
            self.piles[move_from].set_face_up_count(1);
        }
    }

    fn undo_move(&mut self) {
        self.moves_total -= 1;
        let mov = self.moves[self.moves_total];
        self.last_move = if self.moves_total > 0 {
            self.moves[self.moves_total - 1]
        } else {
            Move::default()
        };

        let (move_from, move_to, move_count, move_flip) = mov.values();

        if move_from == PILE_WASTE || move_count == 1 {
            let (to_pile, from_pile) = self.get_mut_piles(move_to, move_from);
            to_pile.pop_card_to(from_pile);
            if (PILE_FOUNDATION_START..=PILE_FOUNDATION_END).contains(&move_to) {
                self.foundation_score -= 1;
            } else if (PILE_FOUNDATION_START..=PILE_FOUNDATION_END).contains(&move_from) {
                self.foundation_score += 1;
            }
        } else {
            let (to_pile, from_pile) = self.get_mut_piles(move_to, move_from);
            to_pile.move_n_cards_to(from_pile, move_count);
        }

        if move_flip && (PILE_TABLEAU_START..=PILE_TABLEAU_END).contains(&move_from) {
            self.piles[move_from].set_face_up_count(move_count);
        }

        if move_from == PILE_WASTE && move_count != 0 {
            if !move_flip {
                let (from_pile, to_pile) = self.get_mut_piles(PILE_WASTE, PILE_STOCK);
                from_pile.move_n_cards_reversed_to(to_pile, move_count);
            } else {
                self.round_count -= 1;
                let size = self.piles[PILE_STOCK].size as isize
                    + self.piles[PILE_WASTE].size as isize
                    - move_count as isize;
                if size >= 1 {
                    let (from_pile, to_pile) = self.get_mut_piles(PILE_STOCK, PILE_WASTE);
                    from_pile.move_n_cards_reversed_to(to_pile, size as usize);
                } else {
                    let (from_pile, to_pile) = self.get_mut_piles(PILE_WASTE, PILE_STOCK);
                    from_pile.move_n_cards_reversed_to(to_pile, -size as usize);
                }
            }
        }
    }

    fn export_actions(&self) -> Vec<Action> {
        let mut actions = vec![];
        let mut stock_size = self.initial_piles[PILE_STOCK].size;
        let mut waste_size = self.initial_piles[PILE_WASTE].size;
        let draw_count = self.draw_count();
        let mut board = self.initial_board.clone();

        for i in 0..self.moves_total {
            let mov = self.moves[i];
            let (move_from, move_to, move_count, move_flip) = mov.values();
            if move_from == PILE_WASTE {
                if !move_flip {
                    for _ in 0..move_count.div_ceil(draw_count) {
                        actions.push(Action::Draw);
                        board.draw();
                    }
                    stock_size -= move_count;
                    waste_size += move_count;
                } else {
                    if stock_size == 0 {
                        actions.push(Action::Redeal);
                        board.draw();
                    }
                    let times = stock_size.div_ceil(draw_count);
                    for _ in 0..times {
                        actions.push(Action::Draw);
                        board.draw();
                        if board.need_redeal() {
                            actions.push(Action::Redeal);
                            board.draw();
                        }
                    }
                    let times = (move_count - stock_size).div_ceil(draw_count);
                    for _ in 0..times {
                        actions.push(Action::Draw);
                        board.draw();
                    }
                    let times = stock_size as i32 + waste_size as i32 - move_count as i32;
                    waste_size = (waste_size as i32 - times) as usize;
                    stock_size = (stock_size as i32 + times) as usize;
                }

                waste_size -= 1;

                if (PILE_FOUNDATION_START..=PILE_FOUNDATION_END).contains(&move_to) {
                    let idx = move_to - PILE_FOUNDATION_START;
                    actions.push(Action::WasteToFoundation(idx));
                    board.move_waste_to_foundation(idx);
                } else if (PILE_TABLEAU_START..=PILE_TABLEAU_END).contains(&move_to) {
                    let idx = move_to - PILE_TABLEAU_START;
                    actions.push(Action::WasteToTableau(idx));
                    board.move_waste_to_tableau(idx);
                }
            } else if (PILE_TABLEAU_START..=PILE_TABLEAU_END).contains(&move_from) {
                let from_idx = move_from - PILE_TABLEAU_START;
                if (PILE_FOUNDATION_START..=PILE_FOUNDATION_END).contains(&move_to) {
                    let to_idx = move_to - PILE_FOUNDATION_START;
                    actions.push(Action::TableauToFoundation(from_idx, to_idx));
                    board.move_tableau_to_foundation(from_idx, to_idx);
                } else if (PILE_TABLEAU_START..=PILE_TABLEAU_END).contains(&move_to) {
                    let to_index = move_to - PILE_TABLEAU_START;
                    actions.push(Action::TableauToTableau(from_idx, to_index, move_count));
                    board.move_tableau_to_tableau(from_idx, to_index, move_count);
                }
            } else if (PILE_FOUNDATION_START..=PILE_FOUNDATION_END).contains(&move_from) {
                let from_index = move_from - PILE_FOUNDATION_START;
                if (PILE_TABLEAU_START..=PILE_TABLEAU_END).contains(&move_to) {
                    let to_index = move_to - PILE_TABLEAU_START;
                    actions.push(Action::FoundationToTableau(from_index, to_index));
                    board.move_foundation_to_tableau(from_index, to_index);
                }
            }
        }
        actions
    }

    pub fn set_board(&mut self, board: Board) {
        let mut foundation_score = 0;
        let mut foundation_slots: u8 = 0;
        self.suits_to_foundations.fill(MAX_SUIT as usize);

        {
            let pile = &mut self.initial_piles[PILE_STOCK];
            pile.reset();
            for card in board.stock.iter() {
                pile.push_card(card.into());
            }
        }

        {
            let pile = &mut self.initial_piles[PILE_WASTE];
            pile.reset();
            for card in board.waste.iter() {
                pile.push_card(card.into());
            }
        }

        for i in 0..TOTAL_FOUNDATIONS {
            let pile = &mut self.initial_piles[PILE_FOUNDATION_START + i];
            pile.reset();
            let card = board.foundations[i];
            let Some(card) = card else {
                continue;
            };
            let suit = card.suit();
            let rank = card.rank();
            foundation_score += rank + 1;
            for j in 0..=rank {
                pile.push_card(CardExt::new_with_rank_suit(j, suit));
            }
            self.suits_to_foundations[suit as usize] = PILE_FOUNDATION_START + i;
            foundation_slots |= 1 << i
        }

        for i in 0..MAX_SUIT {
            if self.suits_to_foundations[i as usize] == MAX_SUIT as usize {
                for j in 0..TOTAL_FOUNDATIONS {
                    if foundation_slots & (1 << j) == 0 {
                        self.suits_to_foundations[i as usize] = PILE_FOUNDATION_START + j;
                        foundation_slots |= 1 << j;
                        break;
                    }
                }
            }
        }

        for i in 0..TOTAL_TABLEAUS {
            let pile = &mut self.initial_piles[PILE_TABLEAU_START + i];
            pile.reset();
            for card in board.tableaus[i].cards.iter() {
                pile.push_card(card.into());
            }
            pile.set_face_up_count(board.tableaus[i].face_up_count);
        }

        self.initial_board = board;
        self.initial_foundation_score = foundation_score;

        self.reset();
    }

    pub fn get_board(&self) -> Board {
        let mut board = Board::default();

        {
            let stock_pile = &self.piles[PILE_STOCK];
            for i in 0..stock_pile.size {
                board.stock.push(Card::new_with_id(stock_pile.get(i).id));
            }
        }

        {
            let waste_pile = &self.piles[PILE_WASTE];
            for i in 0..waste_pile.size {
                board.waste.push(Card::new_with_id(waste_pile.get(i).id));
            }
        }

        for i in 0..TOTAL_FOUNDATIONS {
            let card = self.piles[PILE_FOUNDATION_START + i].peek_top();
            if card.is_unknown() {
                continue;
            }
            board.foundations[i] = Some(Card::new_with_id(card.id));
        }

        for i in 0..TOTAL_TABLEAUS {
            let pile = &self.piles[PILE_TABLEAU_START + i];
            for j in 0..pile.size {
                board.tableaus[i]
                    .cards
                    .push(Card::new_with_id(pile.get(j).id));
                board.tableaus[i].face_up_count = pile.face_up_count();
            }
        }

        board
    }

    fn reset(&mut self) {
        self.foundation_score = self.initial_foundation_score;
        self.foundation_minimum = 0;
        self.moves_total = 0;
        self.round_count = 1;
        self.last_move = Move::default();
        self.piles[..].clone_from_slice(&self.initial_piles[..]);
    }

    fn get_mut_piles(&mut self, idx_a: usize, idx_b: usize) -> (&mut Pile, &mut Pile) {
        if idx_a < idx_b {
            let (a, b) = self.piles.split_at_mut(idx_b);
            (&mut a[idx_a], &mut b[0])
        } else {
            let (a, b) = self.piles.split_at_mut(idx_a);
            (&mut b[0], &mut a[idx_b])
        }
    }
}

#[derive(Debug, Clone)]
pub struct SolveResult {
    pub minimal: bool,
    pub states: i32,
    pub elapsed: Duration,
    pub actions: Vec<Action>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solve() {
        const BOARD_STR: &str = r#"Stock: 5♣3♣6♦Q♦A♠5♦K♠4♥5♥4♣7♠Q♣J♣6♠2♥2♣3♠9♥K♦7♦7♥J♠A♦8♣
Tableau1: |9♦
Tableau2: 7♣|9♣
Tableau3: A♣2♠|3♦
Tableau4: K♥T♠T♣|T♦
Tableau5: 8♠Q♥6♥6♣|J♦
Tableau6: 8♥Q♠5♠3♥K♣|4♦
Tableau7: 8♦A♥9♠J♥2♦4♠|T♥
DrawCount: 1
"#;

        let board = Board::parse(BOARD_STR).unwrap();
        let result = solve(board, 200_000, true).unwrap();
        assert_eq!(result.states, 166066);
        assert_eq!(result.actions.len(), 114);
        let encoded_actions = klondike_common::action::format_actions(&result.actions);
        println!("{encoded_actions}");
    }
}
