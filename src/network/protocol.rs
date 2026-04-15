use serde::{Serialize, Deserialize};
use crate::engine::cards::Card;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GameMessage {
    // Demande de jetons quand on est à sec (Le "Refill")
    RequestFunds { amount: u32, player_id: String },
    
    // Envoi des cartes (pour le multi)
    DistributeCards { dealer_cards: Vec<Card>, players_cards: Vec<(String, Vec<Card>)> },
    
    // Action d'un joueur distant
    PlayerAction { player_id: String, action: String },
    
    // Message système (ex: "Grego a rejoint la table")
    System(String),
}