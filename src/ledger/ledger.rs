use super::account::{Account, TransactionError};
use super::transaction::Transaction;
use super::ClientId;
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};

/// Build the ledger.
/// It takes a stream of transactions, and gradually builds itself.
/// It will stream errors while building it, and once it has processed
/// all the transactions, it will send all account balances to the relevant
/// channel.
pub fn build(
    transactions: Receiver<Transaction>,
    accounts_tx: Sender<(ClientId, Account)>,
) -> Receiver<TransactionError> {
    let (tx, rx) = mpsc::channel();

    // We apply all transactions in a new thread, to be able to stream errors as
    // we go.
    std::thread::spawn(move || {
        let mut ledger: HashMap<ClientId, Account> = HashMap::new();

        for transaction in transactions {
            let balance = ledger
                .entry(transaction.client_id)
                .or_insert_with(Account::new);

            if let Err(err) = balance.apply(&transaction) {
                tx.send(err).unwrap(); // Would only fail if the rx is disconnected, which should not happen here.
            };
        }

        // We can only start sending account information once we have processed all the transactions.
        for (client_id, account) in ledger {
            accounts_tx.send((client_id, account)).unwrap(); // Would only fail if the rx is disconnected, which should not happen here.
        }
    });

    rx
}
