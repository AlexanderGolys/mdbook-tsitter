//! Capture names → CSS classes. Captures are emitted as classes (not inline
//! colours) so themes live in a stylesheet the book ships, exactly like the
//! rest of mdBook's styling. The actual event-to-HTML rendering lives in
//! [`crate::grammar`], where it has access to every grammar for injection.

/// CSS class prefix for every generated span, e.g. `ts-keyword`.
const CLASS_PREFIX: &str = "ts";

/// Build the `class="…"` attribute bytes for each highlight index, given the
/// capture names in index order. A dotted capture yields one class per prefix
/// so a stylesheet can target the broad group or the specific kind, e.g.
/// `keyword.operator` → `class="ts-keyword ts-keyword-operator"`.
///
/// Captures whose name starts with `_` are internal helpers by tree-sitter
/// convention and are not styled, so they get an empty attribute.
pub fn class_attributes(names: &[String]) -> Vec<Vec<u8>> {
    names.iter().map(|name| class_attribute(name)).collect()
}

fn class_attribute(name: &str) -> Vec<u8> {
    if name.starts_with('_') {
        return Vec::new();
    }
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
