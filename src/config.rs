//! The `[preprocessor.tsitter]` section of `book.toml`.
//!
//! ```toml
//! [preprocessor.tsitter]
//!
//! # A language from the built-in grammar list needs no source: its pinned
//! # grammar is fetched and compiled on first build.
//! [preprocessor.tsitter.languages]
//! rust = {}
//! typescript = { aliases = ["ts"] }
//!
//! # Any other grammar is named by its repository URL or local path.
//! [preprocessor.tsitter.languages.mylang]
//! git = "https://github.com/someone/tree-sitter-mylang"
//! # rev = "v1.0.0"                             # optional; tag, branch or commit
//! # location = "mylang"                        # optional; grammar subdirectory
//!
//! # Deprecated: point at a compiled parser and a highlights query.
//! [preprocessor.tsitter.languages.lua]
//! library = "parsers/libtree-sitter-lua.so"   # compiled grammar (relative to book root)
//! highlights = "queries/lua/highlights.scm"    # tree-sitter highlights query
//! # symbol = "tree_sitter_lua"                 # optional; defaults to tree_sitter_<name>
//! # injections = "queries/lua/injections.scm"  # optional
//! # locals = "queries/lua/locals.scm"          # optional
//! # aliases = ["lua"]                           # fence tags; defaults to the table key
//! ```
//!
//! Query paths given for a fetched grammar override the repository's own.

use std::collections::HashMap;

use anyhow::{Context, Result};
use mdbook_preprocessor::PreprocessorContext;
use serde::Deserialize;

/// Parsed preprocessor configuration. Unknown keys (e.g. mdBook's own
/// `command`, `renderers`) are ignored.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    /// The languages to highlight, keyed by language name.
    pub languages: HashMap<String, LanguageConfig>,
    /// Whether languages embedded in a block (via a grammar's injections query)
    /// are highlighted with their own grammar. Only languages already
    /// configured here are ever used — injection never loads a new grammar — so
    /// an embedded language that is not configured is simply left as-is. Set
    /// `inject = false` to switch injection off entirely, e.g. if an injected
    /// grammar misbehaves.
    pub inject: bool,
    /// Whether the bundled stylesheet is embedded in every chapter with a
    /// highlighted block. It sits in a cascade layer, so any `additional-css`
    /// rule overrides it; set `default-css = false` only to drop it entirely,
    /// e.g. for a stylesheet that replaces it.
    #[serde(rename = "default-css")]
    pub default_css: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            languages: HashMap::new(),
            inject: true,
            default_css: true,
        }
    }
}

/// One dynamically loaded grammar, taken from a compiled parser on disk
/// (`library`), a git repository (`git`), or the built-in grammar list.
#[derive(Debug, Deserialize)]
pub struct LanguageConfig {
    /// URL of the grammar's git repository, fetched and compiled on demand;
    /// defaults to the built-in list's repository for the language.
    pub git: Option<String>,
    /// Tag, branch or commit of `git` to use; defaults to the built-in list's
    /// pinned revision, or for other repositories their default branch.
    pub rev: Option<String>,
    /// Subdirectory of `git` holding the grammar, for repositories that
    /// contain several grammars; found from the repository's
    /// `tree-sitter.json` or the built-in list when not given.
    pub location: Option<String>,
    /// Path to the compiled parser shared object (`.so` on Linux, `.dylib` on
    /// macOS, `.dll` on Windows). Deprecated in favour of the built-in list
    /// and `git`; a build using it prints a warning.
    pub library: Option<String>,
    /// The parser's exported constructor symbol in `library`; defaults to
    /// `tree_sitter_<name>` (with `-` mapped to `_`). Deprecated with
    /// `library`.
    pub symbol: Option<String>,
    /// Path to the highlights query (`highlights.scm`); required with
    /// `library`, defaults to the fetched grammar's own.
    pub highlights: Option<String>,
    /// Optional injections query for embedded languages.
    pub injections: Option<String>,
    /// Optional locals query for scope-aware highlighting.
    pub locals: Option<String>,
    /// Code-fence tags this grammar handles; defaults to the table key.
    #[serde(default)]
    pub aliases: Vec<String>,
}

impl Config {
    /// Read the preprocessor's table out of the book context, falling back to
    /// an empty configuration when the section is absent.
    pub fn from_context(ctx: &PreprocessorContext) -> Result<Self> {
        ctx.config
            .get::<Self>("preprocessor.tsitter")
            .context("invalid [preprocessor.tsitter] configuration")
            .map(Option::unwrap_or_default)
    }
}
