//! Differential tests against mdBook itself: every block is rendered twice by a
//! real `mdbook build`, once left to mdBook (`notreesitter`) and once
//! highlighted by this preprocessor. Once highlight spans and our own marker
//! classes are stripped, both renderings must have the same structure: the
//! same `<pre>`/`<code>` classes (playground, edition, annotations), the same
//! hidden-line spans, and the same text the Run and Copy buttons read.
//!
//! Needs `mdbook` on `PATH` (or in `$MDBOOK`) and the parsers committed under
//! `examples/languages`; the test is skipped with a note when either is absent.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const EXAMPLE_BOOK: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/languages");
const PREPROCESSOR: &str = env!("CARGO_BIN_EXE_mdbook-tsitter");
const SKIP_TAG: &str = "notreesitter";
/// Classes only the preprocessor adds, absent from mdBook's own rendering.
const OWN_CLASSES: [&str; 3] = ["treesitter", "no-highlight", SKIP_TAG];
const HIDDEN_LINE_SPAN: &str = "<span class=\"boring\">";
const LANGUAGE_ATTRIBUTE: &str = "data-language";

/// Fenced blocks exercising every native feature: playground eligibility,
/// edition handling, the implicit `fn main`, and hidden-line syntaxes.
const CASES: &[(&str, &str)] = &[
    ("rust", "let x = 1;\n"),
    ("rust", "fn main() {\n    println!(\"hi\");\n}\n"),
    (
        "rust",
        "# use std::fmt;\n# fn main() {\nlet s = \"<&>'\";\n# }\n",
    ),
    ("rust", "#![allow(dead_code)]\n\nstruct Unused;\n"),
    ("rust", "x();\n#\n  # indented();\n#\tnot_hidden();\n"),
    ("rust", "/* a\n# b */\nlet c = 1;\n"),
    ("rust,ignore", "visible();\n#\n"),
    (
        "rust,ignore",
        "# hidden();\n## shown\n#[derive(Debug)]\nstruct S;\n",
    ),
    ("rust,noplayground", "# hidden();\nlet x = 1;\n"),
    ("rust,noplaypen", "let x = 1;\n"),
    ("rust,mdbook-runnable", "let x = 1;\n"),
    ("rust,edition2015", "let x = 1;\n"),
    ("rust,should_panic,no_run", "panic!();\n"),
    ("rust,editable", "fn main() {}\n"),
    ("rust,editable,ignore", "fn main() {}\n"),
    ("rust,hidelines=~", "~not_a_marker();\n# hidden();\n"),
    ("rust compile_fail", "let x: u8 = 256;\n"),
    ("typescript", "const a: number = 1;\n"),
    ("typescript", "~import x from \"y\";\nconst a = x;\n"),
    (
        "typescript,hidelines=!!",
        "!!import x from \"y\";\n  !!const b = 2;\nconst a = x;\n",
    ),
];

/// Book configurations the cases are rendered under.
const BOOK_CONFIGS: &[(&str, &str)] = &[
    ("defaults", ""),
    ("edition", "[rust]\nedition = \"2021\"\n"),
    (
        "not-runnable",
        "[rust]\nedition = \"2018\"\n[output.html.playground]\nrunnable = false\n",
    ),
    (
        "editable",
        "[rust]\nedition = \"2024\"\n[output.html.playground]\neditable = true\n",
    ),
    (
        "hidelines",
        "[output.html.code.hidelines]\ntypescript = \"~\"\n",
    ),
];

#[test]
fn highlighted_blocks_match_mdbook_structure() {
    let Some(mdbook) = prerequisites() else {
        return;
    };

    let mut failures = Vec::new();
    for (config_name, config) in BOOK_CONFIGS {
        let book = build_book(&mdbook, config_name, config);
        for (index, (fence, _)) in CASES.iter().enumerate() {
            let native = structure(&code_block(&book, &format!("native-{index}")));
            let highlighted = structure(&code_block(&book, &format!("highlighted-{index}")));
            if native != highlighted {
                failures.push(format!(
                    "[{config_name}] ```{fence}\n  mdBook:      {native:?}\n  tree-sitter: {highlighted:?}",
                ));
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}

#[test]
fn highlighted_blocks_are_ours_alone() {
    let Some(mdbook) = prerequisites() else {
        return;
    };
    // With the default config no block is editable, so every case is ours.
    let book = build_book(&mdbook, "highlight-check", "");
    for (index, (fence, _)) in CASES.iter().enumerate() {
        let html = code_block(&book, &format!("highlighted-{index}"));
        assert!(
            html.contains("class=\"ts-"),
            "```{fence} was not highlighted: {html}"
        );
        let (pre_tag, code_tag, _) = split_block(&html);
        for tag in [pre_tag, code_tag] {
            let class = attribute(tag, "class").unwrap_or_default();
            assert!(
                !names_highlightjs_language(class),
                "```{fence} would be re-highlighted by highlight.js: {tag}"
            );
        }
    }
}

/// Whether a `class` value contains what highlight.js reads as a language
/// request — `lang-…` or `language-…` after a word boundary — which it honours
/// before `no-highlight`.
fn names_highlightjs_language(class: &str) -> bool {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    ["lang-", "language-"].iter().any(|marker| {
        class
            .match_indices(marker)
            .any(|(at, _)| !class[..at].ends_with(is_word))
    })
}

/// The parts of a rendered code block that mdBook's features depend on.
#[derive(Debug, PartialEq)]
struct BlockStructure {
    pre_classes: BTreeSet<String>,
    code_classes: BTreeSet<String>,
    /// The code text with hidden lines bracketed as `⟦…⟧`.
    text: String,
}

fn structure(block_html: &str) -> BlockStructure {
    let (pre_tag, code_tag, code_html) = split_block(block_html);
    let mut code_classes = classes(code_tag);
    // We carry the fence tag as `data-language`; mdBook as a `language-*` class.
    if let Some(lang) = attribute(code_tag, LANGUAGE_ATTRIBUTE) {
        code_classes.insert(format!("language-{lang}"));
    }
    BlockStructure {
        pre_classes: classes(pre_tag),
        code_classes,
        text: text_with_hidden_lines(code_html),
    }
}

/// A `<pre><code>…</code></pre>` element as its `<pre>` tag, its `<code>` tag,
/// and the HTML inside `<code>`.
fn split_block(block_html: &str) -> (&str, &str, &str) {
    let pre_tag = &block_html[..block_html.find('>').unwrap() + 1];
    let code_start = block_html.find("<code").unwrap();
    let code_open_end = code_start + block_html[code_start..].find('>').unwrap() + 1;
    let code_end = block_html.rfind("</code>").unwrap();
    (
        pre_tag,
        &block_html[code_start..code_open_end],
        &block_html[code_open_end..code_end],
    )
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!(" {name}=\"");
    let value = &tag[tag.find(&marker)? + marker.len()..];
    Some(&value[..value.find('"').unwrap()])
}

fn classes(tag: &str) -> BTreeSet<String> {
    attribute(tag, "class")
        .unwrap_or_default()
        .split_whitespace()
        .filter(|class| !OWN_CLASSES.contains(class))
        .map(str::to_string)
        .collect()
}

/// Strip every tag except hidden-line spans, which become `⟦…⟧`, and decode
/// entities, leaving the text a reader (or the playground) sees.
fn text_with_hidden_lines(html: &str) -> String {
    let mut text = String::new();
    let mut open_spans: Vec<bool> = Vec::new();
    let mut rest = html;
    while let Some(tag_start) = rest.find('<') {
        text.push_str(&decode_entities(&rest[..tag_start]));
        let tag_end = tag_start + rest[tag_start..].find('>').unwrap() + 1;
        let tag = &rest[tag_start..tag_end];
        if tag.starts_with("</") {
            if open_spans.pop() == Some(true) {
                text.push('⟧');
            }
        } else if !tag.ends_with("/>") {
            let hidden = tag == HIDDEN_LINE_SPAN;
            if hidden {
                text.push('⟦');
            }
            open_spans.push(hidden);
        }
        rest = &rest[tag_end..];
    }
    text.push_str(&decode_entities(rest));
    text
}

fn decode_entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&amp;", "&")
}

/// Render every case twice (natively and highlighted) under one book config,
/// returning the output directory.
fn build_book(mdbook: &Path, name: &str, extra_config: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("parity-{name}"));
    let _ = fs::remove_dir_all(&root);
    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();

    let mut summary = String::from("# Summary\n\n");
    for (index, (fence, code)) in CASES.iter().enumerate() {
        let native_fence = format!("{fence},{SKIP_TAG}");
        for (chapter, fence) in [("native", native_fence.as_str()), ("highlighted", fence)] {
            let file = format!("{chapter}-{index}.md");
            fs::write(
                src.join(&file),
                format!("# Case\n\n```{fence}\n{code}```\n"),
            )
            .unwrap();
            summary.push_str(&format!("- [{chapter} {index}]({file})\n"));
        }
    }
    fs::write(src.join("SUMMARY.md"), summary).unwrap();

    let example = Path::new(EXAMPLE_BOOK);
    let language = |name: &str| {
        format!(
            "[preprocessor.tsitter.languages.{name}]\nlibrary = {:?}\nhighlights = {:?}\n",
            example.join(format!("parsers/{name}.so")),
            example.join(format!("queries/{name}/highlights.scm")),
        )
    };
    fs::write(
        root.join("book.toml"),
        format!(
            "[book]\ntitle = \"parity\"\n\n[preprocessor.tsitter]\ncommand = {PREPROCESSOR:?}\n\n{}\n{}\n{extra_config}",
            language("rust"),
            language("typescript"),
        ),
    )
    .unwrap();

    let output = Command::new(mdbook)
        .arg("build")
        .arg(&root)
        .output()
        .expect("running mdbook");
    assert!(
        output.status.success(),
        "mdbook build failed for `{name}`:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    root.join("book")
}

/// The first `<pre>…</pre>` element of a rendered chapter.
fn code_block(book: &Path, chapter: &str) -> String {
    let html = fs::read_to_string(book.join(format!("{chapter}.html"))).unwrap();
    let start = html.find("<pre").unwrap();
    let end = start + html[start..].find("</pre>").unwrap() + "</pre>".len();
    html[start..end].to_string()
}

/// The `mdbook` binary, or `None` (with a note) when the test cannot run.
fn prerequisites() -> Option<PathBuf> {
    let Some(mdbook) = mdbook_binary() else {
        eprintln!("skipping: `mdbook` not found on PATH or in $MDBOOK");
        return None;
    };
    let parsers = Path::new(EXAMPLE_BOOK).join("parsers");
    if !["rust", "typescript"]
        .iter()
        .all(|lang| parsers.join(format!("{lang}.so")).exists())
    {
        eprintln!(
            "skipping: example parsers not found in {}",
            parsers.display()
        );
        return None;
    }
    Some(mdbook)
}

fn mdbook_binary() -> Option<PathBuf> {
    if let Some(path) = env::var_os("MDBOOK") {
        return Some(path.into());
    }
    env::split_paths(&env::var_os("PATH")?)
        .map(|dir| dir.join("mdbook"))
        .find(|candidate| candidate.is_file())
}
