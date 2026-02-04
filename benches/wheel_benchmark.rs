use criterion::{Criterion, black_box, criterion_group, criterion_main};
use sharded_timing_wheel::wheel::TimingWheel;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use rand::Rng;

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

    // Pregenrate random deadlines
    let mut rng = rand::thread_rng();
    let mut random_deadlines = Vec::with_capacity(n);
    for _ in 0..n {
        // Random deadlines between 1 and 1000000
        random_deadlines.push(rng.gen_range(1..1000_000));
    }
    let mut group = c.benchmark_group("Insertion");
    group.sample_size(10); // Reduce samples because 1M takes time

    group.bench_function("Wheel Insert 1M", |b| {
        b.iter(|| {
            let mut wheel = TimingWheel::new();
            // using the pre-calculated random deadlines
            for (i,&deadline) in random_deadlines.iter().enumerate() {
                wheel.insert(black_box(i), black_box(deadline));
            }
        })
    });

    group.bench_function("Heap Insert 1M", |b| {
        b.iter(|| {
            let mut heap = BinaryHeap::new();
            for (i,&deadline) in random_deadlines.iter().enumerate()  {
                heap.push(Reverse(black_box(deadline)));
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
