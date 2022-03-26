use crate::ledger::TransactionId;

use super::account::{Account, TransactionError, TransactionState};

impl Account {
    pub(super) fn apply_dispute(&mut self, tx_id: TransactionId) -> Result<(), TransactionError> {
        let (tx_state, amount) = self.get_tx_state(tx_id)?;
        match tx_state {
            TransactionState::Deposited => {
                // We're doing two balance operations: add to held, subtract from
                // available. If one of them fails, we should roll back both of them.
                // That's why we're making a clone of the held balance first,
                // and restoring it if the second operation fails/
                let saved_held_balance = self.held.clone();
                self.held.add(amount)?;
                if let Err(e) = self.available.subtract(amount) {
                    self.held = saved_held_balance;
                    return Err(e);
                };

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
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use std::{collections::HashMap, str::FromStr};

    use crate::ledger::account::{
        account::{Account, TransactionError, TransactionState},
        balance::Balance,
    };

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
    fn test_resolve_nok_overflow_held() {
        let very_big_number = Decimal::from_str("70000000000000000000000000000").unwrap();

        let mut acc = Account {
            frozen: false,
            available: Balance::new(dec!(0), dec!(0)),
            held: Balance::new(very_big_number, dec!(0)),
            tx_states: HashMap::from([(1, (TransactionState::Deposited, very_big_number))]),
        };

        let got = acc.apply_dispute(1);
        assert_eq!(Err(TransactionError::Overflow), got);
        assert_eq!(dec!(0), acc.available.amount());
        assert_eq!(very_big_number, acc.held.amount());
    }

    #[test]
    fn test_resolve_nok_overflow_available() {
        let very_big_number = Decimal::from_str("70000000000000000000000000000").unwrap();

        let mut acc = Account {
            frozen: false,
            available: Balance::new(dec!(0), very_big_number),
            held: Balance::new(dec!(0), dec!(0)),
            tx_states: HashMap::from([(1, (TransactionState::Deposited, very_big_number))]),
        };

        let got = acc.apply_dispute(1);
        assert_eq!(Err(TransactionError::Overflow), got);
        assert_eq!(-very_big_number, acc.available.amount());
        assert_eq!(dec!(0), acc.held.amount());
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
