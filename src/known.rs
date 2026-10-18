//! The built-in list of known grammar repositories, so a language from it
//! needs no source in `book.toml`. The list is `assets/grammars.json`,
//! generated from nvim-treesitter's parser list by
//! `scripts/update-grammar-list.lua`; every entry is pinned to a revision.
//! It also names nvim-treesitter's queries for exactly those revisions, which
//! known grammars use in preference to their repositories' own.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

/// The embedded grammar list.
const GRAMMAR_LIST: &str = include_str!("../assets/grammars.json");

/// Where a known grammar lives.
#[derive(Debug, Deserialize)]
pub struct KnownGrammar {
    /// URL of the grammar's git repository.
    pub url: String,
    /// The commit the grammar is pinned to.
    pub revision: String,
    /// Subdirectory holding the grammar, in repositories with several.
    pub location: Option<String>,
}

/// Where the pinned queries for known grammars live: one directory per
/// language, holding nvim-treesitter style query files.
#[derive(Debug, Deserialize)]
pub struct QuerySource {
    /// URL of the repository holding the queries.
    pub url: String,
    /// The commit the grammar revisions were pinned at.
    pub revision: String,
    /// Directory of the per-language query directories.
    pub directory: String,
}

#[derive(Deserialize)]
struct GrammarList {
    queries: QuerySource,
    grammars: HashMap<String, KnownGrammar>,
}

fn list() -> &'static GrammarList {
    static LIST: OnceLock<GrammarList> = OnceLock::new();
    LIST.get_or_init(|| serde_json::from_str(GRAMMAR_LIST).expect("assets/grammars.json is valid"))
}

fn grammars() -> &'static HashMap<String, KnownGrammar> {
    &list().grammars
}

/// The pinned queries for known grammars.
pub fn query_source() -> &'static QuerySource {
    &list().queries
}

/// The known grammar for a language name; `-` and `_` are interchangeable,
/// so `c-sharp` finds `c_sharp`.
pub fn lookup(name: &str) -> Option<&'static KnownGrammar> {
    grammars().get(&name.replace('-', "_"))
}

/// Known language names that differ from `name` only in case or separators,
/// e.g. `c_sharp` for `csharp`, sorted.
pub fn similar_names(name: &str) -> Vec<&'static str> {
    let wanted = bare(name);
    let mut names: Vec<&str> = grammars()
        .keys()
        .filter(|known| bare(known) == wanted)
        .map(String::as_str)
        .collect();
    names.sort_unstable();
    names
}

/// `name` lowercased without `-` or `_`.
fn bare(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, '-' | '_'))
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_list_parses_and_is_pinned() {
        assert!(!grammars().is_empty());
        for (name, grammar) in grammars() {
            assert!(
                grammar.url.starts_with("https://"),
                "{name}: {}",
                grammar.url
            );
            assert!(!grammar.revision.is_empty(), "{name} has no revision");
        }
    }

    #[test]
    fn lookup_accepts_either_separator() {
        assert!(lookup("rust").is_some());
        assert_eq!(
            lookup("c-sharp").map(|g| &g.url),
            lookup("c_sharp").map(|g| &g.url)
        );
        assert!(lookup("no-such-language").is_none());
    }

    #[test]
    fn similar_names_ignore_separators_and_case() {
        assert_eq!(similar_names("CSharp"), ["c_sharp"]);
    }
}
