//! mdBook's native code-block features, kept for the blocks we highlight.
//!
//! After rendering Markdown, mdBook's HTML renderer decorates each code block:
//! runnable Rust blocks become playgrounds (an edition class and a hidden
//! `fn main` wrapper) and hidden lines are wrapped in `<span class="boring">`.
//! It only decorates a `<code>` element holding a single text node, so the
//! pre-highlighted HTML we splice in would lose all of it. This module applies
//! the same rules up front and describes the result as generic decorations
//! (element classes and per-line visibility) that the renderer lays over the
//! highlighted source, without knowing which feature produced them.

use std::collections::HashMap;

use anyhow::{Context, Result};
use mdbook_preprocessor::config::Config as BookConfig;

/// Fence tag that mdBook gives Rust's playground and hidden-line rules.
const RUST_FENCE: &str = "rust";
/// Annotations that keep a Rust block out of the playground.
const NON_RUNNABLE_ANNOTATIONS: [&str; 3] = ["ignore", "noplayground", "noplaypen"];
/// Annotation that makes a Rust block runnable even when the book disables it.
const FORCE_RUNNABLE_ANNOTATION: &str = "mdbook-runnable";
/// Annotation that turns a playground into an in-page editor.
const EDITABLE_ANNOTATION: &str = "editable";
/// Class prefix carrying the playground's Rust edition, e.g. `edition2021`.
const EDITION_CLASS_PREFIX: &str = "edition";
/// Annotation prefix choosing a block's hidden-line marker, e.g. `hidelines=!!!`.
const HIDELINES_ANNOTATION_PREFIX: &str = "hidelines=";
/// Class mdBook's script looks for to attach the Run button.
const PLAYGROUND_CLASS: &str = "playground";

/// The book-wide settings that decide how mdBook decorates a code block.
#[derive(Debug, Clone)]
pub struct NativeFeatures {
    /// `[rust] edition`, as the class mdBook adds to playground blocks.
    pub edition_class: Option<String>,
    /// `[output.html.playground] runnable`.
    pub runnable: bool,
    /// `[output.html.playground] editable`.
    pub editable: bool,
    /// `[output.html.code.hidelines]`: hidden-line prefix per fence tag.
    pub hidelines: HashMap<String, String>,
}

/// A fenced block rewritten the way mdBook would display it, ready to be
/// highlighted and decorated.
#[derive(Debug, PartialEq)]
pub struct PreparedBlock {
    /// The code to highlight: hidden-line markers removed and, for a
    /// playground, wrapped in `fn main` when it has none.
    pub source: String,
    /// Classes for the `<pre>` element.
    pub pre_classes: Vec<String>,
    /// Classes for the `<code>` element.
    pub code_classes: Vec<String>,
    /// One flag per line of `source`: whether the line is hidden by default.
    hidden_lines: Vec<bool>,
}

/// How a block marks lines that are compiled or run but hidden from readers.
#[derive(Debug, Clone, Copy)]
enum HiddenLineSyntax<'a> {
    /// rustdoc's syntax: `# code` or a bare `#` is hidden, `##` escapes a
    /// visible `#`, and anything else (e.g. `#[derive]`) is ordinary code.
    Rustdoc,
    /// A line starting (after indentation) with this prefix is hidden.
    Prefix(&'a str),
}

impl Default for NativeFeatures {
    /// mdBook's defaults: playgrounds are runnable, not editable, and no
    /// edition or hidden-line prefixes are configured.
    fn default() -> Self {
        Self {
            edition_class: None,
            runnable: true,
            editable: false,
            hidelines: HashMap::new(),
        }
    }
}

impl NativeFeatures {
    /// Read the relevant settings from the book configuration.
    pub fn from_book(config: &BookConfig) -> Result<Self> {
        let html = config.html_config().unwrap_or_default();
        let edition_class = config
            .rust
            .edition
            .map(|edition| {
                serde_json::to_value(edition)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_string))
                    .map(|year| format!("{EDITION_CLASS_PREFIX}{year}"))
                    .context("unrecognised `[rust] edition`")
            })
            .transpose()?;
        Ok(Self {
            edition_class,
            runnable: html.playground.runnable,
            editable: html.playground.editable,
            hidelines: html.code.hidelines,
        })
    }

    /// Prepare a fenced block for highlighting, or `None` when mdBook must
    /// keep it: an editable playground is replaced by an in-page editor that
    /// reads plain text, so highlighting it would only be thrown away.
    ///
    /// Every annotation becomes a `<code>` class, as mdBook does, so features
    /// keyed on annotations (in mdBook or in a theme) see the same markup.
    pub fn prepare(&self, lang: &str, annotations: &[&str], code: &str) -> Option<PreparedBlock> {
        let has = |annotation: &str| annotations.contains(&annotation);
        let is_rust = lang == RUST_FENCE;
        let is_playground = is_rust
            && ((self.runnable && !NON_RUNNABLE_ANNOTATIONS.iter().any(|a| has(a)))
                || has(FORCE_RUNNABLE_ANNOTATION));
        if is_playground && self.editable && has(EDITABLE_ANNOTATION) {
            return None;
        }

        let mut pre_classes = Vec::new();
        let mut code_classes: Vec<String> = annotations.iter().map(|a| a.to_string()).collect();
        let mut code = code.to_string();
        if is_playground {
            pre_classes.push(PLAYGROUND_CLASS.to_string());
            let has_edition = annotations
                .iter()
                .any(|a| a.starts_with(EDITION_CLASS_PREFIX));
            if let Some(edition) = self.edition_class.as_ref().filter(|_| !has_edition) {
                code_classes.push(edition.clone());
            }
            if let Some(wrapped) = wrap_rust_main(&code) {
                code = wrapped;
            }
        }

        let syntax = if is_rust {
            Some(HiddenLineSyntax::Rustdoc)
        } else {
            annotations
                .iter()
                .find_map(|a| a.strip_prefix(HIDELINES_ANNOTATION_PREFIX))
                .or_else(|| self.hidelines.get(lang).map(String::as_str))
                .map(HiddenLineSyntax::Prefix)
        };
        let mut lines: Vec<(String, bool)> = code
            .split_inclusive('\n')
            .map(|line| match syntax {
                Some(syntax) => syntax.reveal(line),
                None => (line.to_string(), false),
            })
            .collect();

        // mdBook rejoins rustdoc-syntax lines without a final newline.
        if let (Some(HiddenLineSyntax::Rustdoc), Some((last, _))) = (syntax, lines.last_mut()) {
            if last.ends_with('\n') {
                last.pop();
            }
        }

        Some(PreparedBlock {
            source: lines.iter().map(|(text, _)| text.as_str()).collect(),
            pre_classes,
            code_classes,
            hidden_lines: lines.iter().map(|&(_, hidden)| hidden).collect(),
        })
    }
}

impl PreparedBlock {
    /// Lay the block's line decorations over its highlighted `html`: hidden
    /// lines are wrapped in `<span class="boring">`, which mdBook's script
    /// collapses behind a "Show hidden lines" button. The highlighter closes
    /// and reopens its spans at every newline, so each line of `html` is
    /// self-contained and can be wrapped on its own. The highlighter also ends
    /// its output with a newline, which is dropped when `source` has none.
    pub fn decorate(&self, html: &str) -> String {
        let html = match self.source.ends_with('\n') {
            true => html,
            false => html.strip_suffix('\n').unwrap_or(html),
        };
        let mut html_lines = html.split_inclusive('\n');
        self.hidden_lines
            .iter()
            // An empty last line has no text in `html`, but a hidden one still
            // gets its (empty) span, as mdBook renders it.
            .map(|&hidden| (html_lines.next().unwrap_or_default(), hidden))
            .map(|(line, hidden)| {
                if hidden {
                    format!("<span class=\"boring\">{line}</span>")
                } else {
                    line.to_string()
                }
            })
            .collect()
    }
}

impl HiddenLineSyntax<'_> {
    /// The line as it is displayed (marker removed), and whether it is hidden.
    fn reveal(self, line: &str) -> (String, bool) {
        let indent_len = line.len() - line.trim_start().len();
        let (indent, rest) = line.split_at(indent_len);
        match self {
            Self::Prefix(prefix) => match rest.strip_prefix(prefix) {
                Some(hidden) => (format!("{indent}{hidden}"), true),
                None => (line.to_string(), false),
            },
            Self::Rustdoc => match rest.strip_prefix('#') {
                Some(escaped) if escaped.starts_with('#') => (format!("{indent}{escaped}"), false),
                Some(hidden) if hidden.starts_with(' ') => {
                    (format!("{indent}{}", &hidden[1..]), true)
                }
                Some(bare) if bare.trim_end_matches(['\r', '\n']).is_empty() => {
                    (format!("{indent}{bare}"), true)
                }
                _ => (line.to_string(), false),
            },
        }
    }
}

/// Wrap Rust code that has no `fn main` in a hidden one, as rustdoc and mdBook
/// do, keeping leading inner attributes (`#![…]`) at crate level.
fn wrap_rust_main(code: &str) -> Option<String> {
    if code.contains("fn main") || code.contains("quick_main!") {
        return None;
    }
    let attrs_len: usize = code
        .split_inclusive('\n')
        .take_while(|line| {
            let line = line.trim_start();
            line.is_empty() || line.starts_with("#![")
        })
        .map(str::len)
        .sum();
    let (attrs, body) = code.split_at(attrs_len);
    let newline = if body.is_empty() || body.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    Some(format!(
        "# #![allow(unused)]\n{attrs}# fn main() {{\n{body}{newline}# }}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book_defaults() -> NativeFeatures {
        NativeFeatures {
            edition_class: Some("edition2021".into()),
            ..Default::default()
        }
    }

    fn prepare(features: &NativeFeatures, fence: &str, code: &str) -> Option<PreparedBlock> {
        let mut tokens = fence.split(',');
        let lang = tokens.next().unwrap();
        let annotations: Vec<&str> = tokens.collect();
        features.prepare(lang, &annotations, code)
    }

    fn hidden(block: &PreparedBlock) -> Vec<bool> {
        block.hidden_lines.clone()
    }

    #[test]
    fn rust_block_becomes_playground_with_hidden_main() {
        let block = prepare(&book_defaults(), "rust", "let x = 1;\n").unwrap();
        assert_eq!(block.pre_classes, ["playground"]);
        assert_eq!(block.code_classes, ["edition2021"]);
        assert_eq!(
            block.source,
            "#![allow(unused)]\nfn main() {\nlet x = 1;\n}"
        );
        assert_eq!(hidden(&block), [true, true, false, true]);
    }

    #[test]
    fn main_wrapper_keeps_inner_attributes_at_crate_level() {
        let block = prepare(
            &book_defaults(),
            "rust",
            "#![allow(dead_code)]\n\nlet x = 1;",
        )
        .unwrap();
        assert_eq!(
            block.source,
            "#![allow(unused)]\n#![allow(dead_code)]\n\nfn main() {\nlet x = 1;\n}"
        );
        assert_eq!(hidden(&block), [true, false, false, true, false, true]);
    }

    #[test]
    fn existing_main_is_not_wrapped() {
        for code in ["fn main() {}", "quick_main!(run);"] {
            let block = prepare(&book_defaults(), "rust", &format!("{code}\n")).unwrap();
            assert_eq!(block.source, code);
        }
    }

    #[test]
    fn explicit_edition_overrides_book_edition() {
        let block = prepare(&book_defaults(), "rust,edition2018", "fn main() {}\n").unwrap();
        assert_eq!(block.code_classes, ["edition2018"]);
    }

    #[test]
    fn missing_book_edition_adds_no_class() {
        let block = prepare(&NativeFeatures::default(), "rust", "fn main() {}\n").unwrap();
        assert!(block.code_classes.is_empty());
    }

    #[test]
    fn non_runnable_annotations_disable_playground() {
        for fence in ["rust,ignore", "rust,noplayground", "rust,noplaypen"] {
            let block = prepare(&book_defaults(), fence, "let x = 1;\n").unwrap();
            assert!(block.pre_classes.is_empty(), "{fence}");
            assert_eq!(block.source, "let x = 1;", "{fence}");
        }
    }

    #[test]
    fn other_annotations_keep_playground_and_become_classes() {
        let block = prepare(
            &book_defaults(),
            "rust,should_panic,no_run",
            "fn main() {}\n",
        )
        .unwrap();
        assert_eq!(block.pre_classes, ["playground"]);
        assert_eq!(
            block.code_classes,
            ["should_panic", "no_run", "edition2021"]
        );
    }

    #[test]
    fn book_can_disable_playgrounds_unless_forced() {
        let disabled = NativeFeatures {
            runnable: false,
            ..book_defaults()
        };
        let plain = prepare(&disabled, "rust", "fn main() {}\n").unwrap();
        assert!(plain.pre_classes.is_empty());
        let forced = prepare(&disabled, "rust,mdbook-runnable", "fn main() {}\n").unwrap();
        assert_eq!(forced.pre_classes, ["playground"]);
    }

    #[test]
    fn editable_playground_is_left_to_mdbook() {
        let editable = NativeFeatures {
            editable: true,
            ..book_defaults()
        };
        assert!(prepare(&editable, "rust,editable", "fn main() {}\n").is_none());
        assert!(prepare(&book_defaults(), "rust,editable", "fn main() {}\n").is_some());
        assert!(prepare(&editable, "rust,editable,ignore", "fn main() {}\n").is_some());
    }

    #[test]
    fn only_the_rust_fence_tag_gets_rust_rules() {
        let block = prepare(&book_defaults(), "rs", "# hidden\nlet x = 1;\n").unwrap();
        assert!(block.pre_classes.is_empty());
        assert_eq!(block.source, "# hidden\nlet x = 1;\n");
        assert_eq!(hidden(&block), [false, false]);
    }

    #[test]
    fn rustdoc_hidden_line_syntax() {
        let block = prepare(
            &book_defaults(),
            "rust,ignore",
            "# use std::fmt;\n#\n  # indented();\n## escaped\n#[derive(Debug)]\n#![attr]\n#\tnot_hidden\nplain\n",
        )
        .unwrap();
        assert_eq!(
            block.source,
            "use std::fmt;\n\n  indented();\n# escaped\n#[derive(Debug)]\n#![attr]\n#\tnot_hidden\nplain"
        );
        assert_eq!(
            hidden(&block),
            [true, true, true, false, false, false, false, false]
        );
    }

    #[test]
    fn rustdoc_bare_marker_on_last_line_keeps_an_empty_hidden_line() {
        let block = prepare(&book_defaults(), "rust,ignore", "x\n#\n").unwrap();
        assert_eq!(block.source, "x\n");
        assert_eq!(hidden(&block), [false, true]);
        assert_eq!(block.decorate("x\n"), "x\n<span class=\"boring\"></span>");
    }

    #[test]
    fn rustdoc_drops_the_final_newline_like_mdbook() {
        let block = prepare(&book_defaults(), "rust,ignore", "a\nb\n").unwrap();
        assert_eq!(block.source, "a\nb");
        assert_eq!(block.decorate("<i>a</i>\nb\n"), "<i>a</i>\nb");
    }

    #[test]
    fn prefix_syntax_keeps_the_final_newline() {
        let block = prepare(&book_defaults(), "typescript,hidelines=~", "~a\nb\n").unwrap();
        assert_eq!(block.source, "a\nb\n");
        assert_eq!(
            block.decorate("a\nb\n"),
            "<span class=\"boring\">a\n</span>b\n"
        );
    }

    #[test]
    fn hidden_prefix_from_book_config() {
        let python = NativeFeatures {
            hidelines: HashMap::from([("python".into(), "~".into())]),
            ..book_defaults()
        };
        let block = prepare(&python, "python", "~import os\n  ~x = 1\nprint(1)\n").unwrap();
        assert!(block.pre_classes.is_empty());
        assert_eq!(block.source, "import os\n  x = 1\nprint(1)\n");
        assert_eq!(hidden(&block), [true, true, false]);
    }

    #[test]
    fn hidden_prefix_annotation_overrides_book_config() {
        let python = NativeFeatures {
            hidelines: HashMap::from([("python".into(), "~".into())]),
            ..book_defaults()
        };
        let block = prepare(&python, "python,hidelines=!!", "!!import os\n~x = 1\n").unwrap();
        assert_eq!(block.code_classes, ["hidelines=!!"]);
        assert_eq!(block.source, "import os\n~x = 1\n");
        assert_eq!(hidden(&block), [true, false]);
    }

    #[test]
    fn languages_without_hidden_syntax_are_unchanged() {
        let block = prepare(&book_defaults(), "python", "# comment\nx = 1\n").unwrap();
        assert_eq!(block.source, "# comment\nx = 1\n");
        assert_eq!(hidden(&block), [false, false]);
    }

    #[test]
    fn hidden_lines_wrap_whole_highlighted_lines() {
        let block = prepare(&book_defaults(), "rust,ignore", "# a\nb\n# c").unwrap();
        assert_eq!(
            block.decorate("<i>a</i>\n<i>b</i>\nc"),
            "<span class=\"boring\"><i>a</i>\n</span><i>b</i>\n<span class=\"boring\">c</span>"
        );
    }
}
