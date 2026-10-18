# Side-by-side blocks

{{#include ../../../../README.md:side-by-side}}

## Examples

### Before and after

The two panes from [Panes](#panes), with the change in full:

```rust,pane="Before"
fn total_length(words: &[String]) -> usize {
    let mut total = 0;
    for i in 0..words.len() {
        total += words[i].len();
    }
    total
}
```

```rust,pane="After"
fn total_length(words: &[String]) -> usize {
    words.iter().map(String::len).sum()
}
```

### One idea, several languages

Three panes, each in its own language:

```python,pane="Python"
def sum_squares(xs):
    return sum(x * x for x in xs)
```

```go,pane="Go"
func sumSquares(xs []int) int {
	total := 0
	for _, x := range xs {
		total += x * x
	}
	return total
}
```

```haskell,pane="Haskell"
sumSquares :: [Int] -> Int
sumSquares = sum . map (^ 2)
```

### Comparing with highlight.js

A `compare` block. The [language chapters](../examples/languages.md) show
longer comparisons, written as two panes each:

```typescript,compare
type Shape =
  | { kind: "circle"; radius: number }
  | { kind: "square"; side: number };

const area = (shape: Shape): number =>
  shape.kind === "circle"
    ? Math.PI * shape.radius ** 2
    : shape.side ** 2;
```
