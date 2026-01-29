use criterion::{Criterion, black_box, criterion_group, criterion_main};
use sharded_timing_wheel::wheel::TimingWheel;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

// Helper to find and remove from heap (simulating cancellation)
fn heap_cancel(heap: &mut BinaryHeap<Reverse<u64>>, target: u64) {
    let mut vec = heap.clone().into_vec();
    if let Some(pos) = vec.iter().position(|x| x.0 == target) {
        vec.remove(pos);
    }
    *heap = BinaryHeap::from(vec);
}

fn benchmark_insert(c: &mut Criterion) {
    // Increase to 1 Million to make log(N) hurt more
    let n = 1_000_000;

    let mut group = c.benchmark_group("Insertion");
    group.sample_size(10); // Reduce samples because 1M takes time

    group.bench_function("Wheel Insert 1M", |b| {
        b.iter(|| {
            let mut wheel = TimingWheel::new();
            // Pre-allocating fixes the resize overhead, making the comparison fair
            // (You'd need to expose a with_capacity method, but for now standard is fine)
            for i in 0..n {
                wheel.insert(black_box(i), black_box(i as u64));
            }
        })
    });

    group.bench_function("Heap Insert 1M", |b| {
        b.iter(|| {
            let mut heap = BinaryHeap::new();
            for i in 0..n {
                heap.push(Reverse(black_box(i as u64)));
            }
        })
    });
    group.finish();
}

fn benchmark_cancel(c: &mut Criterion) {
    let n = 10_000; // Smaller N because Heap cancel is SO slow

    let mut group = c.benchmark_group("Cancellation");

    group.bench_function("Wheel Cancel", |b| {
        b.iter_with_setup(
            || {
                let mut wheel = TimingWheel::new();
                let mut ids = Vec::with_capacity(n);
                for i in 0..n {
                    ids.push(wheel.insert(i, i as u64));
                }
                (wheel, ids)
            },
            |(mut wheel, ids)| {
                // Measure time to cancel all of them
                for id in ids {
                    wheel.cancel(id);
                }
            },
        )
    });

    group.bench_function("Heap Cancel", |b| {
        b.iter_with_setup(
            || {
                let mut heap = BinaryHeap::new();
                for i in 0..n {
                    heap.push(Reverse(i as u64));
                }
                heap
            },
            |mut heap| {
                // Measure time to cancel items (Worst case O(N) per item)
                for i in 0..n {
                    // Simulating finding and removing specific items
                    heap_cancel(&mut heap, i as u64);
                }
            },
        )
    });
    group.finish();
}

criterion_group!(benches, benchmark_insert, benchmark_cancel);
criterion_main!(benches);
