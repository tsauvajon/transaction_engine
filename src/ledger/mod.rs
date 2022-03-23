pub mod account;
pub mod process;
pub mod transaction;

// Using named types doesn't provide any compiler help, but it helps a lot with
// readability.
// Consider the following, when creating the ledger HashMap:
// (1) ledger: HashMap<u16, Account>
// (2) ledger: HashMap<ClientId, Account>
// Implementation (1) would most likely need comments, and could be confusing.
// Implementation (2) is self-explanatory.
// Besides, maintenance is easier: changing client ids e.g. from u16 to u32 is trivial.
pub type ClientId = u16;
pub type TransactionId = u32;

// I decided to use a decimal library instead of the built-in f32 type, to be
// safer when dealing with money, and making the decimal precision easier to
// deal with.
pub type Amount = rust_decimal::Decimal;
const DECIMAL_PRECISION: u32 = 4;
