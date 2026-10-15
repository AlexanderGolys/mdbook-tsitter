//! A grammar repository's own description of itself: `tree-sitter.json`,
//! which lists each grammar's subdirectory and query files, and `package.json`,
//! which pins the grammars its queries build on. Query files of such a
//! dependency are named by their npm path, e.g.
//! `node_modules/tree-sitter-javascript/queries/highlights.scm`.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use semver::Version;
use serde::Deserialize;

/// The repository's grammar manifest.
const MANIFEST_FILE: &str = "tree-sitter.json";
/// The repository's npm package description.
const PACKAGE_FILE: &str = "package.json";
/// Directory prefix of query files that belong to a dependency grammar.
const DEPENDENCY_DIR: &str = "node_modules/";

/// The grammars a repository declares in `tree-sitter.json`.
#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub grammars: Vec<ManifestGrammar>,
}

/// One grammar of a [`Manifest`]. Query paths are relative to the repository
/// root and listed in priority order.
#[derive(Debug, Clone, Deserialize)]
pub struct ManifestGrammar {
    pub name: String,
    /// The grammar's subdirectory; the repository root when absent or `.`.
    path: Option<String>,
    #[serde(default)]
    highlights: QueryFiles,
    #[serde(default)]
    injections: QueryFiles,
    #[serde(default)]
    locals: QueryFiles,
}

/// A query given as one path or as a list of paths.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(untagged)]
enum QueryFiles {
    #[default]
    None,
    One(String),
    Many(Vec<String>),
}

impl Manifest {
    /// Read `tree-sitter.json` from a repository root, if it has one.
    pub fn read(repo: &Path) -> Result<Option<Self>> {
        let path = repo.join(MANIFEST_FILE);
        if !path.is_file() {
            return Ok(None);
        }
        let text =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_str(&text)
            .map(Some)
            .with_context(|| format!("parsing {}", path.display()))
    }

    /// The grammar in subdirectory `location`, or, without one, the grammar
    /// called `name` or the repository's only grammar.
    pub fn grammar(&self, name: &str, location: Option<&str>) -> Option<&ManifestGrammar> {
        match location {
            Some(location) => self
                .grammars
                .iter()
                .find(|grammar| grammar.location() == Some(location.trim_matches('/'))),
            None => self
                .grammars
                .iter()
                .find(|grammar| grammar.name == name.replace('-', "_"))
                .or(match self.grammars.as_slice() {
                    [only] => Some(only),
                    _ => None,
                }),
        }
    }
}

impl ManifestGrammar {
    /// The grammar's subdirectory, or `None` when it is the repository root.
    pub fn location(&self) -> Option<&str> {
        let path = self.path.as_deref()?.trim_start_matches("./");
        let path = path.trim_matches('/');
        (!path.is_empty() && path != ".").then_some(path)
    }

    /// The query files of `kind` (`highlights`, `injections` or `locals`).
    pub fn queries(&self, kind: &str) -> &[String] {
        let files = match kind {
            "highlights" => &self.highlights,
            "injections" => &self.injections,
            "locals" => &self.locals,
            _ => &QueryFiles::None,
        };
        match files {
            QueryFiles::None => &[],
            QueryFiles::One(file) => std::slice::from_ref(file),
            QueryFiles::Many(files) => files,
        }
    }
}

/// A query file that lives in a dependency grammar's repository.
#[derive(Debug, PartialEq)]
pub struct DependencyQuery<'a> {
    /// The dependency's npm package, e.g. `tree-sitter-javascript`.
    pub package: &'a str,
    /// The file's path within that package, e.g. `queries/highlights.scm`.
    pub file: &'a str,
}

impl<'a> DependencyQuery<'a> {
    /// Split a manifest query path that points into `node_modules`.
    pub fn parse(path: &'a str) -> Option<Self> {
        let (package, file) = path.strip_prefix(DEPENDENCY_DIR)?.split_once('/')?;
        Some(Self { package, file })
    }

    /// The language name the dependency is known by, e.g. `javascript`.
    pub fn language(&self) -> &'a str {
        self.package
            .strip_prefix("tree-sitter-")
            .unwrap_or(self.package)
    }
}

#[derive(Deserialize)]
struct Package {
    #[serde(default)]
    dependencies: HashMap<String, String>,
    #[serde(default, rename = "devDependencies")]
    dev_dependencies: HashMap<String, String>,
    #[serde(default, rename = "peerDependencies")]
    peer_dependencies: HashMap<String, String>,
}

/// The release tag (`v0.23.1`) of the version of `package` that the
/// repository's `package.json` depends on, when it names a single version.
pub fn dependency_tag(repo: &Path, package: &str) -> Option<String> {
    let text = fs::read_to_string(repo.join(PACKAGE_FILE)).ok()?;
    let manifest: Package = serde_json::from_str(&text).ok()?;
    let range = [
        &manifest.dependencies,
        &manifest.dev_dependencies,
        &manifest.peer_dependencies,
    ]
    .into_iter()
    .find_map(|dependencies| dependencies.get(package))?;
    release_tag(range)
}

/// `^0.23.1` → `v0.23.1`; ranges that do not pin one version give `None`.
fn release_tag(range: &str) -> Option<String> {
    let version = range.trim().trim_start_matches(['^', '~', '=', 'v']);
    Version::parse(version)
        .ok()
        .map(|version| format!("v{version}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(json: &str) -> Manifest {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn picks_grammar_by_location_name_or_as_only_one() {
        let multi = manifest(
            r#"{ "grammars": [
                { "name": "typescript", "path": "typescript" },
                { "name": "tsx", "path": "tsx" } ] }"#,
        );
        assert_eq!(multi.grammar("ts", Some("tsx/")).unwrap().name, "tsx");
        assert_eq!(
            multi.grammar("typescript", None).unwrap().location(),
            Some("typescript")
        );
        assert!(multi.grammar("ts", None).is_none());

        let single = manifest(r#"{ "grammars": [ { "name": "rust", "path": "." } ] }"#);
        let rust = single.grammar("rs", None).unwrap();
        assert_eq!(rust.location(), None);
    }

    #[test]
    fn queries_accept_one_path_or_a_list() {
        let grammar = &manifest(
            r#"{ "grammars": [ { "name": "x",
                "highlights": ["queries/highlights.scm", "node_modules/tree-sitter-y/queries/highlights.scm"],
                "injections": "queries/injections.scm" } ] }"#,
        )
        .grammars[0];
        assert_eq!(grammar.queries("highlights").len(), 2);
        assert_eq!(grammar.queries("injections"), ["queries/injections.scm"]);
        assert!(grammar.queries("locals").is_empty());
    }

    #[test]
    fn dependency_queries_name_their_package() {
        let query =
            DependencyQuery::parse("node_modules/tree-sitter-javascript/queries/highlights.scm")
                .unwrap();
        assert_eq!(query.package, "tree-sitter-javascript");
        assert_eq!(query.file, "queries/highlights.scm");
        assert_eq!(query.language(), "javascript");
        assert!(DependencyQuery::parse("queries/highlights.scm").is_none());
    }

    #[test]
    fn release_tags_need_a_single_version() {
        assert_eq!(release_tag("^0.23.1").as_deref(), Some("v0.23.1"));
        assert_eq!(release_tag("0.21.0").as_deref(), Some("v0.21.0"));
        assert_eq!(release_tag(">=0.20 <0.22"), None);
        assert_eq!(release_tag("*"), None);
    }
}
