use crate::ledger::{account::account::Account, Amount, ClientId};

use serde::Serialize;
use std::sync::mpsc::Receiver;

#[derive(Serialize)]
struct AccountRecord {
    #[serde(rename = "client")]
    client_id: ClientId,

    #[serde(rename = "available")]
    available_amount: Amount,

    #[serde(rename = "held")]
    held_amount: Amount,

    #[serde(rename = "total")]
    total_amount: Amount,

    #[serde(rename = "locked")]
    frozen: bool,
}

impl AccountRecord {
    fn new(client_id: ClientId, acc: &Account) -> Self {
        Self {
            client_id,
            available_amount: acc.available.amount(),
            held_amount: acc.held.amount(),
            total_amount: acc.total_amount(),
            frozen: acc.frozen,
        }
    }
}

// Writes the received accounts to the given stream.
pub fn write(
    output_stream: impl std::io::Write,
    accounts: Receiver<(ClientId, Account)>,
) -> Result<(), std::io::Error> {
    let mut writer = csv::Writer::from_writer(output_stream);

    for (client_id, account) in accounts {
        let record = AccountRecord::new(client_id, &account);
        writer.serialize(record)?;
    }

    Ok(())
}

#[cfg(test)]
mod write_tests {
    use crate::ledger::account::{account::Account, balance::Balance};

    use rust_decimal_macros::dec;
    use std::sync::mpsc;

    #[test]
    fn test_write_accounts() {
        let (accounts_tx, accounts) = mpsc::channel();
        let mut output_stream = Vec::new();
        for account in vec![
            (1, dec!(5.0), dec!(1.0), false),
            (2, dec!(1.234), dec!(123.4), false),
            (3, dec!(500.005), dec!(600.006), true),
        ] {
            let (client_id, available, held, frozen) = account;
            let mut account = Account::new();
            account.available = Balance::new(available, dec!(0));
            account.held = Balance::new(held, dec!(0));
            account.frozen = frozen;
            accounts_tx.send((client_id, account)).unwrap();
        }
        drop(accounts_tx);

        super::write(&mut output_stream, accounts).unwrap();

        let want = r#"client,available,held,total,locked
1,5.0,1.0,6.0,false
2,1.234,123.4,124.634,false
3,500.005,600.006,1100.011,true
"#;
        assert_eq!(want.to_string(), String::from_utf8(output_stream).unwrap(),);
    }
}
