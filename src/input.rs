use crate::ledger::transaction::{self, Transaction};

use rust_decimal::Decimal;
use serde::Deserialize;
use std::sync::mpsc::{self, Receiver, Sender};

#[derive(Debug, PartialEq)]
pub enum Error {
    Csv(String),    // CSV is malformed
    Format(String), // Data format is incorrect
}

impl From<csv::Error> for Error {
    fn from(err: csv::Error) -> Self {
        Self::Csv(err.to_string())
    }
}

impl From<<TransactionRecord as TryInto<Transaction>>::Error> for Error {
    fn from(err: <TransactionRecord as TryInto<Transaction>>::Error) -> Self {
        Self::Format(err.to_string())
    }
}

// When parsing, I'm making the assumption that we want to completely abort
// on errors.
// When we're reading a CSV file, it makes sense to fix the CSV (or the code),
// then try again.
// For a real-world scenario where we're receiving a stream of events instead,
// we would probably filter out bad rows and send them to an external system
// for analysis and recovery.
pub fn parse(
    input_stream: (impl std::io::Read + Send + 'static),
) -> (Receiver<Transaction>, Receiver<Error>) {
    let (transaction_tx, transaction_rx): (Sender<Transaction>, Receiver<Transaction>) =
        mpsc::channel();
    let (error_tx, error_rx): (Sender<Error>, Receiver<Error>) = mpsc::channel();

    let buffered = std::io::BufReader::new(input_stream);
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(buffered);

    // Moving to a new thread so we can start processing the transactions immediately.
    std::thread::spawn(move || {
        for record in reader.deserialize::<TransactionRecord>() {
            match convert(record) {
                Ok(transaction) => transaction_tx.send(transaction).unwrap(), // Would only fail if the rx is disconnected, which should not happen here.
                Err(err) => error_tx.send(err).unwrap(), // Would only fail if the rx is disconnected, which should not happen here.
            };
        }
    });

    (transaction_rx, error_rx)
}

// Convert from a csv deserialise result into a transaction result.
fn convert(record: Result<TransactionRecord, csv::Error>) -> Result<Transaction, Error> {
    Ok(record?.try_into()?)
}

#[test]
// Parsing well-formed data should return a vector of Transaction.
fn test_parse_ok() {
    let data = r#"type,client,tx,amount
deposit,1,1,1.0
withdrawal,1,4,1.5
dispute,1,1,
resolve,1,1,
chargeback,1,1,"#;
    let reader = std::io::Cursor::new(data);
    let (transactions, errors) = parse(reader);

    assert_eq!(5, transactions.iter().count());
    assert_eq!(0, errors.iter().count());
}

#[test]
fn test_parse_ok_with_whitespace() {
    let data = r#"type,     client,     tx,amount
deposit, 1, 1, 1.0
withdrawal , 1 , 4 , 1.5
dispute ,   1   ,   1   ,
    resolve ,1,1,
        chargeback                  ,1,1,"#;
    let reader = std::io::Cursor::new(data);
    let (transactions, errors) = parse(reader);

    assert_eq!(5, transactions.iter().count());
    assert_eq!(0, errors.iter().count());
}

#[test]
// Parsing incorrectly formatted data should return an Err.
fn test_parse_invalid_format() {
    for (data, err_contains) in vec![
        (
            r#"type,client,tx,amount
some_unknown_tx_type,1,1,1.0"#,
            "unknown variant `some_unknown_tx_type`",
        ),
        (
            r#"type,client,tx,amount
dispute,,1,"#, // missing client
            "cannot parse integer from empty string",
        ),
        (
            r#"type,client,tx,amount
dispute,1,,"#, // missing tx
            "cannot parse integer from empty string",
        ),
        (
            r#"type,client,tx,amount
dispute,1,1"#,
            "found record with 3 fields, but the previous record has 4 fields",
        ),
        (
            r#"type,client,tx,amount
dispute,1,1,,,,"#,
            "found record with 7 fields, but the previous record has 4 fields",
        ),
    ] {
        let reader = std::io::Cursor::new(data);
        let (transactions, errors) = parse(reader);

        assert_eq!(0, transactions.iter().count());

        let errs: Vec<Error> = errors.iter().collect();
        assert_eq!(1, errs.len());

        match &errs[0] {
            Error::Csv(msg) => assert!(msg.contains(err_contains), "{:?}", msg),
            _ => panic!("unexpected error"),
        }
    }
}

#[test]
// Parsing deposits or withdrawals without an amount should fail to convert
// into a Transaction.
fn test_parse_invalid_data() {
    for (data, want_err) in vec![
        (
            r#"type,client,tx,amount
deposit,1,1,"#,
            Error::Format("missing amount for deposit".to_string()),
        ),
        (
            r#"type,client,tx,amount
withdrawal,1,1,"#,
            Error::Format("missing amount for withdrawal".to_string()),
        ),
    ] {
        let reader = std::io::Cursor::new(data);
        let (transactions, errors) = parse(reader);

        assert_eq!(0, transactions.iter().count());

        let errs: Vec<Error> = errors.iter().collect();
        assert_eq!(vec![want_err], errs);
    }
}

// I have a TransactionRecord type because I can't directly deserialise into my "domain" type, i.e. Transaction.
// See https://github.com/BurntSushi/rust-csv/issues/211.
//
// This gives me way more flexibility in crafting a clean Transaction type,
// that makes the rest of the code easier to reason about.
// Besides, the internal Transaction type makes no assumption on how the transactions
// are actually formatted, so both domain logic and parsing are easier to maintain.
#[derive(Debug, Deserialize)]
pub struct TransactionRecord {
    #[serde(rename = "type")]
    tx_type: TransactionRecordType,

    #[serde(rename = "client")]
    client_id: u16,

    #[serde(rename = "tx")]
    transaction_id: u32,

    amount: Option<Decimal>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionRecordType {
    Withdrawal,
    Deposit,
    Dispute,
    Resolve,
    Chargeback,
}

impl TryFrom<TransactionRecord> for Transaction {
    type Error = &'static str;
    fn try_from(record: TransactionRecord) -> Result<Self, Self::Error> {
        let client_id = record.client_id;
        let tx_id = record.transaction_id;
        let tx_type = match record.tx_type {
            TransactionRecordType::Withdrawal => {
                transaction::Type::Withdrawal(match record.amount {
                    Some(amount) => amount,
                    None => return Err("missing amount for withdrawal"),
                })
            }
            TransactionRecordType::Deposit => transaction::Type::Deposit(match record.amount {
                Some(amount) => amount,
                None => return Err("missing amount for deposit"),
            }),
            TransactionRecordType::Dispute => transaction::Type::Dispute,
            TransactionRecordType::Resolve => transaction::Type::Resolve,
            TransactionRecordType::Chargeback => transaction::Type::Chargeback,
        };

        Ok(Self::new(tx_type, client_id, tx_id))
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
                amount: Some(Decimal::new(12, 1)),
            },
            Transaction::new(transaction::Type::Withdrawal(Decimal::new(12, 1)), 1, 5),
        ),
        (
            TransactionRecord {
                tx_type: TransactionRecordType::Deposit,
                client_id: 2,
                transaction_id: 4,
                amount: Some(Decimal::new(21, 1)),
            },
            Transaction::new(transaction::Type::Deposit(Decimal::new(21, 1)), 2, 4),
        ),
        (
            TransactionRecord {
                tx_type: TransactionRecordType::Dispute,
                client_id: 33,
                transaction_id: 333,
                amount: None,
            },
            Transaction::new(transaction::Type::Dispute, 33, 333),
        ),
        (
            TransactionRecord {
                tx_type: TransactionRecordType::Resolve,
                client_id: 44,
                transaction_id: 444,
                amount: None,
            },
            Transaction::new(transaction::Type::Resolve, 44, 444),
        ),
        (
            TransactionRecord {
                tx_type: TransactionRecordType::Chargeback,
                client_id: 55,
                transaction_id: 555,
                amount: None,
            },
            Transaction::new(transaction::Type::Chargeback, 55, 555),
        ),
    ];

    for (record, tx) in test_cases {
        assert_eq!(tx, record.try_into().unwrap());
    }
}

#[test]
// When the records are malformed, they should return an Err.
fn test_transaction_record_into_transaction_invalid_data() {
    let record = TransactionRecord {
        tx_type: TransactionRecordType::Withdrawal,
        client_id: 1,
        transaction_id: 2,
        amount: None,
    };

    let got = Transaction::try_from(record);
    assert_eq!(Err("missing amount for withdrawal"), got);
}
