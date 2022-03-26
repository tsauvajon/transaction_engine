use crate::ledger::TransactionId;

use super::account::{Account, TransactionError, TransactionState};

impl Account {
    pub(super) fn apply_resolve(&mut self, tx_id: TransactionId) -> Result<(), TransactionError> {
        let (tx_state, amount) = self.get_tx_state(tx_id)?;
        match tx_state {
            TransactionState::Disputed => {
                if self.held.amount() < amount {
                    return Err(TransactionError::NotEnoughFunds);
                }

                self.available.credit += amount;
                self.held.debit += amount;
                self.tx_states
                    .insert(tx_id, (TransactionState::Deposited, amount));

                Ok(())
            }
            _ => Err(TransactionError::InvalidTransaction),
        }
    }
}

#[cfg(test)]
mod resolve_tests {
    use crate::ledger::account::account::{Balance, TransactionError};

    use super::{Account, TransactionState};
    use rust_decimal_macros::dec;
    use std::collections::HashMap;

    #[test]
    fn test_resolve_ok() {
        let mut acc = Account {
            frozen: false,
            available: Balance::new(dec!(10.0), dec!(0)),
            held: Balance::new(dec!(10.0), dec!(0)),
            tx_states: HashMap::from([(1, (TransactionState::Disputed, dec!(5.0)))]),
        };

        let got = acc.apply_resolve(1);
        assert_eq!(Ok(()), got);
        assert_eq!(dec!(15.0), acc.available.amount());
        assert_eq!(dec!(5.0), acc.held.amount());
    }

    #[test]
    fn test_resolve_nok_negative_amount() {
        let mut acc = Account {
            frozen: false,
            available: Balance::new(dec!(10.0), dec!(0)),
            held: Balance::new(dec!(1.0), dec!(0)),
            tx_states: HashMap::from([(1, (TransactionState::Disputed, dec!(5.0)))]),
        };

        let got = acc.apply_resolve(1);
        assert_eq!(Err(TransactionError::NotEnoughFunds), got);
        assert_eq!(dec!(10.0), acc.available.amount());
        assert_eq!(dec!(1.0), acc.held.amount());
    }

    #[test]
    fn test_resolve_unknown_tx() {
        let mut acc = Account {
            frozen: false,
            available: Balance::new(dec!(10.0), dec!(0)),
            held: Balance::new(dec!(10.0), dec!(0)),
            tx_states: HashMap::new(),
        };

        let got = acc.apply_resolve(1);
        assert_eq!(Err(TransactionError::UnknownTransaction), got);
        assert_eq!(dec!(10.0), acc.available.amount());
        assert_eq!(dec!(10.0), acc.held.amount());
        assert_eq!(false, acc.frozen);
    }

    #[test]
    fn test_resolve_incorrect_state() {
        for state in vec![
            TransactionState::Withdrawn,
            TransactionState::Deposited,
            TransactionState::ChargedBack,
        ] {
            let mut acc = Account {
                frozen: false,
                available: Balance::new(dec!(99.99), dec!(0)),
                held: Balance::new(dec!(88.88), dec!(0)),
                tx_states: HashMap::from([(1, (state, dec!(123.456)))]),
            };

            let got = acc.apply_resolve(1);
            assert_eq!(Err(TransactionError::InvalidTransaction), got);
            assert_eq!(dec!(99.99), acc.available.amount());
            assert_eq!(dec!(88.88), acc.held.amount());
        }
    }
}
