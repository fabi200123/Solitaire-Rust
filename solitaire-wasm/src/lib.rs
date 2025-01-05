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
    selected_card: Option<(Card, usize, usize)>, // (Card, source pile index, source type)
    dragging_card: Option<(Card, f64, f64, usize, usize)>, // (Card, mouse offset X, mouse offset Y, original pile index, original pile type)
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
}

impl GameState {
    fn create_deck() -> Vec<Card> {
        let suits = vec!["hearts", "diamonds", "clubs", "spades"];
        let ranks = vec!["A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"];
        let mut deck = Vec::new();

        for suit in suits {
            for rank in &ranks {
                deck.push(Card::new(rank, suit));
            }
        }

        deck
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
            selected_card: None,
            dragging_card: None,
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
                card.draw(&self.ctx);
            } else {
                #[allow(deprecated)]
                self.ctx.set_stroke_style(&JsValue::from_str("black"));
                self.ctx.set_line_width(2.0);
                self.ctx.stroke_rect(500.0 + i as f64 * 120.0, 20.0, 100.0, 150.0);
            }
        }

        // Render stock
        if let Some(card) = self.stock.last_mut() {
            card.x = 20.0;
            card.y = 20.0;
            card.draw(&self.ctx);
        }
    }

    fn handle_stock_click(&mut self) {
        if let Some(card) = self.stock.last_mut() {
            card.face_up = true;
            self.render();
        }
    }

    fn handle_mousedown(&mut self, x: f64, y: f64) {
        // Check tableau piles
        for (pile_idx, pile) in self.tableau.iter_mut().enumerate() {
            if let Some(card) = pile.last() {
                if card.contains(x, y) {
                    self.dragging_card = Some((card.clone(), x - card.x, y - card.y, pile_idx, 0));
                    pile.pop();
                    self.render();
                    return;
                }
            }
        }
    }

    fn handle_mousemove(&mut self, x: f64, y: f64) {
        if let Some((ref mut card, offset_x, offset_y, _, _)) = self.dragging_card {
            card.x = x - offset_x;
            card.y = y - offset_y;
            self.render();
        }
    }

    fn handle_mouseup(&mut self, x: f64, y: f64) {
        if let Some((card, _, _, pile_idx, pile_type)) = self.dragging_card.take() {
            let valid_drop = self.try_drop_card(&card, x, y);

            if !valid_drop {
                match pile_type {
                    0 => self.tableau[pile_idx].push(card),
                    _ => {}
                }
            }
            self.render();
        }
    }

    fn try_drop_card(&mut self, card: &Card, x: f64, y: f64) -> bool {
        // Check tableau piles for valid drop
        for pile in self.tableau.iter_mut() {
            if pile.last().map_or(true, |last_card| last_card.contains(x, y)) {
                pile.push(card.clone());
                return true;
            }
        }

        // Check foundation piles for valid drop (add rules for foundation here if needed)
        false
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
        let on_mousedown = Closure::wrap(Box::new(move |event: MouseEvent| {
            let x = event.offset_x() as f64;
            let y = event.offset_y() as f64;
            game_state.borrow_mut().handle_mousedown(x, y);
        }) as Box<dyn FnMut(_)>);

        canvas
            .add_event_listener_with_callback("mousedown", on_mousedown.as_ref().unchecked_ref())
            .unwrap();
        on_mousedown.forget();
    }

    {
        let game_state = game_state.clone();
        let on_mousemove = Closure::wrap(Box::new(move |event: MouseEvent| {
            let x = event.offset_x() as f64;
            let y = event.offset_y() as f64;
            game_state.borrow_mut().handle_mousemove(x, y);
        }) as Box<dyn FnMut(_)>);

        canvas
            .add_event_listener_with_callback("mousemove", on_mousemove.as_ref().unchecked_ref())
            .unwrap();
        on_mousemove.forget();
    }

    {
        let game_state = game_state.clone();
        let on_mouseup = Closure::wrap(Box::new(move |event: MouseEvent| {
            let x = event.offset_x() as f64;
            let y = event.offset_y() as f64;
            game_state.borrow_mut().handle_mouseup(x, y);
        }) as Box<dyn FnMut(_)>);

        canvas
            .add_event_listener_with_callback("mouseup", on_mouseup.as_ref().unchecked_ref())
            .unwrap();
        on_mouseup.forget();
    }

    game_state.borrow_mut().render();
    Ok(())
}
