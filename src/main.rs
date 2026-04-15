// mod engine;
// mod network;
// mod wallet;

// use eframe::egui;
// use engine::cards::{Card, Deck, Suit, Rank};
// use engine::game::Hand;
// use network::protocol::GameMessage;

// #[derive(PartialEq, Clone, Copy)]
// enum ThemeMode { Light, Dark, Auto }

// #[cfg(target_arch = "wasm32")]
// fn main() {
//     let web_options = eframe::WebOptions::default();
//     wasm_bindgen_futures::spawn_local(async {
//         eframe::WebRunner::new()
//             .start("the_canvas_id", web_options, Box::new(|cc| Box::new(BlackjackApp::new(cc))))
//             .await
//             .expect("failed to start eframe");
//     });
// }

// struct BlackjackApp {
//     deck: Deck,
//     player_hands: Vec<Hand>,
//     dealer_hand: Hand,
//     wallet: wallet::Wallet, 
//     current_bet: u32,       
//     status: String,
//     in_game: bool,
//     theme: ThemeMode,
//     is_multiplayer: bool,
// }

// impl BlackjackApp {
//     fn new(cc: &eframe::CreationContext<'_>) -> Self {
//         cc.egui_ctx.set_pixels_per_point(1.4);
//         Self {
//             deck: Deck::new(),
//             player_hands: vec![Hand::new(0)],
//             dealer_hand: Hand::new(0),
//             wallet: wallet::Wallet::new(5000),
//             current_bet: 0,
//             status: "FAITES VOS JEUX".to_string(),
//             in_game: false,
//             theme: ThemeMode::Auto,
//             is_multiplayer: false,
//         }
//     }

//     fn start_hand(&mut self) {
//         self.deck = Deck::new();
//         self.deck.shuffle();
//         self.player_hands = vec![Hand::new(self.current_bet)];
//         self.dealer_hand = Hand::new(0);
//         for _ in 0..2 { if let Some(c) = self.deck.draw() { self.player_hands[0].add_card(c); } }
//         if let Some(c) = self.deck.draw() { self.dealer_hand.add_card(c); }
//         self.in_game = true;
//         self.status = "À VOUS DE JOUER".to_string();
//     }

//     fn resolve_winner(&mut self) {
//         let d_score = self.dealer_hand.score();
//         let mut total_gain = 0;
//         for hand in &self.player_hands {
//             let p_score = hand.score();
//             if p_score <= 21 && (d_score > 21 || p_score > d_score) { total_gain += hand.bet * 2; }
//             else if p_score <= 21 && p_score == d_score { total_gain += hand.bet; }
//         }
//         if total_gain > 0 {
//             self.wallet.add_funds(total_gain);
//             self.status = format!("GAGNÉ ! +{}€", total_gain);
//         } else { self.status = "BANQUE GAGNE".to_string(); }
//         self.current_bet = 0;
//         self.in_game = false;
//     }
// }

// impl eframe::App for BlackjackApp {
//     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//         let is_dark = ctx.style().visuals.dark_mode;
//         let bg_color = if is_dark { egui::Color32::from_rgb(18, 18, 20) } else { egui::Color32::from_rgb(245, 245, 247) };
//         let glass_bg = if is_dark { egui::Color32::from_white_alpha(15) } else { egui::Color32::from_black_alpha(10) };

//         egui::CentralPanel::default().frame(egui::Frame::none().fill(bg_color)).show(ctx, |ui| {
//             ui.vertical_centered(|ui| {
//                 ui.add_space(10.0);
                
//                 // BARRE MULTI / THÈME
//                 ui.horizontal(|ui| {
//                     ui.checkbox(&mut self.is_multiplayer, "🌐 ONLINE");
//                     ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
//                         ui.selectable_value(&mut self.theme, ThemeMode::Auto, "🌓");
//                         ui.selectable_value(&mut self.theme, ThemeMode::Dark, "🌙");
//                         ui.selectable_value(&mut self.theme, ThemeMode::Light, "☀️");
//                     });
//                 });

//                 ui.add_space(20.0);
                
//                 // SOLDE INTERACTIF (REFUND SI 0)
//                 let solde_color = if self.wallet.balance() == 0 { egui::Color32::from_rgb(255, 69, 58) } else { egui::Color32::from_rgb(48, 209, 88) };
//                 if ui.button(egui::RichText::new(format!("{} €", self.wallet.balance())).size(50.0).strong().color(solde_color)).clicked() && self.wallet.balance() == 0 {
//                     self.wallet.add_funds(1000);
//                     self.status = "RECHARGE +1000€".to_string();
//                 }
//                 ui.label(egui::RichText::new("SOLDE DISPONIBLE").size(10.0).weak());

//                 ui.add_space(30.0);
//                 draw_hand_glass(ui, "BANQUE", &self.dealer_hand, is_dark);
//                 ui.add_space(30.0);
//                 for hand in &self.player_hands {
//                     draw_hand_glass(ui, "VOTRE MAIN", hand, is_dark);
//                 }

//                 ui.add_space(20.0);
//                 ui.label(egui::RichText::new(&self.status).size(22.0).color(egui::Color32::from_rgb(10, 132, 255)).strong());

//                 ui.add_space(30.0);

//                 // --- ZONE DES BOUTONS ---
//                 ui.horizontal(|ui| {
//                     let btn_area_width = 550.0;
//                     ui.add_space((ui.available_width() - btn_area_width) / 2.0);

//                     if !self.in_game {
//                         // BOUTONS DE MISE
//                         for amt in [10, 50, 100, 1000] {
//                             if ui.add(apple_btn(format!("+{}", amt), is_dark)).clicked() {
//                                 if self.wallet.remove_funds(amt).is_ok() { self.current_bet += amt; }
//                             }
//                         }
//                         if ui.add(apple_btn("ALL IN", is_dark).fill(egui::Color32::from_rgb(255, 59, 48))).clicked() {
//                             let b = self.wallet.balance(); let _ = self.wallet.remove_funds(b); self.current_bet += b;
//                         }
//                         ui.add_space(10.0);
//                         if ui.add(egui::Button::new(egui::RichText::new("PLAY").strong()).fill(egui::Color32::from_rgb(48, 209, 88)).min_size(egui::vec2(100.0, 45.0))).clicked() && self.current_bet > 0 {
//                             self.start_hand();
//                         }
//                     } else {
//                         // ACTIONS DE JEU
//                         if ui.add(apple_btn("HIT", is_dark)).clicked() {
//                             if let Some(c) = self.deck.draw() { 
//                                 self.player_hands[0].add_card(c); 
//                                 if self.player_hands[0].score() > 21 { self.resolve_winner(); } 
//                             }
//                         }
//                         if ui.add(apple_btn("STAND", is_dark)).clicked() {
//                             while self.dealer_hand.score() < 17 { if let Some(c) = self.deck.draw() { self.dealer_hand.add_card(c); } else { break; } }
//                             self.resolve_winner();
//                         }
//                     }
//                 });
                
//                 if !self.in_game && self.current_bet > 0 {
//                     ui.add_space(10.0);
//                     if ui.button(egui::RichText::new("ANNULER LA MISE").color(egui::Color32::GRAY)).clicked() {
//                         self.wallet.add_funds(self.current_bet);
//                         self.current_bet = 0;
//                     }
//                 }
//             });
//         });
//         ctx.request_repaint();
//     }
// }

// fn apple_btn(text: impl Into<String>, is_dark: bool) -> egui::Button<'static> {
//     let bg = if is_dark { egui::Color32::from_white_alpha(20) } else { egui::Color32::from_black_alpha(15) };
//     egui::Button::new(egui::RichText::new(text).size(15.0).strong())
//         .fill(bg).rounding(15.0).min_size(egui::vec2(85.0, 45.0))
// }

// fn draw_hand_glass(ui: &mut egui::Ui, label: &str, hand: &Hand, is_dark: bool) {
//     ui.label(egui::RichText::new(label).size(10.0).weak());
//     ui.horizontal(|ui| {
//         let w = hand.cards.len() as f32 * 90.0;
//         ui.add_space((ui.available_width() - w) / 2.0);
//         for card in &hand.cards { draw_card_apple(ui, card, is_dark); }
//     });
// }

// fn draw_card_apple(ui: &mut egui::Ui, card: &Card, is_dark: bool) {
//     let (color, suit) = match card.suit {
//         Suit::Hearts | Suit::Diamonds => (egui::Color32::from_rgb(255, 59, 48), if card.suit == Suit::Hearts {"♥"} else {"♦"}),
//         _ => (if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }, if card.suit == Suit::Clubs {"♣"} else {"♠"}),
//     };
//     let rank = match card.rank {
//         Rank::Number(n) => n.to_string(), Rank::Jack => "J".to_string(), Rank::Queen => "Q".to_string(), Rank::King => "K".to_string(), Rank::Ace => "A".to_string(),
//     };
//     let (rect, _) = ui.allocate_at_least(egui::vec2(80.0, 110.0), egui::Sense::hover());
//     ui.painter().rect_filled(rect, 15.0, if is_dark { egui::Color32::from_white_alpha(10) } else { egui::Color32::from_black_alpha(5) });
//     ui.painter().rect_stroke(rect, 15.0, egui::Stroke::new(1.0, egui::Color32::from_white_alpha(30)));
//     ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, format!("{}\n{}", rank, suit), egui::FontId::proportional(25.0), color);
// }
mod engine;
mod network;
mod wallet;

use eframe::egui;
use engine::cards::{Card, Deck, Suit, Rank};
use engine::game::Hand;
use network::protocol::GameMessage;

#[derive(PartialEq, Clone, Copy)]
enum ThemeMode { Light, Dark, Auto }

#[cfg(target_arch = "wasm32")]
fn main() {
    let web_options = eframe::WebOptions::default();
    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start("the_canvas_id", web_options, Box::new(|cc| Box::new(BlackjackApp::new(cc))))
            .await
            .expect("failed to start eframe");
    });
}

struct BlackjackApp {
    deck: Deck,
    player_hands: Vec<Hand>, // Support pour le Split (plusieurs mains)
    dealer_hand: Hand,
    wallet: wallet::Wallet, 
    current_bet: u32,       
    status: String,
    in_game: bool,
    theme: ThemeMode,
    is_multiplayer: bool,
}

impl BlackjackApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Optimisation pour ton écran 4K
        cc.egui_ctx.set_pixels_per_point(1.4);
        Self {
            deck: Deck::new(),
            player_hands: vec![Hand::new(0)],
            dealer_hand: Hand::new(0),
            wallet: wallet::Wallet::new(5000),
            current_bet: 0,
            status: "PLACEZ VOS JETONS".to_string(),
            in_game: false,
            theme: ThemeMode::Auto,
            is_multiplayer: false,
        }
    }

    fn start_hand(&mut self) {
        self.deck = Deck::new();
        self.deck.shuffle();
        self.player_hands = vec![Hand::new(self.current_bet)];
        self.dealer_hand = Hand::new(0);
        
        // Distribution initiale
        for _ in 0..2 { if let Some(c) = self.deck.draw() { self.player_hands[0].add_card(c); } }
        if let Some(c) = self.deck.draw() { self.dealer_hand.add_card(c); }
        
        self.in_game = true;
        self.status = "À VOUS DE JOUER".to_string();
    }

    fn resolve_winner(&mut self) {
        let d_score = self.dealer_hand.score();
        let mut total_gain = 0;

        for hand in &self.player_hands {
            let p_score = hand.score();
            if p_score <= 21 && (d_score > 21 || p_score > d_score) {
                total_gain += hand.bet * 2;
            } else if p_score <= 21 && p_score == d_score {
                total_gain += hand.bet;
            }
        }

        if total_gain > 0 {
            self.wallet.add_funds(total_gain);
            self.status = format!("RÉSULTAT : +{}€", total_gain);
        } else {
            self.status = "LA BANQUE GAGNE.".to_string();
        }
        
        self.current_bet = 0;
        self.in_game = false;
    }
}

impl eframe::App for BlackjackApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Gestion auto du thème
        match self.theme {
            ThemeMode::Light => ctx.set_visuals(egui::Visuals::light()),
            ThemeMode::Dark => ctx.set_visuals(egui::Visuals::dark()),
            ThemeMode::Auto => {}
        }

        let is_dark = ctx.style().visuals.dark_mode;
        let bg_color = if is_dark { egui::Color32::from_rgb(18, 18, 20) } else { egui::Color32::from_rgb(245, 245, 247) };
        let glass_bg = if is_dark { egui::Color32::from_white_alpha(15) } else { egui::Color32::from_black_alpha(10) };

        egui::CentralPanel::default().frame(egui::Frame::none().fill(bg_color)).show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(15.0);
                
                // BARRE TOP NAVIGATION
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.is_multiplayer, "🌐 ONLINE");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.selectable_value(&mut self.theme, ThemeMode::Auto, "🌓");
                        ui.selectable_value(&mut self.theme, ThemeMode::Dark, "🌙");
                        ui.selectable_value(&mut self.theme, ThemeMode::Light, "☀️");
                    });
                });

                ui.add_space(20.0);
                
                // AFFICHAGE SOLDE + SYSTÈME REFILL (Clique si 0€)
                let solde_color = if self.wallet.balance() == 0 { egui::Color32::from_rgb(255, 69, 58) } else { egui::Color32::from_rgb(48, 209, 88) };
                if ui.button(egui::RichText::new(format!("{} €", self.wallet.balance())).size(55.0).strong().color(solde_color)).clicked() && self.wallet.balance() == 0 {
                    self.wallet.add_funds(1000);
                    self.status = "RECHARGE ACCEPTÉE (+1000€)".to_string();
                }
                ui.label(egui::RichText::new("SOLDE DISPONIBLE").size(10.0).weak());

                ui.add_space(30.0);

                // TAPIS DE JEU (BANQUE ET JOUEUR)
                draw_hand_glass(ui, "BANQUE", &self.dealer_hand, is_dark);
                ui.add_space(30.0);
                for hand in &self.player_hands {
                    draw_hand_glass(ui, "VOTRE MAIN", hand, is_dark);
                }

                ui.add_space(20.0);
                ui.label(egui::RichText::new(&self.status).size(22.0).color(egui::Color32::from_rgb(10, 132, 255)).strong());

                ui.add_space(30.0);

                // BARRE D'ACTIONS (MISES OU JEU)
                ui.horizontal(|ui| {
                    let total_w = 600.0;
                    ui.add_space((ui.available_width() - total_w) / 2.0);

                    if !self.in_game {
                        // Boutons de mise
                        for amt in [10, 50, 100, 1000] {
                            if ui.add(apple_btn(format!("+{}", amt), is_dark)).clicked() {
                                if self.wallet.remove_funds(amt).is_ok() { self.current_bet += amt; }
                            }
                        }
                        if ui.add(apple_btn("ALL IN", is_dark).fill(egui::Color32::from_rgb(255, 59, 48))).clicked() {
                            let b = self.wallet.balance(); let _ = self.wallet.remove_funds(b); self.current_bet += b;
                        }
                        ui.add_space(15.0);
                        if ui.add(egui::Button::new(egui::RichText::new("PLAY").strong()).fill(egui::Color32::from_rgb(48, 209, 88)).min_size(egui::vec2(100.0, 45.0))).clicked() && self.current_bet > 0 {
                            self.start_hand();
                        }
                    } else {
                        // Boutons Hit / Stand
                        if ui.add(apple_btn("HIT", is_dark)).clicked() {
                            if let Some(c) = self.deck.draw() { 
                                self.player_hands[0].add_card(c); 
                                if self.player_hands[0].score() > 21 { self.resolve_winner(); } 
                            }
                        }
                        if ui.add(apple_btn("STAND", is_dark)).clicked() {
                            while self.dealer_hand.score() < 17 { if let Some(c) = self.deck.draw() { self.dealer_hand.add_card(c); } else { break; } }
                            self.resolve_winner();
                        }
                    }
                });
            });
        });
        ctx.request_repaint();
    }
}

// --- HELPERS GRAPHIQUES ---

fn apple_btn(text: impl Into<String>, is_dark: bool) -> egui::Button<'static> {
    let bg = if is_dark { egui::Color32::from_white_alpha(20) } else { egui::Color32::from_black_alpha(15) };
    egui::Button::new(egui::RichText::new(text).size(15.0).strong())
        .fill(bg).rounding(15.0).min_size(egui::vec2(85.0, 45.0))
}

fn draw_hand_glass(ui: &mut egui::Ui, label: &str, hand: &Hand, is_dark: bool) {
    ui.label(egui::RichText::new(label).size(10.0).weak());
    ui.horizontal(|ui| {
        let w = hand.cards.len() as f32 * 90.0;
        ui.add_space((ui.available_width() - w) / 2.0);
        for card in &hand.cards {
            draw_card_apple(ui, card, is_dark);
        }
    });
}

fn draw_card_apple(ui: &mut egui::Ui, card: &Card, is_dark: bool) {
    let (color, suit) = match card.suit {
        Suit::Hearts | Suit::Diamonds => (egui::Color32::from_rgb(255, 59, 48), if card.suit == Suit::Hearts {"♥"} else {"♦"}),
        _ => (if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }, if card.suit == Suit::Clubs {"♣"} else {"♠"}),
    };
    
    let rank = match card.rank {
        Rank::Number(n) => n.to_string(),
        Rank::Jack => "J".to_string(), Rank::Queen => "Q".to_string(),
        Rank::King => "K".to_string(), Rank::Ace => "A".to_string(),
    };

    let (rect, _) = ui.allocate_at_least(egui::vec2(80.0, 110.0), egui::Sense::hover());
    
    // Effet Glassmorphism
    ui.painter().rect_filled(rect, 15.0, if is_dark { egui::Color32::from_white_alpha(10) } else { egui::Color32::from_black_alpha(5) });
    ui.painter().rect_stroke(rect, 15.0, egui::Stroke::new(1.0, egui::Color32::from_white_alpha(30)));
    
    // Texte de la carte
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, format!("{}\n{}", rank, suit), egui::FontId::proportional(25.0), color);
}