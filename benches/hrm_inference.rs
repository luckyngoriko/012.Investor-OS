//! HRM Inference Benchmark
//!
//! Measures inference latency on CPU vs GPU (RTX 3090).
//! Run with: cargo bench --features cuda

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use investor_os::hrm::{HRMConfig, InferenceEngine, HRM};

fn bench_inference_cpu(c: &mut Criterion) {
    let mut group = c.benchmark_group("hrm_inference_cpu");

    let engine = InferenceEngine::new();
    let signals = vec![0.8, 0.9, 0.7, 15.0, 0.0, 0.5];

    group.bench_function("single_inference", |b| {
        b.iter(|| {
            let _ = engine.infer(black_box(&signals));
        });
    });

    // Batch sizes
    for batch_size in [1, 4, 8, 16, 32].iter() {
        let batch: Vec<Vec<f32>> = (0..*batch_size)
            .map(|_| vec![0.8, 0.9, 0.7, 15.0, 0.0, 0.5])
            .collect();

        group.bench_with_input(
            BenchmarkId::new("batch_inference", batch_size),
            batch_size,
            |b, _| {
                b.iter(|| {
                    let _ = engine.infer_batch(black_box(&batch));
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "cuda")]
fn bench_inference_gpu(c: &mut Criterion) {
    use investor_os::hrm::DeviceConfig;

    let mut group = c.benchmark_group("hrm_inference_gpu");
    group.sample_size(100);

    let config = HRMConfig::default().with_device(DeviceConfig::Cuda);

    let hrm = HRM::new(&config).expect("Failed to create HRM with CUDA");
    let signals = vec![0.8, 0.9, 0.7, 15.0, 0.0, 0.5];

    group.bench_function("single_inference_cuda", |b| {
        b.iter(|| {
            let _ = hrm.infer(black_box(&signals));
        });
    });

    // Batch sizes
    for batch_size in [1, 4, 8, 16, 32, 64, 128].iter() {
        let batch: Vec<Vec<f32>> = (0..*batch_size)
            .map(|_| vec![0.8, 0.9, 0.7, 15.0, 0.0, 0.5])
            .collect();

        group.bench_with_input(
            BenchmarkId::new("batch_inference_cuda", batch_size),
            batch_size,
            |b, _| {
                b.iter(|| {
                    let _ = hrm.infer_batch(black_box(&batch));
                });
            },
        );
    }

    group.finish();
}

#[cfg(not(feature = "cuda"))]
fn bench_inference_gpu(_c: &mut Criterion) {
    // No-op when CUDA not enabled
}

criterion_group!(benches, bench_inference_cpu, bench_inference_gpu);
criterion_main!(benches);
