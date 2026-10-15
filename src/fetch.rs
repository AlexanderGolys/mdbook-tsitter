//! Grammars fetched from their git repository: one named by `git` in
//! `book.toml`, or a known language from the built-in list ([`crate::known`]).
//! A shallow checkout of the requested revision is cached per user and its
//! parser is compiled with the platform's C compiler, so a book needs no
//! prebuilt parser of its own.
//!
//! The repository's `tree-sitter.json` ([`crate::manifest`]) locates the
//! grammar and lists its queries, including queries it builds on from other
//! grammars, which are fetched the same way.
//!
//! Cache layout, under the user cache directory (`~/.cache` outside Windows):
//!
//! ```text
//! mdbook-tsitter/grammars/<repo-url>@<rev>/   checkout of the grammar repository
//! mdbook-tsitter/parsers/<repo-url>@<rev>/    compiled parser library
//! ```
//!
//! A checkout is reused for as long as it exists, so an unpinned grammar stays
//! at the commit first fetched; pin `rev` to choose the version explicitly.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use anyhow::{anyhow, bail, Context, Result};
use etcetera::BaseStrategy;
use tree_sitter::Language;
use tree_sitter_loader::{CompileConfig, Loader};

use crate::config::LanguageConfig;
use crate::manifest::{dependency_tag, DependencyQuery, Manifest, ManifestGrammar};
use crate::{known, warn};

/// Directory under the user cache directory that holds every cached grammar.
const CACHE_DIR_NAME: &str = "mdbook-tsitter";
/// Ref fetched when no `rev` is configured: the repository's default branch.
const DEFAULT_REV: &str = "HEAD";
/// Directory of a tree-sitter grammar that holds its generated C sources.
const GRAMMAR_SOURCE_DIR: &str = "src";
/// Generated parser source under [`GRAMMAR_SOURCE_DIR`].
const PARSER_SOURCE: &str = "parser.c";
/// Grammar definition that the parser source is generated from.
const GRAMMAR_DEFINITION: &str = "grammar.js";
/// The tree-sitter CLI, used only to generate a parser a repository omits.
const TREE_SITTER_CLI: &str = "tree-sitter";
/// Directory of a grammar repository that holds its queries.
const QUERY_DIR: &str = "queries";

/// Where a grammar's repository is and which version of it to use.
pub struct GrammarRepo {
    pub url: String,
    /// Tag, branch or commit; the default branch when `None`.
    pub rev: Option<String>,
    /// The grammar's subdirectory, when the repository has several.
    pub location: Option<String>,
    /// The language's name in the built-in list, when this is the listed
    /// repository; its pinned queries then apply.
    pub known_as: Option<String>,
}

impl GrammarRepo {
    /// The repository for language `name`: its configured `git`, or else its
    /// entry in the built-in list. `rev` and `location` override the entry's.
    /// A `git` path to a local repository is resolved against the book
    /// `root`, and its `rev` to a commit, so new commits are picked up.
    pub fn configured(name: &str, cfg: &LanguageConfig, root: &Path) -> Result<Self> {
        let known_name = name.replace('-', "_");
        if let Some(repository) = cfg
            .git
            .as_deref()
            .and_then(|git| local_repository(root, git))
        {
            let rev = cfg.rev.as_deref().unwrap_or(DEFAULT_REV);
            let commit = git(
                &repository,
                &["rev-parse", "--verify", &format!("{rev}^{{commit}}")],
            )
            .with_context(|| format!("resolving `{rev}` in {}", repository.display()))?;
            return Ok(Self {
                url: repository.display().to_string(),
                rev: Some(commit),
                location: cfg.location.clone(),
                known_as: None,
            });
        }
        if let Some(url) = &cfg.git {
            let listed =
                known::lookup(name).is_some_and(|known| repo_id(&known.url) == repo_id(url));
            return Ok(Self {
                url: url.clone(),
                rev: cfg.rev.clone(),
                location: cfg.location.clone(),
                known_as: listed.then_some(known_name),
            });
        }
        let known = known::lookup(name).ok_or_else(|| unknown_language(name))?;
        Ok(Self {
            url: known.url.clone(),
            rev: cfg.rev.clone().or_else(|| Some(known.revision.clone())),
            location: cfg.location.clone().or_else(|| known.location.clone()),
            known_as: Some(known_name),
        })
    }
}

/// The local repository a `git` value names: a path, absolute or relative to
/// the book `root`, to an existing directory. URLs (`scheme://…`, `user@host:…`)
/// are never local.
fn local_repository(root: &Path, git: &str) -> Option<PathBuf> {
    let is_url = git.contains("://")
        || git
            .split_once(':')
            .is_some_and(|(host, _)| host.contains('@'));
    if is_url {
        return None;
    }
    root.join(git)
        .canonicalize()
        .ok()
        .filter(|path| path.is_dir())
}

/// The error for a language with no source that the built-in list lacks.
fn unknown_language(name: &str) -> anyhow::Error {
    let similar = known::similar_names(name);
    let hint = match similar.as_slice() {
        [] => String::new(),
        names => format!(
            " Did you mean {}? Use it as the table key and add `aliases = [\"{name}\"]`.",
            names
                .iter()
                .map(|known| format!("`{known}`"))
                .collect::<Vec<_>>()
                .join(" or ")
        ),
    };
    anyhow!(
        "language `{name}` is not in the built-in grammar list; set `git` to its grammar \
         repository or `library` to a compiled parser.{hint}"
    )
}

/// A grammar's query sources by kind, all from one origin; `None` where it
/// has none.
#[derive(Default)]
pub struct GrammarQueries {
    pub highlights: Option<String>,
    pub injections: Option<String>,
    pub locals: Option<String>,
}

/// A cached checkout of one revision of a repository.
struct Checkout {
    root: PathBuf,
    /// Names the checkout and its compiled parser in the cache.
    key: String,
}

impl Checkout {
    /// Check out `rev` of `url`, reusing a cached checkout.
    fn fetch(url: &str, rev: Option<&str>) -> Result<Self> {
        let rev = rev.unwrap_or(DEFAULT_REV);
        let key = format!("{}@{}", path_safe(repo_id(url)), path_safe(rev));
        let root = cache_dir()?.join("grammars").join(&key);
        if !root.is_dir() {
            eprintln!("mdbook-tsitter: fetching {url} at {rev}");
            clone_shallow(url, rev, &root)?;
        }
        Ok(Self { root, key })
    }
}

/// The per-user cache directory of this preprocessor.
fn cache_dir() -> Result<PathBuf> {
    Ok(etcetera::choose_base_strategy()
        .context("locating the user cache directory")?
        .cache_dir()
        .join(CACHE_DIR_NAME))
}

/// A grammar checked out from its git repository.
pub struct FetchedGrammar {
    /// The language name the grammar is configured under.
    name: String,
    checkout: Checkout,
    /// The grammar's entry in the repository's `tree-sitter.json`, if any.
    manifest: Option<ManifestGrammar>,
    /// Directory of the grammar itself: the checkout root, or its
    /// subdirectory in repositories that hold several grammars.
    grammar: PathBuf,
    /// The language's name in the built-in list, for its pinned queries.
    known_as: Option<String>,
}

impl FetchedGrammar {
    /// Check out the repository of language `name` and locate its grammar,
    /// generating the parser source if the repository does not commit it.
    pub fn fetch(name: &str, repo: &GrammarRepo) -> Result<Self> {
        let checkout = Checkout::fetch(&repo.url, repo.rev.as_deref())?;
        let manifest = Manifest::read(&checkout.root)?;
        let entry = manifest
            .as_ref()
            .and_then(|manifest| manifest.grammar(name, repo.location.as_deref()))
            .cloned();
        let location = repo
            .location
            .as_deref()
            .or_else(|| entry.as_ref().and_then(ManifestGrammar::location));
        let grammar = location.map_or_else(|| checkout.root.clone(), |l| checkout.root.join(l));

        if !grammar
            .join(GRAMMAR_SOURCE_DIR)
            .join(PARSER_SOURCE)
            .is_file()
        {
            if grammar.join(GRAMMAR_DEFINITION).is_file() {
                generate_parser(&grammar)?;
            } else {
                bail!("{}", missing_grammar(repo, manifest.as_ref()));
            }
        }

        Ok(Self {
            name: name.to_string(),
            checkout,
            manifest: entry,
            grammar,
            known_as: repo.known_as.clone(),
        })
    }

    /// Load the grammar's parser, compiling it on first use or when its
    /// sources are newer than the cached library.
    pub fn compile(&self) -> Result<Language> {
        let sources = self.grammar.join(GRAMMAR_SOURCE_DIR);
        let parser_cache = cache_dir()?.join("parsers").join(&self.checkout.key);
        Loader::with_parser_lib_path(parser_cache)
            .load_language_at_path(CompileConfig::new(&sources, None, None))
            .with_context(|| {
                format!(
                    "compiling the parser in {} (a C compiler must be installed)",
                    self.grammar.display()
                )
            })
    }

    /// The grammar's own queries, or `None` if it ships no highlights query.
    pub fn own_queries(&self) -> Result<Option<GrammarQueries>> {
        let Some(highlights) = self.own_query("highlights")? else {
            return Ok(None);
        };
        Ok(Some(GrammarQueries {
            highlights: Some(highlights),
            injections: self.own_query("injections")?,
            locals: self.own_query("locals")?,
        }))
    }

    /// Whether this is the listed repository of a language in the built-in
    /// list, whose pinned queries then apply.
    pub fn is_known(&self) -> bool {
        self.known_as.is_some()
    }

    /// For a known grammar, nvim-treesitter's queries tested against its
    /// pinned revision; `None` for other grammars or when there are none.
    pub fn pinned_queries(&self) -> Result<Option<GrammarQueries>> {
        let Some(language) = &self.known_as else {
            return Ok(None);
        };
        let Some(highlights) = pinned_query(language, "highlights")? else {
            return Ok(None);
        };
        Ok(Some(GrammarQueries {
            highlights: Some(highlights),
            injections: pinned_query(language, "injections")?,
            locals: pinned_query(language, "locals")?,
        }))
    }

    /// The grammar's own query of `kind`. Files listed in `tree-sitter.json`
    /// are joined, fetching any that belong to another grammar; without such
    /// a list the query is `queries/<kind>.scm`, beside the grammar or at the
    /// repository root.
    fn own_query(&self, kind: &str) -> Result<Option<String>> {
        let listed = self
            .manifest
            .as_ref()
            .map_or(&[][..], |manifest| manifest.queries(kind));
        if listed.is_empty() {
            let file = format!("{kind}.scm");
            return [&self.grammar, &self.checkout.root]
                .into_iter()
                .map(|dir| dir.join(QUERY_DIR).join(&file))
                .find(|path| path.is_file())
                .map(|path| read_query(&path))
                .transpose();
        }

        // Manifests sometimes list files that no longer exist; those are
        // skipped, and a manifest listing none of its files counts as no query.
        let mut parts = Vec::new();
        // Manifests list files most specific first (a grammar's own before
        // the grammar it extends), but a later pattern wins in
        // tree-sitter-highlight, so they are joined in reverse.
        for file in listed.iter().rev() {
            if let Some(path) = self.listed_query_path(file)? {
                parts.push(read_query(&path)?);
            }
        }
        Ok((!parts.is_empty()).then(|| parts.join("\n")))
    }

    /// Where a query file listed in `tree-sitter.json` is: in this checkout,
    /// or in the checkout of the dependency grammar it names. `None` when it
    /// cannot be found.
    fn listed_query_path(&self, file: &str) -> Result<Option<PathBuf>> {
        let Some(dependency) = DependencyQuery::parse(file) else {
            let path = self.checkout.root.join(file);
            return Ok(path.is_file().then_some(path));
        };
        let Some(known) = known::lookup(dependency.language()) else {
            warn(&format!(
                "`{}`: its `{file}` query builds on `{}`, which is not in the built-in grammar \
                 list, so that part is skipped; set the query's key to supply a complete one",
                self.name, dependency.package
            ));
            return Ok(None);
        };
        let checkout = self.dependency_checkout(&dependency, known)?;
        let path = checkout.root.join(dependency.file);
        Ok(path.is_file().then_some(path))
    }

    /// Check out the dependency at the version this repository's
    /// `package.json` pins, falling back to the built-in list's revision.
    fn dependency_checkout(
        &self,
        dependency: &DependencyQuery,
        known: &known::KnownGrammar,
    ) -> Result<Checkout> {
        if let Some(tag) = dependency_tag(&self.checkout.root, dependency.package) {
            match Checkout::fetch(&known.url, Some(&tag)) {
                Ok(checkout) => return Ok(checkout),
                Err(error) => warn(&format!(
                    "`{}`: {} {tag} could not be fetched ({error:#}); using its pinned revision",
                    self.name, dependency.package
                )),
            }
        }
        Checkout::fetch(&known.url, Some(&known.revision))
    }
}

/// Query `kind` for known `language` from the pinned query source, with the
/// queries it inherits joined before it, as nvim-treesitter loads them.
fn pinned_query(language: &str, kind: &str) -> Result<Option<String>> {
    let source = known::query_source();
    let checkout = Checkout::fetch(&source.url, Some(&source.revision))?;
    inherited_query(
        &checkout.root.join(&source.directory),
        language,
        kind,
        &mut Vec::new(),
    )
}

/// The query `<language>/<kind>.scm` under `queries`, preceded by the queries
/// of the languages its `; inherits:` modeline names. `visited` guards
/// against inheritance cycles.
fn inherited_query(
    queries: &Path,
    language: &str,
    kind: &str,
    visited: &mut Vec<String>,
) -> Result<Option<String>> {
    if visited.iter().any(|seen| seen == language) {
        return Ok(None);
    }
    visited.push(language.to_string());
    let path = queries.join(language).join(format!("{kind}.scm"));
    if !path.is_file() {
        return Ok(None);
    }
    let own = read_query(&path)?;
    let mut parts = Vec::new();
    for base in inherited_languages(&own) {
        parts.extend(inherited_query(queries, &base, kind, visited)?);
    }
    parts.push(own);
    Ok(Some(parts.join("\n")))
}

/// Languages named by a query's leading `; inherits: a,(b)` modeline. A
/// parenthesised name is inherited only by a query loaded directly, which is
/// how every query here is loaded.
fn inherited_languages(query: &str) -> Vec<String> {
    query
        .lines()
        .take_while(|line| line.starts_with(';'))
        .filter_map(|line| {
            line.trim_start_matches(';')
                .trim()
                .strip_prefix("inherits:")
        })
        .flat_map(|names| names.split(','))
        .map(|name| name.trim().trim_matches(['(', ')']).to_string())
        .filter(|name| !name.is_empty())
        .collect()
}

/// The error for a repository whose grammar could not be located.
fn missing_grammar(repo: &GrammarRepo, manifest: Option<&Manifest>) -> String {
    let rev = repo.rev.as_deref().unwrap_or(DEFAULT_REV);
    let place = repo
        .location
        .as_deref()
        .map(|location| format!(" under `{location}`"))
        .unwrap_or_default();
    let choices = manifest
        .map(|manifest| {
            manifest
                .grammars
                .iter()
                .filter_map(|grammar| grammar.location())
                .map(|location| format!("`{location}`"))
                .collect::<Vec<_>>()
        })
        .filter(|choices| !choices.is_empty())
        .map(|choices| format!(" It holds grammars in {}.", choices.join(", ")))
        .unwrap_or_default();
    format!(
        "{} at {rev} has no grammar{place}; set `location` to the grammar's subdirectory.{choices}",
        repo.url
    )
}

/// Generate `src/parser.c` for a grammar whose repository omits it.
fn generate_parser(grammar: &Path) -> Result<()> {
    eprintln!(
        "mdbook-tsitter: generating the parser in {}",
        grammar.display()
    );
    let output = Command::new(TREE_SITTER_CLI)
        .arg("generate")
        .current_dir(grammar)
        .output()
        .with_context(|| {
            format!(
                "{} commits no generated parser; generating it needs the tree-sitter CLI \
                 and Node.js",
                grammar.display()
            )
        })?;
    if !output.status.success() {
        bail!(
            "`tree-sitter generate` failed in {}: {}",
            grammar.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn read_query(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("reading query {}", path.display()))
}

/// Shallow-fetch `rev` of `url` into `destination`. The checkout is staged
/// beside `destination` and moved into place only once complete, so an
/// interrupted fetch never leaves a partial checkout in the cache.
fn clone_shallow(url: &str, rev: &str, destination: &Path) -> Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow!("{} has no parent directory", destination.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    let staging = parent.join(format!(".staging-{}", process::id()));
    if staging.exists() {
        fs::remove_dir_all(&staging).with_context(|| format!("clearing {}", staging.display()))?;
    }

    let fetched = git(&staging, &["init", "--quiet"])
        .and_then(|_| git(&staging, &["fetch", "--quiet", "--depth", "1", url, rev]))
        .and_then(|_| git(&staging, &["checkout", "--quiet", "--detach", "FETCH_HEAD"]))
        .with_context(|| format!("fetching {url} at {rev}"));
    if let Err(error) = fetched {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }

    if let Err(error) = fs::rename(&staging, destination) {
        let _ = fs::remove_dir_all(&staging);
        // Another build fetching the same grammar finished first.
        if !destination.is_dir() {
            return Err(error)
                .with_context(|| format!("moving checkout to {}", destination.display()));
        }
    }
    Ok(())
}

/// Run `git` in `dir` (created if needed) and return its trimmed output,
/// failing with its stderr. Output is captured because stdout carries the
/// preprocessor protocol, and credential prompts are disabled so a mistyped
/// URL fails instead of hanging the build.
fn git(dir: &Path, args: &[&str]) -> Result<String> {
    fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let output = Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .context("running git (is it installed?)")?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// A git URL without its scheme and `.git` suffix, e.g.
/// `github.com/tree-sitter/tree-sitter-rust`, so forks sharing a repository
/// name are cached apart.
fn repo_id(url: &str) -> &str {
    let location = url.split_once("://").map_or(url, |(_, rest)| rest);
    let location = location.trim_end_matches('/');
    location.strip_suffix(".git").unwrap_or(location)
}

/// `text` with every character that is unsafe in a file name replaced by `-`.
fn path_safe(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "-_.".contains(c) {
                c
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_id_drops_scheme_and_git_suffix() {
        for url in [
            "https://github.com/tree-sitter/tree-sitter-rust",
            "https://github.com/tree-sitter/tree-sitter-rust.git",
            "https://github.com/tree-sitter/tree-sitter-rust/",
        ] {
            assert_eq!(repo_id(url), "github.com/tree-sitter/tree-sitter-rust");
        }
    }

    #[test]
    fn inherited_languages_come_from_the_leading_modeline() {
        let query = "; inherits: ecma,(jsx)\n; other comment\n(identifier) @variable\n";
        assert_eq!(inherited_languages(query), ["ecma", "jsx"]);
        assert!(inherited_languages("(identifier) @variable\n; inherits: c\n").is_empty());
    }

    #[test]
    fn urls_are_never_local_repositories() {
        let root = std::env::temp_dir();
        assert!(
            local_repository(&root, "https://github.com/tree-sitter/tree-sitter-rust").is_none()
        );
        assert!(local_repository(&root, "git@github.com:tree-sitter/tree-sitter-rust").is_none());
        assert!(local_repository(&root, ".").is_some());
        assert!(local_repository(&root, "no-such-directory-here").is_none());
    }

    #[test]
    fn cache_keys_are_path_safe() {
        assert_eq!(
            path_safe(repo_id("https://github.com/tree-sitter/tree-sitter-rust")),
            "github.com-tree-sitter-tree-sitter-rust"
        );
        assert_eq!(path_safe("feature/v1.2"), "feature-v1.2");
    }
}
