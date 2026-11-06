# Hand-marked tokens

{{#include ../../../../README.md:marks}}

## Example

A grammar sees a trait, a struct and an enum as the same thing: a type name.
A language server knows which is which, and so can the page:

```rust,lsp,noplayground
trait §interface|Area§ {
    fn §method.declaration|area§(&self) -> f64;
}

struct §struct|Circle§(f64);
enum §enum|Shape§ { Round(§struct|Circle§), Flat }

fn total(shapes: &[§enum|Shape§]) -> f64 {
    shapes.iter().map(|shape| shape.§method|area§()).sum()
}
```

Marks only change what you see when the book styles them differently from the
grammar. The bundled colours follow the nearest tree-sitter capture, so on their
own `interface`, `struct` and `enum` all look like `type`. This book gives each
a colour of its own with four lines, using the palette roles every theme
defines, so they follow the theme:

```css
:root {
  --lsp-interface: var(--ts-purple);
  --lsp-struct: var(--ts-orange);
  --lsp-enum: var(--ts-red);
  --lsp-method: var(--ts-cyan);
}
```

Nothing else is read in such a block, so `$` and other code need no escaping. A
modifier styles what the server knows about a name, here with a dotted underline
on `readonly`:

```bash,lsp
echo $§variable.readonly|HOME§ in $(pwd)
```
