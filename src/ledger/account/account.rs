use crate::ledger::{
    transaction::{self, Transaction},
    Amount, TransactionId,
};

use rust_decimal_macros::dec;
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

/// A balance is a sum of credits (adds more to the balance)
/// and debits (remove money from the balance).
///
/// In this simple implementation, we only have the "current"
/// credit and debit, and mutate the balance.
/// In a more production-ready implementation, we'd have a
/// collection of debits and collection of credits instead.
///
/// It could also have helps to add or remove money, compare
/// to another balance...
pub struct Balance {
    pub(super) credit: Amount,
    pub(super) debit: Amount,
}

impl Balance {
    pub fn amount(&self) -> Amount {
        self.credit - self.debit
    }

    pub const fn new(credit: Amount, debit: Amount) -> Self {
        Self { credit, debit }
    }
}

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

/// Account is a state-machine, to which you can apply transactions.
///
/// In other words, every time you apply a transaction to the Account, it will
/// update its own state to be an accurate representation of the current
/// account balance and state.
///
/// In the assignment PDF, an account is interchangeably called account, account
/// balance, client account, asset account.
pub struct Account {
    pub frozen: bool,
    pub available: Balance,
    pub held: Balance,

    // tx_states holds the last known state of each transaction.
    pub(super) tx_states: HashMap<TransactionId, (TransactionState, Amount)>,
}

impl Account {
    pub fn new() -> Self {
        Self {
            frozen: false,
            available: Balance::new(dec!(0), dec!(0)),
            held: Balance::new(dec!(0), dec!(0)),
            tx_states: HashMap::new(),
        }
    }

    /// Calculate the total amount stored in the balance.
    pub fn total_amount(&self) -> Amount {
        self.available.amount() + self.held.amount()
    }

    // Note:
    // I'm making the assumption that clients cannot dispute withdrawals.
    // I'm basing that on the fact that the PDF says that disputes
    // "decrease the available funds", i.e. cancels a deposit, but
    // never the opposite. This also seems to generally make sense.
    pub fn apply(&mut self, tx: &Transaction) -> Result<(), TransactionError> {
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

    /// Get the current stored state of a transaction, as well as the transaction amount.
    pub(super) fn get_tx_state(
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
    use rust_decimal_macros::dec;
    use std::collections::HashMap;

    let mut acc = Account {
        frozen: true,
        available: Balance::new(dec!(3.0), dec!(0)),
        held: Balance::new(dec!(1.0), dec!(2.0)),
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
        available: Balance::new(dec!(3.0), dec!(0)),
        held: Balance::new(dec!(1.0), dec!(2.0)),
        tx_states: HashMap::new(),
    };
    assert_eq!(dec!(2.0), acc.total_amount());
}

#[test]
fn test_available_amount() {
    use rust_decimal_macros::dec;
    use std::collections::HashMap;

    let acc = Account {
        frozen: false,
        available: Balance::new(dec!(3.0), dec!(0)),
        held: Balance::new(dec!(1.0), dec!(2.0)),
        tx_states: HashMap::new(),
    };
    assert_eq!(dec!(3.0), acc.available.amount());
}

#[test]
fn test_held_amount() {
    use rust_decimal_macros::dec;
    use std::collections::HashMap;

    let acc = Account {
        frozen: false,
        available: Balance::new(dec!(3.0), dec!(0)),
        held: Balance::new(dec!(1.0), dec!(2.0)),
        tx_states: HashMap::new(),
    };
    assert_eq!(dec!(-1.0), acc.held.amount());
}
