//! Handles clients balances through a ledger.
//!
//! Ledger: contains the balances of all the clients.
//! Balance: essentially a combination of state machine and transaction history.
//! TODO: expand the documentation, as this is the most important part of the program.

use std::collections::HashMap;

// Using named types doesn't provide any compiler help, but it helps a lot with
// readability.
// Consider the following, when creating the ledger HashMap:
// (1) ledger: HashMap<u16, Balance>
// (2) ledger: HashMap<ClientId, Balance>
// Implementation (1) would most likely need comments, and could be confusing.
// Implementation (2) is self-explanatory.
// Besides, maintenance is easier: changing client ids e.g. from u16 to u32 is trivial.
pub type ClientId = u16;
pub type TransactionId = u32;
pub type Amount = f32; // TODO 4 decimal places for all amounts. Use bigdecimal??

#[derive(Debug, PartialEq)]
pub enum TransactionType {
    Withdrawal(Amount), // Add a debit to the available balance.
    Deposit(Amount),    // Add a credit to the available balance.
    Dispute,            // Move funds from available to held.
    Resolve,            // Move funds from held to available.
    Chargeback,         // Remove funds from held. Immediately freeze the account.
}

#[derive(Debug, PartialEq)]
pub struct Transaction {
    pub tx_type: TransactionType,
    pub client_id: ClientId,
    pub tx_id: TransactionId,
}

pub enum TransactionState {
    Withdrawn,
    Deposited,
    Disputed,
    ChargedBack,
}

/// A ledger builds clients balances from a list of transactions.
pub struct Ledger {
    pub ledger: HashMap<ClientId, Balance>,
}

impl Ledger {
    /// To build a ledger, we need a list of transactions.
    /// The ledger will build its own state, by applying each transaction to
    /// the correct client's balance.
    ///
    /// TODO: accept a stream of transactions, instead of a vec, and move the
    /// execution out of "new".
    pub fn new(transactions: Vec<Transaction>) -> Self {
        let mut ledger: HashMap<ClientId, Balance> = HashMap::new();

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

/// Note: I chose to keep errors simple here.
/// In a real-world scenario, we would most likely need some debugging info
/// (e.g. tx_id, client_id, amount, tx_type and some info about the current state)
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

    pub available_amount: Amount, // TODO: serialise as `available` (TODO: bigdecimal?)
    pub held_amount: Amount,      // TODO: serialise as `held`

    // tx_states is a state machine. For example, you can only dispute a
    // Deposited transaction, and you can only resolve a Disputed transaction.
    tx_states: HashMap<TransactionId, (TransactionState, Amount)>,
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
    fn apply(&mut self, tx: Transaction) -> Result<(), TransactionError> {
        // When an account is frozen, no transaction whatsoever should be applied to it.
        if self.frozen {
            return Err(TransactionError::FrozenAccount);
        }

        match tx.tx_type {
            TransactionType::Withdrawal(amount) => self.apply_withdrawal(tx.tx_id, amount),
            TransactionType::Deposit(amount) => self.apply_deposit(tx.tx_id, amount),
            TransactionType::Dispute => self.apply_dispute(tx.tx_id),
            TransactionType::Resolve => self.apply_resolve(tx.tx_id),
            TransactionType::Chargeback => self.apply_chargeback(tx.tx_id),
        }
    }

    // Get the current stored state of a transaction, as well as the transaction amount.
    fn get_tx_state(
        &self,
        tx_id: &TransactionId,
    ) -> Result<(&TransactionState, Amount), TransactionError> {
        let (tx_state, amount) = self
            .tx_states
            .get(&tx_id)
            .ok_or(TransactionError::UnknownTransaction)?;

        Ok((tx_state, *amount))
    }

    fn apply_withdrawal(
        &mut self,
        tx_id: TransactionId,
        amount: Amount,
    ) -> Result<(), TransactionError> {
        if amount > self.available_amount {
            return Err(TransactionError::NotEnoughFunds);
        }
        self.available_amount -= amount;

        // If we've already seen that transaction, we probably have a data issue.
        if self.tx_states.contains_key(&tx_id) {
            return Err(TransactionError::DuplicateTransaction);
        }

        self.tx_states
            .insert(tx_id, (TransactionState::Withdrawn, amount));

        Ok(())
    }

    fn apply_deposit(
        &mut self,
        tx_id: TransactionId,
        amount: Amount,
    ) -> Result<(), TransactionError> {
        self.available_amount += amount;

        // If we've already seen that transaction, we probably have a data issue.
        if self.tx_states.contains_key(&tx_id) {
            return Err(TransactionError::DuplicateTransaction);
        }

        self.tx_states
            .insert(tx_id, (TransactionState::Deposited, amount));

        Ok(())
    }

    fn apply_dispute(&mut self, tx_id: TransactionId) -> Result<(), TransactionError> {
        let (tx_state, amount) = self.get_tx_state(&tx_id)?;
        match tx_state {
            TransactionState::Deposited => {
                self.available_amount -= amount;
                self.held_amount += amount;
                self.tx_states
                    .insert(tx_id, (TransactionState::Disputed, amount.to_owned()));
            }
            _ => return Err(TransactionError::InvalidTransaction),
        };

        Ok(())
    }

    fn apply_resolve(&mut self, tx_id: TransactionId) -> Result<(), TransactionError> {
        let (tx_state, amount) = self.get_tx_state(&tx_id)?;
        match tx_state {
            TransactionState::Disputed => {
                self.available_amount += amount;
                self.held_amount -= amount;
                self.tx_states
                    .insert(tx_id, (TransactionState::Deposited, amount.to_owned()));
            }
            _ => return Err(TransactionError::InvalidTransaction),
        };

        Ok(())
    }

    fn apply_chargeback(&mut self, tx_id: TransactionId) -> Result<(), TransactionError> {
        let (_, amount) = self.get_tx_state(&tx_id)?;

        self.held_amount -= amount;
        // No matter what the state is, we'll set it to "charged back".
        self.tx_states
            .insert(tx_id, (TransactionState::ChargedBack, amount.to_owned()));

        self.frozen = true;

        Ok(())
    }
}
