use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Serialize, Deserialize};

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Suit { Hearts, Diamonds, Clubs, Spades }

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Rank { Number(u8), Jack, Queen, King, Ace }

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Card { pub suit: Suit, pub rank: Rank }

pub struct Deck { pub cards: Vec<Card> }

impl Deck {
    pub fn new() -> Self {
        let mut cards = Vec::new();
        let suits = [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades];
        let ranks = [
            Rank::Ace, Rank::Jack, Rank::Queen, Rank::King,
            Rank::Number(2), Rank::Number(3), Rank::Number(4), Rank::Number(5),
            Rank::Number(6), Rank::Number(7), Rank::Number(8), Rank::Number(9),
            Rank::Number(10),
        ];
        for &suit in &suits {
            for &rank in &ranks {
                cards.push(Card { suit, rank });
            }
        }
        Deck { cards }
    }
    pub fn shuffle(&mut self) {
        let mut rng = thread_rng();
        self.cards.shuffle(&mut rng);
    }
    pub fn draw(&mut self) -> Option<Card> { self.cards.pop() }
}