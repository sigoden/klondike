use crate::common::*;

use eframe::egui;
use egui::{
    Color32, CornerRadius, Id, LayerId, Order, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2,
};

const CARD_SIZE: Vec2 = Vec2::new(90.0, 130.0);
const CARD_PADDING: f32 = 10.0;
const TABLEAU_CARD_V_OFFSET: f32 = 25.0; // Vertical offset of cards in tableau pile
const WASTE_CARD_H_OFFSET: f32 = 20.0; // Horizontal offset of cards in waste pile
const AUTOPLAY_INTERVAL: f64 = 3.0; // Duration between autoplay moves

pub struct KlondikeApp {
    init_board: Board,
    board: Board,
    solution: Option<(Vec<SolutionMove>, usize, Option<Board>)>,
    foundation_rects: [Rect; 4],
    tableau_rects: [Rect; 7],
    stock_rect: Rect,
    waste_rect: Rect,
    dragged_cards: Vec<Card>,
    drag_source: Option<PileId>,
    drag_offset: Vec2,
    animations: Vec<CardAnimation>,
    history: Vec<GameMove>,
    redo_stack: Vec<GameMove>,
    autofinish: Autofinish,
    hook_moved: bool,
    score: u8,
    start_time: f64,
    end_time: Option<f64>,
    autoplay: bool,
    next_play_time: f64,
}

impl eframe::App for KlondikeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- Hotkey handling ---
        if ctx.input_mut(|i| i.key_pressed(egui::Key::Z)) {
            self.undo(ctx);
        }
        if ctx.input_mut(|i| i.key_pressed(egui::Key::X)) {
            self.redo(ctx);
        }
        if ctx.input_mut(|i| i.key_pressed(egui::Key::N)) {
            self.renew();
        }
        if ctx.input_mut(|i| i.key_pressed(egui::Key::G)) {
            self.replay();
        }
        if ctx.input_mut(|i| i.key_pressed(egui::Key::P)) {
            self.toggle_autoplay();
        }

        if self.start_time == 0.0 {
            self.start_time = ctx.input(|i| i.time);
        }

        let pointer = ctx.input(|i| i.pointer.clone());
        let is_pointer_down = pointer.any_down();
        let is_pointer_released = pointer.any_released();

        // If mouse is released, record drop position. We will handle it after UI rendering.
        let mut drop_pos = None;
        if is_pointer_released && !self.dragged_cards.is_empty() {
            drop_pos = pointer.interact_pos();
        }

        // --- UI rendering ---
        egui::TopBottomPanel::bottom("toolbar").show(ctx, |ui| {
            self.draw_toolbar(ui, ctx);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.style_mut().spacing.item_spacing = Vec2::splat(CARD_PADDING);

            // Draw top area (stock, waste pile, foundation piles, control buttons)
            ui.horizontal(|ui| {
                self.draw_stock(ui);
                self.draw_waste(ui);
                ui.add_space(
                    CARD_SIZE.x + CARD_PADDING
                        - (self.board.draw_count.saturating_sub(1)) as f32 * WASTE_CARD_H_OFFSET,
                );
                for i in 0..4 {
                    self.draw_foundation(ui, i);
                }
            });

            ui.add_space(CARD_PADDING);

            // Draw tableau piles
            ui.horizontal_top(|ui| {
                for i in 0..7 {
                    self.draw_tableau_pile(ui, i);
                }
            });

            // If dragging, draw dragged cards on top layer
            if !self.dragged_cards.is_empty()
                && let Some(drag_pos) = pointer.interact_pos()
            {
                self.draw_dragged_cards(ctx, drag_pos + self.drag_offset);
            }
        });

        self.update_and_draw_animations(ctx);

        if !self.animations.is_empty() {
            ctx.request_repaint();
            return;
        }

        if self.hook_moved {
            self.handle_moved(ctx);
        }

        if self.autoplay {
            self.handle_autoplay(ctx);
            ctx.request_repaint();
            return;
        }

        if let Some(pos) = drop_pos {
            self.handle_drop(pos);
        }

        // If mouse is not pressed, ensure no cards are being dragged
        if !is_pointer_down && !self.dragged_cards.is_empty() {
            self.return_dragged_cards();
        }

        if self.score == 52 {
            self.popup_win(ctx);
        } else {
            match self.autofinish {
                Autofinish::Asking => {
                    self.popup_autofinish(ctx);
                }
                Autofinish::InProgress => {
                    self.autofinish_step(ctx);
                }
                _ => {}
            }
        }

        ctx.request_repaint();
    }
}

impl KlondikeApp {
    pub fn new(board: Board) -> Self {
        Self {
            init_board: board.clone(),
            board,
            solution: None,
            foundation_rects: [Rect::ZERO; 4],
            tableau_rects: [Rect::ZERO; 7],
            stock_rect: Rect::ZERO,
            waste_rect: Rect::ZERO,

            dragged_cards: Vec::new(),
            drag_source: None,
            drag_offset: Vec2::ZERO,

            animations: Vec::new(),

            history: Vec::new(),
            redo_stack: Vec::new(),

            autofinish: Autofinish::Idle,
            hook_moved: false,
            score: 0,
            start_time: 0.0,
            end_time: None,

            autoplay: false,
            next_play_time: 0.0,
        }
    }

    /// Solve the current game with the given moves
    pub fn solve(&mut self, moves: Vec<SolutionMove>) {
        self.solution = Some((moves, 0, None));
        self.autoplay = true;
    }

    /// Renew the game
    pub fn renew(&mut self) {
        let board = Board::new(rand::random(), self.board.draw_count);
        *self = Self::new(board);
    }

    /// Replay the game
    pub fn replay(&mut self) {
        let solution = self.solution.take();
        *self = Self::new(self.init_board.clone());
        if let Some((moves, _, _)) = solution {
            self.solve(moves);
        }
    }

    /// Draw a card in the specified rectangle
    fn paint_card(painter: &egui::Painter, rect: Rect, card: &Card) {
        let bg_color = if card.face_up {
            Color32::from_gray(248)
        } else {
            Color32::from_rgb(0, 128, 128)
        };
        painter.rect_filled(rect, CornerRadius::same(5), bg_color);
        painter.rect_stroke(
            rect,
            CornerRadius::same(5),
            Stroke::new(1.0, Color32::from_gray(100)),
            StrokeKind::Inside,
        );

        if card.face_up {
            let text_color = card.color();
            let (rank_symbol, suit_symbol) = card.symbols();
            let rank_symbol = if rank_symbol == 'T' {
                "10".to_string()
            } else {
                rank_symbol.to_string()
            };
            let font_id = egui::FontId::proportional(20.0);
            let padding = Vec2::new(3.0, 3.0);

            painter.text(
                rect.min + padding,
                egui::Align2::LEFT_TOP,
                rank_symbol.clone(),
                font_id.clone(),
                text_color,
            );
            painter.text(
                Pos2::new(rect.max.x - padding.x, rect.min.y + padding.y),
                egui::Align2::RIGHT_TOP,
                suit_symbol,
                font_id.clone(),
                text_color,
            );
            painter.text(
                Pos2::new(rect.min.x + padding.x, rect.max.y - padding.y),
                egui::Align2::LEFT_BOTTOM,
                suit_symbol,
                font_id.clone(),
                text_color,
            );
            painter.text(
                rect.max - padding,
                egui::Align2::RIGHT_BOTTOM,
                rank_symbol,
                font_id,
                text_color,
            );
        }
    }

    /// Draw an empty pile placeholder in the specified rectangle
    fn paint_empty_pile(painter: &egui::Painter, rect: Rect) {
        painter.rect_stroke(
            rect,
            CornerRadius::same(5),
            Stroke::new(1.0, Color32::from_gray(100)),
            StrokeKind::Inside,
        );
    }

    /// Draw stock pile
    fn draw_stock(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(CARD_SIZE, Sense::click());
        self.stock_rect = rect;

        if response.clicked() && self.animations.is_empty() {
            if self.board.stock.is_empty() {
                if !self.board.waste.is_empty() {
                    self.apply_and_record_move(
                        ui.ctx(),
                        self.build_game_move(PileId::Waste, PileId::Stock, self.board.waste.len()),
                    );
                }
            } else {
                let draw_count = self.board.draw_count.min(self.board.stock.len());
                if draw_count > 0 {
                    self.apply_and_record_move(
                        ui.ctx(),
                        self.build_game_move(PileId::Stock, PileId::Waste, draw_count),
                    );
                }
            }
        }

        let painter = ui.painter_at(rect);
        if self.board.stock.is_empty() {
            Self::paint_empty_pile(&painter, rect);
        } else {
            Self::paint_card(&painter, rect, &Card::new_with_id(0));
        }
    }

    /// Draw waste pile
    fn draw_waste(&mut self, ui: &mut egui::Ui) {
        let waste_width =
            CARD_SIZE.x + (self.board.draw_count.saturating_sub(1)) as f32 * WASTE_CARD_H_OFFSET;
        let (_, rect) = ui.allocate_space(Vec2::new(waste_width, CARD_SIZE.y));
        self.waste_rect = rect;

        if self.board.waste.is_empty() {
            return;
        }

        let waste_len = self.board.waste.len();
        let draw_count = self.board.draw_count.min(waste_len);
        let start_idx = waste_len - draw_count;

        let mut top_card_rect = Rect::ZERO;

        for i in 0..draw_count {
            let card_idx = start_idx + i;
            let card = self.board.waste[card_idx];
            let card_pos = self.get_card_pos(PileId::Waste, Some(i));
            let card_rect = Rect::from_min_size(card_pos, CARD_SIZE);
            Self::paint_card(ui.painter(), card_rect, &card);
            if i == draw_count - 1 {
                top_card_rect = card_rect;
            }
        }

        let top_card_idx = waste_len - 1;
        let top_card_response = ui.interact(
            top_card_rect,
            Id::new("waste_top_card"),
            Sense::click_and_drag(),
        );

        if top_card_response.clicked() {
            let source = PileId::Waste;
            if !self.try_auto_move_to_foundation(ui.ctx(), source, top_card_idx) {
                self.try_auto_move_to_tableau(ui.ctx(), source, top_card_idx);
            }
        }

        if top_card_response.drag_started()
            && self.dragged_cards.is_empty()
            && self.animations.is_empty()
        {
            self.start_drag(PileId::Waste, top_card_idx, &top_card_response);
        }
    }

    /// Draw foundation pile
    fn draw_foundation(&mut self, ui: &mut egui::Ui, i: usize) {
        let (rect, response) = ui.allocate_exact_size(CARD_SIZE, Sense::drag());
        self.foundation_rects[i] = rect;
        let painter = ui.painter_at(rect);

        if let Some(&card) = self.board.foundations[i].last() {
            Self::paint_card(&painter, rect, &card);

            if response.drag_started()
                && self.dragged_cards.is_empty()
                && self.animations.is_empty()
            {
                self.start_drag(
                    PileId::Foundation(i),
                    self.board.foundations[i].len() - 1,
                    &response,
                );
            }
        } else {
            Self::paint_empty_pile(&painter, rect);
        }
    }

    /// Draw tableau pile
    fn draw_tableau_pile(&mut self, ui: &mut egui::Ui, i: usize) {
        let pile = self.board.tableaus[i].clone();

        let pile_height = if pile.is_empty() {
            CARD_SIZE.y
        } else {
            CARD_SIZE.y + (pile.len() - 1) as f32 * TABLEAU_CARD_V_OFFSET
        };

        let (_, pile_rect) = ui.allocate_space(Vec2::new(CARD_SIZE.x, pile_height));
        self.tableau_rects[i] = pile_rect;

        if pile.is_empty() {
            Self::paint_empty_pile(ui.painter(), pile_rect);
        } else {
            for (j, card) in pile.iter().enumerate() {
                let card_pos = self.get_card_pos(PileId::Tableau(i), Some(j));
                let card_rect = Rect::from_min_size(card_pos, CARD_SIZE);

                if card.face_up {
                    let response = ui.interact(
                        card_rect,
                        Id::new(("tableau", i, j)),
                        Sense::click_and_drag(),
                    );

                    if response.clicked() {
                        let source = PileId::Tableau(i);
                        let ctx = ui.ctx();
                        if !self.try_auto_move_to_foundation(ctx, source, j) {
                            self.try_auto_move_to_tableau(ctx, source, j);
                        }
                    }

                    if response.drag_started()
                        && self.dragged_cards.is_empty()
                        && self.animations.is_empty()
                    {
                        self.start_drag(PileId::Tableau(i), j, &response);
                    }
                }
                Self::paint_card(ui.painter(), card_rect, card);
            }
        }
    }

    /// Draw dragged cards on top layer
    fn draw_dragged_cards(&self, ctx: &egui::Context, pos: Pos2) {
        let layer_id = LayerId::new(Order::Tooltip, Id::new("drag_layer"));
        let painter = ctx.layer_painter(layer_id);

        for (i, card) in self.dragged_cards.iter().enumerate() {
            let card_pos = pos + Vec2::new(0.0, i as f32 * TABLEAU_CARD_V_OFFSET);
            let card_rect = Rect::from_min_size(card_pos, CARD_SIZE);
            Self::paint_card(&painter, card_rect, card);
        }
    }

    /// Draw toolbar
    fn draw_toolbar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.menu_button("☰", |ui| {
                if ui
                    .add(egui::Button::new("New Game").shortcut_text("N"))
                    .clicked()
                {
                    self.renew();
                    ui.close();
                }
                if ui
                    .add(egui::Button::new("Replay Game").shortcut_text("G"))
                    .clicked()
                {
                    self.replay();
                    ui.close();
                }
                ui.separator();
                let undo_button = egui::Button::new("Undo").shortcut_text("Z");
                if ui
                    .add_enabled(!self.history.is_empty(), undo_button)
                    .clicked()
                {
                    self.undo(ui.ctx());
                    ui.close();
                }
                let redo_button = egui::Button::new("Redo").shortcut_text("X");
                if ui
                    .add_enabled(!self.redo_stack.is_empty(), redo_button)
                    .clicked()
                {
                    self.redo(ui.ctx());
                    ui.close();
                }
            });

            if self.solution.is_some() {
                let autoplay_button = egui::Button::new(if self.autoplay { "⏸" } else { "▶" });
                let hover_text = if self.autoplay {
                    "Pause Autoplay (P)"
                } else {
                    "Resume Autoplay (P)"
                };
                if ui.add(autoplay_button).on_hover_text(hover_text).clicked() {
                    self.toggle_autoplay();
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("Score: {}", self.score));
                ui.separator();
                ui.label(format!("Moves: {}", self.history.len()));
                ui.separator();
                let time = if let Some(end_time) = self.end_time {
                    end_time - self.start_time
                } else {
                    ctx.input(|i| i.time) - self.start_time
                };
                let minutes = (time / 60.0).floor() as u32;
                let seconds = (time % 60.0).floor() as u32;
                ui.label(format!("Time: {:02}:{:02}", minutes.min(99), seconds));
            });
        });
    }

    /// Update and draw card animations
    fn update_and_draw_animations(&mut self, ctx: &egui::Context) {
        if self.animations.is_empty() {
            return;
        }

        let now = ctx.input(|i| i.time);
        let mut finished_animations = Vec::new();

        let layer_id = LayerId::new(Order::Tooltip, Id::new("animation_layer"));
        let painter = ctx.layer_painter(layer_id);

        for (idx, anim) in self.animations.iter().enumerate() {
            let elapsed = now - anim.start_time;
            let progress = (elapsed / anim.duration).min(1.0);

            let t = 1.0 - (1.0 - progress).powi(3);
            let x = egui::lerp(anim.start_pos.x..=anim.end_pos.x, t as f32);
            let y = egui::lerp(anim.start_pos.y..=anim.end_pos.y, t as f32);
            let current_pos = Pos2::new(x, y);
            let card_rect = Rect::from_min_size(current_pos, CARD_SIZE);

            Self::paint_card(&painter, card_rect, &anim.card);

            if progress >= 1.0 {
                finished_animations.push(idx);
            }
        }

        for &idx in finished_animations.iter() {
            let anim = &self.animations[idx];
            let card = anim.card;

            match anim.destination {
                PileId::Foundation(i) => self.board.foundations[i].push(card),
                PileId::Tableau(i) => self.board.tableaus[i].push(card),
                PileId::Waste => self.board.waste.push(card),
                PileId::Stock => self.board.stock.push(card),
            }

            if !anim.reverse {
                self.try_flip_tableau_top_card(anim.source);
            }
        }

        for &idx in finished_animations.iter().rev() {
            self.animations.remove(idx);
        }

        self.hook_moved = true;
    }

    /// Apply a move and record it in history
    fn apply_and_record_move(&mut self, ctx: &egui::Context, game_move: GameMove) {
        self.history.push(game_move.clone());
        self.redo_stack.clear();
        self.apply_move(ctx, game_move, false);
    }

    /// Undo the last move
    fn undo(&mut self, ctx: &egui::Context) {
        if self.animations.is_empty()
            && let Some(last_move) = self.history.pop()
        {
            self.apply_move(ctx, last_move.clone(), true);
            self.redo_stack.push(last_move);
        }
    }

    /// Redo the last undone move
    fn redo(&mut self, ctx: &egui::Context) {
        if self.animations.is_empty()
            && let Some(move_to_redo) = self.redo_stack.pop()
        {
            self.history.push(move_to_redo.clone());
            self.apply_move(ctx, move_to_redo, false);
        }
    }

    /// Execute a game move (for new moves and redo)
    fn apply_move(&mut self, ctx: &egui::Context, game_move: GameMove, reverse: bool) {
        let GameMove {
            source,
            destination,
            count,
            source_flip,
        } = game_move;
        let cards = match reverse {
            false => self.take_cards(source, count),
            true => self.take_cards(destination, count),
        };
        let cards_len = cards.len();
        if reverse
            && source_flip
            && let PileId::Tableau(source_idx) = source
        {
            let pile = &mut self.board.tableaus[source_idx];
            let pile_len = pile.len();
            pile[pile_len - 1].face_up = false;
        }
        let create_animation = |(card, start_pos, end_pos)| {
            let (start_pos, end_pos, source, destination) = if reverse {
                (end_pos, start_pos, destination, source)
            } else {
                (start_pos, end_pos, source, destination)
            };
            CardAnimation {
                card,
                start_pos,
                end_pos,
                start_time: ctx.input(|i| i.time),
                duration: 0.2,
                source,
                destination,
                reverse,
            }
        };
        let animations: Vec<_> = match (source, destination) {
            (PileId::Stock, PileId::Waste) => {
                let draw_count = (cards_len + self.board.waste.len()).min(self.board.draw_count);
                cards
                    .into_iter()
                    .rev()
                    .enumerate()
                    .map(|(i, mut card)| {
                        let offset = if reverse {
                            draw_count - 1 - i
                        } else {
                            draw_count + i - cards_len
                        };
                        card.face_up = !reverse;
                        (
                            card,
                            self.get_card_pos(source, None),
                            self.get_card_pos(destination, Some(offset)),
                        )
                    })
                    .map(create_animation)
                    .collect()
            }
            (PileId::Waste, PileId::Stock) => cards
                .into_iter()
                .rev()
                .enumerate()
                .map(|(i, mut card)| {
                    card.face_up = reverse;
                    let limit = cards_len.min(self.board.draw_count);
                    let offset = if reverse {
                        limit.saturating_sub(cards_len - i)
                    } else {
                        limit.saturating_sub(i + 1)
                    };
                    (
                        card,
                        self.get_card_pos(source, Some(offset)),
                        self.get_card_pos(destination, None),
                    )
                })
                .map(create_animation)
                .collect(),
            (_, PileId::Foundation(_)) => {
                let card = cards[0];
                let start_pos = match source {
                    PileId::Waste => self.get_card_pos(
                        source,
                        Some(self.board.waste.len().min(self.board.draw_count - 1)),
                    ),
                    PileId::Tableau(source_idx) => {
                        self.get_card_pos(source, Some(self.board.tableaus[source_idx].len()))
                    }
                    _ => unreachable!(),
                };
                vec![create_animation((
                    card,
                    start_pos,
                    self.get_card_pos(destination, None),
                ))]
            }
            (_, PileId::Tableau(dest_idx)) => cards
                .into_iter()
                .enumerate()
                .map(|(i, card)| {
                    let start_pos = match source {
                        PileId::Waste => self.get_card_pos(
                            source,
                            Some(self.board.waste.len().min(self.board.draw_count - 1)),
                        ),
                        PileId::Foundation(_) => self.get_card_pos(source, None),
                        PileId::Tableau(source_idx) => {
                            let pile = &self.board.tableaus[source_idx];
                            self.get_card_pos(source, Some(pile.len() + i))
                        }
                        _ => unreachable!(),
                    };
                    (
                        card,
                        start_pos,
                        self.get_card_pos(
                            destination,
                            Some(self.board.tableaus[dest_idx].len() + i),
                        ),
                    )
                })
                .map(create_animation)
                .collect(),
            _ => vec![],
        };
        self.animations.extend(animations);
    }

    /// Start dragging
    fn start_drag(&mut self, source: PileId, card_idx: usize, response: &egui::Response) {
        let cards_to_drag = match source {
            PileId::Waste => self.board.waste.pop().map(|c| vec![c]),
            PileId::Foundation(i) => self.board.foundations[i].pop().map(|c| vec![c]),
            PileId::Tableau(i) => {
                let pile = &mut self.board.tableaus[i];
                if card_idx < pile.len() && pile[card_idx].face_up {
                    Some(pile.drain(card_idx..).collect())
                } else {
                    None
                }
            }
            PileId::Stock => unreachable!(),
        };

        if let Some(cards) = cards_to_drag {
            self.dragged_cards = cards;
            self.drag_source = Some(source);
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let card_rect = response.rect;
                self.drag_offset = card_rect.min - pointer_pos;
            }
        }
    }

    /// Handle card drop
    fn handle_drop(&mut self, drop_pos: Pos2) {
        let mut drop_target: Option<PileId> = None;

        for i in 0..4 {
            if self.foundation_rects[i].contains(drop_pos) && self.can_place_on_foundation(i) {
                drop_target = Some(PileId::Foundation(i));
                break;
            }
        }
        if drop_target.is_none() {
            for i in 0..7 {
                if self.tableau_rects[i].contains(drop_pos) && self.can_place_on_tableau(i) {
                    drop_target = Some(PileId::Tableau(i));
                    break;
                }
            }
        }

        match (self.drag_source, drop_target) {
            (Some(source), Some(destination)) => {
                let game_move = self.build_game_move(source, destination, self.dragged_cards.len());
                self.history.push(game_move);
                self.redo_stack.clear();

                match destination {
                    PileId::Foundation(i) => {
                        self.board.foundations[i].append(&mut self.dragged_cards)
                    }
                    PileId::Tableau(i) => self.board.tableaus[i].append(&mut self.dragged_cards),
                    _ => unreachable!(),
                }

                self.try_flip_tableau_top_card(source);
                self.dragged_cards.clear();
                self.drag_source = None;

                self.hook_moved = true;
            }
            _ => {
                self.return_dragged_cards();
            }
        }
    }

    /// Return dragged cards to original place
    fn return_dragged_cards(&mut self) {
        if let Some(source) = self.drag_source.take() {
            let mut cards = std::mem::take(&mut self.dragged_cards);
            match source {
                PileId::Waste => self.board.waste.append(&mut cards),
                PileId::Foundation(i) => self.board.foundations[i].append(&mut cards),
                PileId::Tableau(i) => self.board.tableaus[i].append(&mut cards),
                PileId::Stock => unreachable!(),
            }
        }
    }

    fn popup_win(&mut self, ctx: &egui::Context) {
        egui::Window::new("Victory")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label("Congratulations, you won the game!");
                    if ui.button("Play Again").clicked() {
                        self.renew();
                    }
                });
            });
    }

    fn popup_autofinish(&mut self, ctx: &egui::Context) {
        egui::Window::new("Autofinish")
            .collapsible(false)
            .resizable(false)
            .fixed_size([360.0, 60.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("All cards are face up and in order, do you want to autofinish?");
                ui.add_space(10.0);
                ui.columns(2, |columns| {
                    columns[0].with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.add_space(40.);
                            if ui.button("Yes").clicked() {
                                self.autofinish = Autofinish::InProgress;
                            }
                        },
                    );
                    columns[1].with_layout(
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.add_space(40.);
                            if ui.button("No").clicked() {
                                self.autofinish = Autofinish::Rejected;
                            }
                        },
                    );
                });
            });
    }

    /// Perform one autofinish step
    fn autofinish_step(&mut self, ctx: &egui::Context) {
        let waste_len = self.board.waste.len();
        if waste_len > 0 && self.try_auto_move_to_foundation(ctx, PileId::Waste, waste_len - 1) {
            return;
        }

        for i in 0..7 {
            let pile_len = self.board.tableaus[i].len();
            if pile_len > 0
                && self.try_auto_move_to_foundation(ctx, PileId::Tableau(i), pile_len - 1)
            {
                return;
            }
        }

        self.autofinish = Autofinish::Succeed;
    }

    fn handle_autoplay(&mut self, ctx: &egui::Context) {
        let Some((moves, index, board)) = self.solution.as_mut() else {
            self.autoplay = false;
            return;
        };

        match board {
            Some(board) => {
                if &self.board != board {
                    self.autoplay = false;
                    return;
                }
            }
            None => {
                *board = Some(self.board.clone());
            }
        }

        let now = ctx.input(|i| i.time);

        if self.next_play_time == 0.0 {
            self.next_play_time = now + AUTOPLAY_INTERVAL;
        }

        if now < self.next_play_time {
            return;
        }

        let Some((from, to, count)) = moves.get(*index).cloned() else {
            return;
        };
        let mut factor = 1.0;
        match (from, to) {
            (PileId::Stock, PileId::Waste) => {
                let count = self.board.draw_count.min(self.board.stock.len());
                self.apply_and_record_move(ctx, self.build_game_move(from, to, count));
                factor = 0.75;
            }
            (PileId::Waste, PileId::Stock) => {
                self.apply_and_record_move(
                    ctx,
                    self.build_game_move(from, to, self.board.waste.len()),
                );
                factor = 0.75;
            }
            _ => {
                if let (PileId::Tableau(from_idx), PileId::Tableau(to_idx)) = (from, to) {
                    factor = 1.25;

                    // If the card being moved is not the topmost face-up card in the pile, increase the factor
                    let pile = &self.board.tableaus[from_idx];
                    let card_idx = pile.len() - count;
                    let card = pile.get(card_idx);
                    if card_idx > 0
                        && let Some(true) = pile.get(card_idx - 1).map(|c| c.face_up)
                    {
                        factor += 0.5;
                    }

                    // If the tableau being moved to is not the leftmost one, increase the factor
                    if let Some(card) = card {
                        let mut has_other_place = false;
                        for idx in 0..to_idx {
                            if idx == from_idx {
                                continue;
                            }
                            if self.can_place_card_on_tableau(idx, card) {
                                has_other_place = true;
                                break;
                            }
                        }
                        if has_other_place {
                            factor += 0.5;
                        }
                    }
                }
                self.apply_and_record_move(ctx, self.build_game_move(from, to, count));
            }
        }
        if let Some((_, index, board)) = self.solution.as_mut() {
            *index += 1;
            *board = None;
        }

        self.next_play_time = now + AUTOPLAY_INTERVAL * factor;
    }

    fn toggle_autoplay(&mut self) {
        if self.solution.is_none() {
            return;
        }
        self.autoplay = !self.autoplay;
        if self.autoplay {
            self.next_play_time = 0.0;
        }
    }

    fn handle_moved(&mut self, ctx: &egui::Context) {
        let score = self.board.score();
        let is_win = score == 52;
        if is_win {
            if self.end_time.is_none() {
                self.end_time = Some(ctx.input(|i| i.time));
            }
        } else if !self.autoplay
            && matches!(self.autofinish, Autofinish::Idle)
            && self.board.can_autofinish()
        {
            self.autofinish = Autofinish::Asking;
        }
        self.score = score;
        self.hook_moved = false;
    }

    /// Try to auto-move card to foundation pile
    fn try_auto_move_to_foundation(
        &mut self,
        ctx: &egui::Context,
        source: PileId,
        card_idx: usize,
    ) -> bool {
        let card_to_move = match source {
            PileId::Waste => match self.board.waste.last() {
                None => return false,
                Some(card) => *card,
            },
            PileId::Tableau(i) => {
                let pile = &self.board.tableaus[i];
                match pile.get(card_idx) {
                    None => return false,
                    Some(card) => {
                        if !card.face_up || card_idx != pile.len() - 1 {
                            return false;
                        }
                        *card
                    }
                }
            }
            _ => return false,
        };

        for i in 0..4 {
            if self.can_place_card_on_foundation(i, &card_to_move) {
                self.apply_and_record_move(
                    ctx,
                    self.build_game_move(source, PileId::Foundation(i), 1),
                );
                return true;
            }
        }
        false
    }

    /// Try to auto-move card to tableau pile
    fn try_auto_move_to_tableau(
        &mut self,
        ctx: &egui::Context,
        source: PileId,
        card_idx: usize,
    ) -> bool {
        let (top_card_to_move, count) = match source {
            PileId::Waste => match self.board.waste.last() {
                None => return false,
                Some(card) => (*card, 1),
            },
            PileId::Tableau(i) => {
                let pile = &self.board.tableaus[i];
                match pile.get(card_idx) {
                    None => return false,
                    Some(card) => (*card, pile.len() - card_idx),
                }
            }
            _ => return false,
        };

        for i in 0..7 {
            if let PileId::Tableau(source_idx) = source
                && i == source_idx
            {
                continue;
            }
            if self.can_place_card_on_tableau(i, &top_card_to_move) {
                self.apply_and_record_move(
                    ctx,
                    self.build_game_move(source, PileId::Tableau(i), count),
                );
                return true;
            }
        }
        false
    }

    fn try_flip_tableau_top_card(&mut self, source: PileId) {
        if let PileId::Tableau(i) = source
            && let Some(card) = self.board.tableaus[i].last_mut()
        {
            card.face_up = true;
        }
    }

    fn take_cards(&mut self, source: PileId, count: usize) -> Vec<Card> {
        match source {
            PileId::Stock => {
                let start_idx = self.board.stock.len() - count;
                self.board.stock.drain(start_idx..).collect()
            }
            PileId::Waste => {
                let start_idx = self.board.waste.len() - count;
                self.board.waste.drain(start_idx..).collect()
            }
            PileId::Foundation(i) => {
                let start_idx = self.board.foundations[i].len() - count;
                self.board.foundations[i].drain(start_idx..).collect()
            }
            PileId::Tableau(i) => {
                let start_idx = self.board.tableaus[i].len() - count;
                self.board.tableaus[i].drain(start_idx..).collect()
            }
        }
    }

    fn get_card_pos(&self, pile_id: PileId, offset: Option<usize>) -> Pos2 {
        let offset = offset.unwrap_or(0);
        match pile_id {
            PileId::Stock => self.stock_rect.min,
            PileId::Waste => {
                self.waste_rect.min + Vec2::new(offset as f32 * WASTE_CARD_H_OFFSET, 0.0)
            }
            PileId::Foundation(i) => self.foundation_rects[i].min,
            PileId::Tableau(i) => {
                self.tableau_rects[i].min + Vec2::new(0.0, offset as f32 * TABLEAU_CARD_V_OFFSET)
            }
        }
    }

    fn build_game_move(&self, source: PileId, destination: PileId, count: usize) -> GameMove {
        let mut source_flip = false;
        if let PileId::Tableau(source_idx) = source {
            let pile = &self.board.tableaus[source_idx];
            let pile_len = pile.len();
            if pile_len > count {
                source_flip = !pile[pile_len - count - 1].face_up;
            }
        }
        GameMove {
            source,
            destination,
            count,
            source_flip,
        }
    }

    fn can_place_on_foundation(&self, foundation_idx: usize) -> bool {
        if self.dragged_cards.len() != 1 {
            return false;
        }
        let card = self.dragged_cards.first().unwrap();
        self.can_place_card_on_foundation(foundation_idx, card)
    }

    fn can_place_card_on_foundation(&self, foundation_idx: usize, card: &Card) -> bool {
        let foundation = &self.board.foundations[foundation_idx];
        match foundation.last() {
            None => card.is_ace(),
            Some(top_card) => top_card.suit() == card.suit() && card.rank() == top_card.rank() + 1,
        }
    }

    fn can_place_on_tableau(&self, tableau_idx: usize) -> bool {
        let card = match self.dragged_cards.first() {
            Some(c) => c,
            None => return false,
        };
        self.can_place_card_on_tableau(tableau_idx, card)
    }

    fn can_place_card_on_tableau(&self, tableau_idx: usize, card: &Card) -> bool {
        let tableau_pile = &self.board.tableaus[tableau_idx];
        match tableau_pile.last() {
            None => card.is_king(),
            Some(top_card) => {
                top_card.face_up
                    && top_card.color() != card.color()
                    && top_card.rank() == card.rank() + 1
            }
        }
    }
}
