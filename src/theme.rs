//! The stylesheet embedded in highlighted chapters. Like mdBook's own theme
//! files, it is replaced from the book's theme directory: `highlight_ts.css`
//! takes the bundled stylesheet's place, as `highlight.css` takes the place of
//! mdBook's highlight.js theme. Without it, colours from the highlight.js
//! stylesheets a book replaces are carried over, so a theme that only styles
//! highlight.js colours tree-sitter blocks to match.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::DEFAULT_CSS;

/// Everforest's book colours and tree-sitter capture styles.
pub const EVERFOREST_CSS: &str = include_str!("../assets/everforest.css");

/// Theme-picker registration and dark editor support for Everforest.
const EVERFOREST_JS: &str = include_str!("../assets/everforest.js");

/// Embed the complete book theme outside the highlighting cascade layer so
/// its interface variables override mdBook's unlayered built-in colours.
pub fn everforest_assets() -> String {
    format!("\n\n<style>\n{EVERFOREST_CSS}</style>\n<script>\n{EVERFOREST_JS}</script>\n")
}

/// File in the theme directory that replaces the bundled stylesheet.
pub const THEME_STYLESHEET: &str = "highlight_ts.css";

/// mdBook's highlight.js stylesheets, each with a selector for the mdBook
/// themes it is active under (book.js enables one per theme).
const HIGHLIGHTJS_STYLESHEETS: [(&str, &str); 3] = [
    ("highlight.css", ":root:not(.coal):not(.navy):not(.ayu)"),
    ("tomorrow-night.css", ".coal, .navy"),
    ("ayu-highlight.css", ".ayu"),
];

/// The bundled stylesheet's colour roles, each with the highlight.js
/// selectors whose colour it takes, in order of preference.
const ROLE_SOURCES: [(&str, &[&str]); 13] = [
    ("--ts-comment", &[".hljs-comment"]),
    ("--ts-keyword", &[".hljs-keyword"]),
    ("--ts-string", &[".hljs-string"]),
    (
        "--ts-escape",
        &[".hljs-char.escape_", ".hljs-regexp", ".hljs-string"],
    ),
    ("--ts-number", &[".hljs-number", ".hljs-literal"]),
    (
        "--ts-function",
        &[".hljs-title.function_", ".hljs-function", ".hljs-title"],
    ),
    (
        "--ts-declaration",
        &[".hljs-title.function_", ".hljs-title"],
    ),
    (
        "--ts-type",
        &[".hljs-type", ".hljs-title.class_", ".hljs-built_in"],
    ),
    ("--ts-special", &[".hljs-variable", ".hljs-built_in"]),
    (
        "--ts-namespace",
        &[".hljs-title.class_", ".hljs-type", ".hljs-keyword"],
    ),
    (
        "--ts-attribute",
        &[".hljs-attribute", ".hljs-attr", ".hljs-meta"],
    ),
    ("--ts-heading", &[".hljs-section", ".hljs-title"]),
    ("--ts-link", &[".hljs-link", ".hljs-string"]),
];

/// The highlighting stylesheet for a book whose theme directory is `theme_dir`.
pub fn stylesheet(theme_dir: &Path) -> Result<String> {
    let replacement = theme_dir.join(THEME_STYLESHEET);
    if replacement.is_file() {
        return read(&replacement);
    }
    let mut css = DEFAULT_CSS.to_string();
    for (file, themes) in HIGHLIGHTJS_STYLESHEETS {
        let path = theme_dir.join(file);
        if !path.is_file() {
            continue;
        }
        let roles = roles_from_highlightjs(&read(&path)?);
        if !roles.is_empty() {
            css.push_str(&format!(
                "\n/* Colours from the book's `{file}`. */\n{themes} {{\n{roles}}}\n"
            ));
        }
    }
    Ok(css)
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("reading `{}`", path.display()))
}

/// Declarations setting each colour role a highlight.js stylesheet colours.
fn roles_from_highlightjs(css: &str) -> String {
    let colours = selector_colours(css);
    ROLE_SOURCES
        .iter()
        .filter_map(|(role, selectors)| {
            let colour = selectors
                .iter()
                .find_map(|selector| colours.get(*selector))?;
            Some(format!("  {role}: {colour};\n"))
        })
        .collect()
}

/// The `color` a stylesheet's top-level rules give each selector, later rules
/// winning. Selectors are keyed with their whitespace normalised.
fn selector_colours(css: &str) -> HashMap<String, String> {
    let mut colours = HashMap::new();
    for rule in without_comments(css).split('}') {
        let Some((selectors, declarations)) = rule.split_once('{') else {
            continue;
        };
        // At-rules and the rules nested in them are not top-level rules.
        if selectors.contains('@') || declarations.contains('{') {
            continue;
        }
        let Some(colour) = declarations
            .split(';')
            .filter_map(|declaration| declaration.split_once(':'))
            .filter(|(property, _)| property.trim() == "color")
            .map(|(_, value)| value.replace("!important", "").trim().to_string())
            .next_back()
        else {
            continue;
        };
        for selector in selectors.split(',') {
            let selector = selector.split_whitespace().collect::<Vec<_>>().join(" ");
            colours.insert(selector, colour.clone());
        }
    }
    colours
}

fn without_comments(css: &str) -> String {
    let mut text = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        text.push_str(&rest[..start]);
        rest = rest[start..]
            .find("*/")
            .map_or("", |end| &rest[start + end + 2..]);
    }
    text.push_str(rest);
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    const HIGHLIGHT_CSS: &str = "
        /* Comment */
        .hljs-comment, .hljs-quote { color: #575757; }
        .hljs-keyword,
        .hljs-selector-tag { color: #9d00ec; }
        .hljs-string { font-style: normal; color: #008200 !important; }
        .ruby .hljs-title { color: #000; }
        .hljs-title { color: #0030f2; }
        @media (prefers-color-scheme: dark) { .hljs-keyword { color: #fff; } }
        .hljs { background: #f6f7f6; color: #000; }
    ";

    #[test]
    fn roles_take_the_colours_of_their_highlightjs_classes() {
        let roles = roles_from_highlightjs(HIGHLIGHT_CSS);
        assert!(roles.contains("--ts-comment: #575757;"));
        assert!(roles.contains("--ts-keyword: #9d00ec;"));
        assert!(roles.contains("--ts-string: #008200;"));
        assert!(roles.contains("--ts-function: #0030f2;"));
        // A role whose classes the stylesheet does not colour is left out.
        assert!(!roles.contains("--ts-attribute"));
    }

    #[test]
    fn only_top_level_rules_for_the_selector_itself_count() {
        let colours = selector_colours(HIGHLIGHT_CSS);
        assert_eq!(colours[".hljs-keyword"], "#9d00ec");
        assert_eq!(colours[".hljs-title"], "#0030f2");
        assert_eq!(colours[".ruby .hljs-title"], "#000");
    }

    #[test]
    fn theme_stylesheet_replaces_the_bundled_one() {
        let theme_dir = test_theme_dir("replacement");
        fs::write(
            theme_dir.join(THEME_STYLESHEET),
            ".ts-keyword { color: red; }",
        )
        .unwrap();
        fs::write(theme_dir.join("highlight.css"), HIGHLIGHT_CSS).unwrap();
        assert_eq!(
            stylesheet(&theme_dir).unwrap(),
            ".ts-keyword { color: red; }"
        );
    }

    #[test]
    fn highlightjs_colours_apply_to_the_themes_using_that_stylesheet() {
        let theme_dir = test_theme_dir("highlightjs");
        fs::write(theme_dir.join("tomorrow-night.css"), HIGHLIGHT_CSS).unwrap();
        let css = stylesheet(&theme_dir).unwrap();
        let derived = css.strip_prefix(DEFAULT_CSS).unwrap();
        assert!(derived.contains(".coal, .navy {\n  --ts-comment: #575757;"));
        assert!(!derived.contains(":root"));
    }

    #[test]
    fn without_theme_files_the_bundled_stylesheet_is_used() {
        let theme_dir = test_theme_dir("empty");
        assert_eq!(stylesheet(&theme_dir).unwrap(), DEFAULT_CSS);
    }

    fn test_theme_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "mdbook-tsitter-theme-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
