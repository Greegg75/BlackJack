use serde::{Serialize, Deserialize};
use crate::engine::cards::Card;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GameMessage {
    // Annonce sa présence dans le lobby
    JoinLobby { player_id: String },

    // Synchronise sa main avec les autres joueurs
    SyncHand { player_id: String, cards: Vec<Card>, score: u8 },

    // Demande de jetons quand on est à sec
    RequestFunds { amount: u32, player_id: String },

    // Distribution des cartes (hôte → joueurs)
    DistributeCards { dealer_cards: Vec<Card>, players_cards: Vec<(String, Vec<Card>)> },

    // Action d'un joueur distant
    PlayerAction { player_id: String, action: String },

    // Message système
    System(String),

    // Ping / chat futur
    Ping(String),
}
