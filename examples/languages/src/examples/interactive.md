# Interactive blocks

Highlighted blocks keep mdBook's own code-block features. Hover a block to see
its buttons: copy, run in the Rust Playground, and show hidden lines.

## Runnable Rust

A plain `rust` block runs in the playground with the book's edition. It has no
`fn main`, so the hidden wrapper mdBook adds is revealed by the eye button.

```rust
let primes: Vec<u32> = (2..50)
    .filter(|n| (2..*n).take_while(|d| d * d <= *n).all(|d| n % d != 0))
    .collect();
println!("{primes:?}");
```

## Hidden lines

Lines starting with `# ` are compiled and run but hidden from readers until
they are revealed.

```rust
# use std::collections::BTreeMap;
#
# fn word_counts(text: &str) -> BTreeMap<&str, usize> {
#     let mut counts = BTreeMap::new();
#     for word in text.split_whitespace() {
#         *counts.entry(word).or_insert(0) += 1;
#     }
#     counts
# }
#
let counts = word_counts("the quick fox jumps over the lazy dog the end");
for (word, count) in &counts {
    println!("{word:>5}: {count}");
}
```

## Annotations

Annotations work as they do in mdBook. `should_panic` keeps the Run button and
the panic shows up in the output:

```rust,should_panic
let readings = [21.5, 22.0, f64::NAN];
let average = readings.iter().sum::<f64>() / readings.len() as f64;
assert!(!average.is_nan(), "a reading is missing");
```

`noplayground` keeps the highlighting but removes the Run button:

```rust,noplayground
// Reads from stdin, which the playground cannot provide.
let mut line = String::new();
std::io::stdin().read_line(&mut line)?;
```

`ignore` marks code that is not meant to compile:

```rust,ignore
let config = load_config()?; // defined elsewhere
serve(config.address).await;
```

## Hidden lines in other languages

Other languages hide lines with a prefix of your choice, given per block with
`hidelines=<prefix>` or per language in `[output.html.code.hidelines]`. This
TypeScript block hides its imports and helper behind `~`:

```typescript,hidelines=~
~import { readFileSync } from "node:fs";
~
~function parse(source: string): Record<string, string> {
~  return Object.fromEntries(
~    source.split("\n").filter(Boolean).map((line) => line.split("=", 2)),
~  );
~}
~
const env = parse(readFileSync(".env", "utf8"));
console.log(`connecting to ${env.HOST}:${env.PORT}`);
```

This book sets `python = "~~"` in `[output.html.code.hidelines]`:

```python
~~from dataclasses import dataclass
~~
@dataclass
class Point:
    x: float
    y: float

    def norm(self) -> float:
        return (self.x**2 + self.y**2) ** 0.5

print(Point(3, 4).norm())
```

## Editable playgrounds

With `editable = true` under `[output.html.playground]`, an `editable` block
becomes an in-page editor. mdBook replaces it with its own editor, so it is
left to mdBook rather than highlighted by tree-sitter:

```rust,editable
fn main() {
    let name = "tree-sitter";
    println!("Hello from {name}! Edit me and press run.");
}
```
