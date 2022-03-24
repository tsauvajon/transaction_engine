use super::{Amount, ClientId, TransactionId};

#[derive(Debug, PartialEq)]
pub enum Type {
    Withdrawal(Amount), // Add a debit to the available balance.
    Deposit(Amount),    // Add a credit to the available balance.
    Dispute,            // Move funds from available to held.
    Resolve,            // Move funds from held to available.
    Chargeback,         // Remove funds from held. Immediately freeze the account.
}

#[derive(Debug, PartialEq)]
pub struct Transaction {
    pub(super) tx_type: Type,
    pub(super) client_id: ClientId,
    pub(super) tx_id: TransactionId,
}

impl Transaction {
    // The new() function ensures we can only create amounts with a decimal precision of 4.
    pub fn new(tx_type: Type, client_id: ClientId, tx_id: TransactionId) -> Self {
        let tx_type = match tx_type {
            Type::Deposit(amount) => Type::Deposit(amount.round_dp(super::DECIMAL_PRECISION)),
            Type::Withdrawal(amount) => Type::Withdrawal(amount.round_dp(super::DECIMAL_PRECISION)),
            _ => tx_type,
        };

        Self {
            tx_type,
            client_id,
            tx_id,
        }
    }
}

#[test]
// Decimal precision is 4 places. We should be unable to have more precise amounts.
fn test_transaction_decimal_precision() {
    use rust_decimal_macros::dec;

    for (raw_amount, want_amount) in vec![
        (dec!(1.0), dec!(1.0)),
        (dec!(0.999999), dec!(1.0)),
        (dec!(1.0000001), dec!(1.0)),
        (dec!(1.2345), dec!(1.2345)),
        (dec!(1.23459), dec!(1.2346)),
    ] {
        let tx = Transaction::new(Type::Withdrawal(raw_amount), 1, 1);
        assert_eq!(Type::Withdrawal(want_amount), tx.tx_type);
    }
}
