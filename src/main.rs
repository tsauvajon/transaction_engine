use std::fs::File;
use transaction_engine::run::run;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert!(args.len() >= 2);
    let filename = &args[1];

    let input_stream = File::open(filename).expect("could not open the file");
    let output_stream = std::io::stdout();

    run(input_stream, output_stream);
}
