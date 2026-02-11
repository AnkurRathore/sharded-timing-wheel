use sharded_timing_wheel::TimingWheel;
use std::hint::black_box;

fn main() {
    let n = 100_000_000;
    let mut wheel = TimingWheel::new();
    
    // Using a simple logic to generate deadlines for the inserts
    for i in 0..n {
        let deadline = (i as u64).wrapping_mul(31337) % 100_000;
        
        
        wheel.insert(black_box(i), black_box(deadline));
    }
}