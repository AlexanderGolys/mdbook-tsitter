# tree-sitter vs. mdBook's highlight.js

The **same** short Rust snippet, rendered twice. The first block goes through
this preprocessor (tree-sitter); the second carries the `notreesitter` opt-out
tag, so mdBook's bundled highlight.js renders it instead. Switch the colour
scheme (top-left brush) to see both follow the theme.

This preprocessor (tree-sitter):

```rust
use std::collections::HashMap;

/// Count how often each word appears.
fn word_counts(text: &str) -> HashMap<&str, u32> {
    let mut counts = HashMap::new();
    for word in text.split_whitespace() {
        *counts.entry(word).or_insert(0) += 1;
    }
    counts
}

fn main() {
    for (word, n) in &word_counts("a rose is a rose") {
        println!("{word}: {n}");
    }
}
```

mdBook's built-in highlight.js:

```rust notreesitter
use std::collections::HashMap;

/// Count how often each word appears.
fn word_counts(text: &str) -> HashMap<&str, u32> {
    let mut counts = HashMap::new();
    for word in text.split_whitespace() {
        *counts.entry(word).or_insert(0) += 1;
    }
    counts
}

fn main() {
    for (word, n) in &word_counts("a rose is a rose") {
        println!("{word}: {n}");
    }
}
```
