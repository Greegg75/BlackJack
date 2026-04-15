pub struct Wallet {
    balance: u32,
}

impl Wallet {
    pub fn new(initial_funds: u32) -> Self {
        Self { balance: initial_funds }
    }
    pub fn balance(&self) -> u32 { self.balance }
    pub fn add_funds(&mut self, amount: u32) { self.balance += amount; }
    pub fn remove_funds(&mut self, amount: u32) -> Result<(), String> {
        if amount <= self.balance {
            self.balance -= amount;
            Ok(())
        } else {
            Err("Fonds insuffisants !".to_string())
        }
    }
}