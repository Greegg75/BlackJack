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

// ─── Thème ───────────────────────────────────────────────────────────────────
#[derive(PartialEq, Clone, Copy)]
enum ThemeMode { Light, Dark, Auto }

// ─── Écrans ──────────────────────────────────────────────────────────────────
#[derive(PartialEq, Clone)]
enum AppScreen { Lobby, Game }

// ─── Point d'entrée WASM ─────────────────────────────────────────────────────
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

// ─── État de l'application ───────────────────────────────────────────────────
struct BlackjackApp {
    my_id: String,
    screen: AppScreen,

    // Lobby
    room_code_input: String,
    lobby_players: Vec<String>,    // IDs des autres joueurs dans le lobby
    connection_status: String,
    last_peer_count: usize,        // Pour détecter les nouvelles connexions

    // Jeu
    deck: Deck,
    player_hands: Vec<Hand>,
    remote_players: HashMap<String, Hand>,
    dealer_hand: Hand,
    wallet: wallet::Wallet,
    current_bet: u32,
    status: String,
    in_game: bool,

    // UI
    theme: ThemeMode,

    // Réseau
    socket: Option<WebRtcSocket>,
}

impl BlackjackApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_pixels_per_point(1.4);
        Self {
            my_id: Uuid::new_v4().to_string()[..6].to_string().to_uppercase(),
            screen: AppScreen::Lobby,

            room_code_input: String::new(),
            lobby_players: Vec::new(),
            connection_status: String::new(),
            last_peer_count: 0,

            deck: Deck::new(),
            player_hands: vec![Hand::new(0)],
            remote_players: HashMap::new(),
            dealer_hand: Hand::new(0),
            wallet: wallet::Wallet::new(5000),
            current_bet: 0,
            status: "PLACEZ VOS JETONS".to_string(),
            in_game: false,

            theme: ThemeMode::Auto,
            socket: None,
        }
    }

    // ── Réseau ────────────────────────────────────────────────────────────────

    fn connect_to_room(&mut self, room_code: &str) {
        // Le code de room forme l'URL — même code = même room
        let safe_code = room_code.trim().to_lowercase()
            .chars().filter(|c| c.is_alphanumeric()).collect::<String>();
        let room_url = format!("wss://matchbox.procedural.dev/blackjack_{}", safe_code);

        let (socket, loop_fut) = WebRtcSocket::new_reliable(&room_url);
        self.socket = Some(socket);
        self.connection_status = "⏳ Connexion en cours...".to_string();

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let _ = loop_fut.await;
        });
    }

    fn disconnect(&mut self) {
        self.socket = None;
        self.lobby_players.clear();
        self.remote_players.clear();
        self.connection_status = String::new();
        self.last_peer_count = 0;
    }

    /// Envoie un message à tous les pairs connectés
    fn broadcast(&mut self, msg: &GameMessage) {
        if let Some(socket) = &mut self.socket {
            if let Ok(json) = serde_json::to_string(msg) {
                let packet = json.into_bytes().into_boxed_slice();
                let peers: Vec<_> = socket.connected_peers().collect();
                for peer in peers {
                    socket.send(packet.clone(), peer);
                }
            }
        }
    }

    fn announce_join(&mut self) {
        let msg = GameMessage::JoinLobby { player_id: self.my_id.clone() };
        self.broadcast(&msg);
    }

    fn broadcast_my_hand(&mut self) {
        let msg = GameMessage::SyncHand {
            player_id: self.my_id.clone(),
            cards: self.player_hands[0].cards.clone(),
            score: self.player_hands[0].score(),
        };
        self.broadcast(&msg);
    }

    // ── Jeu ───────────────────────────────────────────────────────────────────

    fn start_hand(&mut self) {
        self.deck = Deck::new();
        self.deck.shuffle();
        self.player_hands = vec![Hand::new(self.current_bet)];
        self.dealer_hand = Hand::new(0);

        for _ in 0..2 {
            if let Some(c) = self.deck.draw() { self.player_hands[0].add_card(c); }
        }
        if let Some(c) = self.deck.draw() { self.dealer_hand.add_card(c); }

        self.in_game = true;
        self.status = "À VOUS DE JOUER".to_string();
        self.broadcast_my_hand();
    }

    fn resolve_winner(&mut self) {
        let d_score = self.dealer_hand.score();
        let mut total_gain: u32 = 0;
        let mut any_bust = false;

        for hand in &self.player_hands {
            let p_score = hand.score();
            if p_score > 21 {
                any_bust = true;
            } else if d_score > 21 || p_score > d_score {
                total_gain += hand.bet * 2;
            } else if p_score == d_score {
                total_gain += hand.bet;
            }
        }

        if total_gain > 0 {
            let total_bet: u32 = self.player_hands.iter().map(|h| h.bet).sum();
            let net = total_gain - total_bet;
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

// ─── Boucle principale ───────────────────────────────────────────────────────
impl eframe::App for BlackjackApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        // ── Mise à jour réseau ────────────────────────────────────────────────
        let mut need_announce = false;

        if let Some(socket) = &mut self.socket {
            socket.update_peers();

            let current_peer_count = socket.connected_peers().count();

            // Nouveau pair détecté → on s'annonce
            if current_peer_count > self.last_peer_count {
                need_announce = true;
                self.connection_status = format!("🟢 {} joueur(s) en ligne", current_peer_count);
            } else if current_peer_count == 0 {
                self.connection_status = "🟠 En attente de joueurs...".to_string();
            }
            self.last_peer_count = current_peer_count;

            // Réception des messages
            let messages: Vec<_> = socket.receive().collect();
            for (_peer, packet) in messages {
                if let Ok(json_str) = String::from_utf8(packet.into_vec()) {
                    match serde_json::from_str::<GameMessage>(&json_str) {
                        Ok(GameMessage::JoinLobby { player_id }) => {
                            if player_id != self.my_id && !self.lobby_players.contains(&player_id) {
                                self.lobby_players.push(player_id);
                                // On lui répond pour qu'il sache qu'on est là
                                need_announce = true;
                            }
                        }
                        Ok(GameMessage::SyncHand { player_id, cards, score: _ }) => {
                            let mut remote_hand = Hand::new(0);
                            remote_hand.cards = cards;
                            self.remote_players.insert(player_id, remote_hand);
                        }
                        _ => {}
                    }
                }
            }
        }

        if need_announce {
            self.announce_join();
        }

        // ── Thème ─────────────────────────────────────────────────────────────
        match self.theme {
            ThemeMode::Light => ctx.set_visuals(egui::Visuals::light()),
            ThemeMode::Dark  => ctx.set_visuals(egui::Visuals::dark()),
            ThemeMode::Auto  => {}
        }
        let is_dark = ctx.style().visuals.dark_mode;
        let bg_color = if is_dark {
            egui::Color32::from_rgb(18, 18, 20)
        } else {
            egui::Color32::from_rgb(245, 245, 247)
        };

        // ── UI ────────────────────────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(bg_color))
            .show(ctx, |ui| {

            // Barre de thème (toujours visible en haut à droite)
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.selectable_value(&mut self.theme, ThemeMode::Auto, "🌓");
                    ui.selectable_value(&mut self.theme, ThemeMode::Dark, "🌙");
                    ui.selectable_value(&mut self.theme, ThemeMode::Light, "☀️");
                });
            });

            ui.vertical_centered(|ui| {
                match self.screen.clone() {

                    // ══════════════════════════════════════════════════════════
                    // ÉCRAN LOBBY
                    // ══════════════════════════════════════════════════════════
                    AppScreen::Lobby => {
                        ui.add_space(40.0);

                        ui.label(egui::RichText::new("♠  BLACKJACK  ♠")
                            .size(52.0).strong()
                            .color(egui::Color32::from_rgb(10, 132, 255)));

                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(format!("Votre ID : {}", self.my_id)).weak());
                        ui.add_space(50.0);

                        if self.socket.is_none() {
                            // ── Formulaire ────────────────────────────────────
                            ui.label(egui::RichText::new("CODE DE LA ROOM").size(14.0).strong());
                            ui.add_space(10.0);

                            let response = ui.add(
                                egui::TextEdit::singleline(&mut self.room_code_input)
                                    .hint_text("ex: POKER123")
                                    .desired_width(220.0)
                                    .font(egui::TextStyle::Heading)
                            );

                            ui.add_space(20.0);

                            let can_join = !self.room_code_input.trim().is_empty();
                            let join = ui.add_enabled(
                                can_join,
                                egui::Button::new(
                                    egui::RichText::new("🚀  REJOINDRE").strong().size(17.0)
                                )
                                .fill(egui::Color32::from_rgb(10, 132, 255))
                                .min_size(egui::vec2(220.0, 52.0))
                            );

                            let enter_pressed = response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter));

                            if (join.clicked() || enter_pressed) && can_join {
                                let code = self.room_code_input.trim().to_string();
                                self.connect_to_room(&code);
                            }

                            ui.add_space(24.0);
                            ui.label(
                                egui::RichText::new(
                                    "Entrez le même code que vos amis pour jouer ensemble."
                                ).weak().italics().size(13.0)
                            );

                        } else {
                            // ── Salon d'attente ───────────────────────────────
                            ui.label(
                                egui::RichText::new(
                                    format!("Room :  {}", self.room_code_input.trim().to_uppercase())
                                ).size(22.0).strong()
                            );
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(&self.connection_status).size(14.0)
                                    .color(egui::Color32::from_rgb(48, 209, 88))
                            );
                            ui.add_space(32.0);

                            // Liste des joueurs
                            ui.label(egui::RichText::new("JOUEURS DANS LA ROOM").size(12.0).weak());
                            ui.add_space(12.0);

                            // Nous-mêmes
                            ui.label(
                                egui::RichText::new(format!("♦  {} (vous)", self.my_id))
                                    .size(17.0).strong()
                                    .color(egui::Color32::from_rgb(10, 132, 255))
                            );

                            // Les autres
                            for pid in self.lobby_players.clone() {
                                ui.label(
                                    egui::RichText::new(format!("♦  {}", pid)).size(17.0)
                                );
                            }

                            ui.add_space(40.0);

                            if ui.add(
                                egui::Button::new(
                                    egui::RichText::new("▶  COMMENCER LA PARTIE").strong().size(18.0)
                                )
                                .fill(egui::Color32::from_rgb(48, 209, 88))
                                .min_size(egui::vec2(270.0, 56.0))
                            ).clicked() {
                                self.screen = AppScreen::Game;
                                self.status = "PLACEZ VOS JETONS".to_string();
                            }

                            ui.add_space(16.0);

                            if ui.add(apple_btn("← QUITTER LA ROOM", is_dark)).clicked() {
                                self.disconnect();
                            }
                        }
                    }

                    // ══════════════════════════════════════════════════════════
                    // ÉCRAN DE JEU
                    // ══════════════════════════════════════════════════════════
                    AppScreen::Game => {
                        ui.add_space(10.0);

                        // ── Header de jeu ─────────────────────────────────────
                        ui.horizontal(|ui| {
                            if ui.add(apple_btn("← LOBBY", is_dark)).clicked() {
                                self.screen = AppScreen::Lobby;
                                // Rembourse la mise si on quitte en cours
                                if self.current_bet > 0 {
                                    self.wallet.add_funds(self.current_bet);
                                    self.current_bet = 0;
                                }
                                self.in_game = false;
                            }
                            ui.label(
                                egui::RichText::new(
                                    format!("Room: {}  |  ID: {}",
                                        self.room_code_input.trim().to_uppercase(),
                                        self.my_id)
                                ).weak()
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    egui::RichText::new(&self.connection_status)
                                        .small()
                                        .color(egui::Color32::from_rgb(48, 209, 88))
                                );
                            });
                        });

                        ui.add_space(20.0);

                        // ── Solde ─────────────────────────────────────────────
                        let solde_color = if self.wallet.balance() == 0 {
                            egui::Color32::from_rgb(255, 69, 58)
                        } else {
                            egui::Color32::from_rgb(48, 209, 88)
                        };
                        if ui.button(
                            egui::RichText::new(format!("{} €", self.wallet.balance()))
                                .size(55.0).strong().color(solde_color)
                        ).clicked() && self.wallet.balance() == 0 {
                            self.wallet.add_funds(1000);
                            self.status = "RECHARGE ACCEPTÉE (+1000€)".to_string();
                        }

                        ui.add_space(30.0);

                        // ── Tapis ─────────────────────────────────────────────
                        draw_hand_glass(ui, "BANQUE", &self.dealer_hand, is_dark);
                        ui.add_space(40.0);

                        egui::ScrollArea::horizontal().show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    draw_hand_glass(ui, "VOTRE MAIN", &self.player_hands[0], is_dark);
                                });

                                if !self.remote_players.is_empty() {
                                    ui.add_space(30.0);
                                    ui.separator();
                                    ui.add_space(30.0);
                                    for (id, hand) in &self.remote_players {
                                        ui.vertical(|ui| {
                                            draw_hand_glass(ui, &format!("JOUEUR {}", id), hand, is_dark);
                                        });
                                        ui.add_space(20.0);
                                    }
                                }
                            });
                        });

                        ui.add_space(30.0);
                        ui.label(
                            egui::RichText::new(&self.status)
                                .size(22.0)
                                .color(egui::Color32::from_rgb(10, 132, 255))
                                .strong()
                        );
                        ui.add_space(20.0);

                        // ── Mise en cours ─────────────────────────────────────
                        if !self.in_game {
                            if self.current_bet > 0 {
                                ui.label(
                                    egui::RichText::new(format!("MISE : {} €", self.current_bet))
                                        .size(18.0)
                                        .color(egui::Color32::from_rgb(255, 159, 10))
                                        .strong()
                                );
                            } else {
                                ui.label(egui::RichText::new("MISE : 0 €").size(18.0).weak());
                            }
                            ui.add_space(10.0);
                        }

                        // ── Boutons d'action ──────────────────────────────────
                        ui.horizontal(|ui| {
                            let total_w = 680.0;
                            ui.add_space((ui.available_width() - total_w) / 2.0);

                            if !self.in_game {
                                for amt in [10u32, 50, 100, 1000] {
                                    if ui.add(apple_btn(format!("+{}", amt), is_dark)).clicked() {
                                        if self.wallet.remove_funds(amt).is_ok() {
                                            self.current_bet += amt;
                                        }
                                    }
                                }
                                if ui.add(
                                    apple_btn("ALL IN", is_dark)
                                        .fill(egui::Color32::from_rgb(255, 59, 48))
                                ).clicked() {
                                    let b = self.wallet.balance();
                                    let _ = self.wallet.remove_funds(b);
                                    self.current_bet += b;
                                }
                                if self.current_bet > 0 {
                                    if ui.add(apple_btn("✕ ANNULER", is_dark)).clicked() {
                                        self.wallet.add_funds(self.current_bet);
                                        self.current_bet = 0;
                                        self.status = "MISE ANNULÉE".to_string();
                                    }
                                }
                                ui.add_space(15.0);
                                if ui.add(
                                    egui::Button::new(egui::RichText::new("▶ PLAY").strong())
                                        .fill(egui::Color32::from_rgb(48, 209, 88))
                                        .min_size(egui::vec2(100.0, 45.0))
                                ).clicked() && self.current_bet > 0 {
                                    self.start_hand();
                                }
                            } else {
                                if ui.add(apple_btn("HIT", is_dark)).clicked() {
                                    if let Some(c) = self.deck.draw() {
                                        self.player_hands[0].add_card(c);
                                        self.broadcast_my_hand();
                                        if self.player_hands[0].score() > 21 {
                                            self.resolve_winner();
                                        }
                                    }
                                }
                                if ui.add(apple_btn("STAND", is_dark)).clicked() {
                                    while self.dealer_hand.score() < 17 {
                                        if let Some(c) = self.deck.draw() {
                                            self.dealer_hand.add_card(c);
                                        } else { break; }
                                    }
                                    self.resolve_winner();
                                }
                            }
                        });
                    }
                }
            });
        });

        ctx.request_repaint();
    }
}

// ─── Helpers UI ──────────────────────────────────────────────────────────────

fn apple_btn(text: impl Into<String>, is_dark: bool) -> egui::Button<'static> {
    let bg = if is_dark {
        egui::Color32::from_white_alpha(20)
    } else {
        egui::Color32::from_black_alpha(15)
    };
    egui::Button::new(egui::RichText::new(text).size(15.0).strong())
        .fill(bg).rounding(15.0).min_size(egui::vec2(85.0, 45.0))
}

fn draw_hand_glass(ui: &mut egui::Ui, label: &str, hand: &Hand, is_dark: bool) {
    ui.label(egui::RichText::new(label).size(12.0).weak().strong());
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if hand.cards.is_empty() {
            let (rect, _) = ui.allocate_at_least(egui::vec2(80.0, 110.0), egui::Sense::hover());
            ui.painter().rect_stroke(
                rect, 15.0,
                egui::Stroke::new(1.0, egui::Color32::from_gray(100))
            );
        } else {
            for card in &hand.cards { draw_card_apple(ui, card, is_dark); }
        }
    });
    // Score sous la main
    if !hand.cards.is_empty() {
        ui.add_space(6.0);
        ui.label(egui::RichText::new(format!("Score : {}", hand.score())).size(13.0).weak());
    }
}

fn draw_card_apple(ui: &mut egui::Ui, card: &Card, is_dark: bool) {
    let (color, suit) = match card.suit {
        Suit::Hearts   => (egui::Color32::from_rgb(255, 59, 48), "♥"),
        Suit::Diamonds => (egui::Color32::from_rgb(255, 59, 48), "♦"),
        Suit::Clubs    => (if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }, "♣"),
        Suit::Spades   => (if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK }, "♠"),
    };

    let rank = match card.rank {
        Rank::Number(n) => n.to_string(),
        Rank::Jack  => "J".to_string(),
        Rank::Queen => "Q".to_string(),
        Rank::King  => "K".to_string(),
        Rank::Ace   => "A".to_string(),
    };

    let (rect, _) = ui.allocate_at_least(egui::vec2(80.0, 110.0), egui::Sense::hover());
    ui.painter().rect_filled(
        rect, 15.0,
        if is_dark { egui::Color32::from_white_alpha(10) } else { egui::Color32::from_black_alpha(5) }
    );
    ui.painter().rect_stroke(
        rect, 15.0,
        egui::Stroke::new(1.0, egui::Color32::from_white_alpha(30))
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("{}\n{}", rank, suit),
        egui::FontId::proportional(25.0),
        color,
    );
}
