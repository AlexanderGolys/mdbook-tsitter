# Macro hygiene

A `macro_rules!` macro can't see the names of the code that calls it, and that
code can't see the macro's own names, even when they're spelled the same. Each
name belongs to a *syntax context*. Mark a Rust block `hygiene` and the
book shows its macro expansion with every token on the background of its
context: the code as written in one colour, and what each macro produced in
another. Hover a token to see which macro made it.

````markdown
```rust,hygiene
macro_rules! using_a { … }

let four = using_a!(a / 10);
```
````

```rust,hygiene
macro_rules! using_a {
    ($e:expr) => {
        {
            let a = 42;
            $e
        }
    }
}

let four = using_a!(a / 10);
```

The `a` that `let a = 42;` binds is the macro's, and the `a` in `a / 10` is the
caller's, so they're different names and the code does not compile. Passing
the name in from the caller puts both in its context:

```rust,hygiene
macro_rules! using_a {
    ($a:ident, $e:expr) => {
        {
            let $a = 42;
            $e
        }
    }
}

let four = using_a!(a, a / 10);
```

Macros inside macros get a context each:

```rust,hygiene
macro_rules! square {
    ($x:expr) => {{ let v = $x; v * v }};
}

let n = 3;
let total = square!(n + 1) + square!(n);
println!("{total}");
```

## How it works

When the book is built, the block goes to `rustc -Zunpretty=expanded,hygiene`,
as rustdoc compiles a doctest: wrapped in `fn main` if it has none, with its
hidden lines included. rustc prints the expanded code with each identifier's
context, and the shown block is that expansion, as rustc formats it, without
the `macro_rules!` definitions and the prelude rustc adds. Code that fails to
compile is still expanded, which is often the point of a hygiene example.

- **Only identifiers come with a context.** Other tokens, such as `=`, `42` and
  the braces, take one from the syntax tree around them, so their colour is a
  good guess rather than rustc's word.
- **The output format is unstable.** It's a debugging aid of the compiler. The
  preprocessor sets `RUSTC_BOOTSTRAP=1`, so a stable `rustc` works, and
  `$RUSTC` chooses another one.
- **The block isn't runnable.** What's shown is the expansion, not the code as
  written.

The colours are `.hygiene-0` for the code as written and `.hygiene-1`,
`.hygiene-2`, … for the contexts in the order they appear; set
`--hygiene-1` and so on to recolour them.
