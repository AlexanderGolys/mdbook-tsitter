# Example book

The project's documentation site, built with `mdbook-tsitter` itself. It
doubles as a usage example and as an integration test across several grammars.
Its sections are:

- **Introduction** and **User guide**: the documentation from the root
  [`README.md`](../../README.md), split into chapters with `{{#include}}`
  anchors so the README stays the single source, with rendered examples of
  interactive blocks (Rust Playground, hidden lines, annotations) and
  side-by-side blocks, and a custom mdBook theme in the Theming chapter.
- **Examples**: one chapter per language showing tree-sitter's highlighting
  beside mdBook's highlight.js, as side-by-side blocks.
- **Project**: the root [`CONTRIBUTING.md`](../../CONTRIBUTING.md) and the
  README's development notes.

[`book.toml`](book.toml) is a complete configuration example: languages from the
built-in list by name, and a custom grammar by repository. Grammars are fetched
and compiled on the first build and cached afterwards.

[`theme/`](theme) holds the book's own styling: the Everforest theme
(`index.hbs` adds it to mdBook's theme picker, `everforest.css` and
`everforest.js` define it) and `comparison.css`, which widens the language
chapters' side-by-side blocks.

The book runs the preprocessor from this repository through `cargo run`, so
building it needs only Rust and mdBook:

```sh
mdbook build
```
