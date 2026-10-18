# Languages

Each chapter in this section renders the same source twice, side by side: on
the left as `mdbook-tsitter` highlights it with tree-sitter, on the right as
mdBook highlights it with highlight.js. Macaulay2 has a single tree-sitter
example, as highlight.js does not know the language.

Every language here comes from the built-in grammar list and is configured by
name alone, Macaulay2 included: its grammar is not in nvim-treesitter's list,
so the built-in list adds it. This is the book's complete configuration:

```toml
{{#include ../../book.toml}}
```
