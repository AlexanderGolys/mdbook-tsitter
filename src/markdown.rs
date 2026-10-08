//! Finding fenced code blocks in a chapter and replacing the ones we can
//! highlight with pre-rendered HTML. We locate blocks by byte offset with
//! pulldown-cmark (the same parser mdBook uses) and splice raw HTML in place,
//! which mdBook then passes through untouched. mdBook's own block decorations
//! (playground, hidden lines) are reproduced by [`crate::native`].

use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use crate::grammar::Registry;
use crate::native::NativeFeatures;

/// Highlight every fenced code block whose info string names a known grammar.
/// Blocks in other languages (or with no language) are left exactly as written.
pub fn rewrite(content: &str, registry: &Registry, native: &NativeFeatures) -> String {
    let mut replacements: Vec<(Range<usize>, String)> = Vec::new();
    let mut open: Option<OpenBlock> = None;

    let parser = Parser::new_ext(content, Options::all()).into_offset_iter();
    for (event, range) in parser {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                open = Some(OpenBlock {
                    start: range.start,
                    top_level: at_line_start(content, range.start),
                    info: info.to_string(),
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
                    if let Some(html) = render_block(&block, registry, native) {
                        replacements.push((block.start..range.end, html));
                    }
                }
            }
            _ => {}
        }
    }

    splice(content, replacements)
}

/// Fence-info tag that opts a single block out of tree-sitter highlighting,
/// leaving it to mdBook and its highlight.js. E.g. ```` ```rust,notreesitter ````.
const SKIP_TAG: &str = "notreesitter";

/// Class marking a `<pre>` as rendered by this preprocessor, for styling.
const PRE_CLASS: &str = "treesitter";

/// Class that tells highlight.js a block is already highlighted.
const NO_HIGHLIGHT_CLASS: &str = "no-highlight";

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
    /// The fence info string: the language tag followed by annotations.
    info: String,
    code: String,
}

impl OpenBlock {
    fn lang(&self) -> &str {
        fence_tokens(&self.info).next().unwrap_or_default()
    }

    /// The fence tokens after the language tag, e.g. `ignore` in `rust,ignore`.
    fn annotations(&self) -> Vec<&str> {
        fence_tokens(&self.info).skip(1).collect()
    }
}

/// The whitespace/comma-separated tokens of a fence info string, e.g.
/// `rust,no_run` -> `rust`, `no_run`.
fn fence_tokens(info: &str) -> impl Iterator<Item = &str> {
    info.split(|c: char| c.is_whitespace() || c == ',')
        .filter(|token| !token.is_empty())
}

/// Whether `offset` is at the start of a line (column 0) in `content`.
fn at_line_start(content: &str, offset: usize) -> bool {
    offset == 0 || content.as_bytes()[offset - 1] == b'\n'
}

/// Render one block to a standalone HTML element, or `None` to leave it as-is.
fn render_block(block: &OpenBlock, registry: &Registry, native: &NativeFeatures) -> Option<String> {
    let lang = block.lang();
    let annotations = block.annotations();
    if !block.top_level || annotations.contains(&SKIP_TAG) {
        return None;
    }
    let prepared = native.prepare(lang, &annotations, &block.code)?;
    let highlighted = match registry.highlight(lang, &prepared.source)? {
        Ok(html) => prepared.decorate(&html),
        Err(error) => {
            eprintln!("mdbook-tsitter: skipping `{lang}` block: {error:#}");
            return None;
        }
    };
    let pre_class = class_list([PRE_CLASS.to_string()], prepared.pre_classes);
    // `no-highlight` keeps mdBook's highlight.js from re-processing the spans we
    // already produced; the language class is preserved for theming hooks.
    let code_class = class_list(
        [NO_HIGHLIGHT_CLASS.to_string(), format!("language-{lang}")],
        prepared.code_classes,
    );
    Some(format!(
        "\n<pre class=\"{pre_class}\"><code class=\"{code_class}\">{highlighted}</code></pre>\n",
    ))
}

/// Join our own classes and mdBook's native ones into a `class` attribute value.
fn class_list<const N: usize>(own: [String; N], native: Vec<String>) -> String {
    own.into_iter().chain(native).collect::<Vec<_>>().join(" ")
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
