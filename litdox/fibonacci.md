<!-- litter-hash: d27d847 -->
### Source Fragment: `fibonacci`

```rust
/// Returns n-th Fibonacci number.
fn fibonacci_n(n: u32) -> u32 {
    if n <= 1 {
        return n;
    }
    fibonacci_n(n - 1) + fibonacci_n(n - 2)
}

```

[← Back to documentation](../README.md#fibonacci)