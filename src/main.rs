/// Hirarchical Timing Wheel Implementation
/// Based on Varghese and Lauck's paper 
/// "Hashed and Hierarchical Timing Wheels: Efficient Data Structures for Implementing a Timer Facility"

mod slab;
mod wheel;
use crate::wheel::TimingWheel; 
use std::time::Instant;

fn main() {
    println!("Starting Timing Wheel Simulation...");

    let mut wheel = TimingWheel::new();
    let num_timers = 100_000;
    
    println!("-> Inserting {} timers...", num_timers);
    let start_insert = Instant::now();

    // Schedule 100,000 timers with random-ish deadlines
    // to simulate network timeouts ranging from 1ms to 10,000ms
    for i in 0..num_timers {
        let deadline = (i as u64 % 10_000) + 1; // Deadline between 1 and 10,000 ticks
        wheel.insert(format!("Request-{}", i), deadline);
    }

    let insert_time = start_insert.elapsed();
    println!("   Inserted {} timers in {:?}", num_timers, insert_time);
    println!("   Rate: {:.2} million inserts/sec", (num_timers as f64 / insert_time.as_secs_f64()) / 1_000_000.0);

    println!("\n-> Running Tick Loop...");
    let start_tick = Instant::now();
    
    let mut total_expired = 0;
    let mut ticks = 0;

    // Run ticks until all timers have expired
    while total_expired < num_timers {
        let expired = wheel.tick();
        total_expired += expired.len();
        ticks += 1;
        
        
        if ticks % 1000 == 0 {
            println!("   Tick {}: Processed {} timers so far...", ticks, total_expired);
        }
    }

    let tick_time = start_tick.elapsed();
    println!("   Finished in {:?}", tick_time);
    println!("   Total Ticks: {}", ticks);
    println!("   Total Expired: {}", total_expired);
    
    println!("\n SUCCESS: The Wheel handled the load!");
}

