//! Normalising Neovim-specific query predicates into the standard tree-sitter
//! ones that [`tree_sitter_highlight`] actually evaluates.
//!
//! nvim-treesitter queries use `#lua-match?` / `#not-lua-match?`, whose argument
//! is a [Lua pattern] rather than a regex. `tree-sitter-highlight` does not know
//! these predicates, and — crucially — it does not drop the patterns that carry
//! them: it simply ignores the predicate, so the capture applies
//! *unconditionally*. A rule meant to fire only on, say, all-caps identifiers
//! (`((identifier) @constant (#lua-match? @constant "^[A-Z][A-Z%d_]*$"))`) then
//! colours *every* identifier as a constant.
//!
//! [`normalize`] rewrites each `#lua-match?` into the standard `#match?` (and
//! `#not-lua-match?` into `#not-match?`), translating the Lua pattern to the
//! equivalent regex, so the predicate is evaluated as the grammar author meant.
//!
//! The other Neovim-only constructs are handled in the same spirit:
//!
//! - `#contains?` becomes the equivalent `#match?`.
//! - `#match?` regexes are Vim regexes, which may start with the
//!   case-insensitivity flag `\c`; it becomes the equivalent `(?i)`.
//! - `#set!` directives with a second capture
//!   (`(#set! @markup.link.url url @markup.link.url)`), which tree-sitter
//!   rejects outright, only set properties for Neovim's concealing and are
//!   dropped.
//!
//! In a highlights query, a later capture of a node replaces an earlier one
//! even when it is no highlight, so [`normalize_highlights`] also removes
//! Neovim's marker captures (`@spell`, `@conceal`, …) and patterns that only
//! label nodes with helper captures (`@_name`), which would erase colour.
//!
//! Predicates tree-sitter cannot evaluate at all, such as `#has-ancestor?`,
//! are ignored by `tree-sitter-highlight`; their patterns then apply more
//! broadly, which later, more specific patterns usually refine.
//!
//! [Lua pattern]: https://www.lua.org/manual/5.4/manual.html#6.4.1

use std::ops::Range;

/// The Neovim predicate names and the standard predicates they map to.
const PREDICATE_REWRITES: [(&str, &str); 2] = [
    ("#not-lua-match?", "#not-match?"),
    ("#lua-match?", "#match?"),
];

/// The regex predicates, whose patterns Neovim reads as Vim regexes.
const REGEX_PREDICATES: [&str; 2] = ["#match?", "#not-match?"];
/// Vim's case-insensitivity flag, and the regex flag it maps to.
const VIM_IGNORE_CASE: &str = "\\c";
const REGEX_IGNORE_CASE: &str = "(?i)";

/// The property directive, which tree-sitter allows at most one capture in.
const SET_DIRECTIVE: &str = "#set!";
/// Neovim's substring predicate, expressible as a standard `#match?`.
const CONTAINS_PREDICATE: &str = "#contains?";
const MATCH_PREDICATE: &str = "#match?";
/// Neovim's marker captures: they flag nodes for spell checking or
/// concealing, not colour. In tree-sitter-highlight a later capture of a node
/// replaces an earlier one even when it is no highlight, so a marker would
/// erase the node's colour.
const MARKER_CAPTURES: [&str; 4] = ["@spell", "@nospell", "@conceal", "@conceal_lines"];
/// Prefix of helper captures, used by predicates and never highlighted.
const HELPER_CAPTURE_PREFIX: &str = "@_";

/// Make a Neovim query evaluable by tree-sitter. Standard queries are
/// returned unchanged.
pub fn normalize(query: &str) -> String {
    let query = translate_contains(query);
    let query = drop_multi_capture_settings(&query);
    rewrite_lua_matches(&translate_ignore_case_flags(&query))
}

/// [`normalize`] a highlights query, and remove what would erase colour:
/// marker captures, and patterns left without a highlight capture.
pub fn normalize_highlights(query: &str) -> String {
    drop_patterns_without_highlights(&strip_marker_captures(&normalize(query)))
}

/// Rewrite every `(#contains? @capture "a" "b")` into the equivalent
/// `(#match? @capture "a|b")`.
fn translate_contains(query: &str) -> String {
    let replacements = predicate_forms(query)
        .filter(|form| form.name == CONTAINS_PREDICATE)
        .map(|form| {
            let mut capture = "";
            let mut alternatives = Vec::new();
            for token in tokens(&query[form.range.clone()]) {
                let text = &query[form.range.start + token.start..form.range.start + token.end];
                match token.first {
                    '@' => capture = name_at(query, form.range.start + token.start),
                    '"' => {
                        alternatives.extend(split_pattern_literal(text).map(|(_, literal, _)| {
                            literal
                                .chars()
                                .map(escape_regex_literal)
                                .collect::<String>()
                        }))
                    }
                    _ => {}
                }
            }
            let regex = escape_string_literal(&alternatives.join("|"));
            (
                form.range,
                format!("({MATCH_PREDICATE} {capture} \"{regex}\")"),
            )
        })
        .collect();
    splice(query, replacements)
}

/// Turn Vim's `\c` in every `#match?` / `#not-match?` regex into `(?i)`.
fn translate_ignore_case_flags(query: &str) -> String {
    let mut out = String::with_capacity(query.len());
    let mut rest = query;
    while let Some((at, name)) = REGEX_PREDICATES
        .iter()
        .filter_map(|name| rest.find(name).map(|at| (at, *name)))
        .min_by_key(|&(at, _)| at)
    {
        let after_name = &rest[at + name.len()..];
        out.push_str(&rest[..at + name.len()]);
        match split_pattern_literal(after_name) {
            Some((before, regex, tail)) if regex.contains(VIM_IGNORE_CASE) => {
                let regex = format!("{REGEX_IGNORE_CASE}{}", regex.replace(VIM_IGNORE_CASE, ""));
                out.push_str(before);
                out.push('"');
                out.push_str(&escape_string_literal(&regex));
                out.push('"');
                rest = tail;
            }
            _ => rest = after_name,
        }
    }
    out.push_str(rest);
    out
}

/// Remove every `#set!` directive that names more than one capture.
fn drop_multi_capture_settings(query: &str) -> String {
    let removals = predicate_forms(query)
        .filter(|form| form.name == SET_DIRECTIVE)
        .filter(|form| {
            tokens(&query[form.range.clone()])
                .filter(|token| token.first == '@')
                .count()
                > 1
        })
        .map(|form| (form.range, String::new()))
        .collect();
    splice(query, removals)
}

/// Remove marker captures (`@spell`, …) that no predicate of their pattern
/// refers to.
fn strip_marker_captures(query: &str) -> String {
    let mut removals = Vec::new();
    for pattern in top_level_patterns(query) {
        let captures = captures(query, pattern);
        for label in captures.iter().filter(|capture| !capture.in_predicate) {
            let referenced = captures
                .iter()
                .any(|other| other.in_predicate && other.name == label.name);
            if MARKER_CAPTURES.contains(&label.name) && !referenced {
                removals.push((label.range.clone(), String::new()));
            }
        }
    }
    splice(query, removals)
}

/// Remove every top-level pattern that labels nodes only with helper or
/// marker captures. It adds no colour, but would still erase the colour an
/// earlier pattern gave its nodes.
fn drop_patterns_without_highlights(query: &str) -> String {
    let removals = top_level_patterns(query)
        .into_iter()
        .filter(|pattern| {
            !captures(query, pattern.clone()).iter().any(|capture| {
                !capture.in_predicate
                    && !capture.name.starts_with(HELPER_CAPTURE_PREFIX)
                    && !MARKER_CAPTURES.contains(&capture.name)
            })
        })
        .map(|pattern| (pattern, String::new()))
        .collect();
    splice(query, removals)
}

/// A capture name in a query: labelling a node, or referred to by a predicate.
struct Capture<'a> {
    /// The name with its `@`, e.g. `@variable`.
    name: &'a str,
    range: Range<usize>,
    in_predicate: bool,
}

/// The captures within `range` of `query`.
fn captures(query: &str, range: Range<usize>) -> Vec<Capture<'_>> {
    let source = &query[range.clone()];
    let predicates: Vec<Range<usize>> = predicate_forms(source).map(|form| form.range).collect();
    tokens(source)
        .filter(|token| token.first == '@')
        .map(|token| {
            let name = name_at(query, range.start + token.start);
            Capture {
                name,
                range: range.start + token.start..range.start + token.start + name.len(),
                in_predicate: predicates.iter().any(|form| form.contains(&token.start)),
            }
        })
        .collect()
}

/// One significant piece of query source: a whole string literal, or a
/// single character outside string literals, comments and whitespace.
#[derive(Debug, Clone, Copy)]
struct Token {
    start: usize,
    end: usize,
    first: char,
}

/// The tokens of `source`.
fn tokens(source: &str) -> impl Iterator<Item = Token> + '_ {
    let mut chars = source.char_indices();
    std::iter::from_fn(move || loop {
        let (start, c) = chars.next()?;
        match c {
            ';' => {
                for (_, c) in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '"' => {
                let mut end = source.len();
                while let Some((i, c)) = chars.next() {
                    match c {
                        '\\' => {
                            chars.next();
                        }
                        '"' => {
                            end = i + 1;
                            break;
                        }
                        _ => {}
                    }
                }
                return Some(Token {
                    start,
                    end,
                    first: '"',
                });
            }
            c if c.is_whitespace() => {}
            c => {
                return Some(Token {
                    start,
                    end: start + c.len_utf8(),
                    first: c,
                })
            }
        }
    })
}

/// Whether `c` continues a node, capture or predicate name.
fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || "_-.?!#".contains(c)
}

/// The name starting at byte `start` of `source`, e.g. `@variable.builtin`.
fn name_at(source: &str, start: usize) -> &str {
    let rest = &source[start..];
    let len = rest
        .char_indices()
        .skip(1)
        .find(|&(_, c)| !is_name_char(c))
        .map_or(rest.len(), |(i, _)| i);
    &rest[..len]
}

/// A predicate or directive form, `(#name …)`, in a query.
struct PredicateForm<'a> {
    /// The name with its `#`, e.g. `#eq?`.
    name: &'a str,
    /// The form's byte range, parentheses included.
    range: Range<usize>,
}

/// Every predicate or directive form in `query`.
fn predicate_forms(query: &str) -> impl Iterator<Item = PredicateForm<'_>> {
    let all: Vec<Token> = tokens(query).collect();
    let mut forms = Vec::new();
    for (index, token) in all.iter().enumerate() {
        let opens_predicate = token.first == '('
            && all
                .get(index + 1)
                .is_some_and(|next| next.first == '#' && next.start == token.end);
        if opens_predicate {
            let last = index + form_token_count(&all[index..]) - 1;
            forms.push(PredicateForm {
                name: name_at(query, all[index + 1].start),
                range: token.start..all[last].end,
            });
        }
    }
    forms.into_iter()
}

/// The number of tokens in the bracketed form that `tokens` opens with,
/// through its closing bracket (all of `tokens` if it is never closed).
fn form_token_count(tokens: &[Token]) -> usize {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        match token.first {
            '(' | '[' => depth += 1,
            ')' | ']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return index + 1;
                }
            }
            _ => {}
        }
    }
    tokens.len()
}

/// The byte ranges of the top-level patterns in `query`, each with the
/// quantifiers and captures that follow it.
fn top_level_patterns(query: &str) -> Vec<Range<usize>> {
    let all: Vec<Token> = tokens(query).collect();
    let mut patterns = Vec::new();
    let mut index = 0;
    while let Some(first) = all.get(index) {
        let (mut next, mut end) = match first.first {
            '(' | '[' => {
                let next = index + form_token_count(&all[index..]);
                (next, all[next - 1].end)
            }
            '"' => (index + 1, first.end),
            _ => skip_name(&all, index + 1, first.end),
        };
        // Trailing quantifiers and captures belong to the pattern.
        while let Some(token) = all.get(next) {
            match token.first {
                '*' | '+' | '?' => {
                    end = token.end;
                    next += 1;
                }
                '@' => (next, end) = skip_name(&all, next + 1, token.end),
                _ => break,
            }
        }
        patterns.push(first.start..end);
        index = next;
    }
    patterns
}

/// Advance past the name characters that directly follow byte `end`,
/// starting at token `index`; returns the next token index and the new end.
fn skip_name(tokens: &[Token], mut index: usize, mut end: usize) -> (usize, usize) {
    while let Some(token) = tokens
        .get(index)
        .filter(|token| token.start == end && is_name_char(token.first))
    {
        end = token.end;
        index += 1;
    }
    (index, end)
}

/// `query` with each range replaced by its text. Ranges must not overlap;
/// a range inside an earlier one is ignored.
fn splice(query: &str, mut replacements: Vec<(Range<usize>, String)>) -> String {
    replacements.sort_by_key(|(range, _)| range.start);
    let mut out = String::with_capacity(query.len());
    let mut copied = 0;
    for (range, text) in replacements {
        if range.start < copied {
            continue;
        }
        out.push_str(&query[copied..range.start]);
        out.push_str(&text);
        copied = range.end;
    }
    out.push_str(&query[copied..]);
    out
}

/// Rewrite every Neovim `#lua-match?` / `#not-lua-match?` predicate in `query`
/// into the standard `#match?` / `#not-match?`, translating its Lua-pattern
/// argument to a regex.
fn rewrite_lua_matches(query: &str) -> String {
    let mut out = String::with_capacity(query.len());
    let mut rest = query;

    while let Some((standard, after_name, consumed)) = next_rewrite(rest) {
        out.push_str(&rest[..consumed]);
        out.push_str(standard);
        // The pattern is the next string literal after the capture name(s).
        match split_pattern_literal(after_name) {
            Some((before, lua_pattern, tail)) => {
                out.push_str(before);
                out.push('"');
                out.push_str(&escape_string_literal(&lua_pattern_to_regex(&lua_pattern)));
                out.push('"');
                rest = tail;
            }
            // No string literal follows (malformed predicate): leave the rest as
            // is rather than guessing.
            None => rest = after_name,
        }
    }
    out.push_str(rest);
    out
}

/// Find the next Neovim predicate to rewrite. Returns the standard replacement
/// name, the slice immediately after the original name, and the byte offset of
/// the original name within `query`.
fn next_rewrite(query: &str) -> Option<(&'static str, &str, usize)> {
    PREDICATE_REWRITES
        .iter()
        .filter_map(|&(nvim, standard)| {
            query
                .find(nvim)
                .map(|at| (at, standard, &query[at + nvim.len()..]))
        })
        .min_by_key(|&(at, _, _)| at)
        .map(|(at, standard, after)| (standard, after, at))
}

/// Split `after_name` at the first string literal, returning the text before the
/// opening quote, the (unescaped) literal contents, and the text after the
/// closing quote.
fn split_pattern_literal(after_name: &str) -> Option<(&str, String, &str)> {
    let open = after_name.find('"')?;
    let mut contents = String::new();
    let mut chars = after_name[open + 1..].char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '\\' => {
                if let Some((_, escaped)) = chars.next() {
                    contents.push(escaped);
                }
            }
            '"' => {
                let close = open + 1 + i;
                return Some((&after_name[..open], contents, &after_name[close + 1..]));
            }
            _ => contents.push(c),
        }
    }
    None
}

/// Re-escape a regex so it can be embedded back into a query string literal.
fn escape_string_literal(regex: &str) -> String {
    regex.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Translate a [Lua pattern] into an equivalent Rust `regex`.
///
/// Supports the constructs that appear in tree-sitter highlight queries: the
/// `%`-classes (`%d`, `%a`, `%s`, …, and their negations), `%`-escaped
/// literals, character sets `[...]` (including `%`-classes inside them), the
/// anchors `^`/`$`, the quantifiers `*`/`+`/`?`, and Lua's lazy `-` quantifier.
/// Characters that are literal in Lua but special in regex (`{`, `}`, `|`) are
/// escaped.
///
/// [Lua pattern]: https://www.lua.org/manual/5.4/manual.html#6.4.1
fn lua_pattern_to_regex(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len());
    let mut chars = pattern.chars().peekable();
    let mut in_set = false;

    while let Some(c) = chars.next() {
        match c {
            '%' => match chars.next() {
                Some(class) => out.push_str(&translate_percent(class, in_set)),
                None => out.push('%'),
            },
            '[' if !in_set => {
                in_set = true;
                out.push('[');
            }
            ']' => {
                in_set = false;
                out.push(']');
            }
            // Lua's `-` is a lazy zero-or-more quantifier; regex spells it `*?`.
            // At the very start there is nothing to quantify, so treat it as a
            // literal dash.
            '-' if !in_set => out.push_str(if out.is_empty() { "\\-" } else { "*?" }),
            // Literal in Lua, special in regex everywhere.
            '\\' => out.push_str("\\\\"),
            // Literal in Lua, special in regex.
            '{' | '}' | '|' if !in_set => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

/// Translate the character following a `%` in a Lua pattern. `in_set` selects
/// between a standalone class (`[0-9]`) and the bare contents used inside an
/// existing `[...]` set (`0-9`).
fn translate_percent(class: char, in_set: bool) -> String {
    let ranges = match class {
        'a' | 'A' => "a-zA-Z",
        'd' | 'D' => "0-9",
        'l' | 'L' => "a-z",
        'u' | 'U' => "A-Z",
        's' | 'S' => " \\t\\n\\r\\f\\x0b",
        'w' | 'W' => "0-9a-zA-Z",
        'x' | 'X' => "0-9a-fA-F",
        'p' | 'P' => "!-/:-@\\[-`{-~",
        // Not a class letter: a `%`-escaped literal (e.g. `%.`, `%%`, `%-`).
        // Escape it for regex when it is a metacharacter.
        other => return escape_regex_literal(other),
    };
    let negated = class.is_ascii_uppercase();
    match (in_set, negated) {
        // Negation has no bare in-set form, so it is always a standalone
        // `[^...]`. Real highlight queries only ever negate outside a set, so
        // the nested `[[^...]]` this would yield inside one does not arise.
        (_, true) => format!("[^{ranges}]"),
        // Inside an existing set, contribute just the range body; standalone,
        // wrap it in its own set.
        (true, false) => ranges.to_string(),
        (false, false) => format!("[{ranges}]"),
    }
}

/// Escape a single character that was `%`-escaped in a Lua pattern so it is a
/// literal in regex.
fn escape_regex_literal(c: char) -> String {
    const REGEX_META: &str = r".^$*+?()[]{}|\";
    if REGEX_META.contains(c) {
        format!("\\{c}")
    } else {
        c.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_class_inside_set() {
        assert_eq!(
            lua_pattern_to_regex("^[A-Z][A-Z%d_]*$"),
            "^[A-Z][A-Z0-9_]*$"
        );
    }

    #[test]
    fn translates_standalone_class() {
        assert_eq!(lua_pattern_to_regex("%s?@"), "[ \\t\\n\\r\\f\\x0b]?@");
    }

    #[test]
    fn passes_through_plain_anchored_set() {
        assert_eq!(lua_pattern_to_regex("^[A-Z]"), "^[A-Z]");
        assert_eq!(
            lua_pattern_to_regex("^[A-Z][A-Z_0-9]*$"),
            "^[A-Z][A-Z_0-9]*$"
        );
    }

    #[test]
    fn literal_dash_in_set_is_preserved() {
        assert_eq!(lua_pattern_to_regex("^[-][-][-]"), "^[-][-][-]");
        assert_eq!(
            lua_pattern_to_regex("^[-][-](%s?)@"),
            "^[-][-]([ \\t\\n\\r\\f\\x0b]?)@"
        );
    }

    #[test]
    fn escapes_backslash_literals() {
        assert_eq!(
            lua_pattern_to_regex("^\\if[a-zA-Z@]+$"),
            "^\\\\if[a-zA-Z@]+$"
        );
        assert_eq!(lua_pattern_to_regex("[\\]"), "[\\\\]");
    }

    #[test]
    fn escapes_percent_literals() {
        assert_eq!(lua_pattern_to_regex("a%.b"), "a\\.b");
        assert_eq!(lua_pattern_to_regex("100%%"), "100%");
    }

    #[test]
    fn rewrites_lua_match_predicate() {
        let query = r#"((identifier) @constant (#lua-match? @constant "^[A-Z][A-Z%d_]*$"))"#;
        let expected = r#"((identifier) @constant (#match? @constant "^[A-Z][A-Z0-9_]*$"))"#;
        assert_eq!(normalize(query), expected);
    }

    #[test]
    fn rewrites_not_lua_match_predicate() {
        let query = r#"(#not-lua-match? @v "^[A-Z]")"#;
        let expected = r#"(#not-match? @v "^[A-Z]")"#;
        assert_eq!(normalize(query), expected);
    }

    #[test]
    fn drops_set_directives_with_two_captures() {
        let query = r#"((link_destination) @markup.link.url (#set! @markup.link.url url @markup.link.url))"#;
        assert_eq!(normalize(query), "((link_destination) @markup.link.url )");
    }

    #[test]
    fn keeps_set_directives_tree_sitter_accepts() {
        let query =
            r#"((x) @a (#set! @a priority "@b (")) ((y) @c (#set! injection.language "c"))"#;
        assert_eq!(normalize(query), query);
    }

    #[test]
    fn translates_vim_ignore_case_flag() {
        let query = r#"((key) @k (#not-match? @k "\\c^Comments$"))"#;
        assert_eq!(
            normalize(query),
            r#"((key) @k (#not-match? @k "(?i)^Comments$"))"#
        );
    }

    #[test]
    fn translates_contains_into_match() {
        let query = r#"((macro) @m (#contains? @m "assert" "a.b"))"#;
        assert_eq!(
            normalize(query),
            r#"((macro) @m (#match? @m "assert|a\\.b"))"#
        );
    }

    #[test]
    fn strips_marker_captures_and_drops_colourless_patterns() {
        let query = "(string) @string @spell\n\
                     ((comment) @spell ; a (comment\n\
                       (#lua-match? @spell \"^//\")) @_outer\n\
                     ((identifier) @_id (#eq? @_id \"x\"))\n\
                     [(a) (b)]? @c";
        assert_eq!(
            normalize_highlights(query),
            "(string) @string \n\n\n[(a) (b)]? @c"
        );
    }

    #[test]
    fn keeps_patterns_with_standard_predicates() {
        let query = "((identifier) @c (#any-of? @c \"x\" \"y\") (#set! priority 90))\n\"(\" @p";
        assert_eq!(normalize(query), query);
    }

    #[test]
    fn leaves_queries_without_lua_match_untouched() {
        let query = "(identifier) @variable\n((symbol) @b (#eq? @b \"x\"))";
        assert_eq!(normalize(query), query);
    }
}
