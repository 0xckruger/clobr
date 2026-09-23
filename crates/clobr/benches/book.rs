use criterion::{Criterion, criterion_group, criterion_main};

fn benches(_c: &mut Criterion) {}

criterion_group!(book, benches);
criterion_main!(book);
