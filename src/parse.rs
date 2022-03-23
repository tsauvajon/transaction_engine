use super::ledger::{Transaction, TransactionType};
use serde::Deserialize;

#[derive(Debug, PartialEq)]
pub enum Error {
    Csv(String),
    Format(String),
}

impl From<csv::Error> for Error {
    fn from(err: csv::Error) -> Self {
        Error::Csv(err.to_string())
    }
}

impl From<&str> for Error {
    fn from(err: &str) -> Self {
        Error::Format(err.to_string())
    }
}

// When parsing, I'm making the assumption that we want to completely abort
// on errors.
// When we're reading a CSV file, it makes sense to fix the CSV (or the code),
// then try again.
// For a real-world scenario where we're receiving a stream of events instead,
// we would probably filter out bad rows and send them to an external system
// for analysis and recovery.
pub fn parse(input: impl std::io::Read) -> Result<Vec<Transaction>, Error> {
    let buffered = std::io::BufReader::new(input);
    let mut reader = csv::Reader::from_reader(buffered);

    reader
        .deserialize::<TransactionRecord>()
        .map(|r| match r {
            Ok(record) => Ok(record.try_into()?),
            Err(err) => Err(err.into()),
        })
        .collect()
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
    let transactions = parse(reader).expect("parsing should succeed");
    assert_eq!(5, transactions.len());
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
        let got_err = parse(reader);
        assert!(got_err.is_err());

        let err = got_err.err().unwrap();
        match err {
            Error::Csv(msg) => assert!(msg.contains(err_contains), "{:?}", msg),
            Error::Format(_) => panic!("unexpected error"),
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
        let got_err = parse(reader);
        assert_eq!(Err(want_err), got_err);
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

    #[serde(rename = "dispute")]
    Dispute,

    #[serde(rename = "resolve")]
    Resolve,

    #[serde(rename = "chargeback")]
    Chargeback,
}

impl TryFrom<TransactionRecord> for Transaction {
    type Error = &'static str;
    fn try_from(record: TransactionRecord) -> Result<Self, Self::Error> {
        Ok(Self {
            client_id: record.client_id,
            tx_id: record.transaction_id,
            tx_type: match record.tx_type {
                TransactionRecordType::Withdrawal => {
                    TransactionType::Withdrawal(match record.amount {
                        Some(amount) => amount,
                        None => return Err("missing amount for withdrawal"),
                    })
                }
                TransactionRecordType::Deposit => TransactionType::Deposit(match record.amount {
                    Some(amount) => amount,
                    None => return Err("missing amount for deposit"),
                }),
                TransactionRecordType::Dispute => TransactionType::Dispute,
                TransactionRecordType::Resolve => TransactionType::Resolve,
                TransactionRecordType::Chargeback => TransactionType::Chargeback,
            },
        })
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
        assert_eq!(tx, record.try_into().unwrap());
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
    let record = TransactionRecord {
        tx_type: TransactionRecordType::Withdrawal,
        client_id: 1,
        transaction_id: 2,
        amount: None,
    };

    match Transaction::try_from(record) {
        Ok(_) => panic!("unexpected successful conversion"),
        Err(e) => assert_eq!("missing amount for withdrawal", e),
    }
}
