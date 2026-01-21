use rust_decimal::Decimal;
use serde::{Deserialize, Serialize, Serializer};
use std::ops::{Add, AddAssign, Sub, SubAssign};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct Balance(pub Decimal);

impl Balance {
    pub const ZERO: Self = Self(Decimal::ZERO);

    pub fn new(amount: Decimal) -> Self {
        Self(amount)
    }
}

// Implement basic arithmetic for Balance to make it a usable Value Object
impl Add for Balance {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Balance {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl AddAssign for Balance {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl SubAssign for Balance {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum AccountStatus {
    Active,
    Locked,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ClientAccount {
    pub client: u16,
    pub available: Balance,
    pub held: Balance,
    pub total: Balance,
    #[serde(
        rename = "locked",
        serialize_with = "serialize_bool",
        deserialize_with = "deserialize_bool"
    )]
    pub status: AccountStatus,
}

fn serialize_bool<S>(status: &AccountStatus, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_bool(*status == AccountStatus::Locked)
}

fn deserialize_bool<'de, D>(deserializer: D) -> Result<AccountStatus, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let locked = bool::deserialize(deserializer)?;
    if locked {
        Ok(AccountStatus::Locked)
    } else {
        Ok(AccountStatus::Active)
    }
}

impl ClientAccount {
    pub fn new(client: u16) -> Self {
        Self {
            client,
            available: Balance::ZERO,
            held: Balance::ZERO,
            total: Balance::ZERO,
            status: AccountStatus::Active,
        }
    }

    /// Deposits funds into the available balance
    pub fn deposit(&mut self, amount: Balance) {
        self.available += amount;
        self.total += amount;
    }

    /// Withdraws funds from available if sufficient
    pub fn withdraw(&mut self, amount: Balance) -> Result<(), &'static str> {
        if self.available >= amount {
            self.available -= amount;
            self.total -= amount;
            Ok(())
        } else {
            Err("Insufficient funds")
        }
    }

    /// Holds funds (moves from available to held)
    pub fn hold(&mut self, amount: Balance) -> Result<(), &'static str> {
        if self.available >= amount {
            self.available -= amount;
            self.held += amount;
            Ok(())
        } else {
            Err("Insufficient funds to hold")
        }
    }

    /// Resolves a hold (moves from held to available)
    pub fn resolve(&mut self, amount: Balance) -> Result<(), &'static str> {
        if self.held >= amount {
            self.held -= amount;
            self.available += amount;
            Ok(())
        } else {
            Err("Held funds mismatch")
        }
    }

    /// Chargeback (removes from held and locks account)
    pub fn chargeback(&mut self, amount: Balance) -> Result<(), &'static str> {
        if self.held >= amount {
            self.held -= amount;
            self.total -= amount;
            self.status = AccountStatus::Locked;
            Ok(())
        } else {
            Err("Held funds mismatch")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_balance_arithmetic() {
        let b1 = Balance::new(dec!(10.0));
        let b2 = Balance::new(dec!(5.0));
        assert_eq!(b1 + b2, Balance::new(dec!(15.0)));
        assert_eq!(b1 - b2, Balance::new(dec!(5.0)));
    }

    #[test]
    fn test_account_deposit() {
        let mut account = ClientAccount::new(1);
        account.deposit(Balance::new(dec!(10.0)));
        assert_eq!(account.available, Balance::new(dec!(10.0)));
        assert_eq!(account.total, Balance::new(dec!(10.0)));
    }

    #[test]
    fn test_account_withdraw_success() {
        let mut account = ClientAccount::new(1);
        // Manually set for test setup if deposit not working yet?
        // Or assume deposit works. Let's rely on deposit or manual setting.
        account.available = Balance::new(dec!(10.0));
        account.total = Balance::new(dec!(10.0));

        let result = account.withdraw(Balance::new(dec!(5.0)));
        assert!(result.is_ok());
        assert_eq!(account.available, Balance::new(dec!(5.0)));
        assert_eq!(account.total, Balance::new(dec!(5.0)));
    }

    #[test]
    fn test_account_withdraw_insufficient() {
        let mut account = ClientAccount::new(1);
        account.available = Balance::new(dec!(10.0));

        let result = account.withdraw(Balance::new(dec!(20.0)));
        assert!(result.is_err());
        assert_eq!(account.available, Balance::new(dec!(10.0)));
    }

    #[test]
    fn test_account_hold_success() {
        let mut account = ClientAccount::new(1);
        account.available = Balance::new(dec!(10.0));
        account.total = Balance::new(dec!(10.0));

        let result = account.hold(Balance::new(dec!(5.0)));
        assert!(result.is_ok());
        assert_eq!(account.available, Balance::new(dec!(5.0)));
        assert_eq!(account.held, Balance::new(dec!(5.0)));
        assert_eq!(account.total, Balance::new(dec!(10.0)));
    }

    #[test]
    fn test_account_resolve() {
        let mut account = ClientAccount::new(1);
        account.available = Balance::new(dec!(5.0));
        account.held = Balance::new(dec!(5.0));
        account.total = Balance::new(dec!(10.0));

        let result = account.resolve(Balance::new(dec!(5.0)));
        assert!(result.is_ok());
        assert_eq!(account.available, Balance::new(dec!(10.0)));
        assert_eq!(account.held, Balance::new(dec!(0.0)));
        assert_eq!(account.total, Balance::new(dec!(10.0)));
    }

    #[test]
    fn test_account_chargeback() {
        let mut account = ClientAccount::new(1);
        account.available = Balance::new(dec!(5.0));
        account.held = Balance::new(dec!(5.0));
        account.total = Balance::new(dec!(10.0));

        let result = account.chargeback(Balance::new(dec!(5.0)));
        assert!(result.is_ok());
        assert_eq!(account.available, Balance::new(dec!(5.0)));
        assert_eq!(account.held, Balance::new(dec!(0.0)));
        assert_eq!(account.total, Balance::new(dec!(5.0)));
        assert_eq!(account.status, AccountStatus::Locked);
    }
}
