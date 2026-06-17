//! An mdBook preprocessor that highlights fenced code blocks with tree-sitter.
//!
//! The bundled default grammar is Macaulay2 (fence tags `m2` / `macaulay2`).
//! Any other language is added through the `[preprocessor.tree-sitter]` table
//! in `book.toml` by pointing at a compiled parser and a highlights query — see
//! [`config`].

pub mod config;
pub mod grammar;
pub mod markdown;
pub mod render;

use mdbook_preprocessor::book::{Book, BookItem};
use mdbook_preprocessor::errors::Result;
use mdbook_preprocessor::{Preprocessor, PreprocessorContext};

use crate::config::Config;
use crate::grammar::Registry;

/// The preprocessor entry point registered with mdBook as `tree-sitter`.
pub struct TreeSitterPreprocessor;

impl Preprocessor for TreeSitterPreprocessor {
    fn name(&self) -> &str {
        "tree-sitter"
    }

    fn run(&self, ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
        let config = Config::from_context(ctx)?;
        let registry = Registry::build(&ctx.root, &config)?;

        book.for_each_mut(|item| {
            if let BookItem::Chapter(chapter) = item {
                chapter.content = markdown::rewrite(&chapter.content, &registry);
            }
        });

        Ok(book)
    }

    fn supports_renderer(&self, renderer: &str) -> Result<bool> {
        // We emit HTML, so only the HTML renderer is meaningful.
        Ok(renderer == "html")
    }
}
