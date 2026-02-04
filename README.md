# 🕸️ sharded-timing-wheel

**A cache-aware, hierarchical timing wheel implementation based on Varghese & Lauck (1987).**

![Status](https://img.shields.io/badge/status-active_development-green)
![License](https://img.shields.io/badge/license-MIT-blue)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)

## ⚡ The Problem: The "C10M" Timer Challenge

Standard approaches to timer management often rely on **Priority Queues** (like `std::collections::BinaryHeap` or typical Min-Heap implementations). While efficient for general use, these structures suffer at massive scale (1M+ concurrent timers):

1.  **Algorithmic Complexity:** Insertion and Cancellation are **O(log N)**. As connections scale, CPU usage increases non-linearly.
2.  **Pointer Chasing:** Tree-based heaps often scatter nodes across RAM. Traversing the structure causes massive **L1/L2 Cache Misses**.

## 🚀 The Solution

This project implements a **Hierarchical Timing Wheel** (Hashed Wheel Timer) focused on **Data-Oriented Design**.

*   **Complexity:** **O(1)** for Insert, Cancel, and Tick.
*   **Memory Layout:** Uses a custom **Slab Allocator** (Arena) to keep all timer data in contiguous memory, maximizing CPU cache pre-fetching.
*   **Zero-Allocation:** After the initial warmup, the system generates zero heap allocations during runtime.

## 🏗️ Architecture

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

Benchmarks run on `criterion` comparing `sharded-timing-wheel` vs `std::collections::BinaryHeap` using **randomized deadlines** (worst-case scenario).

| Operation | Scale (N) | Heap (Standard) | Wheel (This Crate) | Improvement |
| :--- | :--- | :--- | :--- | :--- |
| **Insert** | 1,000,000 | **15.3 ms** | 57.0 ms | (Slower, but gap narrowed to ~3.7x) |
| **Cancel** | 10,000 | 51.6 ms | **0.029 ms** | **1,700x FASTER** |

### Analysis
*   **Insertion:** The Binary Heap is optimized for simple insertion, but random input forces it to rebalance ($O(\log N)$ swaps), slowing it down to ~15ms. The Wheel pays a constant overhead for Linked List pointer maintenance.
*   **Cancellation:** This is the critical metric. The Wheel destroys the Heap here ($O(1)$ vs $O(N)$), making it the only viable choice for high-throughput network drivers where timers are cancelled frequently.

## 📚 References
1. Varghese, G., & Lauck, A. (1987). Hashed and hierarchical timing wheels: data structures for the efficient implementation of a timer facility.
2. Acton, M. (2014). Data-Oriented Design and C++. (CppCon).
3. Gregg, B. Systems Performance: Enterprise and the Cloud.
## 📄 License
MIT License
