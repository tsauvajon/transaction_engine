mod parse;

use std::fs::File;

fn main() {
    let input = File::open("transactions.csv").unwrap();
    parse::parse_file(input);
}

// TODO 4 decimal places for all amounts

// enum Transaction {
//     Withdrawal { amount: f32 }, // Add a debit to the available balance.
//     Deposit { amount: f32 },    // Add a credit to the available balance.
//     Dispute,                    // Move funds from available to held.
//     Resolve,                    // Move funds from held to available.
//     Chargeback,                 // Remove funds from held. Immediately freeze the account.
// }

// #[derive(Debug, Deserialize)]
// struct TransactionInfo {
//     client_id: u16,
//     tx_id: u32,
// }

// struct Output {
//     client_id: u16,        // client
//     available_amount: f32, // available
//     held_amount: f32,      // held
//     total_amount: f32,     // total,
//     locked: bool,
// }
