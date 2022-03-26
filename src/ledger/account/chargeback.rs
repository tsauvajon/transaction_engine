use crate::ledger::TransactionId;

use super::account::{Account, TransactionError, TransactionState};

impl Account {
    pub(super) fn apply_chargeback(
        &mut self,
        tx_id: TransactionId,
    ) -> Result<(), TransactionError> {
        let (tx_state, amount) = self.get_tx_state(tx_id)?;

        match tx_state {
            TransactionState::Disputed => {
                self.held.subtract(amount)?;
                self.tx_states
                    .insert(tx_id, (TransactionState::ChargedBack, amount));

                self.frozen = true;
                Ok(())
            }
            _ => Err(TransactionError::InvalidTransaction),
        }
    }
}

#[cfg(test)]
mod chargeback_tests {
    use crate::ledger::account::{account::TransactionError, balance::Balance};

    use super::{Account, TransactionState};
    use rust_decimal_macros::dec;
    use std::collections::HashMap;

    #[test]
    fn test_chargeback_ok() {
        let mut acc = Account {
            frozen: false,
            available: Balance::new(dec!(10.0), dec!(0)),
            held: Balance::new(dec!(10.0), dec!(0)),
            tx_states: HashMap::from([(1, (TransactionState::Disputed, dec!(8.0)))]),
        };

        let got = acc.apply_chargeback(1);
        assert_eq!(Ok(()), got);
        assert_eq!(dec!(10.0), acc.available.amount());
        assert_eq!(dec!(2.0), acc.held.amount());
        assert_eq!(true, acc.frozen);
    }

    #[test]
    fn test_chargeback_unknown_tx() {
        let mut acc = Account {
            frozen: false,
            available: Balance::new(dec!(10.0), dec!(0)),
            held: Balance::new(dec!(10.0), dec!(0)),
            tx_states: HashMap::new(),
        };

        let got = acc.apply_chargeback(1);
        assert_eq!(Err(TransactionError::UnknownTransaction), got);
        assert_eq!(dec!(10.0), acc.available.amount());
        assert_eq!(dec!(10.0), acc.held.amount());
        assert_eq!(false, acc.frozen);
    }

    #[test]
    fn test_chargeback_invalid_state() {
        for state in vec![
            TransactionState::Withdrawn,
            TransactionState::Deposited,
            TransactionState::ChargedBack,
        ] {
            let mut acc = Account {
                frozen: false,
                available: Balance::new(dec!(0), dec!(0)),
                held: Balance::new(dec!(88.88), dec!(0)),
                tx_states: HashMap::from([(1, (state, dec!(10.0)))]),
            };

            let got = acc.apply_chargeback(1);
            assert_eq!(Err(TransactionError::InvalidTransaction), got);
            assert_eq!(dec!(0), acc.available.amount());
            assert_eq!(dec!(88.88), acc.held.amount());
            assert_eq!(false, acc.frozen);
        }
    }
}
