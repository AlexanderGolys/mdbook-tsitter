# tree-sitter vs. mdBook's highlight.js

Each pair contains the **same** source twice. The left block goes through this
preprocessor (tree-sitter); the right block carries the `notreesitter` opt-out
tag, so mdBook's bundled highlight.js renders it instead. Switch the colour
scheme (top-left brush) to see both follow the theme.

The pairs sit side by side when there is enough room and stack on smaller
screens.

## Macaulay2

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

**tree-sitter**

```m2
R = QQ[x,y]/ideal(x^2);
use R;
f = i -> x^i + y;
apply(1..3, f)
```

</div>

<div class="highlight-comparison-pane">

**highlight.js**

```m2,notreesitter
R = QQ[x,y]/ideal(x^2);
use R;
f = i -> x^i + y;
apply(1..3, f)
```

</div>

</div>

## Rust

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

**tree-sitter**

```rust
fn squares(xs: &[i32]) -> Vec<i32> {
    xs.iter().map(|n| n * n).collect()
}

fn main() {
    println!("{:?}", squares(&[1, 2, 3]));
}
```

</div>

<div class="highlight-comparison-pane">

**highlight.js**

```rust,notreesitter
fn squares(xs: &[i32]) -> Vec<i32> {
    xs.iter().map(|n| n * n).collect()
}

fn main() {
    println!("{:?}", squares(&[1, 2, 3]));
}
```

</div>

</div>

## Lua

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

**tree-sitter**

```lua
local function greet(name)
  return ("Hello, %s!"):format(name)
end

for _, name in ipairs({ "Ada", "Linus" }) do
  print(greet(name))
end
```

</div>

<div class="highlight-comparison-pane">

**highlight.js**

```lua,notreesitter
local function greet(name)
  return ("Hello, %s!"):format(name)
end

for _, name in ipairs({ "Ada", "Linus" }) do
  print(greet(name))
end
```

</div>

</div>

## Haskell

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

**tree-sitter**

```haskell
data Shape = Circle Double | Square Double

area :: Shape -> Double
area (Circle r) = pi * r ^ 2
area (Square w) = w * w

main = print (area (Circle 3))
```

</div>

<div class="highlight-comparison-pane">

**highlight.js**

```haskell,notreesitter
data Shape = Circle Double | Square Double

area :: Shape -> Double
area (Circle r) = pi * r ^ 2
area (Square w) = w * w

main = print (area (Circle 3))
```

</div>

</div>

## Markdown

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

**tree-sitter**

````markdown
## Release notes

The new renderer is **faster** and supports:

- [links](https://example.com)
- `inline code`

> Highlight once, read everywhere.
````

</div>

<div class="highlight-comparison-pane">

**highlight.js**

````markdown,notreesitter
## Release notes

The new renderer is **faster** and supports:

- [links](https://example.com)
- `inline code`

> Highlight once, read everywhere.
````

</div>

</div>
