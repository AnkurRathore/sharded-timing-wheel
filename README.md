# 🕸️ sharded-timing-wheel

**A cache-aware, hierarchical timing wheel implementation based on Varghese & Lauck (1987).**

![Status](https://img.shields.io/badge/status-active_development-green)
![License](https://img.shields.io/badge/license-MIT-blue)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)
[![DOI](https://img.shields.io/badge/DOI-10.5281%2Fzenodo.18886174-blue)](https://doi.org/10.5281/zenodo.18886174)
## ⚡ The Problem: The "C10M" Timer Challenge

Standard approaches to timer management often rely on **Priority Queues** (like `std::collections::BinaryHeap` or typical Min-Heap implementations). While efficient for general use, these structures suffer at massive scale (1M+ concurrent timers):

1.  **Algorithmic Complexity:** Insertion and Cancellation are **O(log N)**. As connections scale, CPU usage increases non-linearly.
2.  **Pointer Chasing:** Tree-based heaps often scatter nodes across RAM. Traversing the structure causes massive **L1/L2 Cache Misses**.

## The Solution

This project implements a **Hierarchical Timing Wheel** (Hashed Wheel Timer) focused on **Data-Oriented Design**.

*   **Complexity:** **O(1)** for Insert, Cancel, and Tick.
*   **Memory Layout:** Uses a custom **Slab Allocator** (Arena) to keep all timer data in contiguous memory, maximizing CPU cache pre-fetching.
*   **Zero-Allocation:** After the initial warmup, the system generates zero heap allocations during runtime.

## Architecture

### 1. The Engine: Intrusive Slab Allocator 
Instead of using `Vec<Box<Node>>`, I have implemented a custom Slab with an **Intrusive Free List**.

*   **Contiguous Memory:** All timer entries live in a single `Vec<Entry>`.
*   **Intrusive Linked List:** Empty slots form a linked list *inside* the unused memory of the vector.
*   **Index-Based:** I have used `usize` indices instead of pointers, reducing memory overhead on 64-bit systems and avoiding borrow checker complexity.

```text
// Visual representation of the memory layout
[ Occupied(Task A) | Free(Next=4) | Occupied(Task B) | Occupied(Task C) | Free(Next=End) ]
```
### 2. The Algorithm: Varghese & Lauck Hierarchy
I am currently implementing the "Scheme 6" Hierarchical Wheel:
1. Granularity: 4 levels of wheels (Seconds, Minutes, Hours...).
2. Bitwise Optimization: Using powers of 2 (64 slots) to replace expensive Modulo (%) instructions with fast Bitwise AND (&) instructions.
3. Cascading: Timers automatically "fall down" to lower-resolution wheels as time progresses.

## 🔬 Performance Goals
## 🔬 Benchmark Results

**Methodology Update:** Previous benchmarks used sorted inputs. These updated results use **pre-calculated random deadlines** to simulate real-world network traffic and force the Binary Heap to re-balance (sift-up) continuously.

| Operation | Scale (N) | Heap (Standard) | Wheel (v0.2) | Improvement |
| :--- | :--- | :--- | :--- | :--- |
| **Insert** | 1,000,000 | **15.3 ms** | 57.0 ms | (Slower, see analysis below) |
| **Cancel** | 10,000 | 51.6 ms | **0.029 ms** | **1,700x FASTER** 🚀 |

### Performance Analysis
1.  **Insertion Trade-Off:**
    *   The **Binary Heap** benefits from hardware prefetching because it is backed by a contiguous `Vec`. Even with random inputs, the "hot path" (indices 0, 1, 2...) stays in L1 cache.
    *   The **Timing Wheel** pays a constant cost for calculating bit-shifts and writing to random indices in the Slab. Profiling via `samply` confirms that the bottleneck is **L1 Data Cache Misses** during the linked-list write operation (`mov dword [base + index + offset]`).
    *   *Verdict:* Accept slower insertion (~3.7x slower) to gain massive cancellation speed.

2.  **Cancellation (The "C10M" Win):**
    *   This is the critical metric for network drivers (TCP/QUIC).
    *   The **Heap** requires an $O(N)$ linear scan to find a task to cancel.
    *   The **Wheel** uses the Slab index (Handle) to unlink the node in **$O(1)$ constant time**, regardless of how many timers exist.

## Citation
If you use `sharded-timing-wheel` in your research or project, please cite it as:

> Rathore, Ankur. (2026). sharded-timing-wheel: A cache-aware, hierarchical timing wheel implementation (v0.2.0). Zenodo. https://doi.org/10.5281/zenodo.18886174

## References
1. Varghese, G., & Lauck, A. (1987). Hashed and hierarchical timing wheels: data structures for the efficient implementation of a timer facility.
2. Acton, M. (2014). Data-Oriented Design and C++. (CppCon).
3. Gregg, B. Systems Performance: Enterprise and the Cloud.
## 📄 License
MIT License
