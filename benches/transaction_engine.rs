use criterion::{criterion_group, criterion_main, Criterion};
use transaction_engine::run::run;

pub fn bench_calculate_balances_7000_lines(c: &mut Criterion) {
    c.bench_function("calc_balances_large_file_7_000", |b| {
        let data = format!(
            "type,client,tx,amount\n{}",
            r#"deposit,    1,      1,  1.0
        deposit,    2,      2,  2.0
        badly formated record
        deposit,    1,      3,  2.0
        withdrawal, 1,      4,  1.5
        withdrawal, 2,      5,  3.0
        another bad record"#
                .repeat(1_000)
        );
        let cursor = std::io::Cursor::new(data);

        b.iter(move || run(cursor.clone(), std::io::sink()))
    });
}

pub fn bench_calculate_balances_140000_lines(c: &mut Criterion) {
    c.bench_function("calc_balances_large_file_140_000", |b| {
        let data = format!(
            "type,client,tx,amount\n{}",
            r#"deposit,    1,      1,  1.0
        deposit,    2,      2,  2.0
        badly formated record
        deposit,    1,      3,  2.0
        withdrawal, 1,      4,  1.5
        withdrawal, 2,      5,  3.0
        another bad record"#
                .repeat(20_000)
        );
        let cursor = std::io::Cursor::new(data);

        b.iter(move || run(cursor.clone(), std::io::sink()))
    });
}

criterion_group!(
    benches,
    bench_calculate_balances_7000_lines,
    bench_calculate_balances_140000_lines,
);
criterion_main!(benches);
