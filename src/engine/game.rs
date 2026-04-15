use crate::engine::cards::{Card, Rank};

pub enum Action { Hit, Stand }

pub struct Hand {
    pub cards: Vec<Card>,
    pub bet: u32,
    pub active: bool,
}

impl Hand {
    /// Crée une nouvelle main avec une mise spécifique
    pub fn new(bet: u32) -> Self {
        Self { 
            cards: Vec::new(), 
            bet, 
            active: true 
        }
    }

    pub fn add_card(&mut self, card: Card) {
        self.cards.push(card);
    }

    pub fn score(&self) -> u8 {
        let mut total = 0;
        let mut aces = 0;

        for card in &self.cards {
            match card.rank {
                Rank::Number(n) => total += n,
                Rank::Jack | Rank::Queen | Rank::King => total += 10,
                Rank::Ace => {
                    total += 11;
                    aces += 1;
                }
            }
        }

        while total > 21 && aces > 0 {
            total -= 10;
            aces -= 1;
        }
        total
    }

    pub fn can_split(&self) -> bool {
        self.cards.len() == 2 && self.cards[0].rank == self.cards[1].rank
    }
}