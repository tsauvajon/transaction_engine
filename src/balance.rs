//! Handles clients balances through a ledger.
//!
//! Ledger: contains the balances of all the clients.
//! Balance: essentially a combination of state machine and transaction history.
//! -

use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum TransactionType {
    Withdrawal(f32), // Add a debit to the available balance. // TODO 4 decimal places for all amounts. Use bigdecimal??
    Deposit(f32), // Add a credit to the available balance. // TODO 4 decimal places for all amounts. Use bigdecimal??
    Dispute,      // Move funds from available to held.
    Resolve,      // Move funds from held to available.
    Chargeback,   // Remove funds from held. Immediately freeze the account.
}

#[derive(Debug, PartialEq)]
pub struct Transaction {
    pub tx_type: TransactionType,
    pub client_id: u16,
    pub tx_id: u32,
}

pub enum TransactionState {
    Withdrawn,
    Deposited,
    Disputed,
    ChargedBack,
}

// Key = client ID. Value = client Balance.
pub struct Ledger {
    pub ledger: HashMap<u16, Balance>,
}

impl Ledger {
    // To build a ledger, we need a list of transactions.
    // The ledger will build its own state.
    pub fn new(transactions: Vec<Transaction>) -> Self {
        let mut ledger: HashMap<u16, Balance> = HashMap::new();

        for tx in transactions {
            let balance = ledger.entry(tx.client_id).or_insert(Balance::new());

            if let Err(err) = balance.apply(tx) {
                // Here, we simply ignore the errors and keep processing other transactions.
                // It is explicitely stated that Disputes on non-existing transactions should
                // be ignored. I'm extending that behaviour to other kind of inconsistencies
                // that can be found.
                //
                // In a real-world scenario, we'd do more than just print errors.
                // We could either store them in some kind of tracing system where we can learn more about them,
                // or send some info to an external system (e.g. a queue + a dedicated service listening on it)
                // to deal with the error outside of this system.
                // We could also try to recover from some errors.
                println!("failed to apply transaction: {:?}", err);
            };
        }

        Self { ledger }
    }
}

// Note: I chose to keep errors simple here.
// In a real-world scenario, we would most likely need some debugging info
// (e.g. tx_id, client_id, amount, tx_type and some info about the current state)
#[derive(Debug)]
pub enum TransactionError {
    FrozenAccount,
    NotEnoughFunds,
    DuplicateTransaction,
    UnknownTransaction,
    InvalidTransaction,
}

// TODO: impl Serialize
pub struct Balance {
    pub frozen: bool, // TODO: serialise as `blocked`

    pub available_amount: f32, // TODO: serialise as `available` (TODO: bigdecimal?)
    pub held_amount: f32,      // TODO: serialise as `held`

    // Key: transaction ID. Value: state of that transaction and amount.
    // tx_states is a state machine. For example, you can only dispute a
    // Deposited transaction, and you can only resolve a Disputed transaction.
    pub tx_states: HashMap<u32, (TransactionState, f32)>,
}

impl Balance {
    fn new() -> Self {
        Balance {
            frozen: false,
            available_amount: 0.0,
            held_amount: 0.0,
            tx_states: HashMap::new(),
        }
    }

    pub fn total_amount(&self) -> f32 {
        self.available_amount + self.held_amount
    }

    // This could be improved by only storing transactions, and getting
    // the "current state" on demand, instead of mutating itself.
    //
    // I chose the "mutation" approach to make it easier to reason about, but
    // I believe it is an inferior approach, mainly because we lose traceability.
    //
    // I will refactor this if I can find the time to do it.
    fn apply(&mut self, tx: Transaction) -> Result<(), TransactionError> {
        // When an account is frozen, no transaction whatsoever should be applied to it.
        if self.frozen {
            return Err(TransactionError::FrozenAccount);
        }

        match tx.tx_type {
            TransactionType::Withdrawal(amount) => {
                if amount > self.available_amount {
                    return Err(TransactionError::NotEnoughFunds);
                }
                self.available_amount -= amount;

                // If we've already seen that transaction, we probably have a data issue.
                if self.tx_states.contains_key(&tx.tx_id) {
                    return Err(TransactionError::DuplicateTransaction);
                }

                self.tx_states
                    .insert(tx.tx_id, (TransactionState::Withdrawn, amount));
            }
            TransactionType::Deposit(amount) => {
                self.available_amount += amount;

                // If we've already seen that transaction, we probably have a data issue.
                if self.tx_states.contains_key(&tx.tx_id) {
                    return Err(TransactionError::DuplicateTransaction);
                }

                self.tx_states
                    .insert(tx.tx_id, (TransactionState::Deposited, amount));
            }
            TransactionType::Dispute => {
                // TODO: extract all duplicate code into functions.

                // Note:
                // I'm making the assumption that clients cannot dispute withdrawals.
                // I'm basing that on the fact that the PDF says that disputes
                // "decrease the available funds", i.e. cancels a deposit, but
                // never the opposite.

                // Get the state, or return an error if we don't know about this tx.
                let (tx_state, amount) = self
                    .tx_states
                    .get(&tx.tx_id)
                    .ok_or(TransactionError::UnknownTransaction)?;

                // Release self.tx_states so we can write into it.
                let tx_state = tx_state.to_owned();
                let amount = amount.to_owned();

                // Using the state machine, we know what to do in each possible situation.
                match tx_state {
                    TransactionState::Deposited => {
                        self.available_amount -= amount;
                        self.held_amount += amount;
                        self.tx_states
                            .insert(tx.tx_id, (TransactionState::Disputed, amount.to_owned()));
                    }
                    _ => return Err(TransactionError::InvalidTransaction),
                };
            }
            TransactionType::Resolve => {
                // Get the state, or return an error if we don't know about this tx.
                let (tx_state, amount) = self
                    .tx_states
                    .get(&tx.tx_id)
                    .ok_or(TransactionError::UnknownTransaction)?;

                // Release self.tx_states so we can write into it.
                let tx_state = tx_state.to_owned();
                let amount = amount.to_owned();

                // Using the state machine, we know what to do in each possible situation.
                match tx_state {
                    TransactionState::Disputed => {
                        self.available_amount += amount;
                        self.held_amount -= amount;
                        self.tx_states
                            .insert(tx.tx_id, (TransactionState::Deposited, amount.to_owned()));
                    }
                    _ => return Err(TransactionError::InvalidTransaction),
                };
            }
            TransactionType::Chargeback => {
                // Get the state, or return an error if we don't know about this tx.
                let (_, amount) = self
                    .tx_states
                    .get(&tx.tx_id)
                    .ok_or(TransactionError::UnknownTransaction)?;

                // Release self.tx_states so we can write into it.
                let amount = amount.to_owned();

                self.held_amount -= amount;
                // No matter what the state is, we'll set it to "charged back".
                self.tx_states
                    .insert(tx.tx_id, (TransactionState::ChargedBack, amount.to_owned()));

                self.frozen = true;
            }
        };

        Ok(())
    }
}
