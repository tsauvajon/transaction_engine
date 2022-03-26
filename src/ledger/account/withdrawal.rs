use crate::ledger::{Amount, TransactionId};

use super::account::{Account, TransactionError, TransactionState};

impl Account {
    pub(super) fn apply_withdrawal(
        &mut self,
        tx_id: TransactionId,
        amount: Amount,
    ) -> Result<(), TransactionError> {
        if amount > self.available.amount() {
            return Err(TransactionError::NotEnoughFunds);
        }

        // If we've already seen that transaction, we probably have a data issue.
        if self.tx_states.contains_key(&tx_id) {
            return Err(TransactionError::DuplicateTransaction);
        }

        self.available.subtract(amount)?;
        self.tx_states
            .insert(tx_id, (TransactionState::Withdrawn, amount));

        Ok(())
    }
}

#[cfg(test)]
mod withdrawal_tests {
    use crate::ledger::account::{account::TransactionError, balance::Balance};

    use super::{Account, TransactionState};
    use rust_decimal_macros::dec;
    use std::collections::HashMap;

    #[test]
    fn test_withdrawal_ok() {
        let mut acc = Account {
            frozen: false,
            available: Balance::new(dec!(3.0), dec!(0)),
            held: Balance::new(dec!(1.0), dec!(0)),
            tx_states: HashMap::new(),
        };

        let got = acc.apply_withdrawal(1, dec!(3.0));
        assert_eq!(Ok(()), got);
        assert_eq!(dec!(0), acc.available.amount());
        assert_eq!(dec!(1.0), acc.held.amount());
    }

    #[test]
    fn test_withdrawal_not_enough_funds() {
        let mut acc = Account {
            frozen: false,
            available: Balance::new(dec!(2.5), dec!(0)),
            held: Balance::new(dec!(1.0), dec!(0)),
            tx_states: HashMap::new(),
        };

        let got = acc.apply_withdrawal(1, dec!(3.0));
        assert_eq!(Err(TransactionError::NotEnoughFunds), got);
        assert_eq!(dec!(2.5), acc.available.amount());
        assert_eq!(dec!(1.0), acc.held.amount());
    }

    #[test]
    fn test_withdrawal_already_exists() {
        for state in vec![
            TransactionState::Withdrawn,
            TransactionState::Deposited,
            TransactionState::Disputed,
            TransactionState::ChargedBack,
        ] {
            let mut acc = Account {
                frozen: false,
                available: Balance::new(dec!(99.99), dec!(0)),
                held: Balance::new(dec!(88.88), dec!(0)),
                tx_states: HashMap::from([(1, (state, dec!(123.456)))]),
            };

            let got = acc.apply_withdrawal(1, dec!(3.0));
            assert_eq!(Err(TransactionError::DuplicateTransaction), got);
            assert_eq!(dec!(99.99), acc.available.amount());
            assert_eq!(dec!(88.88), acc.held.amount());
        }
    }
}
