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
grammars. It runs the preprocessor from this checkout through `cargo run`, so
`mdbook serve` shows changes to the code too. Its grammars are fetched on the
first build and cached afterwards:

```sh
mdbook build examples/languages
```

The user guide chapters include sections of the root `README.md` through
`ANCHOR` comments, so documentation changes go in the README.

## Built-in grammar list

`assets/grammars.json` lists where known grammars live, pinned to the
revisions nvim-treesitter tests. It is generated, never edited by hand, by a
task that runs on any platform and needs only Cargo and `git`:

```sh
cargo xtask update-grammar-list
```

It reads nvim-treesitter's parser list at the tip of its main branch; pass a
commit (`cargo xtask update-grammar-list <commit>`) to read it there instead.

Grammars nvim-treesitter does not list, such as Macaulay2's, go in
`assets/additional-grammars.json`, which is edited by hand. Each entry names a
language and gives its repository `url`, the commit to pin as `revision`, and
the grammar's subdirectory as `location` when the repository has several. A
name cannot be in both lists.

After refreshing it, build a book using a broad set of languages to check that
every grammar still fetches, compiles, and has valid queries.

## Scope

The package stays language-agnostic: please do not add grammars or
language-specific behaviour to the crate itself. Behaviour for a class of
grammars belongs in generic handling (the grammar manifest, query
normalisation), and new languages belong in a book's configuration, in a
refreshed grammar list when nvim-treesitter supports them, or otherwise in
`assets/additional-grammars.json`.

## Pull requests

- Keep changes focused and the commit history readable.
- Match the surrounding code's style and comment density.
- Update the README and the example when behaviour or configuration changes.
