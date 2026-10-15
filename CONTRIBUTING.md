# Contributing

Thanks for your interest in `mdbook-tsitter`. The crate is deliberately
small and grammar-agnostic — it ships no grammar of its own and highlights
whatever you configure in `book.toml`, fetching grammars it knows by name.

## Building

```sh
cargo build
cargo clippy --all
cargo fmt
```

Clippy should be clean and `cargo fmt` applied before a change is considered
done. Public items carry doc comments.

## Running the example

The book under [`examples/languages`](examples/languages) is the documentation
site, and doubles as a usage example and an integration test across several
grammars. Its grammars are fetched on the first build and cached afterwards:

```sh
cargo build
PATH="$PWD/target/debug:$PATH" mdbook build examples/languages
```

The user guide chapters include sections of the root `README.md` through
`ANCHOR` comments, so documentation changes go in the README.

## Built-in grammar list

`assets/grammars.json` lists where known grammars live, pinned to the
revisions nvim-treesitter tests. It is generated, never edited by hand:

```sh
./scripts/update-grammar-list.sh
```

After refreshing it, build a book using a broad set of languages to check that
every grammar still fetches, compiles, and has valid queries.

## Scope

The package stays language-agnostic: please do not add grammars or
language-specific behaviour to the crate itself. Behaviour for a class of
grammars belongs in generic handling (the grammar manifest, query
normalisation), and new languages belong in a book's configuration or, when
nvim-treesitter supports them, in a refreshed grammar list.

## Pull requests

- Keep changes focused and the commit history readable.
- Match the surrounding code's style and comment density.
- Update the README and the example when behaviour or configuration changes.
