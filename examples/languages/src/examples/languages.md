# Languages

Each chapter in this section renders the same source twice: on the left as
mdBook highlights it with highlight.js, on the right as `mdbook-tsitter`
highlights it with tree-sitter. Macaulay2 has a single tree-sitter example, as
highlight.js does not know the language.

Every language here except Macaulay2 comes from the built-in grammar list and
is configured by name alone. Macaulay2 uses a custom grammar, named by its
repository and pinned to a release. This is the book's complete configuration:

```toml
{{#include ../../book.toml}}
```
