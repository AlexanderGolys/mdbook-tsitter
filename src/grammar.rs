//! Grammar resolution: maps a code-fence language tag (e.g. `rust`) to a
//! configured tree-sitter [`HighlightConfiguration`]. Every grammar is loaded
//! at runtime as described in `book.toml`: fetched and
//! compiled from its git repository (known by name or configured), or from a
//! prebuilt shared library.
//! Dynamic loading is what makes the preprocessor work for *any* language.
//!
//! All grammars in a run share one capture-index space (see [`Registry::build`])
//! so that spans produced by an injected sub-language resolve to the same CSS
//! classes as the host language.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use tree_sitter::Language;
use tree_sitter_highlight::{
    Error as HighlightError, Highlight, HighlightConfiguration, HighlightEvent, Highlighter,
    HtmlRenderer,
};

use crate::config::{Config, LanguageConfig};
use crate::fetch::{FetchedGrammar, GrammarQueries, GrammarRepo};
use crate::semantic::Mark;
use crate::{predicates, render};

/// All grammars known to a single preprocessor run. Each grammar is stored once
/// in `configs` and reachable through every fence tag (alias) in `by_alias`.
/// `classes` is the shared class table, indexed by the global capture index
/// every config was configured against.
pub struct Registry {
    configs: Vec<HighlightConfiguration>,
    by_alias: HashMap<String, usize>,
    /// Whether each of `configs` reads `§` as a token marker in every block.
    lsp: Vec<bool>,
    classes: Vec<Vec<u8>>,
    /// Whether embedded languages are resolved and highlighted. When `false`,
    /// the injection callback always declines, so blocks render with only their
    /// host grammar.
    inject: bool,
}

impl Registry {
    /// Load and configure every grammar named in `book.toml`.
    pub fn build(root: &Path, config: &Config) -> Result<Self> {
        let mut configs: Vec<HighlightConfiguration> = Vec::new();
        let mut by_alias: HashMap<String, usize> = HashMap::new();
        let mut lsp: Vec<bool> = Vec::new();

        for (name, lang_cfg) in &config.languages {
            let cfg = load_dynamic(name, lang_cfg, root, config.inject)?;
            let index = configs.len();
            configs.push(cfg);
            lsp.push(lang_cfg.lsp);
            for alias in aliases_of(name, lang_cfg) {
                if by_alias
                    .get(alias)
                    .is_some_and(|&existing| existing != index)
                {
                    return Err(anyhow!(
                        "fence tag `{alias}` is claimed by more than one language; \
                         give each grammar distinct aliases"
                    ));
                }
                by_alias.insert(alias.to_string(), index);
            }
        }

        // Give every grammar one shared list of recognised capture names, so a
        // capture maps to the same highlight index — and therefore the same CSS
        // class — regardless of which grammar (host or injected) produced it.
        let global_names = union_capture_names(&configs);
        for cfg in &mut configs {
            cfg.configure(&global_names);
        }
        let classes = render::class_attributes(&global_names);

        Ok(Self {
            configs,
            by_alias,
            lsp,
            classes,
            inject: config.inject,
        })
    }

    /// Whether every block fenced `lang` reads `§` as a token marker.
    pub fn lsp_by_default(&self, lang: &str) -> bool {
        self.by_alias
            .get(lang)
            .is_some_and(|&index| self.lsp[index])
    }

    /// Highlight `source` as `lang`, or `None` if no grammar handles that fence
    /// tag (the block is then left untouched). Embedded languages are resolved
    /// through the same alias table, so an injected block in an unregistered
    /// language is simply left unhighlighted rather than erroring.
    pub fn highlight(&self, lang: &str, source: &str, marks: &[Mark]) -> Option<Result<String>> {
        let config = self.config_for(lang)?;
        Some(self.render(config, source, marks))
    }

    /// Render one block. Inlined here (rather than in `render`) so the injection
    /// callback and the host config share the `&self` lifetime — the borrow
    /// checker resolves the highlighter's internal lifetime locally, mirroring
    /// tree-sitter-highlight's own C binding.
    fn render(
        &self,
        config: &HighlightConfiguration,
        source: &str,
        marks: &[Mark],
    ) -> Result<String> {
        let mut highlighter = Highlighter::new();
        let events = highlighter
            .highlight(config, source.as_bytes(), None, |name: &str| {
                // Resolve embedded languages only among grammars already built;
                // injection never loads a new grammar. Disabled when `inject` is
                // off, so a misbehaving sub-grammar can be switched out entirely.
                self.inject.then(|| self.config_for(name)).flatten()
            })
            .context("tree-sitter highlighting failed")?;

        // Each mark is a highlight of its own, numbered after the grammars' captures.
        let first_mark = self.classes.len();
        let mark_classes: Vec<Vec<u8>> = marks.iter().map(|mark| mark.class_attribute()).collect();
        let events = overlay_marks(events, marks, first_mark);

        let mut renderer = HtmlRenderer::new();
        renderer
            .render(
                events,
                source.as_bytes(),
                &|highlight: Highlight, out: &mut Vec<u8>| {
                    // An out-of-range index would mean a capture we never configured;
                    // fall back to an empty attribute rather than panic.
                    let attr = self.classes.get(highlight.0).or_else(|| {
                        highlight
                            .0
                            .checked_sub(first_mark)
                            .and_then(|mark| mark_classes.get(mark))
                    });
                    if let Some(attr) = attr {
                        out.extend_from_slice(attr);
                    }
                },
            )
            .context("rendering highlighted HTML failed")?;

        Ok(renderer.lines().collect())
    }

    fn config_for(&self, lang: &str) -> Option<&HighlightConfiguration> {
        self.by_alias.get(lang).map(|&index| &self.configs[index])
    }
}

/// `events` with each mark laid over the source it covers, as highlight
/// number `first_mark` plus its position in `marks`. A mark is opened and
/// closed around every piece of source it covers, so it never straddles the
/// start or end of a capture and the spans stay nested.
fn overlay_marks<'a>(
    events: impl Iterator<Item = Result<HighlightEvent, HighlightError>> + 'a,
    marks: &'a [Mark],
    first_mark: usize,
) -> impl Iterator<Item = Result<HighlightEvent, HighlightError>> + 'a {
    events.flat_map(move |event| match event {
        Ok(HighlightEvent::Source { start, end }) => {
            let mut pieces = Vec::new();
            let mut cursor = start;
            for (index, mark) in marks.iter().enumerate() {
                let covered = mark.range.start.max(start)..mark.range.end.min(end);
                if covered.is_empty() {
                    continue;
                }
                if covered.start > cursor {
                    pieces.push(Ok(HighlightEvent::Source {
                        start: cursor,
                        end: covered.start,
                    }));
                }
                pieces.push(Ok(HighlightEvent::HighlightStart(Highlight(
                    first_mark + index,
                ))));
                pieces.push(Ok(HighlightEvent::Source {
                    start: covered.start,
                    end: covered.end,
                }));
                pieces.push(Ok(HighlightEvent::HighlightEnd));
                cursor = covered.end;
            }
            if cursor < end {
                pieces.push(Ok(HighlightEvent::Source { start: cursor, end }));
            }
            pieces
        }
        other => vec![other],
    })
}

/// The fence tags a configured language answers to: its explicit `aliases`, or
/// its table key when none are given.
fn aliases_of<'a>(name: &'a str, cfg: &'a LanguageConfig) -> impl Iterator<Item = &'a str> {
    let configured = cfg.aliases.iter().map(String::as_str);
    let fallback = cfg.aliases.is_empty().then_some(name);
    configured.chain(fallback)
}

/// The ordered union of every config's capture names, first occurrence kept.
fn union_capture_names(configs: &[HighlightConfiguration]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for config in configs {
        for name in config.names() {
            if !names.iter().any(|existing| existing == name) {
                names.push(name.to_string());
            }
        }
    }
    names
}

/// Load a grammar as configured in `book.toml` from its git repository
/// (configured, or known by name). `root` is the book project root that
/// relative paths resolve against.
fn load_dynamic(
    name: &str,
    cfg: &LanguageConfig,
    root: &Path,
    inject: bool,
) -> Result<HighlightConfiguration> {
    let configure = |language: &Language, fetched: GrammarQueries| {
        configure(name, cfg, root, inject, language.clone(), fetched)
    };
    let grammar = FetchedGrammar::fetch(name, &GrammarRepo::configured(name, cfg, root)?)?;
    let language = grammar.compile()?;
    let reading = || format!("reading the queries of `{name}`");
    // A known grammar prefers the queries tested against its pinned revision,
    // which are richer than most repositories' own; its own are the fallback,
    // and the only source for other grammars.
    let preferred = match grammar.is_known() {
        true => grammar.pinned_queries(),
        false => grammar.own_queries(),
    }
    .with_context(reading)?;
    let preferred_error = match preferred.map(|queries| configure(&language, queries)) {
        Some(Ok(config)) => return Ok(config),
        Some(Err(error)) => Some(error),
        None => None,
    };
    let own = match grammar.is_known() {
        true => grammar.own_queries().with_context(reading)?,
        false => None,
    };
    match (own, preferred_error) {
        (Some(own), error) => {
            if let Some(error) = error {
                eprintln!(
                    "mdbook-tsitter: `{name}`: nvim-treesitter's queries do not match its \
                     parser ({}); using its own",
                    first_line(&error)
                );
            }
            configure(&language, own)
        }
        (None, Some(error)) => Err(error),
        (None, None) => configure(&language, GrammarQueries::default()),
    }
}

/// The first line of an error's root cause, without a trailing colon.
fn first_line(error: &anyhow::Error) -> String {
    let cause = error.root_cause().to_string();
    let line = cause.lines().next().unwrap_or_default();
    line.trim_end_matches(':').to_string()
}

/// Build the highlight configuration of `language` from its `fetched`
/// queries, each overridden by a query file configured in `book.toml`.
/// Queries are normalised: nvim-specific predicates (`#lua-match?`) become
/// standard ones that tree-sitter-highlight evaluates; otherwise the predicate
/// is ignored and its capture fires unconditionally (e.g. every identifier
/// becomes a `@constant`). See [`crate::predicates`].
fn configure(
    name: &str,
    cfg: &LanguageConfig,
    root: &Path,
    inject: bool,
    language: Language,
    fetched: GrammarQueries,
) -> Result<HighlightConfiguration> {
    let query = |configured: Option<&String>, kind: &str, fetched: Option<String>| {
        let text = match configured {
            Some(path) => Some(read_query(&resolve(root, path), name, kind)?),
            None => fetched,
        };
        Ok::<_, anyhow::Error>(text)
    };
    let highlights = query(cfg.highlights.as_ref(), "highlights", fetched.highlights)?
        .map(|text| predicates::normalize_highlights(&text))
        .ok_or_else(|| {
            anyhow!("language `{name}` needs a `highlights` query; its grammar ships none")
        })?;
    // With injection off we never read the injections query, so a broken
    // `injections.scm` cannot fail compilation and take `highlights` down with it.
    let injections = if inject {
        query(cfg.injections.as_ref(), "injections", fetched.injections)?
            .map(|text| predicates::normalize(&text))
            .unwrap_or_default()
    } else {
        String::new()
    };
    let locals = query(cfg.locals.as_ref(), "locals", fetched.locals)?
        .map(|text| predicates::normalize(&text))
        .unwrap_or_default();

    HighlightConfiguration::new(language, name, &highlights, &injections, &locals)
        .with_context(|| format!("invalid tree-sitter queries for `{name}`"))
}

/// Resolve a possibly-relative path against the book root.
fn resolve(root: &Path, path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        root.join(p)
    }
}

fn read_query(path: &Path, lang: &str, kind: &str) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("reading {kind} query for `{lang}` from {}", path.display()))
}
