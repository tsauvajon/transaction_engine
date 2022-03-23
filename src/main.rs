mod ledger;
mod parse;

use ledger::Ledger;
use parse::parse;
use std::fs::File;

fn main() {
    let input = File::open("transactions.csv").unwrap();
    let transactions = parse(input).expect("malformed CSV");

    let ledger = Ledger::new(transactions);

    for (client_id, balance) in ledger.ledger {
        println!(
            "{}: {} + {} = {}; frozen = {}",
            client_id,
            balance.available_amount,
            balance.held_amount,
            balance.total_amount(),
            balance.frozen
        );
    }
}
