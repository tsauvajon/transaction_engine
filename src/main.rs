#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

mod input;
mod ledger;
mod output;

use input::parse;
use ledger::process::process;
use std::{fs::File, sync::mpsc};

fn main() {
    let input_stream = File::open("transactions.csv").expect("could not open the file");
    let output_stream = std::io::stdout();
    run(input_stream, output_stream);
}

fn run(input_stream: (impl std::io::Read + Send + 'static), output_stream: impl std::io::Write) {
    let (transactions, input_errors) = parse(input_stream);

    let (account_tx, accounts) = mpsc::channel();
    let transaction_errors = process(transactions, account_tx);

    let error_handling_threads = output::handle_errors(input_errors, transaction_errors);
    output::write(output_stream, accounts).expect("failed to write the output"); // Should not fail with stdout.

    // Make sure we write all the errors as well.
    for thread in error_handling_threads {
        thread.join().expect("failed to join the threads");
    }
}

#[test]
fn end_to_end_test() {
    let input = r#"type,       client, tx, amount
deposit,    1,      1,  1.0
deposit,    2,      2,  2.0
badly formated record
deposit,    1,      3,  2.0
withdrawal, 1,      4,  1.5
withdrawal, 2,      5,  3.0
another bad record
    "#;

    // Data can appear in any order (account 1 or account 2).
    // To keep the tests as dumb as possible (i.e. I didn't want my test to parse anything),
    // I'm just checking the output against two variants.
    // Another solution could have been to always sort the accounts before writing them,
    // but the assignment clearly states that ordering is not important - it felt
    // unnecessary to add any logic for sorting.
    let expected_output_variant_1 = r#"client,available,held,total,locked
1,1.5,0,1.5,false
2,2,0,2,false
"#;
    let expected_output_variant_2 = r#"client,available,held,total,locked
2,2,0,2,false
1,1.5,0,1.5,false
"#;

    let mut output_stream = Vec::new();
    run(input.as_bytes(), &mut output_stream);

    let actual_output = String::from_utf8(output_stream).unwrap();

    assert!(
        expected_output_variant_1.to_string() == actual_output
            || expected_output_variant_2.to_string() == actual_output,
        "actual: {}\nexpected2: {}\nexpected2: {}",
        actual_output,
        expected_output_variant_1,
        expected_output_variant_1
    );
}
