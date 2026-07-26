# Example book

A buildable book that doubles as the project's documentation site, a usage
example, and an integration test across several grammars. Its chapters are:

- **Introduction** — the textual documentation from the root
  [`README.md`](../../README.md), pulled in with `{{#include}}` while its
  repository-only preview image stays out of the book.
- **Highlighting demos** — one chapter per feature, each explaining itself above
  the code:
  - *Macaulay2* — a well-formed block and one with a syntax error (which still
    highlights, since tree-sitter recovers).
  - *Five languages* — one block each in Macaulay2, Rust, Lua, Haskell and
    Markdown.
  - *Injection* — a Markdown block with embedded `lua` (configured, so it is
    sub-highlighted) and `c` (not configured, so it degrades to plain text).
  - *tree-sitter vs highlight.js* — the same Macaulay2, Rust, Lua, Haskell and
    Markdown samples rendered side by side by both highlighters.
- **Contributing** — the root [`CONTRIBUTING.md`](../../CONTRIBUTING.md), also
  via `{{#include}}`.

Grammars are external (compiled parsers + third-party queries), so they are not
committed. [`setup.sh`](setup.sh) stages them into `parsers/` and `queries/`
(both gitignored), copying from a local nvim-treesitter install by default.

```sh
./setup.sh          # stage parsers + queries (override sources via env vars)
mdbook build        # uses the mdbook-tsitter binary on your PATH
```
