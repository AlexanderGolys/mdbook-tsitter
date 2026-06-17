# mdbook-treesitter

An [mdBook](https://rust-lang.github.io/mdBook/) preprocessor that highlights
fenced code blocks with [tree-sitter](https://tree-sitter.github.io/). It is
grammar-agnostic: it ships no grammar of its own — you point it at a compiled
parser and a highlights query, and it highlights that language. Any language
with a tree-sitter grammar works, and embedded languages are highlighted through
injections.

Highlighting happens at build time. Each tree-sitter capture becomes a
`<span class="ts-…">`, so colours live in a stylesheet (like the rest of
mdBook's theming) rather than being baked into the HTML.

## How it works

A preprocessor receives each chapter as Markdown and returns modified Markdown.
This one parses the Markdown with the same parser mdBook uses
([pulldown-cmark](https://docs.rs/pulldown-cmark)), finds every fenced code
block whose info string names a configured grammar, highlights its contents, and
splices in a ready-made HTML block:

```html
<pre class="treesitter"><code class="no-highlight language-rust">…spans…</code></pre>
```

The `no-highlight` class stops mdBook's default highlight.js from touching the
spans. Blocks in unconfigured languages (or with no language tag) are left
exactly as written, so mdBook's default highlighter still handles them.

## Install

```sh
cargo install mdbook-treesitter
```

The `mdbook-treesitter` binary must be on your `PATH`.

## Set up a book

1. Enable the preprocessor and configure a language in `book.toml`:

   ```toml
   [preprocessor.treesitter]

   [preprocessor.treesitter.languages.rust]
   library = "parsers/rust.so"            # compiled parser, relative to the book root
   highlights = "queries/rust/highlights.scm"
   ```

2. Write the default theme into your book and reference it:

   ```sh
   mdbook-treesitter css > theme/treesitter.css
   ```

   ```toml
   [output.html]
   additional-css = ["theme/treesitter.css"]
   ```

See [`examples/languages`](examples/languages) for a complete, buildable book
covering several languages and injection.

## Configuration

Everything lives under `[preprocessor.treesitter]` in `book.toml`.

Top-level:

| key      | default | meaning                                                                                                                                                                                                                                                              |
| -------- | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `inject` | `true`  | Highlight languages embedded in a block via a grammar's injections query. Only configured languages are ever used; injection never loads a new grammar. Set `false` to switch it off (and skip loading `injections.scm`, so a broken injections query can't break highlighting). |

Per language, under `[preprocessor.treesitter.languages.<name>]`:

| key          | required | meaning                                                                       |
| ------------ | -------- | ----------------------------------------------------------------------------- |
| `library`    | yes      | Path to the compiled parser shared object.                                    |
| `highlights` | yes      | Path to the highlights query (`highlights.scm`).                              |
| `symbol`     | no       | Parser constructor symbol; defaults to `tree_sitter_<name>` (`-` → `_`).      |
| `injections` | no       | Path to an injections query, for embedded languages.                          |
| `locals`     | no       | Path to a locals query, for scope-aware highlighting.                         |
| `aliases`    | no       | Code-fence tags this grammar handles; defaults to the table key.              |

Paths are relative to the book root.

### Getting a parser shared object and queries

Most grammars live in a `tree-sitter-<lang>` repository. With the
[tree-sitter CLI](https://github.com/tree-sitter/tree-sitter):

```sh
git clone https://github.com/<owner>/tree-sitter-nix
cd tree-sitter-nix
tree-sitter build --output libtree-sitter-nix.so
```

The highlights query is the grammar's `queries/highlights.scm`. An existing
[nvim-treesitter](https://github.com/nvim-treesitter/nvim-treesitter) install is
also a convenient source of both compiled parsers and queries.

## Capture names and CSS classes

There is no fixed list of supported captures — the preprocessor reads the
capture names out of each grammar's own query, so it supports whatever that
query defines. A capture becomes a class with the `ts-` prefix and dots turned
into hyphens, and every prefix is emitted so broad rules cascade and specific
ones override:

| capture            | classes                          |
| ------------------ | -------------------------------- |
| `keyword`          | `ts-keyword`                     |
| `keyword.operator` | `ts-keyword ts-keyword-operator` |
| `string.regexp`    | `ts-string ts-string-regexp`     |

Captures whose name starts with `_` are treated as internal and not styled.

The default stylesheet (`mdbook-treesitter css`) styles the
[standard tree-sitter / nvim-treesitter capture names](https://github.com/nvim-treesitter/nvim-treesitter/blob/main/CONTRIBUTING.md#highlights)
(`@comment`, `@keyword`, `@string`, `@function`, `@type`, `@variable`, …), so a
grammar whose query uses those names is styled out of the box. Add rules for any
extra captures your grammar defines.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at
your option.
