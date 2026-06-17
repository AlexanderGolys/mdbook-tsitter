//! Finding fenced code blocks in a chapter and replacing the ones we can
//! highlight with pre-rendered HTML. We locate blocks by byte offset with
//! pulldown-cmark (the same parser mdBook uses) and splice raw HTML in place,
//! which mdBook then passes through untouched.

use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use crate::grammar::Registry;

/// Highlight every fenced code block whose info string names a known grammar.
/// Blocks in other languages (or with no language) are left exactly as written.
pub fn rewrite(content: &str, registry: &Registry) -> String {
    let mut replacements: Vec<(Range<usize>, String)> = Vec::new();
    let mut open: Option<OpenBlock> = None;

    let parser = Parser::new_ext(content, Options::all()).into_offset_iter();
    for (event, range) in parser {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                open = Some(OpenBlock {
                    start: range.start,
                    top_level: at_line_start(content, range.start),
                    lang: fence_language(&info).to_string(),
                    code: String::new(),
                });
            }
            Event::Text(text) => {
                if let Some(block) = open.as_mut() {
                    block.code.push_str(&text);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(block) = open.take() {
                    if let Some(html) = render_block(&block, registry) {
                        replacements.push((block.start..range.end, html));
                    }
                }
            }
            _ => {}
        }
    }

    splice(content, replacements)
}

/// A fenced block being accumulated between its start and end events.
struct OpenBlock {
    start: usize,
    /// Whether the opening fence sits at column 0. Such a fence cannot be inside
    /// a list item or blockquote (those require indentation or a `>` prefix), so
    /// its content carries no container prefix and the raw byte range we replace
    /// matches the text we highlight. Indented or nested fences are left to
    /// mdBook, since pulldown-cmark strips their prefixes from the text and the
    /// splice would otherwise corrupt the surrounding structure.
    top_level: bool,
    lang: String,
    code: String,
}

/// The first language token of a fence info string. mdBook/rustdoc allow
/// comma- or space-separated annotations (e.g. `m2,no_run`), so the grammar tag
/// is everything up to the first separator.
fn fence_language(info: &str) -> &str {
    info.split(|c: char| c.is_whitespace() || c == ',')
        .find(|token| !token.is_empty())
        .unwrap_or_default()
}

/// Whether `offset` is at the start of a line (column 0) in `content`.
fn at_line_start(content: &str, offset: usize) -> bool {
    offset == 0 || content.as_bytes()[offset - 1] == b'\n'
}

/// Render one block to a standalone HTML element, or `None` to leave it as-is.
fn render_block(block: &OpenBlock, registry: &Registry) -> Option<String> {
    if !block.top_level {
        return None;
    }
    let highlighted = match registry.highlight(&block.lang, &block.code)? {
        Ok(html) => html,
        Err(error) => {
            eprintln!(
                "mdbook-tree-sitter: skipping `{}` block: {error:#}",
                block.lang
            );
            return None;
        }
    };
    // `no-highlight` keeps mdBook's highlight.js from re-processing the spans we
    // already produced; the language class is preserved for theming hooks.
    Some(format!(
        "\n<pre class=\"tree-sitter\"><code class=\"no-highlight language-{lang}\">{highlighted}</code></pre>\n",
        lang = block.lang,
    ))
}

/// Apply replacements to `content`, working back-to-front so earlier byte
/// offsets stay valid.
fn splice(content: &str, mut replacements: Vec<(Range<usize>, String)>) -> String {
    if replacements.is_empty() {
        return content.to_string();
    }
    replacements.sort_by_key(|(range, _)| range.start);
    let mut out = content.to_string();
    for (range, html) in replacements.into_iter().rev() {
        out.replace_range(range, &html);
    }
    out
}
