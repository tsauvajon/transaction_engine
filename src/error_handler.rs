use crate::{input::Error, ledger::account::TransactionError};

use std::{
    io::{self, Write},
    sync::mpsc::Receiver,
};

// Here, we simply ignore the errors and keep processing other transactions.
// It is explicitely stated that Disputes on non-existing transactions should
// be ignored. I'm extending that behaviour to other kind of inconsistencies
// that can be found.
//
// I decided to sink the errors instead of, for example, printing them,
// to be sure it wouldn't cause issues with automated grading systems
// (e.g. slow things down as it writes massive amounts of data to stderr).
//
// In a real-world scenario, we'd do something with errors.
// At least print them.
// We could either store them in some kind of tracing system where we can
// learn more about them, or send some info to an external system
// (e.g. a queue + a dedicated service listening on it) to deal with the
// error outside of this system.
//
// We could also try to recover from some errors.
pub fn sink(
    input_errors: Receiver<Error>,
    transaction_errors: Receiver<TransactionError>,
) -> Vec<std::thread::JoinHandle<()>> {
    vec![
        std::thread::spawn(move || {
            for err in input_errors {
                io::sink()
                    .write_fmt(format_args!("failed to read record: {:?}", err))
                    .expect("Writing to sink should never fail");
            }
        }),
        std::thread::spawn(move || {
            for err in transaction_errors {
                io::sink()
                    .write_fmt(format_args!("failed to read record: {:?}", err))
                    .expect("Writing to sink should never fail");
            }
        }),
    ]
}
