use serde::Deserialize;

pub fn parse_file(input: impl std::io::Read) {
    let buffered = std::io::BufReader::new(input);
    let mut reader = csv::Reader::from_reader(buffered);

    for row in reader.deserialize() {
        let tx: TransactionRecord = row.unwrap(); // todo handle error
        println!("{:?}", tx);
    }
}

// Having a Csv type because I can't directly deserialise into my "domain" type.
// See https://github.com/BurntSushi/rust-csv/issues/211.
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

type TransactionRecord = (TransactionRecordType, u16, u32, Option<f32>);
