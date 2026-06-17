//! Turning tree-sitter highlight events into HTML. Captures are emitted as CSS
//! classes (not inline colours) so themes live in a stylesheet the book ships,
//! exactly like the rest of mdBook's styling.

use anyhow::{Context, Result};
use tree_sitter_highlight::{Highlight, HighlightConfiguration, Highlighter, HtmlRenderer};

/// CSS class prefix for every generated span, e.g. `ts-keyword`.
const CLASS_PREFIX: &str = "ts";

/// Build the `class="…"` attribute bytes for each highlight index, given the
/// capture names in index order. A dotted capture yields one class per prefix
/// so a stylesheet can target the broad group or the specific kind, e.g.
/// `keyword.operator` → `class="ts-keyword ts-keyword-operator"`.
pub fn class_attributes(names: &[String]) -> Vec<Vec<u8>> {
    names.iter().map(|name| class_attribute(name)).collect()
}

fn class_attribute(name: &str) -> Vec<u8> {
    let mut classes = Vec::new();
    let mut current = String::from(CLASS_PREFIX);
    for part in name.split('.') {
        current.push('-');
        current.push_str(&sanitize(part));
        classes.push(current.clone());
    }
    format!("class=\"{}\"", classes.join(" ")).into_bytes()
}

/// Reduce a capture-name segment to characters safe and conventional in a CSS
/// class: lowercase letters, digits and `-`.
fn sanitize(segment: &str) -> String {
    segment
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

/// Highlight `source` with `config` and render the spans to an HTML fragment.
/// The result contains only the highlighted spans — the caller wraps it in
/// `<pre><code>`.
pub fn to_html(
    config: &HighlightConfiguration,
    classes: &[Vec<u8>],
    source: &str,
) -> Result<String> {
    let mut highlighter = Highlighter::new();
    let events = highlighter
        .highlight(config, source.as_bytes(), None, |_| None)
        .context("tree-sitter highlighting failed")?;

    let mut renderer = HtmlRenderer::new();
    renderer
        .render(
            events,
            source.as_bytes(),
            &|highlight: Highlight, output: &mut Vec<u8>| {
                // An out-of-range index would mean a capture we never configured;
                // fall back to an empty attribute rather than panic.
                if let Some(attr) = classes.get(highlight.0) {
                    output.extend_from_slice(attr);
                }
            },
        )
        .context("rendering highlighted HTML failed")?;

    Ok(renderer.lines().collect())
}
