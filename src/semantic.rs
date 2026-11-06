//! Hand-written semantic tokens. Tree-sitter sees only syntax, so a block can
//! mark what a language server knows on top of it. In a block that opts in,
//! `§` is the marker: `§§` is a literal `§`, and a single `§` opens or closes a
//! token. The text between an opening and a closing `§` is `token|text`, so
//! `enum §enum|E§ { }` renders `E` as an `enum` token, and
//! `§function.defaultLibrary|scan§` renders `scan` as a `function` token with
//! the `defaultLibrary` modifier. A `§` in the original code is doubled, which leaves
//! nothing that could be mistaken for a token; the sign is rare in code. A token
//! gets the class `lsp-<type>` and one `lsp-mod-<modifier>` per modifier,
//! lowercased. The markers are removed before parsing, so the grammar and the
//! copy button see plain code.

use std::ops::Range;

use anyhow::{bail, Result};

use crate::render::sanitize;

/// Prefix of the class of a token's type; a modifier's class adds
/// [`MODIFIER_INFIX`].
const CLASS_PREFIX: &str = "lsp";

/// Sets modifier classes apart from type classes.
const MODIFIER_INFIX: &str = "mod";

/// Opens and closes a token, and written twice stands for itself.
const MARKER: char = '§';

/// Separates a token's type and modifiers from its text.
const TOKEN_SEPARATOR: char = '|';

/// A semantic token written by hand over part of a source.
#[derive(Debug, PartialEq, Eq)]
pub struct Mark {
    pub range: Range<usize>,
    pub token_type: String,
    pub modifiers: Vec<String>,
}

impl Mark {
    /// The `class="…"` attribute for the token.
    pub fn class_attribute(&self) -> Vec<u8> {
        let type_class = format!("{CLASS_PREFIX}-{}", sanitize(&self.token_type));
        let modifier_classes = self
            .modifiers
            .iter()
            .map(|modifier| format!("{CLASS_PREFIX}-{MODIFIER_INFIX}-{}", sanitize(modifier)));
        let classes: Vec<String> = std::iter::once(type_class)
            .chain(modifier_classes)
            .collect();
        format!("class=\"{}\"", classes.join(" ")).into_bytes()
    }
}

/// Source text with its marks removed, and where they were.
#[derive(Debug, PartialEq, Eq)]
pub struct MarkedSource {
    pub source: String,
    /// In source order and disjoint.
    pub marks: Vec<Mark>,
}

/// Remove the markers from `text`. Fails when a `§` is left open, or a token
/// has no valid `type.modifier|` prefix.
pub fn strip_marks(text: &str) -> Result<MarkedSource> {
    let mut source = String::with_capacity(text.len());
    let mut marks = Vec::new();
    // The `token|text` being read between two markers.
    let mut token_text: Option<String> = None;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        let target = token_text.as_mut().unwrap_or(&mut source);
        if c != MARKER {
            target.push(c);
        } else if chars.next_if_eq(&MARKER).is_some() {
            target.push(MARKER);
        } else if let Some(closed) = token_text.take() {
            let start = source.len();
            let mark = parse_token_text(&closed)?;
            source.push_str(mark.1);
            marks.push(Mark {
                range: start..source.len(),
                token_type: mark.0 .0,
                modifiers: mark.0 .1,
            });
        } else {
            token_text = Some(String::new());
        }
    }
    if let Some(open) = token_text {
        bail!(
            "`{MARKER}{open}` is never closed; write `{MARKER}{MARKER}` for a literal `{MARKER}`"
        );
    }
    Ok(MarkedSource { source, marks })
}

/// The token type and modifiers, and the text, of `token|text`.
fn parse_token_text(token_text: &str) -> Result<((String, Vec<String>), &str)> {
    let Some((token, text)) = token_text.split_once(TOKEN_SEPARATOR) else {
        bail!("`{MARKER}{token_text}{MARKER}` needs a token before `{TOKEN_SEPARATOR}`, as in `{MARKER}function|print{MARKER}`");
    };
    let valid = token.split('.').all(|name| {
        !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    });
    if !valid {
        bail!("`{token}` is not a token: expected a type and modifiers separated by `.`");
    }
    let mut names = token.split('.').map(str::to_string);
    let token_type = names.next().unwrap_or_default();
    Ok(((token_type, names.collect()), text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_removed_and_located() {
        let marked = strip_marks("enum §enum|E§ { .. }").unwrap();
        assert_eq!(marked.source, "enum E { .. }");
        assert_eq!(
            marked.marks,
            [Mark {
                range: 5..6,
                token_type: "enum".into(),
                modifiers: vec![]
            }]
        );
    }

    #[test]
    fn several_tokens_on_a_line() {
        let marked = strip_marks("§a|x§ + §b.c|yy§").unwrap();
        assert_eq!(marked.source, "x + yy");
        assert_eq!(marked.marks[1].range, 4..6);
        assert_eq!(marked.marks[1].modifiers, ["c"]);
    }

    #[test]
    fn dollars_and_other_code_pass_through() {
        let marked = strip_marks("echo ${HOME}$$ | $(pwd) §a|x§").unwrap();
        assert_eq!(marked.source, "echo ${HOME}$$ | $(pwd) x");
        assert_eq!(marked.marks[0].range, 24..25);
    }

    #[test]
    fn doubled_markers_are_the_original_code() {
        let marked = strip_marks("a §§ b §t|§§x§§y§").unwrap();
        assert_eq!(marked.source, "a § b §x§y");
        assert_eq!(marked.marks[0].range, 7..13);
        assert_eq!(strip_marks("§§§§").unwrap().source, "§§");
    }

    #[test]
    fn a_token_has_a_class_for_its_type_and_one_for_each_modifier() {
        let marked = strip_marks("§function.defaultLibrary.async|scan§").unwrap();
        let attribute = String::from_utf8(marked.marks[0].class_attribute()).unwrap();
        assert_eq!(
            attribute,
            "class=\"lsp-function lsp-mod-defaultlibrary lsp-mod-async\""
        );
    }

    #[test]
    fn malformed_tokens_are_errors() {
        for text in ["§a|b", "a § b", "§ab§", "§|b§", "§a b|c§", "§a..b|c§"] {
            assert!(strip_marks(text).is_err(), "{text}");
        }
    }
}
