use super::transaction::{self, Transaction};
use super::{Amount, TransactionId};
use std::collections::HashMap;

/// Note: I chose to keep errors simple here.
/// In a real-world scenario, we would most likely need some debugging info
/// (e.g. `tx_id`, `client_id`, `amount`, `tx_type` and some info about the current state)
#[derive(Debug, PartialEq)]
pub enum TransactionError {
    /// Account is already frozen, so no more transactions can be applied.
    FrozenAccount,

    /// Funds in the account are unsufficient for a withdrawal.
    NotEnoughFunds,

    /// A transaction with the same ID already exists.
    DuplicateTransaction,

    /// The dispute, resolve or chargeback references a transaction that doesn't exist.
    UnknownTransaction,

    /// A dispute or resolve is applied on a transaction, but the current transaction
    /// state doesn't allow it.
    InvalidTransaction,
}

/// The current state of a transaction, used to know whether we apply a new
/// status on it or not.
pub enum TransactionState {
    /// An amount has been withdrawn.
    Withdrawn,

    /// An amount has been deposited.
    Deposited,

    /// The deposit was disputed.
    Disputed,

    /// The deposit was charged back.
    ChargedBack,
}

/// Account is a state-machine, to which you can apply transactions.
///
/// In other words, every time you apply a transaction to the Account, it will
/// update its own state to be an accurate representation of the current
/// account balance and state.
///
/// In the assignment PDF, a balance is interchangeably called account, account
/// balance, client account, asset account.
pub struct Account {
    pub frozen: bool,
    pub available_amount: Amount,
    pub held_amount: Amount,

    // tx_states is a state machine. For example, you can only dispute a
    // Deposited transaction, and you can only resolve a Disputed transaction.
    tx_states: HashMap<TransactionId, (TransactionState, Amount)>,
}

impl Account {
    pub fn new() -> Self {
        Self {
            frozen: false,
            available_amount: rust_decimal::Decimal::default(),
            held_amount: rust_decimal::Decimal::default(),
            tx_states: HashMap::new(),
        }
    }

    /// Get the total amount stored in the balance.
    pub fn total_amount(&self) -> Amount {
        self.available_amount + self.held_amount
    }

    // This could be improved by only storing transactions, and getting
    // the "current state" on demand, instead of mutating itself.
    //
    // I chose the "mutation" approach to make it easier to reason about, but
    // I believe it is an inferior approach, mainly because we lose traceability.
    //
    // I will refactor this if I can find the time to do it.
    //
    // Note:
    // I'm making the assumption that clients cannot dispute withdrawals.
    // I'm basing that on the fact that the PDF says that disputes
    // "decrease the available funds", i.e. cancels a deposit, but
    // never the opposite.
    pub(super) fn apply(&mut self, tx: &Transaction) -> Result<(), TransactionError> {
        // When an account is frozen, no transaction whatsoever should be applied to it.
        if self.frozen {
            return Err(TransactionError::FrozenAccount);
        }

        match tx.tx_type {
            transaction::Type::Withdrawal(amount) => self.apply_withdrawal(tx.tx_id, amount),
            transaction::Type::Deposit(amount) => self.apply_deposit(tx.tx_id, amount),
            transaction::Type::Dispute => self.apply_dispute(tx.tx_id),
            transaction::Type::Resolve => self.apply_resolve(tx.tx_id),
            transaction::Type::Chargeback => self.apply_chargeback(tx.tx_id),
        }
    }

    // Get the current stored state of a transaction, as well as the transaction amount.
    fn get_tx_state(
        &self,
        tx_id: TransactionId,
    ) -> Result<(&TransactionState, Amount), TransactionError> {
        let (tx_state, amount) = self
            .tx_states
            .get(&tx_id)
            .ok_or(TransactionError::UnknownTransaction)?;

        Ok((tx_state, *amount))
    }
}

#[test]
fn test_apply_on_frozen_account() {
    use crate::ledger::account::TransactionError;

    use rust_decimal_macros::dec;
    use std::collections::HashMap;

    let mut acc = Account {
        frozen: true,
        available_amount: dec!(3.0),
        held_amount: dec!(1.0),
        tx_states: HashMap::new(),
    };

    let transaction = Transaction {
        tx_type: transaction::Type::Deposit(dec!(5000)),
        client_id: 15,
        tx_id: 12,
    };

    let got = acc.apply(&transaction);
    assert_eq!(Err(TransactionError::FrozenAccount), got);
}

#[test]
fn test_total_amount() {
    use rust_decimal_macros::dec;
    use std::collections::HashMap;

    let acc = Account {
        frozen: false,
        available_amount: dec!(3.0),
        held_amount: dec!(1.0),
        tx_states: HashMap::new(),
    };
    assert_eq!(dec!(4.0), acc.total_amount());
}

impl Account {
    fn apply_withdrawal(
        &mut self,
        tx_id: TransactionId,
        amount: Amount,
    ) -> Result<(), TransactionError> {
        if amount > self.available_amount {
            return Err(TransactionError::NotEnoughFunds);
        }

        // If we've already seen that transaction, we probably have a data issue.
        if self.tx_states.contains_key(&tx_id) {
            return Err(TransactionError::DuplicateTransaction);
        }

        self.available_amount -= amount;
        self.tx_states
            .insert(tx_id, (TransactionState::Withdrawn, amount));

        Ok(())
    }
}

#[cfg(test)]
mod withdrawal_tests {
    use crate::ledger::account::TransactionError;

    use super::{Account, TransactionState};
    use rust_decimal_macros::dec;
    use std::collections::HashMap;

    #[test]
    fn test_withdrawal_ok() {
        let mut acc = Account {
            frozen: false,
            available_amount: dec!(3.0),
            held_amount: dec!(1.0),
            tx_states: HashMap::new(),
        };

        let got = acc.apply_withdrawal(1, dec!(3.0));
        assert_eq!(Ok(()), got);
        assert_eq!(dec!(0), acc.available_amount);
        assert_eq!(dec!(1.0), acc.held_amount);
    }

    #[test]
    fn test_withdrawal_not_enough_funds() {
        let mut acc = Account {
            frozen: false,
            available_amount: dec!(2.5),
            held_amount: dec!(1.0),
            tx_states: HashMap::new(),
        };

        let got = acc.apply_withdrawal(1, dec!(3.0));
        assert_eq!(Err(TransactionError::NotEnoughFunds), got);
        assert_eq!(dec!(2.5), acc.available_amount);
        assert_eq!(dec!(1.0), acc.held_amount);
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
                available_amount: dec!(99.99),
                held_amount: dec!(88.88),
                tx_states: HashMap::from([(1, (state, dec!(123.456)))]),
            };

            let got = acc.apply_withdrawal(1, dec!(3.0));
            assert_eq!(Err(TransactionError::DuplicateTransaction), got);
            assert_eq!(dec!(99.99), acc.available_amount);
            assert_eq!(dec!(88.88), acc.held_amount);
        }
    }
}

impl Account {
    fn apply_deposit(
        &mut self,
        tx_id: TransactionId,
        amount: Amount,
    ) -> Result<(), TransactionError> {
        // We've already seen that transaction, so we probably have a data issue.
        // We can safely return an error.
        if self.tx_states.contains_key(&tx_id) {
            return Err(TransactionError::DuplicateTransaction);
        }

        self.available_amount += amount;

        self.tx_states
            .insert(tx_id, (TransactionState::Deposited, amount));

        Ok(())
    }
}

#[cfg(test)]
mod deposit_tests {
    use crate::ledger::account::TransactionError;

    use super::{Account, TransactionState};
    use rust_decimal_macros::dec;
    use std::collections::HashMap;

    #[test]
    fn test_deposit_ok() {
        let mut acc = Account {
            frozen: false,
            available_amount: dec!(3.0),
            held_amount: dec!(1.0),
            tx_states: HashMap::new(),
        };

        let got = acc.apply_deposit(1, dec!(3.0));
        assert_eq!(Ok(()), got);
        assert_eq!(dec!(6.0), acc.available_amount);
        assert_eq!(dec!(1.0), acc.held_amount);
    }

    #[test]
    fn test_deposit_already_exists() {
        for state in vec![
            TransactionState::Withdrawn,
            TransactionState::Deposited,
            TransactionState::Disputed,
            TransactionState::ChargedBack,
        ] {
            let mut acc = Account {
                frozen: false,
                available_amount: dec!(99.99),
                held_amount: dec!(88.88),
                tx_states: HashMap::from([(1, (state, dec!(123.456)))]),
            };

            let got = acc.apply_deposit(1, dec!(3.0));
            assert_eq!(Err(TransactionError::DuplicateTransaction), got);
            assert_eq!(dec!(99.99), acc.available_amount);
            assert_eq!(dec!(88.88), acc.held_amount);
        }
    }
}

impl Account {
    fn apply_dispute(&mut self, tx_id: TransactionId) -> Result<(), TransactionError> {
        let (tx_state, amount) = self.get_tx_state(tx_id)?;
        match tx_state {
            TransactionState::Deposited => {
                self.available_amount -= amount;
                self.held_amount += amount;
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
    use crate::ledger::account::TransactionError;

    use super::{Account, TransactionState};
    use rust_decimal_macros::dec;
    use std::collections::HashMap;

    #[test]
    fn test_dispute_ok() {
        let mut acc = Account {
            frozen: false,
            available_amount: dec!(8.0),
            held_amount: dec!(1.0),
            tx_states: HashMap::from([(1, (TransactionState::Deposited, dec!(5.0)))]),
        };

        let got = acc.apply_dispute(1);
        assert_eq!(Ok(()), got);
        assert_eq!(dec!(3.0), acc.available_amount);
        assert_eq!(dec!(6.0), acc.held_amount);
    }

    #[test]
    fn test_dispute_ok_negative_amount() {
        let mut acc = Account {
            frozen: false,
            available_amount: dec!(0.0),
            held_amount: dec!(1.0),
            tx_states: HashMap::from([(1, (TransactionState::Deposited, dec!(5.0)))]),
        };

        let got = acc.apply_dispute(1);
        assert_eq!(Ok(()), got);
        assert_eq!(dec!(-5.0), acc.available_amount);
        assert_eq!(dec!(6.0), acc.held_amount);
    }

    #[test]
    fn test_dispute_unknown_tx() {
        let mut acc = Account {
            frozen: false,
            available_amount: dec!(10.0),
            held_amount: dec!(10.0),
            tx_states: HashMap::new(),
        };

        let got = acc.apply_dispute(1);
        assert_eq!(Err(TransactionError::UnknownTransaction), got);
        assert_eq!(dec!(10.0), acc.available_amount);
        assert_eq!(dec!(10.0), acc.held_amount);
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
                available_amount: dec!(99.99),
                held_amount: dec!(88.88),
                tx_states: HashMap::from([(1, (state, dec!(123.456)))]),
            };

            let got = acc.apply_dispute(1);
            assert_eq!(Err(TransactionError::InvalidTransaction), got);
            assert_eq!(dec!(99.99), acc.available_amount);
            assert_eq!(dec!(88.88), acc.held_amount);
        }
    }
}

impl Account {
    fn apply_resolve(&mut self, tx_id: TransactionId) -> Result<(), TransactionError> {
        let (tx_state, amount) = self.get_tx_state(tx_id)?;
        match tx_state {
            TransactionState::Disputed => {
                if self.held_amount < amount {
                    return Err(TransactionError::NotEnoughFunds);
                }

                self.available_amount += amount;
                self.held_amount -= amount;
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
    use crate::ledger::account::TransactionError;

    use super::{Account, TransactionState};
    use rust_decimal_macros::dec;
    use std::collections::HashMap;

    #[test]
    fn test_resolve_ok() {
        let mut acc = Account {
            frozen: false,
            available_amount: dec!(10.0),
            held_amount: dec!(10.0),
            tx_states: HashMap::from([(1, (TransactionState::Disputed, dec!(5.0)))]),
        };

        let got = acc.apply_resolve(1);
        assert_eq!(Ok(()), got);
        assert_eq!(dec!(15.0), acc.available_amount);
        assert_eq!(dec!(5.0), acc.held_amount);
    }

    #[test]
    fn test_resolve_nok_negative_amount() {
        let mut acc = Account {
            frozen: false,
            available_amount: dec!(10.0),
            held_amount: dec!(1.0),
            tx_states: HashMap::from([(1, (TransactionState::Disputed, dec!(5.0)))]),
        };

        let got = acc.apply_resolve(1);
        assert_eq!(Err(TransactionError::NotEnoughFunds), got);
        assert_eq!(dec!(10.0), acc.available_amount);
        assert_eq!(dec!(1.0), acc.held_amount);
    }

    #[test]
    fn test_resolve_unknown_tx() {
        let mut acc = Account {
            frozen: false,
            available_amount: dec!(10.0),
            held_amount: dec!(10.0),
            tx_states: HashMap::new(),
        };

        let got = acc.apply_resolve(1);
        assert_eq!(Err(TransactionError::UnknownTransaction), got);
        assert_eq!(dec!(10.0), acc.available_amount);
        assert_eq!(dec!(10.0), acc.held_amount);
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
                available_amount: dec!(99.99),
                held_amount: dec!(88.88),
                tx_states: HashMap::from([(1, (state, dec!(123.456)))]),
            };

            let got = acc.apply_resolve(1);
            assert_eq!(Err(TransactionError::InvalidTransaction), got);
            assert_eq!(dec!(99.99), acc.available_amount);
            assert_eq!(dec!(88.88), acc.held_amount);
        }
    }
}

impl Account {
    fn apply_chargeback(&mut self, tx_id: TransactionId) -> Result<(), TransactionError> {
        let (tx_state, amount) = self.get_tx_state(tx_id)?;

        match tx_state {
            TransactionState::Disputed => {
                self.held_amount -= amount;
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
    use crate::ledger::account::TransactionError;

    use super::{Account, TransactionState};
    use rust_decimal_macros::dec;
    use std::collections::HashMap;

    #[test]
    fn test_chargeback_ok() {
        let mut acc = Account {
            frozen: false,
            available_amount: dec!(10.0),
            held_amount: dec!(10.0),
            tx_states: HashMap::from([(1, (TransactionState::Disputed, dec!(8.0)))]),
        };

        let got = acc.apply_chargeback(1);
        assert_eq!(Ok(()), got);
        assert_eq!(dec!(10.0), acc.available_amount);
        assert_eq!(dec!(2.0), acc.held_amount);
        assert_eq!(true, acc.frozen);
    }

    #[test]
    fn test_chargeback_unknown_tx() {
        let mut acc = Account {
            frozen: false,
            available_amount: dec!(10.0),
            held_amount: dec!(10.0),
            tx_states: HashMap::new(),
        };

        let got = acc.apply_chargeback(1);
        assert_eq!(Err(TransactionError::UnknownTransaction), got);
        assert_eq!(dec!(10.0), acc.available_amount);
        assert_eq!(dec!(10.0), acc.held_amount);
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
                available_amount: dec!(0),
                held_amount: dec!(88.88),
                tx_states: HashMap::from([(1, (state, dec!(10.0)))]),
            };

            let got = acc.apply_chargeback(1);
            assert_eq!(Err(TransactionError::InvalidTransaction), got);
            assert_eq!(dec!(0), acc.available_amount);
            assert_eq!(dec!(88.88), acc.held_amount);
            assert_eq!(false, acc.frozen);
        }
    }
}
