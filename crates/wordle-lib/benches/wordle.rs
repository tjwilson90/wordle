use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::fs::File;
use std::io::BufReader;
use wordle_lib::{Dictionary, OffsetDictionary, WordDictionary, WordMatch, LEGAL_GUESSES};

fn word_guesses_partition(c: &mut Criterion) {
    let dict = &WordDictionary::new(LEGAL_GUESSES);
    let guess = *b"scamp";
    c.bench_function("word_guesses_partition_large", |b| {
        b.iter(|| dict.partition(black_box(guess)))
    });
    let dict = &dict.partition(guess).remove(&WordMatch::ABSENT).unwrap();
    let guess = *b"ghoul";
    c.bench_function("word_guesses_partition_small", |b| {
        b.iter(|| dict.partition(black_box(guess)))
    });
}

fn offset_guesses_partition(c: &mut Criterion) {
    let dict = &OffsetDictionary {
        words: (0..12972).collect(),
    };
    let guess = 9622;
    c.bench_function("offset_guesses_partition_large", |b| {
        b.iter(|| dict.partition(black_box(guess)))
    });
    let dict = &dict.partition(guess).remove(&WordMatch::ABSENT).unwrap();
    let guess = 4341;
    c.bench_function("offset_guesses_partition_small", |b| {
        b.iter(|| dict.partition(black_box(guess)))
    });
}

criterion_group!(benches, word_guesses_partition, offset_guesses_partition);
criterion_main!(benches);
