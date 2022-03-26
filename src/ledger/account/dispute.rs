use crate::ledger::TransactionId;

use super::account::{Account, TransactionError, TransactionState};

impl Account {
    pub(super) fn apply_dispute(&mut self, tx_id: TransactionId) -> Result<(), TransactionError> {
        let (tx_state, amount) = self.get_tx_state(tx_id)?;
        match tx_state {
            TransactionState::Deposited => {
                self.available.debit += amount;
                self.held.credit += amount;
                self.tx_states
                    .insert(tx_id, (TransactionState::Disputed, amount));

                Ok(())
            }
            _ => Err(TransactionError::InvalidTransaction),
        }
    }
}

#[cfg(test)]
mod dispute_tests {
    use rust_decimal_macros::dec;
    use std::collections::HashMap;

    use crate::ledger::account::account::{Account, Balance, TransactionError, TransactionState};

    #[test]
    fn test_dispute_ok() {
        let mut acc = Account {
            frozen: false,
            available: Balance::new(dec!(8.0), dec!(0)),
            held: Balance::new(dec!(1.0), dec!(0)),
            tx_states: HashMap::from([(1, (TransactionState::Deposited, dec!(5.0)))]),
        };

        let got = acc.apply_dispute(1);
        assert_eq!(Ok(()), got);
        assert_eq!(dec!(3.0), acc.available.amount());
        assert_eq!(dec!(6.0), acc.held.amount());
    }

    #[test]
    fn test_dispute_ok_negative_amount() {
        let mut acc = Account {
            frozen: false,
            available: Balance::new(dec!(0.0), dec!(0)),
            held: Balance::new(dec!(1.0), dec!(0)),
            tx_states: HashMap::from([(1, (TransactionState::Deposited, dec!(5.0)))]),
        };

        let got = acc.apply_dispute(1);
        assert_eq!(Ok(()), got);
        assert_eq!(dec!(-5.0), acc.available.amount());
        assert_eq!(dec!(6.0), acc.held.amount());
    }

    #[test]
    fn test_dispute_unknown_tx() {
        let mut acc = Account {
            frozen: false,
            available: Balance::new(dec!(10.0), dec!(0)),
            held: Balance::new(dec!(10.0), dec!(0)),
            tx_states: HashMap::new(),
        };

        let got = acc.apply_dispute(1);
        assert_eq!(Err(TransactionError::UnknownTransaction), got);
        assert_eq!(dec!(10.0), acc.available.amount());
        assert_eq!(dec!(10.0), acc.held.amount());
        assert_eq!(false, acc.frozen);
    }

    #[test]
    fn test_dispute_incorrect_state() {
        for state in vec![
            TransactionState::Withdrawn,
            TransactionState::Disputed,
            TransactionState::ChargedBack,
        ] {
            let mut acc = Account {
                frozen: false,
                available: Balance::new(dec!(99.99), dec!(0)),
                held: Balance::new(dec!(88.88), dec!(0)),
                tx_states: HashMap::from([(1, (state, dec!(123.456)))]),
            };

            let got = acc.apply_dispute(1);
            assert_eq!(Err(TransactionError::InvalidTransaction), got);
            assert_eq!(dec!(99.99), acc.available.amount());
            assert_eq!(dec!(88.88), acc.held.amount());
        }
    }
}
