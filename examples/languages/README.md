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
  - *Injection* — a Markdown block with embedded `lua` (configured, so it is
    sub-highlighted) and `toml` (not configured, so it degrades to plain text).
  - *Highlighting, side by side* — substantial, identical samples in Rust,
    Python, TypeScript, JavaScript, Go, Java, C, C++, Bash, PHP, Lua and Haskell
    rendered in two wide columns.
- **Contributing** — the root [`CONTRIBUTING.md`](../../CONTRIBUTING.md), also
  via `{{#include}}`.

Grammars are external compiled parsers plus third-party queries. The deployed
copies are committed so cloud builds are reproducible.
[`setup.sh`](setup.sh) refreshes them from a local nvim-treesitter install.

```sh
./setup.sh          # stage parsers + queries (override sources via env vars)
mdbook build        # uses the mdbook-tsitter binary on your PATH
```
