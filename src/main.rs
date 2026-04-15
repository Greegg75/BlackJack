mod engine;
mod network;
mod wallet;

use eframe::egui;
use engine::cards::{Card, Deck, Suit, Rank};
use engine::game::Hand;
use network::protocol::GameMessage;

use matchbox_socket::WebRtcSocket;
use std::collections::HashMap;
use uuid::Uuid;

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
    my_id: String,
    deck: Deck,
    player_hands: Vec<Hand>, // Tes propres mains (pour le split)
    remote_players: HashMap<String, Hand>, // Les mains de tes potes
    dealer_hand: Hand,
    wallet: wallet::Wallet, 
    current_bet: u32,       
    status: String,
    in_game: bool,
    theme: ThemeMode,
    
    // Réseau
    is_multiplayer: bool,
    socket: Option<WebRtcSocket>,
}

impl BlackjackApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_pixels_per_point(1.4);
        Self {
            my_id: Uuid::new_v4().to_string()[..6].to_string(), // ID court
            deck: Deck::new(),
            player_hands: vec![Hand::new(0)],
            remote_players: HashMap::new(),
            dealer_hand: Hand::new(0),
            wallet: wallet::Wallet::new(5000),
            current_bet: 0,
            status: "PLACEZ VOS JETONS".to_string(),
            in_game: false,
            theme: ThemeMode::Auto,
            is_multiplayer: false,
            socket: None,
        }
    }

    fn setup_network(&mut self) {
        // Connexion à une room publique sur le serveur public Matchbox
        let room_url = "wss://matchbox.procedural.dev/blackjack_room";
        let (socket, loop_fut) = WebRtcSocket::new_reliable(room_url);
        self.socket = Some(socket);
        self.status = "Connexion au serveur...".to_string();
        
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let _ = loop_fut.await; // On ignore silencieusement les erreurs de la boucle réseau
        });
    }

    fn broadcast_my_hand(&mut self) {
        if let Some(socket) = &mut self.socket {
            let msg = GameMessage::SyncHand {
                player_id: self.my_id.clone(),
                cards: self.player_hands[0].cards.clone(),
                score: self.player_hands[0].score(),
            };
            
            if let Ok(json) = serde_json::to_string(&msg) {
                let packet = json.into_bytes().into_boxed_slice();
                
                // On récupère la liste d'abord pour éviter l'erreur de Borrow Checker
                let peers: Vec<_> = socket.connected_peers().collect();
                for peer in peers {
                    socket.send(packet.clone(), peer);
                }
            }
        }
    }

    fn start_hand(&mut self) {
        self.deck = Deck::new();
        self.deck.shuffle();
        self.player_hands = vec![Hand::new(self.current_bet)];
        self.dealer_hand = Hand::new(0);
        
        for _ in 0..2 { if let Some(c) = self.deck.draw() { self.player_hands[0].add_card(c); } }
        if let Some(c) = self.deck.draw() { self.dealer_hand.add_card(c); }
        
        self.in_game = true;
        self.status = "À VOUS DE JOUER".to_string();
        self.broadcast_my_hand(); // Envoie la main initiale aux autres
    }

    fn resolve_winner(&mut self) {
        let d_score = self.dealer_hand.score();
        let mut total_gain: u32 = 0;
        let mut any_bust = false;

        for hand in &self.player_hands {
            let p_score = hand.score();
            if p_score > 21 {
                any_bust = true;
                // Mise perdue, rien à rembourser
            } else if d_score > 21 || p_score > d_score {
                // Victoire : on récupère la mise × 2
                total_gain += hand.bet * 2;
            } else if p_score == d_score {
                // Égalité : on récupère juste la mise
                total_gain += hand.bet;
            }
            // Défaite : rien
        }

        if total_gain > 0 {
            let net = total_gain - self.player_hands.iter().map(|h| h.bet).sum::<u32>();
            self.wallet.add_funds(total_gain);
            self.status = format!("✅ GAGNÉ ! +{} €", net);
        } else if any_bust {
            self.status = "💥 BUST ! BANQUE GAGNE".to_string();
        } else {
            self.status = "❌ BANQUE GAGNE".to_string();
        }
        
        self.current_bet = 0;
        self.in_game = false;
    }
}

impl eframe::App for BlackjackApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- LOGIQUE RÉSEAU ---
        if let Some(socket) = &mut self.socket {
            socket.update_peers();
            for (_peer, packet) in socket.receive() {
                if let Ok(json_str) = String::from_utf8(packet.into_vec()) {
                    // On utilise score: _ pour ignorer le warning
                    if let Ok(GameMessage::SyncHand { player_id, cards, score: _ }) = serde_json::from_str(&json_str) {
                        // On met à jour ou on crée le joueur distant
                        let mut remote_hand = Hand::new(0);
                        remote_hand.cards = cards;
                        self.remote_players.insert(player_id, remote_hand);
                    }
                }
            }
        }

        // On compte les joueurs connectés
        let peer_count = if let Some(socket) = &self.socket {
            socket.connected_peers().count()
        } else {
            0
        };

        // --- GESTION DU THÈME ---
        match self.theme {
            ThemeMode::Light => ctx.set_visuals(egui::Visuals::light()),
            ThemeMode::Dark => ctx.set_visuals(egui::Visuals::dark()),
            ThemeMode::Auto => {}
        }

        let is_dark = ctx.style().visuals.dark_mode;
        let bg_color = if is_dark { egui::Color32::from_rgb(18, 18, 20) } else { egui::Color32::from_rgb(245, 245, 247) };

        egui::CentralPanel::default().frame(egui::Frame::none().fill(bg_color)).show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(15.0);
                
                // --- TOP BAR ---
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut self.is_multiplayer, "🌐 ONLINE").changed() {
                        if self.is_multiplayer { self.setup_network(); }
                    }
                    ui.label(egui::RichText::new(format!("ID: {}", self.my_id)).weak());

                    // Indicateur de connexion
                    if self.is_multiplayer {
                        if peer_count > 0 {
                            ui.label(egui::RichText::new(format!("🟢 {} JOUEUR(S) AVEC VOUS", peer_count))
                                .color(egui::Color32::from_rgb(48, 209, 88)).strong());
                        } else {
                            ui.label(egui::RichText::new("🟠 EN ATTENTE DE JOUEURS...")
                                .color(egui::Color32::from_rgb(255, 159, 10)).weak());
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.selectable_value(&mut self.theme, ThemeMode::Auto, "🌓");
                        ui.selectable_value(&mut self.theme, ThemeMode::Dark, "🌙");
                        ui.selectable_value(&mut self.theme, ThemeMode::Light, "☀️");
                    });
                });


                ui.add_space(20.0);
                
                // --- SOLDE & REFILL ---
                let solde_color = if self.wallet.balance() == 0 { egui::Color32::from_rgb(255, 69, 58) } else { egui::Color32::from_rgb(48, 209, 88) };
                if ui.button(egui::RichText::new(format!("{} €", self.wallet.balance())).size(55.0).strong().color(solde_color)).clicked() && self.wallet.balance() == 0 {
                    self.wallet.add_funds(1000);
                    self.status = "RECHARGE ACCEPTÉE (+1000€)".to_string();
                }

                ui.add_space(30.0);

                // --- TAPIS DE JEU ---
                draw_hand_glass(ui, "BANQUE", &self.dealer_hand, is_dark);
                ui.add_space(40.0);

                // Zone des joueurs (Défilement horizontal pour gérer plusieurs amis)
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // 1. Toi
                        ui.vertical(|ui| {
                            draw_hand_glass(ui, "VOTRE MAIN", &self.player_hands[0], is_dark);
                        });
                        
                        ui.add_space(40.0);
                        ui.separator();
                        ui.add_space(40.0);

                        // 2. Les potes en ligne
                        for (id, hand) in &self.remote_players {
                            ui.vertical(|ui| {
                                draw_hand_glass(ui, &format!("JOUEUR {}", id), hand, is_dark);
                            });
                            ui.add_space(20.0);
                        }
                    });
                });

                ui.add_space(30.0);
                ui.label(egui::RichText::new(&self.status).size(22.0).color(egui::Color32::from_rgb(10, 132, 255)).strong());

                ui.add_space(30.0);

                // --- MISE EN COURS ---
                if !self.in_game {
                    if self.current_bet > 0 {
                        ui.label(egui::RichText::new(format!("MISE : {} €", self.current_bet))
                            .size(18.0)
                            .color(egui::Color32::from_rgb(255, 159, 10))
                            .strong());
                    } else {
                        ui.label(egui::RichText::new("MISE : 0 €").size(18.0).weak());
                    }
                    ui.add_space(8.0);
                }

                // --- ACTIONS ---
                ui.horizontal(|ui| {
                    let total_w = 650.0;
                    ui.add_space((ui.available_width() - total_w) / 2.0);

                    if !self.in_game {
                        for amt in [10, 50, 100, 1000] {
                            if ui.add(apple_btn(format!("+{}", amt), is_dark)).clicked() {
                                if self.wallet.remove_funds(amt).is_ok() { self.current_bet += amt; }
                            }
                        }
                        if ui.add(apple_btn("ALL IN", is_dark).fill(egui::Color32::from_rgb(255, 59, 48))).clicked() {
                            let b = self.wallet.balance();
                            let _ = self.wallet.remove_funds(b);
                            self.current_bet += b;
                        }
                        // Bouton annuler la mise
                        if self.current_bet > 0 {
                            if ui.add(apple_btn("✕ ANNULER", is_dark)).clicked() {
                                self.wallet.add_funds(self.current_bet);
                                self.current_bet = 0;
                                self.status = "MISE ANNULÉE".to_string();
                            }
                        }
                        ui.add_space(15.0);
                        if ui.add(egui::Button::new(egui::RichText::new("▶ PLAY").strong())
                            .fill(egui::Color32::from_rgb(48, 209, 88))
                            .min_size(egui::vec2(100.0, 45.0))).clicked() && self.current_bet > 0 {
                            self.start_hand();
                        }
                    } else {
                        if ui.add(apple_btn("HIT", is_dark)).clicked() {
                            if let Some(c) = self.deck.draw() { 
                                self.player_hands[0].add_card(c); 
                                self.broadcast_my_hand();
                                if self.player_hands[0].score() > 21 { self.resolve_winner(); } 
                            }
                        }
                        if ui.add(apple_btn("STAND", is_dark)).clicked() {
                            while self.dealer_hand.score() < 17 {
                                if let Some(c) = self.deck.draw() { self.dealer_hand.add_card(c); } else { break; }
                            }
                            self.resolve_winner();
                        }
                    }
                });
            });
        });
        ctx.request_repaint(); // Important pour la réactivité réseau
    }
}

fn apple_btn(text: impl Into<String>, is_dark: bool) -> egui::Button<'static> {
    let bg = if is_dark { egui::Color32::from_white_alpha(20) } else { egui::Color32::from_black_alpha(15) };
    egui::Button::new(egui::RichText::new(text).size(15.0).strong())
        .fill(bg).rounding(15.0).min_size(egui::vec2(85.0, 45.0))
}

fn draw_hand_glass(ui: &mut egui::Ui, label: &str, hand: &Hand, is_dark: bool) {
    ui.label(egui::RichText::new(label).size(12.0).weak().strong());
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if hand.cards.is_empty() {
            // Emplacement vide stylisé pour faire propre
            let (rect, _) = ui.allocate_at_least(egui::vec2(80.0, 110.0), egui::Sense::hover());
            ui.painter().rect_stroke(rect, 15.0, egui::Stroke::new(1.0, egui::Color32::from_gray(100)));
        } else {
            for card in &hand.cards { draw_card_apple(ui, card, is_dark); }
        }
    });
}

fn draw_card_apple(ui: &mut egui::Ui, card: &Card, is_dark: bool) {
    let (color, suit) = match card.suit {
        Suit::Hearts | Suit::Diamonds => (egui::Color32::from_rgb(255, 59, 48), if card.suit == Suit::Hearts {"♥"} else {"♦"}),
        _ => (if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }, if card.suit == Suit::Clubs {"♣"} else {"♠"}),
    };
    
    let rank = match card.rank {
        Rank::Number(n) => n.to_string(), Rank::Jack => "J".to_string(), Rank::Queen => "Q".to_string(), Rank::King => "K".to_string(), Rank::Ace => "A".to_string(),
    };

    let (rect, _) = ui.allocate_at_least(egui::vec2(80.0, 110.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 15.0, if is_dark { egui::Color32::from_white_alpha(10) } else { egui::Color32::from_black_alpha(5) });
    ui.painter().rect_stroke(rect, 15.0, egui::Stroke::new(1.0, egui::Color32::from_white_alpha(30)));
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, format!("{}\n{}", rank, suit), egui::FontId::proportional(25.0), color);
}
