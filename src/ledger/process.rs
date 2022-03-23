use super::account::{Account, TransactionError};
use super::transaction::Transaction;
use super::ClientId;
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};

pub fn process(
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

        for (client_id, account) in ledger {
            accounts_tx.send((client_id, account)).unwrap(); // Would only fail if the rx is disconnected, which should not happen here.
        }
    });

    rx
}
