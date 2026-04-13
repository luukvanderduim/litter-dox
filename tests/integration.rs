#![allow(unused)]
use litter_dox::litter;
use std::path::Path;

#[litter(name: "my_test_fn")]
fn my_test_fn() {
    println!("This is a test.");
}

#[test]
fn test_litter_file_exists() {
    // Check if the file was generated in the default directory
    let path = Path::new("litdox/my_test_fn.md");
    assert!(path.exists(), "The markdown file should have been created.");

    // Also check the symlink
    let symlink = Path::new("litdox/my_test_fn.md");
    assert!(symlink.exists(), "The symlink should exist.");
}

#[litter(name = "fibonacci")]
/// fibonacci for n
fn fibonacci_n(n: u32) -> u32 {
    if n <= 1 {
        return n;
    }
    fibonacci_n(n - 1) + fibonacci_n(n - 2)
}

litter_dox::litter_anchors!();
