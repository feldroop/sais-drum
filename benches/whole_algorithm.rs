use std::hint;

use criterion::{Criterion, criterion_group, criterion_main};
use rand::{RngCore, SeedableRng};
use sais_drum::SaisBuilder;

fn large_random_text_vs_divsufsort(c: &mut Criterion) {
    let mut group = c.benchmark_group("vs-divsufsort");
    group.sample_size(10);

    let text = create_random_text(10_000_000);

    group.bench_with_input("sais-drum-large-random", &text, |b, text| {
        b.iter(|| {
            let suffix_array = SaisBuilder::<_>::new().construct_suffix_array(text);
            hint::black_box(suffix_array);
        })
    });

    group.bench_with_input("divsufsort-large-random", &text, |b, text| {
        b.iter(|| {
            let suffix_array = divsufsort::sort(text);
            hint::black_box(suffix_array);
        })
    });

    group.finish();
}

criterion_group!(benches, large_random_text_vs_divsufsort);

criterion_main!(benches);

fn create_random_text(len: usize) -> Vec<u8> {
    let mut text = vec![42u8; len];
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(0x0DDB1A5E5BAD5EEDu64);

    rng.fill_bytes(&mut text);

    text
}
