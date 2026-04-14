#![allow(unused)]
use litter_dox::litter;

#[litter(name = "fibonacci")]
/// Returns n-th Fibonacci number.
fn fibonacci_n(n: u32) -> u32 {
    if n <= 1 {
        return n;
    }
    fibonacci_n(n - 1) + fibonacci_n(n - 2)
}

// Ensure the file is a valid executable
fn main() {}

// Call the anchors macro to update the README
litter_dox::litter_anchors!();
