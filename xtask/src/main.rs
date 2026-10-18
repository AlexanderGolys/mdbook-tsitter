//! Repository maintenance tasks, run with `cargo xtask <task>` on any
//! platform; they need only Cargo and `git`.
//!
//! `update-grammar-list [nvim-treesitter commit]` regenerates
//! `assets/grammars.json`, the built-in list of known grammar repositories,
//! from nvim-treesitter's parser list at the tip of its main branch, or at the
//! given commit. Each entry keeps the repository URL, the pinned revision, and
//! the grammar subdirectory when there is one. The list also records where
//! nvim-treesitter's own queries for these revisions live, used for grammars
//! that ship none.

mod lua_table;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::{env, fs, process};

use lua_table::Table;

const NVIM_TREESITTER: &str = "https://github.com/nvim-treesitter/nvim-treesitter";
const DEFAULT_BRANCH: &str = "main";
/// The parser list in nvim-treesitter: a Lua file returning a table literal.
const PARSER_LIST: &str = "lua/nvim-treesitter/parsers.lua";
/// The directory in nvim-treesitter holding one query directory per language.
const QUERY_DIRECTORY: &str = "runtime/queries";
const USAGE: &str = "usage: cargo xtask update-grammar-list [nvim-treesitter commit]";

type Result<T> = std::result::Result<T, String>;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("update-grammar-list") => update_grammar_list(args.get(1).map(String::as_str)),
        _ => Err(USAGE.to_string()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Where a grammar lives, as nvim-treesitter's `install_info` records it.
struct Grammar {
    url: String,
    revision: String,
    location: Option<String>,
}

fn update_grammar_list(revision: Option<&str>) -> Result<()> {
    let checkout = env::temp_dir().join(format!("mdbook-tsitter-xtask-{}", process::id()));
    let fetched = fetch_parser_list(&checkout, revision.unwrap_or(DEFAULT_BRANCH));
    let _ = fs::remove_dir_all(&checkout);
    let (commit, source) = fetched?;

    let grammars = grammars(&lua_table::parse_returned_table(&source)?)?;
    let output = grammar_list_path();
    fs::write(&output, render(&commit, &grammars))
        .map_err(|error| format!("writing {}: {error}", output.display()))?;
    println!(
        "wrote {} grammars from {NVIM_TREESITTER}@{commit} to {}",
        grammars.len(),
        output.display()
    );
    Ok(())
}

fn grammar_list_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/grammars.json")
}

/// Shallow-fetch nvim-treesitter at `revision` (a branch or a commit) into
/// `checkout`, returning the commit and the parser list's source.
fn fetch_parser_list(checkout: &Path, revision: &str) -> Result<(String, String)> {
    let dir = checkout
        .to_str()
        .ok_or("the temporary directory's path is not UTF-8")?;
    git(&["init", "--quiet", dir])?;
    git(&[
        "-C",
        dir,
        "fetch",
        "--quiet",
        "--depth",
        "1",
        NVIM_TREESITTER,
        revision,
    ])?;
    let commit = git(&["-C", dir, "rev-parse", "FETCH_HEAD"])?
        .trim()
        .to_string();
    let source = git(&["-C", dir, "show", &format!("FETCH_HEAD:{PARSER_LIST}")])?;
    Ok((commit, source))
}

fn git(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|error| format!("running git: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| format!("git output: {error}"))
}

/// The grammars in the parser list that have a repository, by name; entries
/// without one only carry queries.
fn grammars(parsers: &Table) -> Result<BTreeMap<String, Grammar>> {
    let mut grammars = BTreeMap::new();
    for (name, parser) in &parsers.named {
        let Some(install_info) = parser.as_table().and_then(|p| p.field("install_info")) else {
            continue;
        };
        let Some(install_info) = install_info.as_table() else {
            return Err(format!("{name}: install_info is not a table"));
        };
        let Some(url) = install_info.string("url") else {
            continue;
        };
        let revision = install_info
            .string("revision")
            .ok_or_else(|| format!("{name}: no revision"))?;
        grammars.insert(
            name.clone(),
            Grammar {
                url: url.to_string(),
                revision: revision.to_string(),
                location: install_info.string("location").map(str::to_string),
            },
        );
    }
    Ok(grammars)
}

/// The grammar list as JSON, one grammar per line.
fn render(commit: &str, grammars: &BTreeMap<String, Grammar>) -> String {
    let mut lines = vec![
        "{".to_string(),
        format!(
            "  \"queries\": {{ \"url\": {}, \"revision\": {}, \"directory\": {} }},",
            quote(NVIM_TREESITTER),
            quote(commit),
            quote(QUERY_DIRECTORY)
        ),
        "  \"grammars\": {".to_string(),
    ];
    for (index, (name, grammar)) in grammars.iter().enumerate() {
        let mut fields = vec![
            format!("\"url\": {}", quote(&grammar.url)),
            format!("\"revision\": {}", quote(&grammar.revision)),
        ];
        if let Some(location) = &grammar.location {
            fields.push(format!("\"location\": {}", quote(location)));
        }
        let separator = if index + 1 < grammars.len() { "," } else { "" };
        lines.push(format!(
            "    {}: {{ {} }}{separator}",
            quote(name),
            fields.join(", ")
        ));
    }
    lines.push("  }".to_string());
    lines.push("}".to_string());
    lines.join("\n") + "\n"
}

/// `text` as a JSON string.
fn quote(text: &str) -> String {
    let mut quoted = String::from("\"");
    for c in text.chars() {
        if c.is_control() || c == '"' || c == '\\' {
            quoted.push_str(&format!("\\u{:04x}", c as u32));
        } else {
            quoted.push(c);
        }
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;

    const PARSERS: &str = "---@type nvim-ts.parsers
return {
  ada = {
    install_info = {
      revision = 'abc',
      url = 'https://example.com/tree-sitter-ada',
    },
    tier = 2,
  },
  apex = {
    install_info = {
      location = 'apex',
      revision = 'def',
      url = 'https://example.com/tree-sitter-sfapex',
    },
    requires = { 'html', 'html_tags' },
  },
  ecma = {
    readme_note = 'queries required by javascript',
  },
}
";

    #[test]
    fn grammars_come_from_entries_with_a_repository() {
        let grammars = grammars(&lua_table::parse_returned_table(PARSERS).unwrap()).unwrap();
        assert_eq!(grammars.keys().collect::<Vec<_>>(), ["ada", "apex"]);
        assert_eq!(grammars["apex"].location.as_deref(), Some("apex"));
        assert_eq!(grammars["ada"].revision, "abc");
    }

    #[test]
    fn rendered_list_has_one_grammar_per_line() {
        let grammars = grammars(&lua_table::parse_returned_table(PARSERS).unwrap()).unwrap();
        let json = render("123", &grammars);
        assert!(json.contains(
            "    \"apex\": { \"url\": \"https://example.com/tree-sitter-sfapex\", \
             \"revision\": \"def\", \"location\": \"apex\" }\n  }"
        ));
        assert!(json.ends_with("}\n"));
    }
}
