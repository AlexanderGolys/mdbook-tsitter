//! An mdBook preprocessor that highlights fenced code blocks with tree-sitter.
//!
//! Each language is configured in the `[preprocessor.tsitter]` table of
//! `book.toml` by naming its grammar repository or pointing at a compiled
//! parser and a highlights query, and is then loaded dynamically — see
//! [`config`]. The preprocessor is grammar-agnostic: it ships no grammar of
//! its own.

pub mod config;
pub mod fetch;
pub mod grammar;
pub mod known;
pub mod manifest;
pub mod markdown;
pub mod native;
pub mod predicates;
pub mod render;
pub mod theme;

use anyhow::Context;
use mdbook_preprocessor::book::{Book, BookItem};
use mdbook_preprocessor::config::HtmlConfig;
use mdbook_preprocessor::errors::Result;
use mdbook_preprocessor::{Preprocessor, PreprocessorContext};

use crate::config::Config;
use crate::grammar::Registry;
use crate::native::NativeFeatures;

/// The bundled class-based stylesheet, covering mdBook's built-in themes.
pub const DEFAULT_CSS: &str = include_str!("../assets/treesitter.css");

/// Text in the bundled stylesheet's header comment, which marks a copy of it.
const DEFAULT_CSS_SIGNATURE: &str = "Default theme for mdbook-treesitter";

/// Cascade layer holding the embedded stylesheet. Unlayered rules beat layered
/// ones whatever their order, so the book's own `additional-css` overrides it
/// even though the embedded copy comes later in the page.
const CSS_LAYER: &str = "mdbook-tsitter";

/// A stylesheet as a raw HTML block appended to a chapter. A `<style>` block
/// runs to its closing tag, so blank lines inside the CSS are kept.
fn embedded_stylesheet(css: &str) -> String {
    format!("\n\n<style>\n@layer {CSS_LAYER} {{\n{css}}}\n</style>\n")
}

/// Warn about copies of the bundled stylesheet in `additional-css`: books set
/// up before it was embedded had to add one, which is now redundant.
fn warn_about_copied_stylesheets(root: &std::path::Path, html: &HtmlConfig) {
    for stylesheet in &html.additional_css {
        let is_copy = std::fs::read_to_string(root.join(stylesheet))
            .is_ok_and(|css| css.contains(DEFAULT_CSS_SIGNATURE));
        if is_copy {
            warn(&format!(
                "`{}` in `additional-css` is a copy of the bundled stylesheet, which is now \
                 embedded automatically; adding it is deprecated. Remove it, keep only your own \
                 changes in it, or move it to `{}` in the theme directory if it replaces the \
                 bundled theme",
                stylesheet.display(),
                theme::THEME_STYLESHEET,
            ));
        }
    }
}

/// Print a warning about the configuration; the build continues.
pub(crate) fn warn(message: &str) {
    eprintln!("mdbook-tsitter: warning: {message}");
}

/// The preprocessor entry point registered with mdBook as `tsitter`.
pub struct TreeSitterPreprocessor;

impl Preprocessor for TreeSitterPreprocessor {
    fn name(&self) -> &str {
        "tsitter"
    }

    fn run(&self, ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
        let config = Config::from_context(ctx)?;
        let registry = Registry::build(&ctx.root, &config)?;
        let native = NativeFeatures::from_book(&ctx.config)?;
        let html = ctx.config.html_config().unwrap_or_default();
        let markdown_options = markdown::mdbook_markdown_options(&html);
        let embedded_css = if config.default_css {
            warn_about_copied_stylesheets(&ctx.root, &html);
            Some(embedded_stylesheet(&theme::stylesheet(
                &html.theme_dir(&ctx.root),
            )?))
        } else {
            None
        };

        let everforest = ctx
            .config
            .get::<bool>("preprocessor.tsitter.everforest")
            .context("invalid preprocessor.tsitter.everforest option: expected a boolean")?
            .unwrap_or(false)
            .then(theme::everforest_assets);

        book.for_each_mut(|item| {
            if let BookItem::Chapter(chapter) = item {
                let rewritten =
                    markdown::rewrite(&chapter.content, &markdown_options, &registry, &native);
                let highlighted = rewritten != chapter.content;
                chapter.content = rewritten;
                if let Some(css) = embedded_css.as_deref().filter(|_| highlighted) {
                    chapter.content.push_str(css);
                }
                if let Some(assets) = &everforest {
                    chapter.content.push_str(assets);
                }
            }
        });

        Ok(book)
    }

    fn supports_renderer(&self, renderer: &str) -> Result<bool> {
        // We emit HTML, so only the HTML renderer is meaningful.
        Ok(renderer == "html")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everforest_is_available_on_chapters_without_highlighted_blocks() {
        use mdbook_preprocessor::book::Chapter;

        for enabled in [false, true] {
            let mut config = mdbook_preprocessor::config::Config::default();
            config
                .set("preprocessor.tsitter.everforest", enabled)
                .unwrap();
            config
                .set("preprocessor.tsitter.default-css", false)
                .unwrap();
            let context =
                PreprocessorContext::new(std::env::temp_dir(), config, "html".to_string());
            let mut book = Book::new();
            book.push_item(Chapter::new(
                "Intro",
                "Hello".to_string(),
                "intro.md",
                vec![],
            ));
            let processed = TreeSitterPreprocessor.run(&context, book).unwrap();
            let BookItem::Chapter(chapter) = &processed.items[0] else {
                panic!("expected a chapter");
            };
            assert_eq!(chapter.content.contains(theme::EVERFOREST_CSS), enabled);
            assert_eq!(chapter.content.contains("registerEverforest"), enabled);
            if !enabled {
                assert_eq!(chapter.content, "Hello");
            }
        }
    }

    #[test]
    fn default_stylesheet_carries_its_signature() {
        assert!(DEFAULT_CSS.contains(DEFAULT_CSS_SIGNATURE));
    }
}
