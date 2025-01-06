extern crate wasm_bindgen;
extern crate web_sys;
extern crate rand;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement, MouseEvent, window};
use wasm_bindgen::closure::Closure;
use rand::thread_rng;
use rand::seq::SliceRandom;
use std::rc::Rc;
use std::cell::RefCell;

const CARD_WIDTH: f64 = 100.0;
const CARD_HEIGHT: f64 = 150.0;
const PILE_GAP: f64 = 50.0;
const CANVAS_WIDTH: f64 = 7.0 * CARD_WIDTH + 20.0 * PILE_GAP; // 7 tableau piles + gaps
const CANVAS_HEIGHT: f64 = 5.0 * CARD_HEIGHT + 10.0 * PILE_GAP; // Enough for stacked tableau cards

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
            width: CARD_WIDTH,
            height: CARD_HEIGHT,
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
    discard: Vec<Card>, // Discard pile
    selected_card: Option<(Card, usize, usize)>, // (Card, source pile index, source type)
    dragging_card: Option<(Vec<Card>, f64, f64, usize, usize)>, // Vec<Card> to store multiple cards
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
}

impl GameState {
    fn draw_background(&self) {
        self.ctx.set_fill_style(&"green".into());
        self.ctx.fill_rect(0.0, 0.0, self.canvas.width() as f64, self.canvas.height() as f64);
    }

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
                card.face_up = j == i; // Only the top card in each pile is face-up
                tableau[i].push(card);
            }
        }
    
        GameState {
            tableau,
            foundation: vec![vec![]; 4],
            stock: deck,
            discard: Vec::new(),
            selected_card: None,
            dragging_card: None,
            canvas,
            ctx,
        }
    }       
    
    fn render(&mut self) {
        self.ctx.clear_rect(0.0, 0.0, self.canvas.width() as f64, self.canvas.height() as f64);
        self.draw_background();
    
        // Render tableau piles with increased vertical spacing
        for (i, pile) in self.tableau.iter_mut().enumerate() {
            for (j, card) in pile.iter_mut().enumerate() {
                card.x = PILE_GAP + i as f64 * (CARD_WIDTH + PILE_GAP);
                card.y = 200.0 + j as f64 * 60.0 + 50.0;
                card.draw(&self.ctx);
            }
        }
    
        // Render foundation piles
        for (i, pile) in self.foundation.iter_mut().enumerate() {
            if let Some(card) = pile.last_mut() {
                card.x = PILE_GAP + 4.5 * CARD_WIDTH + (i as f64 * (CARD_WIDTH + PILE_GAP));
                card.y = PILE_GAP;
                card.draw(&self.ctx);
            } else {
                // Draw empty foundation slots
                self.ctx.set_stroke_style(&JsValue::from_str("black"));
                self.ctx.set_line_width(2.0);
                self.ctx.stroke_rect(
                    PILE_GAP + 4.5 * CARD_WIDTH + (i as f64 * (CARD_WIDTH + PILE_GAP)),
                    PILE_GAP,
                    CARD_WIDTH,
                    CARD_HEIGHT,
                );
            }
        }
    
        // Render stock pile
        if !self.stock.is_empty() {
            if let Some(card) = self.stock.last_mut() {
                card.x = PILE_GAP;
                card.y = PILE_GAP;
                card.draw(&self.ctx);
            }
        } else {
            // Draw empty stock pile placeholder
            self.ctx.set_stroke_style(&JsValue::from_str("black"));
            self.ctx.set_line_width(2.0);
            self.ctx.stroke_rect(PILE_GAP, PILE_GAP, CARD_WIDTH, CARD_HEIGHT);
        }
    
        // Render discard pile
        if let Some(card) = self.discard.last() {
            let mut card = card.clone(); // Clone the last card to modify its position
            card.x = PILE_GAP + CARD_WIDTH + PILE_GAP;
            card.y = PILE_GAP;
            card.draw(&self.ctx);
        }
    }
                    
    fn handle_stock_click(&mut self) {
        if let Some(mut card) = self.stock.pop() {
            // Flip the top card and move it to the discard pile
            card.face_up = true;
            self.discard.push(card); // Add card to the discard pile
            self.render();
        } else if !self.discard.is_empty() {
            // Recycle the discard pile back into the stock pile
            while let Some(mut card) = self.discard.pop() {
                card.face_up = false; // Flip the card face-down
                self.stock.push(card); // Move to the stock pile
            }
            self.render(); // Ensure proper rendering
        }
    }
     
    fn handle_mousedown(&mut self, x: f64, y: f64) {
        // Check tableau piles
        for (pile_idx, pile) in self.tableau.iter_mut().enumerate() {
            if let Some(card_idx) = pile.iter().position(|card| card.contains(x, y)) {
                if pile[card_idx].face_up {
                    let cards_to_drag = pile.split_off(card_idx); // Split off the dragged cards
                    let offset_x = x - cards_to_drag[0].x;
                    let offset_y = y - cards_to_drag[0].y;
                    self.dragging_card = Some((cards_to_drag, offset_x, offset_y, pile_idx, 0)); // Store dragging info
                    self.render();
                }
                return;
            }
        }

        // Check the foundation piles
        for (pile_idx, pile) in self.foundation.iter_mut().enumerate() {
            if let Some(card) = pile.last() {
                let foundation_x = PILE_GAP + 4.5 * CARD_WIDTH + (pile_idx as f64 * (CARD_WIDTH + PILE_GAP)); // Use the same coordinates as in `render`
                let foundation_y = PILE_GAP;
                if x >= foundation_x && x <= foundation_x + CARD_WIDTH && y >= foundation_y && y <= foundation_y + CARD_HEIGHT {
                    // Drag the card from the foundation pile
                    self.dragging_card = Some((vec![card.clone()], x - card.x, y - card.y, pile_idx, 2)); // 2 indicates foundation pile
                    pile.pop(); // Remove the card from the foundation pile
                    self.render();
                    return;
                }
            }
        }

        // Check the stock pile (whether it has cards or is empty)
        if x >= PILE_GAP && x <= PILE_GAP + CARD_WIDTH && y >= PILE_GAP && y <= PILE_GAP + CARD_HEIGHT {
            self.handle_stock_click(); // Trigger stock pile logic
            return;
        }
    
        // Check the discard pile
        if let Some(card) = self.discard.last() {
            let discard_x = PILE_GAP + CARD_WIDTH + PILE_GAP; // Use the same coordinates as in `render`
            let discard_y = PILE_GAP;
            if x >= discard_x && x <= discard_x + CARD_WIDTH && y >= discard_y && y <= discard_y + CARD_HEIGHT {
                self.dragging_card = Some((vec![card.clone()], x - card.x, y - card.y, 0, 1)); // 1 indicates discard pile
                self.discard.pop(); // Remove card from the discard pile
                self.render();
                return;
            }
        }
    }
                  
    fn handle_mousemove(&mut self, x: f64, y: f64) {
        if let Some((ref mut cards, offset_x, offset_y, _, _)) = self.dragging_card {
            // Clear the previous position of the dragged cards
            for card in cards.iter() {
                self.ctx.clear_rect(card.x, card.y, CARD_WIDTH, CARD_HEIGHT);
            }
    
            // Update the position of the dragged cards
            for (i, card) in cards.iter_mut().enumerate() {
                card.x = x - offset_x;
                card.y = y - offset_y + i as f64 * 30.0; // Offset for stacked cards
            }
    
            // Draw only the dragged cards at their new positions
            for card in cards.iter() {
                card.draw(&self.ctx);
            }
        }
    }      
    
    fn handle_mouseup(&mut self, x: f64, y: f64) {
        if let Some((mut cards, _, _, source_pile_idx, source_pile_type)) = self.dragging_card.take() {
            let valid_drop = if cards.len() == 1 {
                self.try_drop_card(&cards[0], x, y) // Check for a valid drop of a single card
            } else {
                self.try_drop_stack(&cards, x, y) // Check for a valid drop of a stack
            };
    
            if !valid_drop {
                // Return the cards to their original pile if the drop is invalid
                match source_pile_type {
                    0 => self.tableau[source_pile_idx].extend(cards), // Tableau
                    1 => self.discard.push(cards.pop().unwrap()),     // Discard
                    2 => self.foundation[source_pile_idx].push(cards.pop().unwrap()), // Foundation
                    _ => {}
                }
            }
    
            // Turn the last card in the tableau face-up if it's not already
            if source_pile_type == 0 && !self.tableau[source_pile_idx].is_empty() {
                if let Some(last_card) = self.tableau[source_pile_idx].last_mut() {
                    last_card.face_up = true;
                }
            }
    
            self.render();
    
            // Check for a win after every move
            if self.check_game_won() {
                self.celebrate_win(); // Trigger the win animation
            }
        }
    }

    fn try_drop_card(&mut self, card: &Card, x: f64, y: f64) -> bool {
        // Check tableau piles
        for pile in self.tableau.iter_mut() {
            if let Some(target) = pile.last() {
                if target.contains(x, y) && Self::is_valid_tableau_move(card, target) {
                    pile.push(card.clone());
                    return true;
                }
            } else if pile.is_empty() && card.rank == "K" {
                // Only Kings can be placed in empty tableau piles
                pile.push(card.clone());
                return true;
            }
        }
    
        // Check foundation piles
        for pile in self.foundation.iter_mut() {
            if pile.last().map_or(card.rank == "A", |last_card| Self::is_valid_foundation_move(card, last_card)) {
                pile.push(card.clone());
                return true;
            }
        }
    
        false
    }
         
    fn try_drop_stack(&mut self, cards: &[Card], x: f64, y: f64) -> bool {
        for pile in self.tableau.iter_mut() {
            if let Some(target) = pile.last() {
                if target.contains(x, y) && Self::is_valid_tableau_move(&cards[0], target) {
                    pile.extend_from_slice(cards); // Add the entire stack
                    return true;
                }
            } else if pile.is_empty() && cards[0].rank == "K" {
                // Only Kings can be placed in empty tableau piles
                pile.extend_from_slice(cards);
                return true;
            }
        }
    
        false
    }
               
    fn is_red(card: &Card) -> bool {
        card.suit == "hearts" || card.suit == "diamonds"
    }

    fn is_valid_tableau_move(card: &Card, target: &Card) -> bool {
        let rank_order = vec!["A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"];
        let card_index = rank_order.iter().position(|&r| r == card.rank).unwrap();
        let target_index = rank_order.iter().position(|&r| r == target.rank).unwrap();
    
        card_index + 1 == target_index && Self::is_red(card) != Self::is_red(target)
    }    

    fn is_valid_foundation_move(card: &Card, target: &Card) -> bool {
        let rank_order = vec!["A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"];
        let card_index = rank_order.iter().position(|&r| r == card.rank).unwrap();
        let target_index = rank_order.iter().position(|&r| r == target.rank).unwrap();

        card_index == target_index + 1 && card.suit == target.suit
    }

    fn check_game_won(&self) -> bool {
        // Check if all cards are in the foundation piles
        self.foundation.iter().all(|pile| pile.len() == 13) // 13 cards per foundation pile
    }

    fn celebrate_win(&self) {
        // Clear the canvas
        self.ctx.clear_rect(0.0, 0.0, self.canvas.width() as f64, self.canvas.height() as f64);

        // Draw permanent "You Win!" text
        self.ctx.set_font("48px Arial");
        self.ctx.set_fill_style(&"gold".into());
        self.ctx
            .fill_text(
                "🎉 You Win! 🎉",
                self.canvas.width() as f64 / 2.0 - 120.0,
                self.canvas.height() as f64 / 2.0,
            )
            .unwrap();
    
        // Add fade-out animation
        let ctx = self.ctx.clone();
        let canvas = self.canvas.clone();
        let mut opacity = 1.0;
    
        let closure: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None)); // Specify the type explicitly
        let closure_clone = closure.clone();
    
        *closure.borrow_mut() = Some(Closure::wrap(Box::new(move || {
            if opacity > 0.0 {
                ctx.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
                ctx.set_global_alpha(opacity);
                ctx.set_font("48px Arial");
                ctx.set_fill_style(&"gold".into());
                ctx.fill_text(
                    "🎉 You Win! 🎉",
                    canvas.width() as f64 / 2.0 - 120.0,
                    canvas.height() as f64 / 2.0,
                )
                .unwrap();
                opacity -= 0.02; // Gradually reduce opacity
                window()
                    .unwrap()
                    .request_animation_frame(
                        closure_clone.borrow().as_ref().unwrap().as_ref().unchecked_ref(),
                    )
                    .unwrap();
            } else {
                // Reset global alpha for further rendering
                ctx.set_global_alpha(1.0);
            }
        }) as Box<dyn FnMut()>));
    
        window()
            .unwrap()
            .request_animation_frame(closure.borrow().as_ref().unwrap().as_ref().unchecked_ref())
            .unwrap();
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

    // Set the canvas size to match the calculated dimensions
    canvas.set_width(CANVAS_WIDTH as u32);
    canvas.set_height(CANVAS_HEIGHT as u32);

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
