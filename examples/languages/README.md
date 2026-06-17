# Example: multiple languages and injection

A small book that exercises `mdbook-treesitter` across several grammars:

- **Chapter 1** — two Macaulay2 blocks, one well-formed and one with a syntax
  error (which still highlights, since tree-sitter recovers).
- **Chapter 2** — one block each in Macaulay2, Rust, Lua, Haskell and Markdown,
  every grammar loaded dynamically.
- **Chapter 3** — a Markdown block containing embedded `lua` and `c` blocks. Lua
  is configured and gets injected/highlighted; C is not configured and is left
  as plain text (an unregistered injected language degrades gracefully).

Grammars are external — compiled parsers and third-party queries — so they are
not committed. [`setup.sh`](setup.sh) stages them into `parsers/` and `queries/`
(both gitignored), copying from a local nvim-treesitter install by default.

```sh
./setup.sh          # stage parsers + queries (override sources via env vars)
mdbook build        # uses the mdbook-treesitter binary on your PATH
```

See [`setup.sh`](setup.sh) for the source paths it reads and how to override
them.
