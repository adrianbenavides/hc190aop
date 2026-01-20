use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Clone)]
pub struct ClientAccount {
    pub client: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

impl ClientAccount {
    pub fn new(client: u16) -> Self {
        Self {
            client,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            total: Decimal::ZERO,
            locked: false,
        }
    }

    /// Updates the total balance to be available + held
    pub fn update_total(&mut self) {
        self.total = self.available + self.held;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_account_update_total() {
        let mut account = ClientAccount::new(1);
        account.available = dec!(1.5);
        account.held = dec!(0.5);
        account.update_total();
        assert_eq!(account.total, dec!(2.0));
    }
}
