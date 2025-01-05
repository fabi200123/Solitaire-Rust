// Existing imports
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

// Card struct remains unchanged
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
    fn new(rank: &str, suit: &str, x: f64, y: f64) -> Self {
        Card {
            rank: rank.to_string(),
            suit: suit.to_string(),
            face_up: false,
            x,
            y,
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
    
        // Clone the image element for use inside the closure
        let img_clone = img.clone();
        let ctx_clone = ctx.clone();
        let x = self.x;
        let y = self.y;
    
        let closure = Closure::wrap(Box::new(move || {
            ctx_clone
                .draw_image_with_html_image_element(&img_clone, x, y)
                .unwrap();
        }) as Box<dyn FnMut()>);
    
        // Set the onload event to trigger the closure
        img.set_onload(Some(closure.as_ref().unchecked_ref()));
        closure.forget(); // Prevent closure from being dropped
    }    

    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}

// Updated GameState to handle initialization and logic
struct GameState {
    tableau: Vec<Vec<Card>>,
    foundation: Vec<Vec<Card>>,
    stock: Vec<Card>,
    waste: Vec<Card>,
    selected_card: Option<(Card, usize, usize)>, // (Card, source pile index, source type)
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
}

impl GameState {
    fn new(ctx: CanvasRenderingContext2d, canvas: HtmlCanvasElement) -> Self {
        web_sys::console::log_1(&"Creating GameState...".into());
    
        let mut deck = GameState::create_deck();
        deck.shuffle(&mut thread_rng());
        web_sys::console::log_1(&format!("Deck created with {} cards.", deck.len()).into());
    
        let mut tableau = vec![vec![]; 7];
        for i in 0..7 {
            for j in 0..=i {
                let mut card = deck.pop().unwrap();
                card.face_up = j == i;
                tableau[i].push(card.clone());
                web_sys::console::log_1(&format!(
                    "Placed card {} of {} in tableau pile {}.",
                    card.rank, card.suit, i
                ).into());
            }
        }
    
        web_sys::console::log_1(&"Tableau initialized.".into());
    
        let stock = deck;
        web_sys::console::log_1(&format!("Stock initialized with {} cards.", stock.len()).into());
    
        GameState {
            tableau,
            foundation: vec![vec![]; 4],
            stock,
            waste: vec![],
            selected_card: None,
            canvas,
            ctx,
        }
    }

    fn create_deck() -> Vec<Card> {
        let suits = vec!["hearts", "diamonds", "clubs", "spades"];
        let ranks = vec![
            "A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K",
        ];
        let mut deck = Vec::new();

        for suit in &suits {
            for rank in &ranks {
                deck.push(Card::new(rank, suit, 0.0, 0.0));
            }
        }

        deck
    }

    fn render(&mut self) {
        web_sys::console::log_1(&"Rendering game state...".into());
    
        self.ctx.clear_rect(0.0, 0.0, self.canvas.width() as f64, self.canvas.height() as f64);
    
        for (i, pile) in self.tableau.iter_mut().enumerate() {
            for (j, card) in pile.iter_mut().enumerate() {
                // Calculate card positions
                card.x = 20.0 + i as f64 * 120.0;
                card.y = 20.0 + j as f64 * 30.0;
    
                // Log rendering details
                web_sys::console::log_1(
                    &format!(
                        "Rendering card: {} of {} at ({}, {})",
                        card.rank, card.suit, card.x, card.y
                    )
                    .into(),
                );
    
                // Draw the card
                card.draw(&self.ctx);
            }
        }
    
        web_sys::console::log_1(&"Finished rendering game state.".into());
    }
    

    fn handle_click(&mut self, x: f64, y: f64) {
        // Handle clicking on stock
        if self.stock.last().map_or(false, |c| c.contains(x, y)) {
            if let Some(card) = self.stock.pop() {
                let mut card = card;
                card.face_up = true;
                self.waste.push(card);
            }
        }
        // Handle tableau clicks
        for (_i, pile) in self.tableau.iter_mut().enumerate() {
            if let Some(card) = pile.last_mut() {
                if card.contains(x, y) {
                    card.face_up = true;
                }
            }
        }

        // Check for win condition
        self.check_win();
    }

    fn check_win(&self) {
        if self.foundation.iter().all(|pile| pile.len() == 13) {
            web_sys::console::log_1(&"You win!".into());
        }
    }
}

// Event handling in `start`
#[wasm_bindgen]
pub fn start() -> Result<(), JsValue> {
    web_sys::console::log_1(&"Starting Solitaire...".into());

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

    web_sys::console::log_1(&"Canvas and context initialized.".into());

    let game_state = Rc::new(RefCell::new(GameState::new(ctx, canvas.clone())));
    web_sys::console::log_1(&"GameState initialized.".into());

    game_state.borrow_mut().render();
    web_sys::console::log_1(&"Render method invoked.".into());

    Ok(())
}
