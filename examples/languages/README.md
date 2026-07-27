# Example book

A buildable book that doubles as the project's documentation site, a usage
example, and an integration test across several grammars. Its chapters are:

- **Introduction** — the textual documentation from the root
  [`README.md`](../../README.md), pulled in with `{{#include}}` while its
  repository-only preview image stays out of the book.
- **Highlighting demos** — one chapter per language. Macaulay2 has a single
  tree-sitter example; Rust, Python, TypeScript, JavaScript, Go, Java, C, C++,
  Bash, PHP, Lua, and Haskell compare the same source rendered by mdBook and
  tree-sitter in two wide columns.
- **Contributing** — the root [`CONTRIBUTING.md`](../../CONTRIBUTING.md), also
  via `{{#include}}`.

Grammars are external compiled parsers plus third-party queries. The deployed
copies are committed so cloud builds are reproducible.
[`setup.sh`](setup.sh) refreshes them from a local nvim-treesitter install.

```sh
./setup.sh          # stage parsers + queries (override sources via env vars)
mdbook build        # uses the mdbook-tsitter binary on your PATH
```
