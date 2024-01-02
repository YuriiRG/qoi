use std::io::Cursor;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use image::{codecs::qoi::QoiDecoder as ReferenceQoiDecoder, DynamicImage};
use qoi_parser::QoiDecoder;

pub fn decoding_sample_images(c: &mut Criterion) {
    let mut group = c.benchmark_group("decoding_sample_images");

    for image in &[
        ("dice", include_bytes!("../test_images/dice.qoi").as_slice()),
        (
            "testcard",
            include_bytes!("../test_images/testcard.qoi").as_slice(),
        ),
        (
            "kodim10",
            include_bytes!("../test_images/kodim10.qoi").as_slice(),
        ),
        (
            "kodim23",
            include_bytes!("../test_images/kodim23.qoi").as_slice(),
        ),
        (
            "edgecase",
            include_bytes!("../test_images/edgecase.qoi").as_slice(),
        ),
        (
            "qoi_logo",
            include_bytes!("../test_images/qoi_logo.qoi").as_slice(),
        ),
        (
            "testcard_rgba",
            include_bytes!("../test_images/testcard_rgba.qoi").as_slice(),
        ),
        (
            "wikipedia_008",
            include_bytes!("../test_images/wikipedia_008.qoi").as_slice(),
        ),
    ] {
        group.throughput(criterion::Throughput::Bytes(image.1.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("my_decoder", image.0),
            &image.1,
            |b, &image| {
                b.iter(|| {
                    let decoder = QoiDecoder::new(Cursor::new(image)).unwrap();
                    DynamicImage::from_decoder(decoder).unwrap()
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("reference_decoder", image.0),
            &image.1,
            |b, &image| {
                b.iter(|| {
                    let decoder = ReferenceQoiDecoder::new(Cursor::new(image)).unwrap();
                    DynamicImage::from_decoder(decoder).unwrap()
                })
            },
        );
    }
    group.finish();
}

criterion_group!(benches, decoding_sample_images);
criterion_main!(benches);
