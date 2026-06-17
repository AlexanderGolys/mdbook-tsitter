# Five languages

One block per configured grammar, each loaded dynamically from its own parser
and highlights query.

```m2
R = QQ[x,y]/ideal(x^2);
use R; x*y + y
```

```rust
fn main() {
    let xs: Vec<i32> = (0..3).map(|n| n * n).collect();
    println!("{xs:?}");
}
```

```lua
local function add(a, b)
  return a + b
end
print(add(1, 2))
```

```haskell
main :: IO ()
main = putStrLn (show (map (+ 1) [1, 2, 3]))
```

```markdown
# Heading
A *paragraph* with `code` and [a link](https://example.com).
```
