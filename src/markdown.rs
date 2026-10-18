//! Finding fenced code blocks in a chapter and replacing the ones we can
//! highlight with pre-rendered HTML. We locate blocks by byte offset with
//! mdBook's own Markdown parser setup, so a block is found exactly when mdBook
//! would render it as one, and splice raw HTML in place, which mdBook then
//! passes through untouched. mdBook's own block decorations (playground,
//! hidden lines) are reproduced by [`crate::native`].
//!
//! Consecutive blocks labelled with `pane="…"` are laid out side by side, each
//! under its label; `compare` shows one block that way, as tree-sitter and as
//! highlight.js highlight it.

use std::ops::Range;

use mdbook_markdown::pulldown_cmark::{CodeBlockKind, Event, Tag, TagEnd};
use mdbook_markdown::{new_cmark_parser, MarkdownOptions};
use mdbook_preprocessor::config::HtmlConfig;

use crate::grammar::Registry;
use crate::native::NativeFeatures;

/// The Markdown extensions mdBook's HTML renderer parses chapters with.
pub fn mdbook_markdown_options(html: &HtmlConfig) -> MarkdownOptions {
    let mut options = MarkdownOptions::default();
    options.smart_punctuation = html.smart_punctuation;
    options.definition_lists = html.definition_lists;
    options.admonitions = html.admonitions;
    options
}

/// Highlight every fenced code block whose info string names a known grammar,
/// and lay out labelled blocks side by side. Other blocks are left exactly as
/// written.
pub fn rewrite(
    content: &str,
    markdown_options: &MarkdownOptions,
    registry: &Registry,
    native: &NativeFeatures,
) -> String {
    let blocks = fenced_blocks(content, markdown_options);
    let mut replacements: Vec<(Range<usize>, String)> = Vec::new();
    let mut rest = blocks.as_slice();
    while let Some((block, after)) = rest.split_first() {
        if block.pane_label().is_none() {
            if let Some(html) = render_block(block, registry, native) {
                replacements.push((block.range.clone(), html));
            }
            rest = after;
            continue;
        }
        let group_len = 1 + rest
            .windows(2)
            .take_while(|pair| {
                pair[1].pane_label().is_some()
                    && content[pair[0].range.end..pair[1].range.start]
                        .trim()
                        .is_empty()
            })
            .count();
        let (group, after_group) = rest.split_at(group_len);
        let panes: Vec<Pane> = group
            .iter()
            .map(|block| Pane {
                label: block.pane_label().unwrap_or_default(),
                content: highlighted_pre(block, registry, native)
                    .unwrap_or_else(|| block.as_fence()),
            })
            .collect();
        let range = group[0].range.start..group[group_len - 1].range.end;
        replacements.push((range, side_by_side(&panes)));
        rest = after_group;
    }

    splice(content, replacements)
}

/// Fence-info tag that opts a single block out of tree-sitter highlighting,
/// leaving it to mdBook and its highlight.js. E.g. ```` ```rust,notreesitter ````.
const SKIP_TAG: &str = "notreesitter";

/// Fence-info tag that shows a block side by side as tree-sitter and as
/// highlight.js highlight it. E.g. ```` ```rust,compare ````.
const COMPARE_TAG: &str = "compare";

/// Prefix of the fence-info token labelling a block as a pane of a
/// side-by-side group, e.g. `pane="Before"`.
const PANE_PREFIX: &str = "pane=";

/// Labels of the panes `compare` shows: tree-sitter, then highlight.js.
const TREE_SITTER_PANE_LABEL: &str = "tree-sitter";
const HIGHLIGHTJS_PANE_LABEL: &str = "highlight.js";

/// Class marking a `<pre>` as rendered by this preprocessor, for styling.
const PRE_CLASS: &str = "treesitter";

/// Classes of a side-by-side group, of each of its panes, and of the label
/// heading a pane.
const SIDE_BY_SIDE_CLASS: &str = "treesitter-side-by-side";
const PANE_CLASS: &str = "treesitter-pane";
const PANE_LABEL_CLASS: &str = "treesitter-pane-label";

/// Shortest code fence: three backticks.
const MIN_FENCE_LENGTH: usize = 3;

/// Class that tells highlight.js a block is already highlighted. It must come
/// first: without a `language-*` class, highlight.js uses the first class that
/// is either this or a language name, and annotations can be language names.
const NO_HIGHLIGHT_CLASS: &str = "no-highlight";

/// `<code>` attribute carrying the block's fence tag, e.g. `data-language="rust"`.
const LANGUAGE_ATTRIBUTE: &str = "data-language";

/// Characters written as entities in highlighted code. Preprocessors that run
/// after this one see the block as HTML rather than a code fence, so text
/// they would skip inside a fence must not match their syntax: `$` and `\`
/// open math for preprocessors such as mdbook-katex.
const PREPROCESSOR_SYNTAX_ENTITIES: [(char, &str); 2] = [('$', "&#36;"), ('\\', "&#92;")];

/// A fenced block at column 0. Such a fence cannot be inside a list item or
/// blockquote (those require indentation or a `>` prefix), so its content
/// carries no container prefix and the raw byte range we replace matches the
/// text we highlight. Indented or nested fences are left to mdBook, since
/// pulldown-cmark strips their prefixes from the text and the splice would
/// otherwise corrupt the surrounding structure.
struct FencedBlock {
    range: Range<usize>,
    /// The fence info string: the language tag followed by annotations.
    info: String,
    code: String,
}

impl FencedBlock {
    fn lang(&self) -> &str {
        fence_tokens(&self.info)
            .first()
            .copied()
            .unwrap_or_default()
    }

    /// The fence tokens after the language tag that mdBook sees, e.g. `ignore`
    /// in `rust,ignore,compare`; this preprocessor's own tags are left out.
    fn annotations(&self) -> Vec<&str> {
        fence_tokens(&self.info)
            .into_iter()
            .skip(1)
            .filter(|token| *token != COMPARE_TAG && !token.starts_with(PANE_PREFIX))
            .collect()
    }

    fn has_tag(&self, tag: &str) -> bool {
        fence_tokens(&self.info).contains(&tag)
    }

    /// The label of a block that is a pane of a side-by-side group.
    fn pane_label(&self) -> Option<&str> {
        fence_tokens(&self.info)
            .into_iter()
            .find_map(|token| token.strip_prefix(PANE_PREFIX))
            .map(|label| label.trim_matches('"'))
    }

    /// The block as a fence for mdBook to render, without this preprocessor's
    /// own tags.
    fn as_fence(&self) -> String {
        let info = std::iter::once(self.lang())
            .chain(self.annotations())
            .collect::<Vec<_>>()
            .join(",");
        let code = &self.code;
        let fence = "`".repeat(MIN_FENCE_LENGTH.max(longest_backtick_run(code) + 1));
        // The closing fence needs a line of its own, also after an unclosed block.
        let code_end = if code.is_empty() || code.ends_with('\n') {
            ""
        } else {
            "\n"
        };
        format!("{fence}{info}\n{code}{code_end}{fence}")
    }
}

/// One labelled column of a side-by-side group.
struct Pane<'a> {
    label: &'a str,
    /// A highlighted `<pre>`, or a fence for mdBook to render.
    content: String,
}

/// The top-level fenced blocks of a chapter, in order.
fn fenced_blocks(content: &str, markdown_options: &MarkdownOptions) -> Vec<FencedBlock> {
    let mut blocks = Vec::new();
    let mut open: Option<FencedBlock> = None;
    for (event, range) in new_cmark_parser(content, markdown_options).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info)))
                if at_line_start(content, range.start) =>
            {
                open = Some(FencedBlock {
                    range,
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
                if let Some(mut block) = open.take() {
                    block.range.end = range.end;
                    blocks.push(block);
                }
            }
            _ => {}
        }
    }
    blocks
}

/// The whitespace/comma-separated tokens of a fence info string, e.g.
/// `rust,no_run` -> `rust`, `no_run`. Separators inside double quotes belong
/// to the token, so `pane="Before change"` is one token.
fn fence_tokens(info: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut token_start = None;
    let mut quoted = false;
    for (index, c) in info.char_indices() {
        if c == '"' {
            quoted = !quoted;
        }
        let separator = !quoted && (c.is_whitespace() || c == ',');
        match token_start {
            Some(start) if separator => {
                tokens.push(&info[start..index]);
                token_start = None;
            }
            None if !separator => token_start = Some(index),
            _ => {}
        }
    }
    if let Some(start) = token_start {
        tokens.push(&info[start..]);
    }
    tokens
}

/// Whether `offset` is at the start of a line (column 0) in `content`.
fn at_line_start(content: &str, offset: usize) -> bool {
    offset == 0 || content.as_bytes()[offset - 1] == b'\n'
}

/// Render a block outside a side-by-side group to its replacement, or `None`
/// to leave it as-is.
fn render_block(
    block: &FencedBlock,
    registry: &Registry,
    native: &NativeFeatures,
) -> Option<String> {
    let pre = highlighted_pre(block, registry, native)?;
    Some(if block.has_tag(COMPARE_TAG) {
        side_by_side(&[
            Pane {
                label: TREE_SITTER_PANE_LABEL,
                content: pre,
            },
            Pane {
                label: HIGHLIGHTJS_PANE_LABEL,
                content: block.as_fence(),
            },
        ])
    } else {
        format!("\n{pre}\n")
    })
}

/// A block highlighted as a standalone `<pre>` element, or `None` when it is
/// left to mdBook.
fn highlighted_pre(
    block: &FencedBlock,
    registry: &Registry,
    native: &NativeFeatures,
) -> Option<String> {
    if block.has_tag(SKIP_TAG) {
        return None;
    }
    let lang = block.lang();
    let prepared = native.prepare(lang, &block.annotations(), &block.code)?;
    let highlighted = match registry.highlight(lang, &prepared.source)? {
        Ok(html) => escape_preprocessor_syntax(&prepared.decorate(&html)),
        Err(error) => {
            eprintln!("mdbook-tsitter: skipping `{lang}` block: {error:#}");
            return None;
        }
    };
    let pre_class = class_list(PRE_CLASS, prepared.pre_classes);
    let code_class = class_list(NO_HIGHLIGHT_CLASS, prepared.code_classes);
    // The fence tag goes in a data attribute rather than a `language-*` class:
    // mdBook's highlight.js re-highlights any block whose `language-*` class
    // names a language it knows, ignoring `no-highlight`, and its colours would
    // override ours.
    Some(format!(
        "<pre class=\"{pre_class}\"><code class=\"{code_class}\" {LANGUAGE_ATTRIBUTE}=\"{lang}\">{highlighted}</code></pre>",
    ))
}

/// Panes side by side, each under its label. An HTML block ends at a blank
/// line, so blank lines around each pane's content let mdBook parse a fence as
/// Markdown and a `<pre>` as an HTML block of its own, which may contain blank
/// lines.
fn side_by_side(panes: &[Pane]) -> String {
    let panes: String = panes
        .iter()
        .map(|pane| {
            format!(
                "<div class=\"{PANE_CLASS}\">\n\
                 <div class=\"{PANE_LABEL_CLASS}\">{}</div>\n\n\
                 {}\n\n\
                 </div>\n",
                escape_html(pane.label),
                pane.content,
            )
        })
        .collect();
    format!("\n<div class=\"{SIDE_BY_SIDE_CLASS}\">\n{panes}</div>\n")
}

/// The length of the longest run of backticks in `text`.
fn longest_backtick_run(text: &str) -> usize {
    text.split(|c| c != '`').map(str::len).max().unwrap_or(0)
}

/// `text` with the characters HTML gives meaning to written as entities.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Write [`PREPROCESSOR_SYNTAX_ENTITIES`] in rendered code as entities; the
/// browser shows the same text.
fn escape_preprocessor_syntax(html: &str) -> String {
    let mut escaped = String::with_capacity(html.len());
    for c in html.chars() {
        match PREPROCESSOR_SYNTAX_ENTITIES
            .iter()
            .find(|(raw, _)| *raw == c)
        {
            Some((_, entity)) => escaped.push_str(entity),
            None => escaped.push(c),
        }
    }
    escaped
}

/// Join our own class and mdBook's native ones into a `class` attribute value,
/// ours first.
fn class_list(own: &str, native: Vec<String>) -> String {
    std::iter::once(own.to_string())
        .chain(native)
        .collect::<Vec<_>>()
        .join(" ")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn block(info: &str, code: &str) -> FencedBlock {
        FencedBlock {
            range: 0..0,
            info: info.to_string(),
            code: code.to_string(),
        }
    }

    #[test]
    fn quoted_tokens_keep_their_separators() {
        assert_eq!(
            fence_tokens("rust,ignore pane=\"Before, as written\""),
            ["rust", "ignore", "pane=\"Before, as written\""]
        );
    }

    #[test]
    fn pane_label_and_own_tags_stay_out_of_the_fence_for_mdbook() {
        let labelled = block("rust,ignore,pane=\"Before change\"", "let x = 1;\n");
        assert_eq!(labelled.pane_label(), Some("Before change"));
        assert_eq!(labelled.as_fence(), "```rust,ignore\nlet x = 1;\n```");
        assert_eq!(block("rust,compare", "").as_fence(), "```rust\n```");
    }

    #[test]
    fn fence_outlasts_backticks_in_the_code() {
        assert_eq!(
            block("markdown", "````rust\n````").as_fence(),
            "`````markdown\n````rust\n````\n`````"
        );
    }

    #[test]
    fn panes_are_separate_html_blocks_under_their_labels() {
        let html = side_by_side(&[
            Pane {
                label: "<Before>",
                content: "```rust\nlet x = 1;\n```".to_string(),
            },
            Pane {
                label: "After",
                content: "<pre>highlighted</pre>".to_string(),
            },
        ]);
        assert!(html.contains(">&lt;Before&gt;</div>\n\n```rust\nlet x = 1;\n```\n\n</div>"));
        assert!(html.contains(">After</div>\n\n<pre>highlighted</pre>\n\n</div>"));
    }
}
