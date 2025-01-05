extern crate wasm_bindgen;
extern crate web_sys;
extern crate rand;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement, MouseEvent, window};
use wasm_bindgen::closure::Closure;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug, Clone)]
struct Card {
    rank: String,
    suit: String,
    face_up: bool,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Card {
    fn new(rank: &str, suit: &str) -> Self {
        Card {
            rank: rank.to_string(),
            suit: suit.to_string(),
            face_up: false,
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 150.0,
        }
    }

    fn draw(&self, ctx: &CanvasRenderingContext2d) {
        let img = HtmlImageElement::new().unwrap();
        let src = if self.face_up {
            format!("./sprites/{}/{}.jpg", self.suit, self.rank)
        } else {
            "./sprites/cover/cover.jpg".to_string()
        };

        img.set_src(&src);

        let img_clone = img.clone();
        let ctx_clone = ctx.clone();
        let x = self.x;
        let y = self.y;

        let closure = Closure::wrap(Box::new(move || {
            ctx_clone
                .draw_image_with_html_image_element(&img_clone, x, y)
                .unwrap();
        }) as Box<dyn FnMut()>);

        img.set_onload(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
    }

    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}

struct GameState {
    tableau: Vec<Vec<Card>>, // 7 tableau piles
    foundation: Vec<Vec<Card>>, // 4 foundation piles
    stock: Vec<Card>,
    waste: Vec<Card>,
    selected_card: Option<(Card, usize, usize)>, // (Card, source pile index, source type)
    dragging_card: Option<(Card, f64, f64)>, // (Card, mouse offset X, mouse offset Y)
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
}

impl GameState {
    fn create_deck() -> Vec<Card> {
        let suits = vec!["hearts", "diamonds", "clubs", "spades"];
        let ranks = vec!["A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"];
        let mut deck = Vec::new();
    
        for suit in suits {
            for rank in &ranks { // Borrow `ranks` to avoid moving it
                deck.push(Card::new(rank, suit));
            }
        }
    
        deck
    }

    fn can_move_to_tableau(&self, card: &Card, tableau_index: usize) -> bool {
        // Placeholder: allow moving to empty piles or valid stacks
        if let Some(last_card) = self.tableau[tableau_index].last() {
            // Logic to check alternating color and descending rank
            let is_alternating_color = (last_card.suit == "hearts" || last_card.suit == "diamonds")
                != (card.suit == "hearts" || card.suit == "diamonds");
            let is_descending_rank = card.rank == "Q"; // Replace with proper rank comparison
            is_alternating_color && is_descending_rank
        } else {
            // Allow moving Kings to empty piles
            card.rank == "K"
        }
    }

    fn can_move_to_foundation(&self, card: &Card, foundation_index: usize) -> bool {
        if let Some(last_card) = self.foundation[foundation_index].last() {
            // Logic to check matching suit and ascending rank
            last_card.suit == card.suit && card.rank == "2" // Replace with proper rank comparison
        } else {
            // Allow moving Aces to empty foundation piles
            card.rank == "A"
        }
    }

    fn new(ctx: CanvasRenderingContext2d, canvas: HtmlCanvasElement) -> Self {
        let mut deck = GameState::create_deck();
        deck.shuffle(&mut thread_rng());

        let mut tableau = vec![vec![]; 7];
        for i in 0..7 {
            for j in 0..=i {
                let mut card = deck.pop().unwrap();
                card.face_up = j == i;
                tableau[i].push(card);
            }
        }

        GameState {
            tableau,
            foundation: vec![vec![]; 4],
            stock: deck,
            waste: vec![],
            selected_card: None,
            canvas,
            ctx,
        }
    }

    fn render(&mut self) {
        self.ctx.clear_rect(0.0, 0.0, self.canvas.width() as f64, self.canvas.height() as f64);

        // Render tableau piles
        for (i, pile) in self.tableau.iter_mut().enumerate() {
            for (j, card) in pile.iter_mut().enumerate() {
                card.x = 20.0 + i as f64 * 120.0;
                card.y = 200.0 + j as f64 * 30.0;
                card.draw(&self.ctx);
            }
        }

        // Render foundation piles
        for (i, pile) in self.foundation.iter().enumerate() {
            if let Some(card) = pile.last() {
                let mut card = card.clone();
                card.x = 500.0 + i as f64 * 120.0;
                card.y = 20.0;
                card.draw(&self.ctx);
            } else {
                #[allow(deprecated)]
                self.ctx.set_stroke_style(&JsValue::from_str("black"));
                self.ctx.set_line_width(2.0);
                self.ctx.stroke_rect(500.0 + i as f64 * 120.0, 20.0, 100.0, 150.0);
            }
        }

        // Render stock
        if let Some(card) = self.stock.last() {
            let mut card = card.clone();
            card.x = 20.0;
            card.y = 20.0;
            card.draw(&self.ctx);
        }

        // Render waste
        for (i, card) in self.waste.iter_mut().enumerate() {
            card.x = 140.0 + i as f64 * 20.0;
            card.y = 20.0;
            card.draw(&self.ctx);
        }

        // Highlight selected card
        if let Some((card, _, _)) = &self.selected_card {
            #[allow(deprecated)]
            self.ctx.set_stroke_style(&JsValue::from_str("yellow"));
            self.ctx.set_line_width(3.0);
            self.ctx.stroke_rect(card.x, card.y, card.width, card.height);
        }
    }

    fn handle_click(&mut self, x: f64, y: f64) {
        // Handle clicks on stock
        if let Some(card) = self.stock.last_mut() {
            if card.contains(x, y) {
                let mut card = self.stock.pop().unwrap();
                card.face_up = true;
                self.waste.push(card);
                self.render();
                return;
            }
        }

        // Handle tableau clicks
        for i in 0..self.tableau.len() {
            if let Some(card) = self.tableau[i].last() {
                if card.contains(x, y) {
                    if let Some((selected_card, source_index, source_type)) = self.selected_card.take() {
                        if self.can_move_to_tableau(&selected_card, i) {
                            self.tableau[i].push(selected_card.clone());
                            if source_type == 0 {
                                self.tableau[source_index].pop();
                            } else if source_type == 1 {
                                self.waste.pop();
                            }
                            self.render();
                            return;
                        }
                    } else {
                        self.selected_card = Some((card.clone(), i, 0)); // 0 indicates tableau
                        self.render();
                        return;
                    }
                }
            }
        }

        // Handle foundation clicks
        for i in 0..self.foundation.len() {
            if let Some(card) = self.foundation[i].last() {
                if card.contains(x, y) {
                    if let Some((selected_card, source_index, source_type)) = self.selected_card.take() {
                        if self.can_move_to_foundation(&selected_card, i) {
                            self.foundation[i].push(selected_card.clone());
                            if source_type == 0 {
                                self.tableau[source_index].pop();
                            } else if source_type == 1 {
                                self.waste.pop();
                            }
                            self.render();
                            return;
                        }
                    }
                }
            }
        }
    }
}


#[wasm_bindgen]
pub fn start() -> Result<(), JsValue> {
    let window = window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .get_element_by_id("gameCanvas")
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()?;
    let ctx = canvas
        .get_context("2d")?
        .unwrap()
        .dyn_into::<CanvasRenderingContext2d>()?;

    let game_state = Rc::new(RefCell::new(GameState::new(ctx, canvas.clone())));

    {
        let game_state = game_state.clone();
        let on_click = Closure::wrap(Box::new(move |event: MouseEvent| {
            let x = event.offset_x() as f64;
            let y = event.offset_y() as f64;
            game_state.borrow_mut().handle_click(x, y);
        }) as Box<dyn FnMut(_)>);

        canvas
            .add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())
            .unwrap();
        on_click.forget();
    }

    game_state.borrow_mut().render();
    Ok(())
}
