use rust_decimal::Decimal;
use serde::{Serialize, Serializer};

#[derive(Debug, PartialEq, Clone)]
pub enum AccountStatus {
    Active,
    Locked,
}

impl Serialize for AccountStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            AccountStatus::Active => serializer.serialize_bool(false),
            AccountStatus::Locked => serializer.serialize_bool(true),
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Clone)]
pub struct ClientAccount {
    pub client: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    #[serde(rename = "locked")]
    pub status: AccountStatus,
}

impl ClientAccount {
    pub fn new(client: u16) -> Self {
        Self {
            client,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            total: Decimal::ZERO,
            status: AccountStatus::Active,
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

    #[test]
    fn test_account_status_serialization() {
        let status = AccountStatus::Locked;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "true");

        let status = AccountStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "false");
    }
}
