# Example book

The project's documentation site, built with `mdbook-tsitter` itself. It
doubles as a usage example and as an integration test across several grammars.
Its sections are:

- **Introduction** and **User guide**: the documentation from the root
  [`README.md`](../../README.md), split into chapters with `{{#include}}`
  anchors so the README stays the single source.
- **Examples**: one chapter per language comparing mdBook's highlighting with
  tree-sitter's, followed by Rust Playground, hidden lines, and annotations on
  highlighted blocks.
- **Project**: the root [`CONTRIBUTING.md`](../../CONTRIBUTING.md) and the
  README's development notes.

[`book.toml`](book.toml) is a complete configuration example: languages from the
built-in list by name, and a custom grammar by repository. Grammars are fetched
and compiled on the first build and cached afterwards.

```sh
cargo build
PATH="$PWD/../../target/debug:$PATH" mdbook build
```
