use crate::ledger::Amount;

use super::account::TransactionError;

/// A balance is a sum of credits (adds money to the balance)
/// and debits (remove money from the balance).
///
/// In this simple implementation, we only have the "current"
/// credit and debit, and mutate the balance.
/// In a more production-ready implementation, we'd have a
/// collection of debits and collection of credits instead, i.e.
/// an immutable collection of everything that happened.
#[derive(Clone)]
pub struct Balance {
    credit: Amount,
    debit: Amount,
}

impl Balance {
    pub fn amount(&self) -> Amount {
        self.credit - self.debit
    }

    pub fn add(&mut self, amount: Amount) -> Result<(), TransactionError> {
        self.credit = self
            .credit
            .checked_add(amount)
            .ok_or(TransactionError::Overflow)?;

        Ok(())
    }

    pub fn subtract(&mut self, amount: Amount) -> Result<(), TransactionError> {
        self.debit = self
            .debit
            .checked_add(amount)
            .ok_or(TransactionError::Overflow)?;

        Ok(())
    }

    pub const fn new(credit: Amount, debit: Amount) -> Self {
        Self { credit, debit }
    }
}

#[cfg(test)]
mod tests {
    use crate::ledger::account::account::TransactionError;

    use super::Balance;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use std::str::FromStr;

    #[test]
    fn test_balance_amount() {
        use rust_decimal_macros::dec;

        for (credit, debit, want) in vec![
            (dec!(10), dec!(0), dec!(10)),
            (dec!(0), dec!(10), dec!(-10)),
            (dec!(10), dec!(10), dec!(0)),
            (dec!(5), dec!(10), dec!(-5)),
        ] {
            assert_eq!(want, Balance::new(credit, debit).amount());
        }
    }

    #[test]
    fn test_add() {
        let mut balance = Balance::new(dec!(10), dec!(0));
        balance.add(dec!(7)).expect("should not overflow");

        assert_eq!(dec!(17), balance.amount());
    }

    #[test]
    fn test_add_overflow() {
        let very_big_number = Decimal::from_str("70000000000000000000000000000").unwrap();
        let mut balance = Balance::new(very_big_number, dec!(0));

        assert_eq!(
            Err(TransactionError::Overflow),
            balance.add(very_big_number)
        );
    }

    #[test]
    fn test_subtract() {
        let mut balance = Balance::new(dec!(10), dec!(0));
        balance.subtract(dec!(7)).expect("should not overflow");

        assert_eq!(dec!(3), balance.amount());
    }

    #[test]
    fn subtract_overflow() {
        let very_big_number = Decimal::from_str("70000000000000000000000000000").unwrap();
        let mut balance = Balance::new(dec!(0), very_big_number);

        assert_eq!(
            Err(TransactionError::Overflow),
            balance.subtract(very_big_number)
        );
    }
}
