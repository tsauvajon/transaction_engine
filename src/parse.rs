use super::balance::{Transaction, TransactionType};
use serde::Deserialize;

pub fn parse(input: impl std::io::Read) -> Vec<Transaction> {
    let buffered = std::io::BufReader::new(input);
    let mut reader = csv::Reader::from_reader(buffered);

    reader
        .deserialize::<TransactionRecord>()
        .map(|row| row.unwrap().into()) // todo: return a result instead of unwrapping
        .collect()
}

#[test]
// Parsing well-formed data should return a vector of Transaction.
fn test_parse_ok() {
    todo!()
}

#[test]
// Parsing invalid data should return an Err.
fn test_parse_invalid_data() {
    todo!()
}

// I have a TransactionRecord type because I can't directly deserialise into my "domain" type, i.e. Transaction.
// See https://github.com/BurntSushi/rust-csv/issues/211.
//
// This gives me way more flexibility in crafting a clean Transaction type,
// that makes the rest of the code easier to reason about.
// Besides, the internal Transaction type makes no assumption on how the transactions
// are actually formatted, so both domain logic and parsing are easier to maintain.
#[derive(Debug, Deserialize)]
struct TransactionRecord {
    #[serde(rename = "type")]
    tx_type: TransactionRecordType,

    #[serde(rename = "client")]
    client_id: u16,

    #[serde(rename = "tx")]
    transaction_id: u32,

    amount: Option<f32>,
}

#[derive(Debug, Deserialize)]
enum TransactionRecordType {
    #[serde(rename = "withdrawal")]
    Withdrawal,

    #[serde(rename = "deposit")]
    Deposit,

    Dispute,
    Resolve,
    Chargeback,
}

impl From<TransactionRecord> for Transaction {
    fn from(record: TransactionRecord) -> Self {
        Self {
            client_id: record.client_id,
            tx_id: record.transaction_id,
            tx_type: match record.tx_type {
                TransactionRecordType::Withdrawal => {
                    TransactionType::Withdrawal(record.amount.unwrap())
                } // TODO: implement try_from instead
                TransactionRecordType::Deposit => TransactionType::Deposit(record.amount.unwrap()), // TODO: implement try_from instead
                TransactionRecordType::Dispute => TransactionType::Dispute,
                TransactionRecordType::Resolve => TransactionType::Resolve,
                TransactionRecordType::Chargeback => TransactionType::Chargeback,
            },
        }
    }
}

#[test]
// When the records are well formed, they should be correctly converted into Transaction.
fn test_transaction_record_into_transaction_well_formed() {
    let test_cases: Vec<(TransactionRecord, Transaction)> = vec![
        (
            TransactionRecord {
                tx_type: TransactionRecordType::Withdrawal,
                client_id: 1,
                transaction_id: 5,
                amount: Some(1.2),
            },
            Transaction {
                tx_type: TransactionType::Withdrawal(1.2),
                client_id: 1,
                tx_id: 5,
            },
        ),
        (
            TransactionRecord {
                tx_type: TransactionRecordType::Deposit,
                client_id: 2,
                transaction_id: 4,
                amount: Some(2.1),
            },
            Transaction {
                tx_type: TransactionType::Deposit(2.1),
                client_id: 2,
                tx_id: 4,
            },
        ),
        (
            TransactionRecord {
                tx_type: TransactionRecordType::Dispute,
                client_id: 33,
                transaction_id: 333,
                amount: None,
            },
            Transaction {
                tx_type: TransactionType::Dispute,
                client_id: 33,
                tx_id: 333,
            },
        ),
        (
            TransactionRecord {
                tx_type: TransactionRecordType::Resolve,
                client_id: 44,
                transaction_id: 444,
                amount: None,
            },
            Transaction {
                tx_type: TransactionType::Resolve,
                client_id: 44,
                tx_id: 444,
            },
        ),
        (
            TransactionRecord {
                tx_type: TransactionRecordType::Chargeback,
                client_id: 55,
                transaction_id: 555,
                amount: None,
            },
            Transaction {
                tx_type: TransactionType::Chargeback,
                client_id: 55,
                tx_id: 555,
            },
        ),
    ];

    for (record, tx) in test_cases {
        assert_eq!(tx, record.into());
    }
}

#[test]
// Decimal precision is 4 places. The parsed amounts should reflect that.
fn test_transaction_record_into_transaction_decimal_places() {
    todo!();
}

#[test]
// When the records are malformed, they should return an Err.
fn test_transaction_record_into_transaction_invalid_data() {
    todo!();
}
